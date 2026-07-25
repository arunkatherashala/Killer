/*
 * KORE v2 Reader — Pure Java (JDK 11+) — Zero Dependencies
 * ==========================================================
 *
 * Usage:
 *   KoreReader reader = KoreReader.open("data.kore");
 *   System.out.println(reader.info());
 *   long[] ids = reader.readIntColumn("id");
 *   double[] prices = reader.readFloatColumn("price");
 *   String[] names = reader.readStrColumn("name");
 *   boolean[] flags = reader.readBoolColumn("active");
 *   reader.close();
 */
package kore;

import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.util.*;

public class KoreReader implements AutoCloseable {

    /* ── Constants ─────────────────────────────────────────────── */
    private static final int MAGIC = 0x45524F4B; // "KORE" LE
    private static final int HEADER_SIZE = 64;
    private static final int KTYPE_INT = 1, KTYPE_FLOAT = 2, KTYPE_BOOL = 3,
                             KTYPE_STR = 4, KTYPE_BYTES = 5;
    private static final int CODEC_RAW = 0, CODEC_RLE = 1, CODEC_DELTA = 2,
                             CODEC_DICTRLE = 3, CODEC_BITPACK = 4,
                             CODEC_BDICT = 5, CODEC_CDELTA = 6,
                             CODEC_FOR = 7, CODEC_HUFFDICT = 8;
    private static final int RC_SCALE = 4096, RC_BOT = 1 << 16;

    /* ── Schema ────────────────────────────────────────────────── */
    public static class Column {
        public final String name;
        public final int ktype;
        public final boolean encrypted;
        Column(String name, int ktype, boolean encrypted) {
            this.name = name; this.ktype = ktype; this.encrypted = encrypted;
        }
        public String typeName() {
            switch (ktype) {
                case 1: return "Int"; case 2: return "Float"; case 3: return "Bool";
                case 4: return "Str"; case 5: return "Bytes"; default: return "?";
            }
        }
    }

    private static class ColMeta {
        long fileOffset; int compLen; int codec; int nullCount;
        long minI64, maxI64;
    }

    /* ── State ─────────────────────────────────────────────────── */
    private byte[] data;
    public int version;
    public int ncols;
    public long nrows;
    public int nchunks;
    public int chunkSize;
    public long created;
    public Column[] columns;
    public String[] dictionary;
    public int[] chunkNrows;
    public ColMeta[][] colMeta; // [chunk][col]
    private long[] delBitmap;
    private long delCount;

    /* ── CRC32 ─────────────────────────────────────────────────── */
    private static final int[] CRC_TABLE = new int[256];
    static {
        for (int i = 0; i < 256; i++) {
            int c = i;
            for (int j = 0; j < 8; j++)
                c = ((c & 1) != 0) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
            CRC_TABLE[i] = c;
        }
    }
    private static int crc32(byte[] d, int off, int len) {
        int crc = 0xFFFFFFFF;
        for (int i = off; i < off + len; i++)
            crc = CRC_TABLE[(crc ^ d[i]) & 0xFF] ^ (crc >>> 8);
        return crc ^ 0xFFFFFFFF;
    }

    /* ── Little-endian reads ───────────────────────────────────── */
    private static int rd16(byte[] d, int o) { return (d[o]&0xFF)|((d[o+1]&0xFF)<<8); }
    private static int rd32(byte[] d, int o) { return (d[o]&0xFF)|((d[o+1]&0xFF)<<8)|((d[o+2]&0xFF)<<16)|((d[o+3]&0xFF)<<24); }
    private static long rd64(byte[] d, int o) { return (rd32(d,o)&0xFFFFFFFFL)|((rd32(d,o+4)&0xFFFFFFFFL)<<32); }

