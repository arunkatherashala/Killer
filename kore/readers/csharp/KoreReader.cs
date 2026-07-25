/*
 * KORE v2 Reader — C# (.NET 6+) — Zero Dependencies
 * ====================================================
 *
 * Usage:
 *   using Kore;
 *   var reader = KoreReader.Open("data.kore");
 *   Console.WriteLine(reader.Info());
 *   long[]   ids    = reader.ReadIntColumn("id");
 *   double[] prices = reader.ReadFloatColumn("price");
 *   string[] names  = reader.ReadStrColumn("name");
 *   bool[]   flags  = reader.ReadBoolColumn("active");
 */
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

namespace Kore
{
    public class KoreReader
    {
        // ── Constants ────────────────────────────────────────────
        const int HeaderSize = 64;
        const int KTypeInt = 1, KTypeFloat = 2, KTypeBool = 3, KTypeStr = 4, KTypeBytes = 5;
        const int CodecRaw = 0, CodecRLE = 1, CodecDelta = 2, CodecDictRLE = 3,
                  CodecBitpack = 4, CodecBDict = 5, CodecCDelta = 6,
                  CodecFOR = 7, CodecHuffDict = 8;
        const int RcScale = 4096, RcBot = 1 << 16;

        static readonly string[] TypeNames = { "?", "Int", "Float", "Bool", "Str", "Bytes" };

        // ── Schema ───────────────────────────────────────────────
        public record KColumn(string Name, int KType, bool Encrypted)
        {
            public string TypeName => KType >= 0 && KType < TypeNames.Length ? TypeNames[KType] : "?";
        }

        record ColMeta(ulong FileOffset, uint CompLen, int Codec, uint NullCount);

        // ── State ────────────────────────────────────────────────
        byte[] data;
        public int Version { get; private set; }
        public int NCols { get; private set; }
        public ulong NRows { get; private set; }
        public int NChunks { get; private set; }
        public uint ChunkSize { get; private set; }
        public ulong Created { get; private set; }
        public KColumn[] Columns { get; private set; }
        public string[] Dictionary { get; private set; }
        public uint[] ChunkNRows { get; private set; }
        ColMeta[][] colMeta;
        ulong[]? delBitmap;
        ulong delCount;

        // ── CRC32 ────────────────────────────────────────────────
        static readonly uint[] CrcTable = new uint[256];
        static KoreReader()
        {
            for (uint i = 0; i < 256; i++)
            {
                uint c = i;
                for (int j = 0; j < 8; j++) c = (c & 1) != 0 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
                CrcTable[i] = c;
            }
        }
        static uint Crc32(byte[] d, int off, int len)
        {
            uint crc = 0xFFFFFFFF;
            for (int i = off; i < off + len; i++) crc = CrcTable[(crc ^ d[i]) & 0xFF] ^ (crc >> 8);
            return crc ^ 0xFFFFFFFF;
        }

        // ── LE reads ─────────────────────────────────────────────
        static ushort Rd16(byte[] d, int o) => (ushort)(d[o] | (d[o + 1] << 8));
        static uint Rd32(byte[] d, int o) => (uint)(d[o] | (d[o+1]<<8) | (d[o+2]<<16) | (d[o+3]<<24));
        static ulong Rd64(byte[] d, int o) => Rd32(d, o) | ((ulong)Rd32(d, o + 4) << 32);

        // ── Varint ───────────────────────────────────────────────
        static (ulong val, int pos) ReadVarint(byte[] d, int pos)
        {
            ulong result = 0; int shift = 0;
            while (pos < d.Length)
            {
                byte b = d[pos++];
                result |= (ulong)(b & 0x7F) << shift;
                if ((b & 0x80) == 0) break;
                shift += 7;
            }
            return (result, pos);
        }

        static long ZigzagDecode(ulong v) => (long)((v >> 1) ^ (ulong)(-(long)(v & 1)));

        static (long val, int pos) ReadZvar(byte[] d, int pos)
        {
            var (v, p) = ReadVarint(d, pos);
            return (ZigzagDecode(v), p);
        }

        // ── LZ77 ─────────────────────────────────────────────────
        static byte[] Lz77Decompress(byte[] d, int off, int len)
        {
            var o = new List<byte>(len * 4);
            int i = off, end = off + len;
            while (i < end)
            {
                if (d[i] == 0xFF && i + 4 < end)
                {
                    int back = Rd16(d, i + 1), length = Rd16(d, i + 3); i += 5;
                    if (back == 0 && length == 1) o.Add(0xFF);
                    else { int start = o.Count - back; for (int j = 0; j < length; j++) o.Add(o[start + j]); }
                }
                else o.Add(d[i++]);
            }
            return o.ToArray();
        }