    /* ── Varint ────────────────────────────────────────────────── */
    private static int[] readVarint(byte[] d, int pos) {
        long result = 0; int shift = 0;
        while (pos < d.length) {
            int b = d[pos++] & 0xFF;
            result |= (long)(b & 0x7F) << shift;
            if ((b & 0x80) == 0) break;
            shift += 7;
        }
        return new int[]{(int)result, pos};
    }
    private static long[] readVarintLong(byte[] d, int pos) {
        long result = 0; int shift = 0;
        while (pos < d.length) {
            int b = d[pos++] & 0xFF;
            result |= (long)(b & 0x7F) << shift;
            if ((b & 0x80) == 0) break;
            shift += 7;
        }
        return new long[]{result, pos};
    }
    private static long zigzagDecode(long v) { return (v >>> 1) ^ -(v & 1); }
    private static long[] readZvar(byte[] d, int pos) {
        long[] vp = readVarintLong(d, pos);
        return new long[]{zigzagDecode(vp[0]), vp[1]};
    }

    /* ── LZ77 ──────────────────────────────────────────────────── */
    private static byte[] lz77Decompress(byte[] d, int off, int len) {
        // Use ArrayList<Byte> for overlapping copy support
        java.util.ArrayList<Byte> out = new java.util.ArrayList<>(len * 4);
        int i = off, end = off + len;
        while (i < end) {
            if ((d[i] & 0xFF) == 0xFF && i + 4 < end) {
                int back = rd16(d, i + 1);
                int length = rd16(d, i + 3);
                i += 5;
                if (back == 0 && length == 1) { out.add((byte) 0xFF); }
                else {
                    int start = out.size() - back;
                    for (int j = 0; j < length; j++) out.add(out.get(start + j));
                }
            } else {
                out.add(d[i++]);
            }
        }
        byte[] result = new byte[out.size()];
        for (int k = 0; k < out.size(); k++) result[k] = out.get(k);
        return result;
    }

    /* ── Huffman ───────────────────────────────────────────────── */
    private static byte[] huffmanDecompress(byte[] d, int off, int len) {
        if (len < 6) return new byte[0];
        int pos = off;
        int origLen = rd32(d, pos); pos += 4;
        int nsyms = rd16(d, pos); pos += 2;
        if (nsyms == 0) return new byte[0];

        int[] symLens = new int[256];
        if (nsyms <= 64) {
            for (int s = 0; s < nsyms && pos + 2 <= off + len; s++) {
                int sym = d[pos++] & 0xFF;
                symLens[sym] = d[pos++] & 0xFF;
            }
        } else {
            if (pos + 256 > off + len) return new byte[0];
            for (int s = 0; s < 256; s++) symLens[s] = d[pos + s] & 0xFF;
            pos += 256;
        }

        int maxBits = 0;
        for (int s = 0; s < 256; s++) if (symLens[s] > maxBits) maxBits = symLens[s];
        if (maxBits == 0 || maxBits > 24) return new byte[0];

        // Sort (len, sym) pairs
        List<int[]> sorted = new ArrayList<>();
        for (int s = 0; s < 256; s++)
            if (symLens[s] > 0) sorted.add(new int[]{s, symLens[s]});
        sorted.sort((a, b) -> a[1] != b[1] ? a[1] - b[1] : a[0] - b[0]);

        // Build lookup table
        int tableSize = 1 << maxBits;
        int[] lookup = new int[tableSize];
        int[] lookupLen = new int[tableSize];
        Arrays.fill(lookup, -1);

        int code = 0, prevLen = 0;
        for (int[] sl : sorted) {
            int cl = sl[1];
            if (cl > prevLen) { code <<= (cl - prevLen); prevLen = cl; }
            int rev = 0;
            for (int b = 0; b < cl; b++) if ((code & (1 << (cl - 1 - b))) != 0) rev |= (1 << b);
            int fill = 1 << (maxBits - cl);
            for (int f = 0; f < fill; f++) {
                int idx = (f << cl) | rev;
                lookup[idx] = sl[0];
                lookupLen[idx] = cl;
            }
            code++;
        }

        byte[] out = new byte[origLen];
        int oi = 0;
        long bitbuf = 0; int bitsIn = 0;
        while (oi < origLen && (pos < off + len || bitsIn >= maxBits)) {
            while (bitsIn < 56 && pos < off + len) {
                bitbuf |= (long)(d[pos++] & 0xFF) << bitsIn; bitsIn += 8;
            }
            if (bitsIn < 1) break;
            int lookBits = Math.min(bitsIn, maxBits);
            int idx = (int)(bitbuf & ((1L << lookBits) - 1));
            if (idx < tableSize && lookup[idx] >= 0) {
                out[oi++] = (byte)lookup[idx];
                int cl = lookupLen[idx];
                bitbuf >>>= cl; bitsIn -= cl;
            } else break;
        }
        return Arrays.copyOf(out, oi);
    }

    /* ── Range Coder ───────────────────────────────────────────── */
    private static byte[] rangeDecompress(byte[] d, int off, int len) {
        if (len < 2) return new byte[0];
        int p = off;
        int active = rd16(d, p); p += 2;
        if (active == 0) return new byte[0];

        int[] norm = new int[256];
        for (int i = 0; i < active && p < off + len; i++) {
            int sym = d[p++] & 0xFF;
            if (p + 1 >= off + len) break;
            norm[sym] = rd16(d, p); p += 2;
        }
        if (p + 3 >= off + len) return new byte[0];
        int origLen = rd32(d, p); p += 4;

        if (active == 1) {
            int sym = 0;
            for (int i = 0; i < 256; i++) if (norm[i] > 0) { sym = i; break; }
            byte[] out = new byte[origLen];
            Arrays.fill(out, (byte)sym);
            return out;
        }

        int[] cdf = new int[257]; cdf[0] = 0;
        for (int i = 0; i < 256; i++) cdf[i + 1] = cdf[i] + norm[i];
        int[] symLookup = new int[RC_SCALE];
        for (int i = 0; i < 256; i++)
            if (norm[i] > 0) for (int j = cdf[i]; j < cdf[i + 1] && j < RC_SCALE; j++)
                symLookup[j] = i;

        long codeVal = 0;
        for (int i = 0; i < 4; i++)
            codeVal = ((codeVal << 8) | (p < off + len ? (d[p++] & 0xFF) : 0)) & 0xFFFFFFFFL;

        long low = 0, rng = 0xFFFFFFFFL;
        byte[] out = new byte[origLen];
        for (int i = 0; i < origLen; i++) {
            long r = Long.divideUnsigned(rng, RC_SCALE);
            long offset = Long.divideUnsigned((codeVal - low) & 0xFFFFFFFFL, r);
            if (offset >= RC_SCALE) offset = RC_SCALE - 1;
            int sym = symLookup[(int)offset];
            low = (low + r * cdf[sym]) & 0xFFFFFFFFL;
            if (cdf[sym + 1] - cdf[sym] < RC_SCALE) rng = r * (cdf[sym + 1] - cdf[sym]);
            else rng = rng - r * cdf[sym];
            while (Long.compareUnsigned(rng, RC_BOT) < 0) {
                low = (low << 8) & 0xFFFFFFFFL;
                rng = (rng << 8) & 0xFFFFFFFFL;
                codeVal = ((codeVal << 8) | (p < off + len ? (d[p++] & 0xFF) : 0)) & 0xFFFFFFFFL;
            }
            out[i] = (byte)sym;
        }
        return out;
    }

    /* ── Block Decompress ──────────────────────────────────────── */
    private static byte[] decompressBlock(byte[] d, int off, int len) {
        if (len == 0) return new byte[0];
        int tag = d[off] & 0xFF;
        int poff = off + 1, plen = len - 1;
        switch (tag) {
            case 0x02: return Arrays.copyOfRange(d, poff, poff + plen);
            case 0x00: return lz77Decompress(d, poff, plen);
            case 0x01: { byte[] h = huffmanDecompress(d, poff, plen); return lz77Decompress(h, 0, h.length); }
            case 0x03: return huffmanDecompress(d, poff, plen);
            case 0x04: return rangeDecompress(d, poff, plen);
            case 0x05: { byte[] r = rangeDecompress(d, poff, plen); return lz77Decompress(r, 0, r.length); }
            default:   return lz77Decompress(d, poff, plen);
        }
    }