        // ── Huffman ──────────────────────────────────────────────
        static byte[] HuffmanDecompress(byte[] d, int off, int len)
        {
            if (len < 6) return Array.Empty<byte>();
            int pos = off;
            uint origLen = Rd32(d, pos); pos += 4;
            ushort nsyms = Rd16(d, pos); pos += 2;
            if (nsyms == 0) return Array.Empty<byte>();

            int[] symLens = new int[256];
            if (nsyms <= 64)
                for (int s = 0; s < nsyms && pos + 2 <= off + len; s++) { int sym = d[pos++]; symLens[sym] = d[pos++]; }
            else
            {
                if (pos + 256 > off + len) return Array.Empty<byte>();
                for (int s = 0; s < 256; s++) symLens[s] = d[pos + s]; pos += 256;
            }

            int maxBits = symLens.Max();
            if (maxBits == 0 || maxBits > 24) return Array.Empty<byte>();

            var sorted = Enumerable.Range(0, 256).Where(s => symLens[s] > 0)
                .Select(s => (sym: s, cl: symLens[s])).OrderBy(x => x.cl).ThenBy(x => x.sym).ToList();

            int tableSize = 1 << maxBits;
            int[] lookup = new int[tableSize]; int[] lookupLen = new int[tableSize];
            Array.Fill(lookup, -1);

            int code = 0, prevLen = 0;
            foreach (var sl in sorted)
            {
                if (sl.cl > prevLen) { code <<= (sl.cl - prevLen); prevLen = sl.cl; }
                int rev = 0;
                for (int b = 0; b < sl.cl; b++) if ((code & (1 << (sl.cl - 1 - b))) != 0) rev |= (1 << b);
                int fill = 1 << (maxBits - sl.cl);
                for (int f = 0; f < fill; f++) { int idx = (f << sl.cl) | rev; lookup[idx] = sl.sym; lookupLen[idx] = sl.cl; }
                code++;
            }

            byte[] o = new byte[origLen]; int oi = 0;
            ulong bitbuf = 0; int bitsIn = 0;
            while (oi < origLen && (pos < off + len || bitsIn >= maxBits))
            {
                while (bitsIn < 56 && pos < off + len) { bitbuf |= (ulong)d[pos++] << bitsIn; bitsIn += 8; }
                if (bitsIn < 1) break;
                int lookBits = Math.Min(bitsIn, maxBits);
                int idx = (int)(bitbuf & ((1UL << lookBits) - 1));
                if (idx < tableSize && lookup[idx] >= 0) { o[oi++] = (byte)lookup[idx]; int cl = lookupLen[idx]; bitbuf >>= cl; bitsIn -= cl; }
                else break;
            }
            return o.AsSpan(0, oi).ToArray();
        }

        // ── Range Coder ──────────────────────────────────────────
        static byte[] RangeDecompress(byte[] d, int off, int len)
        {
            if (len < 2) return Array.Empty<byte>();
            int p = off;
            int active = Rd16(d, p); p += 2;
            if (active == 0) return Array.Empty<byte>();

            int[] norm = new int[256];
            for (int i = 0; i < active && p < off + len; i++)
            {
                int sym = d[p++]; if (p + 1 >= off + len) break;
                norm[sym] = Rd16(d, p); p += 2;
            }
            if (p + 3 >= off + len) return Array.Empty<byte>();
            int origLen = (int)Rd32(d, p); p += 4;

            if (active == 1) { int sym = Array.FindIndex(norm, n => n > 0); byte[] r = new byte[origLen]; Array.Fill(r, (byte)sym); return r; }

            uint[] cdf = new uint[257];
            for (int i = 0; i < 256; i++) cdf[i + 1] = cdf[i] + (uint)norm[i];
            byte[] symLookup = new byte[RcScale];
            for (int i = 0; i < 256; i++) if (norm[i] > 0) for (uint j = cdf[i]; j < cdf[i + 1] && j < RcScale; j++) symLookup[j] = (byte)i;

            uint codeVal = 0;
            for (int i = 0; i < 4; i++) { codeVal = (codeVal << 8) | (p < off + len ? d[p++] : (byte)0); }

            uint low = 0, rng = 0xFFFFFFFF;
            byte[] o = new byte[origLen];
            for (int i = 0; i < origLen; i++)
            {
                uint rv = rng / (uint)RcScale;
                uint offset = (codeVal - low) / rv;
                if (offset >= RcScale) offset = (uint)(RcScale - 1);
                byte s = symLookup[offset]; int si = s;
                low += rv * cdf[si];
                rng = cdf[si + 1] - cdf[si] < RcScale ? rv * (cdf[si + 1] - cdf[si]) : rng - rv * cdf[si];
                while (rng < RcBot) { low <<= 8; rng <<= 8; codeVal = (codeVal << 8) | (p < off + len ? d[p++] : (byte)0); }
                o[i] = s;
            }
            return o;
        }

        // ── Block Decompress ─────────────────────────────────────
        static byte[] DecompressBlock(byte[] d, int off, int len)
        {
            if (len == 0) return Array.Empty<byte>();
            int tag = d[off];
            int poff = off + 1, plen = len - 1;
            return tag switch
            {
                0x02 => d.AsSpan(poff, plen).ToArray(),
                0x00 => Lz77Decompress(d, poff, plen),
                0x01 => Lz77Decompress(HuffmanDecompress(d, poff, plen), 0, HuffmanDecompress(d, poff, plen).Length),
                0x03 => HuffmanDecompress(d, poff, plen),
                0x04 => RangeDecompress(d, poff, plen),
                0x05 => Lz77Decompress(RangeDecompress(d, poff, plen), 0, RangeDecompress(d, poff, plen).Length),
                _    => Lz77Decompress(d, poff, plen),
            };
        }

        // Fix: avoid double decompress in switch
        static byte[] DecompressBlockSafe(byte[] d, int off, int len)
        {
            if (len == 0) return Array.Empty<byte>();
            int tag = d[off]; int poff = off + 1, plen = len - 1;
            switch (tag)
            {
                case 0x02: return d.AsSpan(poff, plen).ToArray();
                case 0x00: return Lz77Decompress(d, poff, plen);
                case 0x01: { var h = HuffmanDecompress(d, poff, plen); return Lz77Decompress(h, 0, h.Length); }
                case 0x03: return HuffmanDecompress(d, poff, plen);
                case 0x04: return RangeDecompress(d, poff, plen);
                case 0x05: { var r = RangeDecompress(d, poff, plen); return Lz77Decompress(r, 0, r.Length); }
                default:   return Lz77Decompress(d, poff, plen);
            }
        }

        // ── Codec Decoders ───────────────────────────────────────
        static long[] DecodeDeltaInt(byte[] d, int nrows)
        {
            long[] o = new long[nrows]; int pos = 0;
            (long acc, pos) = ReadZvar(d, pos); o[0] = acc;
            for (int i = 1; i < nrows; i++) { long delta; (delta, pos) = ReadZvar(d, pos); acc += delta; o[i] = acc; }
            return o;
        }

        static long[] DecodeRleInt(byte[] d, int nrows)
        {
            var o = new List<long>(nrows); int pos = 0;
            while (o.Count < nrows && pos < d.Length)
            {
                var (count, p1) = ReadVarint(d, pos); pos = p1;
                var (val, p2) = ReadZvar(d, pos); pos = p2;
                for (ulong c = 0; c < count && o.Count < nrows; c++) o.Add(val);
            }
            return o.ToArray();
        }

        static long[] DecodeCdelta(byte[] d, int nrows)
        {
            int pos = 0;
            var (bv, p1) = ReadZvar(d, pos); pos = p1;
            var (sv, _) = ReadZvar(d, pos);
            long[] o = new long[nrows];
            for (int i = 0; i < nrows; i++) o[i] = bv + sv * i;
            return o;
        }

        static long[] DecodeFOR(byte[] d, int nrows)
        {
            int pos = 0;
            var (minval, p) = ReadZvar(d, pos); pos = p;
            int bits = pos < d.Length ? d[pos] : 0; pos++;
            long[] o = new long[nrows];
            if (bits == 0) { Array.Fill(o, minval); return o; }
            ulong bitbuf = 0; int bitpos = 0;
            for (int i = 0; i < nrows; i++)
            {
                while (bitpos < bits && pos < d.Length) { bitbuf |= (ulong)d[pos++] << bitpos; bitpos += 8; }
                ulong mask = (1UL << bits) - 1;
                o[i] = minval + (long)(bitbuf & mask); bitbuf >>= bits; bitpos -= bits;
            }
            return o;
        }