    /* ── Codec Decoders ────────────────────────────────────────── */
    private static long[] decodeDeltaInt(byte[] d, int nrows) {
        long[] out = new long[nrows];
        int pos = 0; long[] vp = readZvar(d, pos);
        long acc = vp[0]; pos = (int)vp[1]; out[0] = acc;
        for (int i = 1; i < nrows; i++) {
            vp = readZvar(d, pos); pos = (int)vp[1];
            acc += vp[0]; out[i] = acc;
        }
        return out;
    }

    private static long[] decodeRleInt(byte[] d, int nrows) {
        long[] out = new long[nrows];
        int pos = 0, idx = 0;
        while (idx < nrows && pos < d.length) {
            int[] cp = readVarint(d, pos); int count = cp[0]; pos = cp[1];
            long[] vp = readZvar(d, pos); long val = vp[0]; pos = (int)vp[1];
            for (int c = 0; c < count && idx < nrows; c++) out[idx++] = val;
        }
        return out;
    }

    private static long[] decodeCdelta(byte[] d, int nrows) {
        long[] out = new long[nrows];
        int pos = 0;
        long[] vp = readZvar(d, pos); long base = vp[0]; pos = (int)vp[1];
        vp = readZvar(d, pos); long step = vp[0];
        for (int i = 0; i < nrows; i++) out[i] = base + step * i;
        return out;
    }

    private static long[] decodeFor(byte[] d, int nrows) {
        long[] out = new long[nrows];
        int pos = 0;
        long[] vp = readZvar(d, pos); long minval = vp[0]; pos = (int)vp[1];
        int bits = (pos < d.length) ? (d[pos] & 0xFF) : 0; pos++;
        if (bits == 0) { Arrays.fill(out, minval); return out; }
        long bitbuf = 0; int bitpos = 0;
        for (int i = 0; i < nrows; i++) {
            while (bitpos < bits && pos < d.length) {
                bitbuf |= (long)(d[pos++] & 0xFF) << bitpos; bitpos += 8;
            }
            long mask = (1L << bits) - 1;
            out[i] = minval + (bitbuf & mask);
            bitbuf >>>= bits; bitpos -= bits;
        }
        return out;
    }

    private static boolean[] decodeBitpack(byte[] d, int nrows) {
        boolean[] out = new boolean[nrows];
        for (int i = 0; i < nrows; i++) {
            int byteIdx = i / 8, bitIdx = i % 8;
            if (byteIdx < d.length) out[i] = ((d[byteIdx] >> bitIdx) & 1) == 1;
        }
        return out;
    }

    private static String[] decodeRleStr(byte[] d, int nrows) {
        String[] out = new String[nrows];
        int pos = 0, idx = 0;
        while (idx < nrows && pos < d.length) {
            int[] cp = readVarint(d, pos); int count = cp[0]; pos = cp[1];
            cp = readVarint(d, pos); int slen = cp[0]; pos = cp[1];
            String s = new String(d, pos, slen, java.nio.charset.StandardCharsets.UTF_8); pos += slen;
            for (int c = 0; c < count && idx < nrows; c++) out[idx++] = s;
        }
        return out;
    }

    private static String[] decodeBdict(byte[] d, int nrows) {
        String[] out = new String[nrows];
        int pos = 0;
        int[] np = readVarint(d, pos); int nuniq = np[0]; pos = np[1];
        String[] dict = new String[nuniq];
        for (int i = 0; i < nuniq; i++) {
            np = readVarint(d, pos); int slen = np[0]; pos = np[1];
            dict[i] = new String(d, pos, slen, java.nio.charset.StandardCharsets.UTF_8); pos += slen;
        }
        int bits = (pos < d.length) ? (d[pos] & 0xFF) : 0; pos++;
        if (bits == 0) { Arrays.fill(out, nuniq > 0 ? dict[0] : ""); return out; }
        long bitbuf = 0; int bitpos = 0;
        for (int i = 0; i < nrows; i++) {
            while (bitpos < bits && pos < d.length) {
                bitbuf |= (long)(d[pos++] & 0xFF) << bitpos; bitpos += 8;
            }
            long mask = (1L << bits) - 1;
            int idx = (int)(bitbuf & mask);
            bitbuf >>>= bits; bitpos -= bits;
            out[i] = (idx < nuniq) ? dict[idx] : "";
        }
        return out;
    }