        static bool[] DecodeBitpack(byte[] d, int nrows)
        {
            bool[] o = new bool[nrows];
            for (int i = 0; i < nrows; i++)
            {
                int byteIdx = i / 8, bitIdx = i % 8;
                if (byteIdx < d.Length) o[i] = ((d[byteIdx] >> bitIdx) & 1) == 1;
            }
            return o;
        }

        static string[] DecodeRleStr(byte[] d, int nrows)
        {
            var o = new List<string>(nrows); int pos = 0;
            while (o.Count < nrows && pos < d.Length)
            {
                var (count, p1) = ReadVarint(d, pos); pos = p1;
                var (slen, p2) = ReadVarint(d, pos); pos = p2;
                string s = Encoding.UTF8.GetString(d, pos, (int)slen); pos += (int)slen;
                for (ulong c = 0; c < count && o.Count < nrows; c++) o.Add(s);
            }
            return o.ToArray();
        }

        static string[] DecodeBdict(byte[] d, int nrows)
        {
            int pos = 0;
            var (nuniq, p) = ReadVarint(d, pos); pos = p;
            string[] dict = new string[nuniq];
            for (ulong i = 0; i < nuniq; i++)
            {
                var (slen, p2) = ReadVarint(d, pos); pos = p2;
                dict[i] = Encoding.UTF8.GetString(d, pos, (int)slen); pos += (int)slen;
            }
            int bits = pos < d.Length ? d[pos] : 0; pos++;
            string[] o = new string[nrows];
            if (bits == 0) { Array.Fill(o, nuniq > 0 ? dict[0] : ""); return o; }
            ulong bitbuf = 0; int bitpos = 0;
            for (int i = 0; i < nrows; i++)
            {
                while (bitpos < bits && pos < d.Length) { bitbuf |= (ulong)d[pos++] << bitpos; bitpos += 8; }
                ulong mask = (1UL << bits) - 1;
                ulong idx = bitbuf & mask; bitbuf >>= bits; bitpos -= bits;
                o[i] = idx < nuniq ? dict[idx] : "";
            }
            return o;
        }

        static string[] DecodeRawStr(byte[] d, int nrows)
        {
            string[] o = new string[nrows]; int pos = 0;
            for (int i = 0; i < nrows; i++)
            {
                if (pos >= d.Length) { o[i] = ""; continue; }
                var (slen, p) = ReadVarint(d, pos); pos = p;
                o[i] = Encoding.UTF8.GetString(d, pos, (int)slen); pos += (int)slen;
            }
            return o;
        }

        // ── Open / Parse ─────────────────────────────────────────
        public static KoreReader Open(string path)
        {
            var r = new KoreReader { data = File.ReadAllBytes(path) };
            r.Parse();
            r.LoadDeleteBitmap(path);
            return r;
        }

        public static KoreReader FromBytes(byte[] data)
        {
            var r = new KoreReader { data = data };
            r.Parse();
            return r;
        }