    private static String[] decodeRawStr(byte[] d, int nrows) {
        String[] out = new String[nrows];
        int pos = 0;
        for (int i = 0; i < nrows; i++) {
            if (pos >= d.length) { out[i] = ""; continue; }
            int[] np = readVarint(d, pos); int slen = np[0]; pos = np[1];
            out[i] = new String(d, pos, slen, java.nio.charset.StandardCharsets.UTF_8); pos += slen;
        }
        return out;
    }

    /* ── Core parsing ──────────────────────────────────────────── */
    public static KoreReader open(String path) throws IOException {
        KoreReader r = new KoreReader();
        r.data = Files.readAllBytes(Path.of(path));
        r.parse();
        r.loadDeleteBitmap(path);
        return r;
    }

    public static KoreReader fromBytes(byte[] data) {
        KoreReader r = new KoreReader();
        r.data = data;
        r.parse();
        return r;
    }

    private void parse() {
        if (data.length < HEADER_SIZE + 12)
            throw new IllegalArgumentException("Not a valid KORE file");
        if (data[0] != 'K' || data[1] != 'O' || data[2] != 'R' || data[3] != 'E')
            throw new IllegalArgumentException("Bad KORE magic");

        version   = data[4] & 0xFF;
        ncols     = rd16(data, 6);
        nrows     = rd64(data, 8);
        nchunks   = rd32(data, 16);
        chunkSize = rd32(data, 20);
        created   = rd64(data, 24);

        int pos = HEADER_SIZE;
        int schemaCompLen = rd32(data, pos); pos += 4;
        byte[] schemaRaw = decompressBlock(data, pos, schemaCompLen); pos += schemaCompLen;

        columns = new Column[ncols];
        int sp = 0;
        for (int i = 0; i < ncols; i++) {
            int[] np = readVarint(schemaRaw, sp); int nlen = np[0]; sp = np[1];
            String name = new String(schemaRaw, sp, nlen, java.nio.charset.StandardCharsets.UTF_8); sp += nlen;
            int ktype = (sp < schemaRaw.length) ? (schemaRaw[sp] & 0xFF) : 4; sp++;
            boolean enc = (sp < schemaRaw.length) ? (schemaRaw[sp] != 0) : false; sp++;
            columns[i] = new Column(name, ktype, enc);
        }

        int dictCompLen = rd32(data, pos); pos += 4;
        byte[] dictRaw = decompressBlock(data, pos, dictCompLen); pos += dictCompLen;
        int dp = 0;
        long[] dvp = readVarintLong(dictRaw, dp); int dc = (int)dvp[0]; dp = (int)dvp[1];
        dictionary = new String[dc];
        for (int i = 0; i < dc; i++) {
            int[] np = readVarint(dictRaw, dp); int slen = np[0]; dp = np[1];
            dictionary[i] = new String(dictRaw, dp, slen, java.nio.charset.StandardCharsets.UTF_8); dp += slen;
        }

        int trailer = data.length - 12;
        int footerCompLen = rd32(data, trailer);
        long footerOffset = rd64(data, trailer + 4);
        byte[] fr = decompressBlock(data, (int)footerOffset, footerCompLen);

        int fp = 0;
        /* int ftNchunks = */ rd32(fr, fp); fp += 4;
        /* int ftNcols = */ rd16(fr, fp); fp += 2;
        chunkNrows = new int[nchunks];
        for (int c = 0; c < nchunks; c++) { chunkNrows[c] = rd32(fr, fp); fp += 4; }

        colMeta = new ColMeta[nchunks][ncols];
        for (int c = 0; c < nchunks; c++) {
            for (int ci = 0; ci < ncols; ci++) {
                ColMeta cm = new ColMeta();
                cm.fileOffset = rd64(fr, fp); fp += 8;
                cm.compLen    = rd32(fr, fp); fp += 4;
                cm.codec      = (fp < fr.length) ? (fr[fp] & 0xFF) : 0; fp++;
                cm.nullCount  = rd32(fr, fp); fp += 4;
                long[] zv = readZvar(fr, fp); cm.minI64 = zv[0]; fp = (int)zv[1];
                zv = readZvar(fr, fp); cm.maxI64 = zv[0]; fp = (int)zv[1];
                long[] sv = readVarintLong(fr, fp); fp = (int)sv[1] + (int)sv[0]; // skip min_str
                sv = readVarintLong(fr, fp); fp = (int)sv[1] + (int)sv[0]; // skip max_str
                fp += 512; // bloom filter
                colMeta[c][ci] = cm;
            }
        }
    }