        void Parse()
        {
            if (data.Length < HeaderSize + 12) throw new InvalidDataException("Not a KORE file");
            if (data[0] != (byte)'K' || data[1] != (byte)'O' || data[2] != (byte)'R' || data[3] != (byte)'E')
                throw new InvalidDataException("Bad KORE magic");

            Version   = data[4];
            NCols     = Rd16(data, 6);
            NRows     = Rd64(data, 8);
            NChunks   = (int)Rd32(data, 16);
            ChunkSize = Rd32(data, 20);
            Created   = Rd64(data, 24);

            int pos = HeaderSize;
            int schemaCompLen = (int)Rd32(data, pos); pos += 4;
            byte[] schemaRaw = DecompressBlockSafe(data, pos, schemaCompLen); pos += schemaCompLen;

            Columns = new KColumn[NCols];
            int sp = 0;
            for (int i = 0; i < NCols; i++)
            {
                var (nlen, p1) = ReadVarint(schemaRaw, sp); sp = p1;
                string name = Encoding.UTF8.GetString(schemaRaw, sp, (int)nlen); sp += (int)nlen;
                int ktype = sp < schemaRaw.Length ? schemaRaw[sp] : 4; sp++;
                bool enc = sp < schemaRaw.Length && schemaRaw[sp] != 0; sp++;
                Columns[i] = new KColumn(name, ktype, enc);
            }

            int dictCompLen = (int)Rd32(data, pos); pos += 4;
            byte[] dictRaw = DecompressBlockSafe(data, pos, dictCompLen); pos += dictCompLen;
            int dp = 0;
            var (dc, dp2) = ReadVarint(dictRaw, dp); dp = dp2;
            Dictionary = new string[dc];
            for (ulong i = 0; i < dc; i++)
            {
                var (slen, p3) = ReadVarint(dictRaw, dp); dp = p3;
                Dictionary[i] = Encoding.UTF8.GetString(dictRaw, dp, (int)slen); dp += (int)slen;
            }

            int trailer = data.Length - 12;
            int footerCompLen = (int)Rd32(data, trailer);
            int footerOffset = (int)Rd64(data, trailer + 4);
            byte[] fr = DecompressBlockSafe(data, footerOffset, footerCompLen);

            int fp = 0; fp += 4; fp += 2;
            ChunkNRows = new uint[NChunks];
            for (int c = 0; c < NChunks; c++) { ChunkNRows[c] = Rd32(fr, fp); fp += 4; }

            colMeta = new ColMeta[NChunks][];
            for (int c = 0; c < NChunks; c++)
            {
                colMeta[c] = new ColMeta[NCols];
                for (int ci = 0; ci < NCols; ci++)
                {
                    ulong fileOff = Rd64(fr, fp); fp += 8;
                    uint compLen = Rd32(fr, fp); fp += 4;
                    int codec = fp < fr.Length ? fr[fp] : 0; fp++;
                    uint nullCount = Rd32(fr, fp); fp += 4;
                    long mn; (mn, fp) = ReadZvar(fr, fp);
                    long mx; (mx, fp) = ReadZvar(fr, fp);
                    ulong slen; (slen, fp) = ReadVarint(fr, fp); fp += (int)slen;
                    (slen, fp) = ReadVarint(fr, fp); fp += (int)slen;
                    fp += 512;
                    colMeta[c][ci] = new ColMeta(fileOff, compLen, codec, nullCount);
                }
            }
        }

        void LoadDeleteBitmap(string path)
        {
            string delPath = path + ".del";
            if (!File.Exists(delPath)) return;
            byte[] bd = File.ReadAllBytes(delPath);
            if (bd.Length < 16) return;
            ulong totalRows = Rd64(bd, 0);
            delCount = Rd64(bd, 8);
            int nwords = (int)((totalRows + 63) / 64);
            delBitmap = new ulong[nwords];
            for (int i = 0; i < nwords && 16 + i * 8 + 8 <= bd.Length; i++)
                delBitmap[i] = Rd64(bd, 16 + i * 8);
        }

        bool IsDeleted(ulong row)
        {
            if (delBitmap == null) return false;
            int word = (int)(row / 64);
            return word < delBitmap.Length && ((delBitmap[word] >> (int)(row % 64)) & 1) == 1;
        }

        byte[] DecodeChunkCol(int ci, int chunk)
        {
            var cm = colMeta[chunk][ci];
            int off = (int)cm.FileOffset;
            uint storedCrc = Rd32(data, off);
            int compLen = (int)Rd32(data, off + 4);
            uint actualCrc = Crc32(data, off + 8, compLen);
            if (actualCrc != storedCrc) throw new InvalidDataException($"CRC mismatch col {ci} chunk {chunk}");
            return DecompressBlockSafe(data, off + 8, compLen);
        }

        public int ColIndex(string name) => Array.FindIndex(Columns, c => c.Name == name);

        // ── Column readers ───────────────────────────────────────
        public long[] ReadIntColumn(string name) => ReadIntColumn(ColIndex(name));
        public long[] ReadIntColumn(int ci)
        {
            var vals = new List<long>();
            ulong globalRow = 0;
            for (int c = 0; c < NChunks; c++)
            {
                byte[] raw = DecodeChunkCol(ci, c);
                int nr = (int)ChunkNRows[c];
                long[] chunk = colMeta[c][ci].Codec switch
                {
                    CodecCDelta => DecodeCdelta(raw, nr),
                    CodecDelta  => DecodeDeltaInt(raw, nr),
                    CodecRLE    => DecodeRleInt(raw, nr),
                    CodecFOR    => DecodeFOR(raw, nr),
                    _           => DecodeDeltaInt(raw, nr),
                };
                foreach (var v in chunk) { if (!IsDeleted(globalRow++)) vals.Add(v); }
            }
            return vals.ToArray();
        }