    private void loadDeleteBitmap(String path) {
        try {
            byte[] bd = Files.readAllBytes(Path.of(path + ".del"));
            if (bd.length < 16) return;
            long totalRows = rd64(bd, 0);
            delCount = rd64(bd, 8);
            int nwords = (int)((totalRows + 63) / 64);
            delBitmap = new long[nwords];
            for (int i = 0; i < nwords && 16 + i * 8 + 8 <= bd.length; i++)
                delBitmap[i] = rd64(bd, 16 + i * 8);
        } catch (IOException | ArrayIndexOutOfBoundsException e) {
            // No delete bitmap
        }
    }

    private boolean isDeleted(long row) {
        if (delBitmap == null) return false;
        int word = (int)(row / 64);
        return word < delBitmap.length && ((delBitmap[word] >>> (row % 64)) & 1) == 1;
    }

    /* ── Column reading ────────────────────────────────────────── */
    private byte[] decodeChunkCol(int ci, int chunkIdx) {
        ColMeta cm = colMeta[chunkIdx][ci];
        int off = (int)cm.fileOffset;
        /* CRC32 (4) + comp_len (4) + data */
        int storedCrc = rd32(data, off);
        int compLen = rd32(data, off + 4);
        int actualCrc = crc32(data, off + 8, compLen);
        if (actualCrc != storedCrc)
            throw new RuntimeException("CRC mismatch col " + ci + " chunk " + chunkIdx);
        return decompressBlock(data, off + 8, compLen);
    }

    public int colIndex(String name) {
        for (int i = 0; i < ncols; i++) if (columns[i].name.equals(name)) return i;
        return -1;
    }

    public long[] readIntColumn(String name) { return readIntColumn(colIndex(name)); }
    public long[] readIntColumn(int ci) {
        List<Long> vals = new ArrayList<>();
        long globalRow = 0;
        for (int c = 0; c < nchunks; c++) {
            byte[] raw = decodeChunkCol(ci, c);
            int nr = chunkNrows[c];
            long[] chunk;
            switch (colMeta[c][ci].codec) {
                case CODEC_CDELTA: chunk = decodeCdelta(raw, nr); break;
                case CODEC_DELTA:  chunk = decodeDeltaInt(raw, nr); break;
                case CODEC_RLE:    chunk = decodeRleInt(raw, nr); break;
                case CODEC_FOR:    chunk = decodeFor(raw, nr); break;
                default:           chunk = decodeDeltaInt(raw, nr); break;
            }
            for (long v : chunk) { if (!isDeleted(globalRow++)) vals.add(v); }
        }
        return vals.stream().mapToLong(Long::longValue).toArray();
    }

    public double[] readFloatColumn(String name) { return readFloatColumn(colIndex(name)); }
    public double[] readFloatColumn(int ci) {
        List<Double> vals = new ArrayList<>();
        long globalRow = 0;
        for (int c = 0; c < nchunks; c++) {
            byte[] raw = decodeChunkCol(ci, c);
            int nr = chunkNrows[c];
            double scale = 10000.0;
            int dataStart = 0;
            if (raw.length >= 2 && (raw[0] & 0xFF) == 0xFE) {
                int se = raw[1] & 0xFF;
                double[] muls = {1, 10, 100, 1000, 10000};
                scale = se <= 4 ? muls[se] : 10000;
                dataStart = 2;
            }
            byte[] codecData = Arrays.copyOfRange(raw, dataStart, raw.length);
            long[] chunk;
            switch (colMeta[c][ci].codec) {
                case CODEC_CDELTA: chunk = decodeCdelta(codecData, nr); break;
                case CODEC_DELTA:  chunk = decodeDeltaInt(codecData, nr); break;
                case CODEC_RLE:    chunk = decodeRleInt(codecData, nr); break;
                case CODEC_FOR:    chunk = decodeFor(codecData, nr); break;
                default:           chunk = decodeDeltaInt(codecData, nr); break;
            }
            for (long v : chunk) { if (!isDeleted(globalRow++)) vals.add(v / scale); }
        }
        return vals.stream().mapToDouble(Double::doubleValue).toArray();
    }

    public boolean[] readBoolColumn(String name) { return readBoolColumn(colIndex(name)); }
    public boolean[] readBoolColumn(int ci) {
        List<Boolean> vals = new ArrayList<>();
        long globalRow = 0;
        for (int c = 0; c < nchunks; c++) {
            byte[] raw = decodeChunkCol(ci, c);
            int nr = chunkNrows[c];
            boolean[] chunk = decodeBitpack(raw, nr);
            for (boolean v : chunk) { if (!isDeleted(globalRow++)) vals.add(v); }
        }
        boolean[] out = new boolean[vals.size()];
        for (int i = 0; i < out.length; i++) out[i] = vals.get(i);
        return out;
    }

    public String[] readStrColumn(String name) { return readStrColumn(colIndex(name)); }
    public String[] readStrColumn(int ci) {
        List<String> vals = new ArrayList<>();
        long globalRow = 0;
        for (int c = 0; c < nchunks; c++) {
            byte[] raw = decodeChunkCol(ci, c);
            int nr = chunkNrows[c];
            String[] chunk;
            switch (colMeta[c][ci].codec) {
                case CODEC_RLE:      chunk = decodeRleStr(raw, nr); break;
                case CODEC_BDICT:    chunk = decodeBdict(raw, nr); break;
                case CODEC_HUFFDICT: chunk = decodeBdict(raw, nr); break;
                default:             chunk = decodeRawStr(raw, nr); break;
            }
            for (String v : chunk) { if (!isDeleted(globalRow++)) vals.add(v); }
        }
        return vals.toArray(new String[0]);
    }

    public String info() {
        StringBuilder sb = new StringBuilder();
        sb.append("KORE v").append(version).append(" | ")
          .append(nrows).append(" rows × ").append(ncols).append(" cols | ")
          .append(nchunks).append(" chunks | ").append(data.length).append(" bytes | [");
        for (int i = 0; i < ncols; i++) {
            if (i > 0) sb.append(", ");
            sb.append(columns[i].name).append(":").append(columns[i].typeName());
        }
        sb.append("]");
        return sb.toString();
    }

    @Override
    public void close() { data = null; }

    /* ── CLI ───────────────────────────────────────────────────── */
    public static void main(String[] args) throws Exception {
        String path = args.length > 0 ? args[0] : "../../test/test_v2.kore";
        KoreReader r = KoreReader.open(path);
        System.out.println(r.info());
        for (Column col : r.columns) {
            System.out.printf("\n%s (%s):%n", col.name, col.typeName());
            switch (col.ktype) {
                case KTYPE_INT: {
                    long[] vals = r.readIntColumn(col.name);
                    for (int i = 0; i < Math.min(vals.length, 10); i++)
                        System.out.printf("  [%d] %d%n", i, vals[i]);
                    break;
                }
                case KTYPE_FLOAT: {
                    double[] vals = r.readFloatColumn(col.name);
                    for (int i = 0; i < Math.min(vals.length, 10); i++)
                        System.out.printf("  [%d] %.4f%n", i, vals[i]);
                    break;
                }
                case KTYPE_BOOL: {
                    boolean[] vals = r.readBoolColumn(col.name);
                    for (int i = 0; i < Math.min(vals.length, 10); i++)
                        System.out.printf("  [%d] %s%n", i, vals[i]);
                    break;
                }
                case KTYPE_STR: {
                    String[] vals = r.readStrColumn(col.name);
                    for (int i = 0; i < Math.min(vals.length, 10); i++)
                        System.out.printf("  [%d] \"%s\"%n", i, vals[i]);
                    break;
                }
            }
        }
        r.close();
        System.out.println("\nDONE");
    }
}