        public double[] ReadFloatColumn(string name) => ReadFloatColumn(ColIndex(name));
        public double[] ReadFloatColumn(int ci)
        {
            var vals = new List<double>();
            ulong globalRow = 0;
            for (int c = 0; c < NChunks; c++)
            {
                byte[] raw = DecodeChunkCol(ci, c);
                int nr = (int)ChunkNRows[c];
                double scale = 10000.0;
                int dataStart = 0;
                if (raw.Length >= 2 && raw[0] == 0xFE)
                {
                    int se = raw[1];
                    double[] muls = {1,10,100,1000,10000};
                    scale = se <= 4 ? muls[se] : 10000;
                    dataStart = 2;
                }
                byte[] cd = raw.AsSpan(dataStart).ToArray();
                long[] chunk = colMeta[c][ci].Codec switch
                {
                    CodecCDelta => DecodeCdelta(cd, nr),
                    CodecDelta  => DecodeDeltaInt(cd, nr),
                    CodecRLE    => DecodeRleInt(cd, nr),
                    CodecFOR    => DecodeFOR(cd, nr),
                    _           => DecodeDeltaInt(cd, nr),
                };
                foreach (var v in chunk) { if (!IsDeleted(globalRow++)) vals.Add(v / scale); }
            }
            return vals.ToArray();
        }

        public bool[] ReadBoolColumn(string name) => ReadBoolColumn(ColIndex(name));
        public bool[] ReadBoolColumn(int ci)
        {
            var vals = new List<bool>();
            ulong globalRow = 0;
            for (int c = 0; c < NChunks; c++)
            {
                byte[] raw = DecodeChunkCol(ci, c);
                bool[] chunk = DecodeBitpack(raw, (int)ChunkNRows[c]);
                foreach (var v in chunk) { if (!IsDeleted(globalRow++)) vals.Add(v); }
            }
            return vals.ToArray();
        }

        public string[] ReadStrColumn(string name) => ReadStrColumn(ColIndex(name));
        public string[] ReadStrColumn(int ci)
        {
            var vals = new List<string>();
            ulong globalRow = 0;
            for (int c = 0; c < NChunks; c++)
            {
                byte[] raw = DecodeChunkCol(ci, c);
                int nr = (int)ChunkNRows[c];
                string[] chunk = colMeta[c][ci].Codec switch
                {
                    CodecRLE      => DecodeRleStr(raw, nr),
                    CodecBDict    => DecodeBdict(raw, nr),
                    CodecHuffDict => DecodeBdict(raw, nr),
                    _             => DecodeRawStr(raw, nr),
                };
                foreach (var v in chunk) { if (!IsDeleted(globalRow++)) vals.Add(v); }
            }
            return vals.ToArray();
        }

        public string Info()
        {
            string cols = string.Join(", ", Columns.Select(c => $"{c.Name}:{c.TypeName}"));
            return $"KORE v{Version} | {NRows:N0} rows × {NCols} cols | {NChunks} chunks | {data.Length:N0} bytes | [{cols}]";
        }

        // ── CLI ──────────────────────────────────────────────────
        static void Main(string[] args)
        {
            string path = args.Length > 0 ? args[0] : "../../test/test_v2.kore";
            var r = Open(path);
            Console.WriteLine(r.Info());
            foreach (var col in r.Columns)
            {
                Console.WriteLine($"\n{col.Name} ({col.TypeName}):");
                switch (col.KType)
                {
                    case KTypeInt:
                        foreach (var (v, i) in r.ReadIntColumn(col.Name).Take(10).Select((v, i) => (v, i)))
                            Console.WriteLine($"  [{i}] {v}"); break;
                    case KTypeFloat:
                        foreach (var (v, i) in r.ReadFloatColumn(col.Name).Take(10).Select((v, i) => (v, i)))
                            Console.WriteLine($"  [{i}] {v:F4}"); break;
                    case KTypeBool:
                        foreach (var (v, i) in r.ReadBoolColumn(col.Name).Take(10).Select((v, i) => (v, i)))
                            Console.WriteLine($"  [{i}] {v}"); break;
                    case KTypeStr:
                        foreach (var (v, i) in r.ReadStrColumn(col.Name).Take(10).Select((v, i) => (v, i)))
                            Console.WriteLine($"  [{i}] \"{v}\""); break;
                }
            }
            Console.WriteLine("\nDONE");
        }
    }
}
