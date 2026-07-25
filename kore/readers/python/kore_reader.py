"""
KORE v2 Reader — Pure Python (Gap #9: Multi-language support)
=============================================================

Read KORE v2 columnar files from Python. Zero external dependencies.

Usage:
    from kore_reader import KoreReader
    reader = KoreReader("data.kore")
    print(reader.info())
    col = reader.read_column("price")
    rows = reader.read_rows(0, 100)
    df = reader.to_dict()  # {col_name: [values]}
"""
import struct
import os
from typing import List, Dict, Any, Optional, Tuple

# ── Constants ─────────────────────────────────────────────────────────────────
KORE_MAGIC = b"KORE"
KORE_V2 = 2
HEADER_SIZE = 64

# Column types
KTYPE_INT   = 1
KTYPE_FLOAT = 2
KTYPE_BOOL  = 3
KTYPE_STR   = 4
KTYPE_BYTES = 5

KTYPE_NAMES = {1: "Int", 2: "Float", 3: "Bool", 4: "Str", 5: "Bytes"}

# Codec IDs
CODEC_RAW     = 0
CODEC_RLE     = 1
CODEC_DELTA   = 2
CODEC_DICTRLE = 3
CODEC_BITPACK = 4
CODEC_BDICT   = 5
CODEC_CDELTA  = 6
CODEC_FOR     = 7
CODEC_HUFFDICT = 8
CODEC_DERIVED = 9

CODEC_NAMES = {0:"Raw",1:"RLE",2:"Delta",3:"DictRLE",4:"Bitpack",
               5:"BDict",6:"CDelta",7:"FOR",8:"HuffDict",9:"Derived"}


# ── Varint helpers ────────────────────────────────────────────────────────────
def read_varint(data: bytes, pos: int) -> Tuple[int, int]:
    """Read LEB128 unsigned varint. Returns (value, new_pos)."""
    result = 0
    shift = 0
    while pos < len(data):
        b = data[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        if b & 0x80 == 0:
            break
        shift += 7
    return result, pos

def zigzag_decode(v: int) -> int:
    """Decode zigzag-encoded signed integer."""
    return (v >> 1) ^ -(v & 1)

def read_zvar(data: bytes, pos: int) -> Tuple[int, int]:
    """Read zigzag-encoded signed varint."""
    v, p = read_varint(data, pos)
    return zigzag_decode(v), p


# ── CRC32 ─────────────────────────────────────────────────────────────────────
def _make_crc_table():
    table = []
    for i in range(256):
        c = i
        for _ in range(8):
            c = (0xEDB88320 ^ (c >> 1)) if (c & 1) else (c >> 1)
        table.append(c & 0xFFFFFFFF)
    return table

_CRC_TABLE = _make_crc_table()

def crc32(data: bytes) -> int:
    crc = 0xFFFFFFFF
    for b in data:
        crc = _CRC_TABLE[(crc ^ b) & 0xFF] ^ (crc >> 8)
    return (crc ^ 0xFFFFFFFF) & 0xFFFFFFFF


# ── LZ77 Decompressor ────────────────────────────────────────────────────────
def lz77_decompress(data: bytes) -> bytes:
    out = bytearray()
    i = 0
    while i < len(data):
        if data[i] == 0xFF and i + 4 < len(data):
            off = struct.unpack_from("<H", data, i + 1)[0]
            length = struct.unpack_from("<H", data, i + 3)[0]
            i += 5
            if off == 0 and length == 1:
                out.append(0xFF)
            else:
                start = len(out) - off
                for j in range(length):
                    out.append(out[start + j])
        else:
            out.append(data[i])
            i += 1
    return bytes(out)


# ── Huffman Decompressor ──────────────────────────────────────────────────────
def huffman_decompress(data: bytes) -> bytes:
    if len(data) < 2:
        return data
    pos = 0
    tag = data[pos]; pos += 1
    sym_lens = {}
    if tag == 0xFF:  # Sparse: [active u8] [(sym,len) pairs] [orig_len u32 LE] [bitstream]
        active = data[pos]; pos += 1
        for _ in range(active):
            if pos + 2 > len(data): break
            sym = data[pos]; pos += 1
            cl = data[pos]; pos += 1
            sym_lens[sym] = cl
    elif tag == 0xFE:  # Full: [256 code lengths] [orig_len u32 LE] [bitstream]
        if pos + 256 > len(data):
            return data
        for s in range(256):
            cl = data[pos + s]
            if cl > 0:
                sym_lens[s] = cl
        pos += 256
    else:
        # Unknown tag — return raw data
        return data
    if pos + 4 > len(data):
        return data
    orig_len = struct.unpack_from("<I", data, pos)[0]; pos += 4
    if not sym_lens:
        return b"\x00" * orig_len
    # Build canonical Huffman codes
    max_bits = max(sym_lens.values())
    sorted_syms = sorted(sym_lens.items(), key=lambda x: (x[1], x[0]))
    code = 0; prev_len = 0; codes = {}
    for sym, cl in sorted_syms:
        if cl > prev_len:
            code <<= (cl - prev_len); prev_len = cl
        codes[(cl, code)] = sym
        code += 1
    # Decode bitstream (MSB-first)
    out = bytearray()
    bit_buf = 0; bits_in = 0
    while len(out) < orig_len and (pos < len(data) or bits_in > 0):
        while bits_in < 24 and pos < len(data):
            bit_buf = (bit_buf << 8) | data[pos]; pos += 1; bits_in += 8
        found = False
        for cl in range(1, max_bits + 1):
            if bits_in < cl:
                break
            c = (bit_buf >> (bits_in - cl)) & ((1 << cl) - 1)
            key = (cl, c)
            if key in codes:
                out.append(codes[key])
                bits_in -= cl
                bit_buf &= (1 << bits_in) - 1 if bits_in > 0 else 0
                found = True
                break
        if not found:
            break
    return bytes(out[:orig_len])


# ── Range Coder Decompressor ──────────────────────────────────────────────────
RC_SCALE = 4096
RC_BOT = 1 << 16

def range_decompress(data: bytes) -> bytes:
    if len(data) < 2:
        return b""
    p = 0
    active = struct.unpack_from("<H", data, p)[0]; p += 2
    if active == 0:
        return b""
    norm = [0] * 256
    for _ in range(active):
        if p >= len(data): return b""
        sym = data[p]; p += 1
        if p + 1 >= len(data): return b""
        norm[sym] = struct.unpack_from("<H", data, p)[0]; p += 2
    if p + 3 >= len(data): return b""
    orig_len = struct.unpack_from("<I", data, p)[0]; p += 4
    if active == 1:
        sym = next((i for i in range(256) if norm[i] > 0), 0)
        return bytes([sym]) * orig_len
    cdf = [0] * 257
    for i in range(256):
        cdf[i + 1] = cdf[i] + norm[i]
    sym_lookup = [0] * RC_SCALE
    for i in range(256):
        if norm[i] > 0:
            for j in range(cdf[i], cdf[i + 1]):
                sym_lookup[j] = i
    coded = data[p:]
    code = 0; cp = 0
    for _ in range(4):
        code = ((code << 8) | (coded[cp] if cp < len(coded) else 0)) & 0xFFFFFFFF
        cp += 1
    low = 0; rng = 0xFFFFFFFF
    out = bytearray()
    for _ in range(orig_len):
        r = rng // RC_SCALE
        offset = min(((code - low) & 0xFFFFFFFF) // r, RC_SCALE - 1)
        sym = sym_lookup[offset]
        si = sym
        low = (low + r * cdf[si]) & 0xFFFFFFFF
        if cdf[si + 1] - cdf[si] < RC_SCALE:
            rng = r * (cdf[si + 1] - cdf[si])
        else:
            rng = rng - r * cdf[si]
        while rng < RC_BOT:
            low = (low << 8) & 0xFFFFFFFF
            rng = (rng << 8) & 0xFFFFFFFF
            code = ((code << 8) | (coded[cp] if cp < len(coded) else 0)) & 0xFFFFFFFF
            cp += 1
        out.append(sym)
    return bytes(out)


# ── Block Decompressor ────────────────────────────────────────────────────────
def decompress_block(data: bytes) -> bytes:
    if not data:
        return b""
    tag = data[0]
    payload = data[1:]
    if tag == 0x02:
        return payload  # raw
    elif tag == 0x00:
        return lz77_decompress(payload)  # LZ77 only
    elif tag == 0x01:
        return lz77_decompress(huffman_decompress(payload))  # Huffman(LZ77)
    elif tag == 0x03:
        return huffman_decompress(payload)  # Huffman only
    elif tag == 0x04:
        return range_decompress(payload)  # Range coder only
    elif tag == 0x05:
        return lz77_decompress(range_decompress(payload))  # Range(LZ77)
    else:
        return lz77_decompress(payload)  # default: LZ77


# ── Column Data Decoders ──────────────────────────────────────────────────────
def decode_delta_int(data: bytes, nrows: int) -> List[int]:
    vals = []
    pos = 0
    base, pos = read_zvar(data, pos)
    vals.append(base)
    acc = base
    for _ in range(1, nrows):
        d, pos = read_zvar(data, pos)
        acc += d
        vals.append(acc)
    return vals

def decode_rle_int(data: bytes, nrows: int) -> List[int]:
    vals = []
    pos = 0
    while len(vals) < nrows and pos < len(data):
        count, pos = read_varint(data, pos)
        val, pos = read_zvar(data, pos)
        vals.extend([val] * count)
    return vals[:nrows]

def decode_cdelta(data: bytes, nrows: int) -> List[int]:
    pos = 0
    base, pos = read_zvar(data, pos)
    step, pos = read_zvar(data, pos)
    return [base + step * i for i in range(nrows)]

def decode_for(data: bytes, nrows: int) -> List[int]:
    pos = 0
    minval, pos = read_zvar(data, pos)
    bits = data[pos] if pos < len(data) else 0
    pos += 1
    if bits == 0:
        return [minval] * nrows
    vals = []
    bitbuf = 0
    bitpos = 0
    for _ in range(nrows):
        while bitpos < bits and pos < len(data):
            bitbuf |= data[pos] << bitpos
            pos += 1
            bitpos += 8
        mask = (1 << bits) - 1
        residual = bitbuf & mask
        bitbuf >>= bits
        bitpos -= bits
        vals.append(minval + residual)
    return vals

def decode_bitpack(data: bytes, nrows: int) -> List[bool]:
    vals = []
    for i in range(nrows):
        byte_idx = i // 8
        bit_idx = i % 8
        if byte_idx < len(data):
            vals.append(bool(data[byte_idx] & (1 << bit_idx)))
        else:
            vals.append(False)
    return vals

def decode_rle_str(data: bytes, nrows: int) -> List[str]:
    vals = []
    pos = 0
    nruns, pos = read_varint(data, pos)
    for _ in range(nruns):
        if len(vals) >= nrows or pos >= len(data):
            break
        count, pos = read_varint(data, pos)
        slen, pos = read_varint(data, pos)
        s = data[pos:pos + slen].decode("utf-8", errors="replace")
        pos += slen
        vals.extend([s] * count)
    return vals[:nrows]

def decode_bdict(data: bytes, nrows: int) -> List[str]:
    pos = 0
    nuniq, pos = read_varint(data, pos)
    dictionary = []
    for _ in range(nuniq):
        slen, pos = read_varint(data, pos)
        s = data[pos:pos + slen].decode("utf-8", errors="replace")
        pos += slen
        dictionary.append(s)
    bits = data[pos] if pos < len(data) else 0
    pos += 1
    if bits == 0:
        return [dictionary[0] if dictionary else ""] * nrows
    vals = []
    bitbuf = 0
    bitpos = 0
    for _ in range(nrows):
        while bitpos < bits and pos < len(data):
            bitbuf |= data[pos] << bitpos
            pos += 1
            bitpos += 8
        mask = (1 << bits) - 1
        idx = bitbuf & mask
        bitbuf >>= bits
        bitpos -= bits
        vals.append(dictionary[idx] if idx < len(dictionary) else "")
    return vals

def decode_huffdict(data: bytes, nrows: int) -> List[str]:
    """Decode HuffDict (codec 8): dictionary + Huffman-coded indices."""
    pos = 0
    nuniq, pos = read_varint(data, pos)
    dictionary = []
    for _ in range(nuniq):
        slen, pos = read_varint(data, pos)
        s = data[pos:pos + slen].decode("utf-8", errors="replace")
        pos += slen
        dictionary.append(s)
    # Remaining bytes are Huffman-compressed indices (tagged format)
    huff = data[pos:]
    hp = 0
    tag = huff[hp]; hp += 1
    sym_lens = {}
    if tag == 0xFF:  # sparse
        active = huff[hp]; hp += 1
        for _ in range(active):
            sym = huff[hp]; hp += 1
            cl = huff[hp]; hp += 1
            sym_lens[sym] = cl
    elif tag == 0xFE:  # full 256
        for s in range(256):
            cl = huff[hp + s]
            if cl > 0:
                sym_lens[s] = cl
        hp += 256
    else:
        return [""] * nrows
    orig_len = struct.unpack_from("<I", huff, hp)[0]; hp += 4
    if not sym_lens:
        return [dictionary[0] if dictionary else ""] * nrows
    max_bits = max(sym_lens.values())
    sorted_syms = sorted(sym_lens.items(), key=lambda x: (x[1], x[0]))
    code = 0; prev_len = 0; codes = {}
    for sym, cl in sorted_syms:
        if cl > prev_len:
            code <<= (cl - prev_len); prev_len = cl
        codes[(cl, code)] = sym
        code += 1
    # Decode bitstream (MSB-first / big-endian bit packing)
    indices = []
    bit_buf = 0; bits_in = 0
    while len(indices) < orig_len and (hp < len(huff) or bits_in > 0):
        while bits_in < 24 and hp < len(huff):
            bit_buf = (bit_buf << 8) | huff[hp]; hp += 1; bits_in += 8
        found = False
        for cl in range(1, max_bits + 1):
            if bits_in < cl:
                break
            c = (bit_buf >> (bits_in - cl)) & ((1 << cl) - 1)
            key = (cl, c)
            if key in codes:
                indices.append(codes[key])
                bits_in -= cl
                bit_buf &= (1 << bits_in) - 1 if bits_in > 0 else 0
                found = True
                break
        if not found:
            break
    vals = []
    for idx in indices[:nrows]:
        vals.append(dictionary[idx] if idx < len(dictionary) else "")
    while len(vals) < nrows:
        vals.append("")
    return vals

def decode_raw_str(data: bytes, nrows: int) -> List[str]:
    vals = []
    pos = 0
    for _ in range(nrows):
        if pos >= len(data):
            vals.append("")
            continue
        slen, pos = read_varint(data, pos)
        s = data[pos:pos + slen].decode("utf-8", errors="replace")
        pos += slen
        vals.append(s)
    return vals


# ── Column Schema ─────────────────────────────────────────────────────────────
class KColumn:
    def __init__(self, name: str, ktype: int, encrypted: bool = False):
        self.name = name
        self.ktype = ktype
        self.encrypted = encrypted

    def __repr__(self):
        return f"KColumn({self.name!r}, {KTYPE_NAMES.get(self.ktype, '?')})"


# ── Footer Column Metadata ───────────────────────────────────────────────────
class ColMeta:
    def __init__(self):
        self.file_offset = 0
        self.comp_len = 0
        self.codec = 0
        self.null_count = 0
        self.min_i64 = 0
        self.max_i64 = 0
        self.min_str = ""
        self.max_str = ""
        # Bloom filter (512 bytes) — stored but not used in Python reader


# ── KORE Reader ───────────────────────────────────────────────────────────────
class KoreReader:
    """Pure Python reader for KORE v2 columnar files."""

    def __init__(self, path: str):
        self.path = path
        with open(path, "rb") as f:
            self.data = f.read()
        self.delete_bitmap: Optional[List[int]] = None  # list of u64 words
        self.deleted_count = 0
        self._parse()
        self._load_delete_bitmap()

    def _parse(self):
        d = self.data
        if len(d) < HEADER_SIZE + 12:
            raise ValueError("Not a valid KORE file (too short)")
        if d[:4] != KORE_MAGIC:
            raise ValueError("Not a KORE file (bad magic)")
        self.version = d[4]
        if self.version not in (1, KORE_V2):
            raise ValueError(f"Unsupported KORE version {self.version}")

        self.ncols = struct.unpack_from("<H", d, 6)[0]
        self.nrows = struct.unpack_from("<Q", d, 8)[0]
        self.nchunks = struct.unpack_from("<I", d, 16)[0]
        self.chunk_size = struct.unpack_from("<I", d, 20)[0]
        self.created = struct.unpack_from("<Q", d, 24)[0]

        # Parse schema
        pos = HEADER_SIZE
        schema_comp_len = struct.unpack_from("<I", d, pos)[0]
        pos += 4
        schema_raw = decompress_block(d[pos:pos + schema_comp_len])
        pos += schema_comp_len

        self.columns = []
        sp = 0
        for _ in range(self.ncols):
            name_len, sp = read_varint(schema_raw, sp)
            name = schema_raw[sp:sp + name_len].decode("utf-8", errors="replace")
            sp += name_len
            ktype = schema_raw[sp] if sp < len(schema_raw) else 4
            sp += 1
            encrypted = (schema_raw[sp] if sp < len(schema_raw) else 0) != 0
            sp += 1
            self.columns.append(KColumn(name, ktype, encrypted))

        # Parse dictionary
        dict_comp_len = struct.unpack_from("<I", d, pos)[0]
        pos += 4
        dict_raw = decompress_block(d[pos:pos + dict_comp_len])
        pos += dict_comp_len

        dp = 0
        dict_count, dp = read_varint(dict_raw, dp)
        self.dictionary = []
        for _ in range(dict_count):
            slen, dp = read_varint(dict_raw, dp)
            self.dictionary.append(dict_raw[dp:dp + slen].decode("utf-8", errors="replace"))
            dp += slen

        # Parse footer (last 12 bytes = footer_comp_len(4) + footer_offset(8))
        trailer_start = len(d) - 12
        footer_comp_len = struct.unpack_from("<I", d, trailer_start)[0]
        footer_offset = struct.unpack_from("<Q", d, trailer_start + 4)[0]
        footer_raw = decompress_block(d[footer_offset:footer_offset + footer_comp_len])

        fp = 0
        ft_nchunks = struct.unpack_from("<I", footer_raw, fp)[0]; fp += 4
        ft_ncols = struct.unpack_from("<H", footer_raw, fp)[0]; fp += 2

        self.chunk_nrows = []
        for _ in range(ft_nchunks):
            nr = struct.unpack_from("<I", footer_raw, fp)[0]; fp += 4
            self.chunk_nrows.append(nr)

        self.col_meta = []  # [chunk_idx][col_idx]
        for _ in range(ft_nchunks):
            chunk_meta = []
            for _ in range(ft_ncols):
                cm = ColMeta()
                cm.file_offset = struct.unpack_from("<Q", footer_raw, fp)[0]; fp += 8
                cm.comp_len = struct.unpack_from("<I", footer_raw, fp)[0]; fp += 4
                cm.codec = footer_raw[fp] if fp < len(footer_raw) else 0; fp += 1
                cm.null_count = struct.unpack_from("<I", footer_raw, fp)[0]; fp += 4
                cm.min_i64, fp = read_zvar(footer_raw, fp)
                cm.max_i64, fp = read_zvar(footer_raw, fp)
                slen, fp = read_varint(footer_raw, fp)
                cm.min_str = footer_raw[fp:fp+slen].decode("utf-8", errors="replace"); fp += slen
                slen, fp = read_varint(footer_raw, fp)
                cm.max_str = footer_raw[fp:fp+slen].decode("utf-8", errors="replace"); fp += slen
                bloom_skip = min(512, len(footer_raw) - fp)
                fp += bloom_skip  # skip bloom filter
                chunk_meta.append(cm)
            self.col_meta.append(chunk_meta)

    def info(self) -> str:
        cols = ", ".join(f"{c.name}:{KTYPE_NAMES.get(c.ktype,'?')}" for c in self.columns)
        size = len(self.data)
        return (f"KORE v{self.version} | {self.nrows:,} rows × {self.ncols} cols | "
                f"{self.nchunks} chunks ({self.chunk_size}r) | {size:,} bytes | [{cols}]")

    def _load_delete_bitmap(self):
        """Load delete bitmap from .kore.del sidecar file if present."""
        del_path = self.path + ".del"
        try:
            with open(del_path, "rb") as f:
                bm_data = f.read()
            if len(bm_data) < 16:
                return
            total_rows = struct.unpack_from("<Q", bm_data, 0)[0]
            self.deleted_count = struct.unpack_from("<Q", bm_data, 8)[0]
            nwords = (total_rows + 63) // 64
            self.delete_bitmap = []
            for i in range(nwords):
                off = 16 + i * 8
                if off + 8 <= len(bm_data):
                    self.delete_bitmap.append(struct.unpack_from("<Q", bm_data, off)[0])
                else:
                    self.delete_bitmap.append(0)
        except FileNotFoundError:
            pass  # No delete bitmap — all rows active

    def is_row_deleted(self, idx: int) -> bool:
        """Check if a row is marked as deleted."""
        if self.delete_bitmap is None:
            return False
        word = idx // 64
        bit = idx % 64
        if word >= len(self.delete_bitmap):
            return True
        return (self.delete_bitmap[word] & (1 << bit)) != 0

    def active_row_count(self) -> int:
        """Number of non-deleted rows."""
        if self.delete_bitmap is None:
            return self.nrows
        return self.nrows - self.deleted_count

    def _decode_col_block(self, ci: int, chunk_idx: int) -> list:
        """Decode a single column from a single chunk."""
        meta = self.col_meta[chunk_idx][ci]
        nrows = self.chunk_nrows[chunk_idx]
        offset = meta.file_offset
        col = self.columns[ci]

        # Read: [crc32(4)] [comp_len(4)] [compressed_data]
        stored_crc = struct.unpack_from("<I", self.data, offset)[0]
        comp_len = struct.unpack_from("<I", self.data, offset + 4)[0]
        compressed = self.data[offset + 8:offset + 8 + comp_len]

        # Verify CRC32
        actual_crc = crc32(compressed)
        if actual_crc != stored_crc:
            raise ValueError(f"CRC32 mismatch for col {ci} chunk {chunk_idx}: "
                           f"stored=0x{stored_crc:08x} actual=0x{actual_crc:08x}")

        raw = decompress_block(compressed)

        # Handle float scale prefix: [0xFE sentinel] [scale_exp byte]
        if col.ktype == KTYPE_FLOAT:
            if len(raw) >= 2 and raw[0] == 0xFE:
                scale_exp = raw[1]
                raw = raw[2:]
                scale = [1.0, 10.0, 100.0, 1000.0, 10000.0][min(scale_exp, 4)]
            else:
                scale = 10000.0  # backward compat: no sentinel
        else:
            scale = 1.0

        codec = meta.codec

        # Decode based on type + codec
        if col.ktype == KTYPE_BOOL:
            if codec == CODEC_BITPACK:
                return decode_bitpack(raw, nrows)
            else:
                ints = decode_rle_int(raw, nrows)
                return [bool(v) for v in ints]

        elif col.ktype in (KTYPE_INT, KTYPE_FLOAT):
            if codec == CODEC_CDELTA:
                ints = decode_cdelta(raw, nrows)
            elif codec == CODEC_DELTA:
                ints = decode_delta_int(raw, nrows)
            elif codec == CODEC_RLE:
                ints = decode_rle_int(raw, nrows)
            elif codec == CODEC_FOR:
                ints = decode_for(raw, nrows)
            else:
                ints = decode_delta_int(raw, nrows)  # fallback

            if col.ktype == KTYPE_FLOAT:
                return [v / scale for v in ints]
            return ints

        elif col.ktype == KTYPE_STR:
            if codec == CODEC_RLE:
                return decode_rle_str(raw, nrows)
            elif codec == CODEC_BDICT:
                return decode_bdict(raw, nrows)
            elif codec == CODEC_RAW:
                return decode_raw_str(raw, nrows)
            elif codec == CODEC_HUFFDICT:
                return decode_huffdict(raw, nrows)
            else:
                return decode_raw_str(raw, nrows)

        return [""] * nrows

    def read_column(self, col_name: str) -> Optional[list]:
        """Read a single column by name (column pruning). Filters deleted rows."""
        ci = None
        for i, c in enumerate(self.columns):
            if c.name == col_name:
                ci = i
                break
        if ci is None:
            return None
        vals = []
        global_row = 0
        for chunk_idx in range(self.nchunks):
            chunk_vals = self._decode_col_block(ci, chunk_idx)
            if self.delete_bitmap is not None:
                for v in chunk_vals:
                    if not self.is_row_deleted(global_row):
                        vals.append(v)
                    global_row += 1
            else:
                vals.extend(chunk_vals)
                global_row += len(chunk_vals)
        return vals

    def read_columns(self, names: List[str]) -> Dict[str, list]:
        """Read multiple columns by name."""
        return {name: self.read_column(name) for name in names if self.read_column(name) is not None}

    def read_all_columns(self) -> Dict[str, list]:
        """Read all columns into a dict."""
        return {c.name: self.read_column(c.name) for c in self.columns}

    def read_rows(self, start: int = 0, end: Optional[int] = None) -> List[Dict[str, Any]]:
        """Read rows as list of dicts (row-oriented)."""
        if end is None:
            end = self.nrows
        cols = self.read_all_columns()
        rows = []
        for i in range(start, min(end, self.nrows)):
            row = {}
            for c in self.columns:
                vals = cols.get(c.name, [])
                row[c.name] = vals[i] if i < len(vals) else None
            rows.append(row)
        return rows

    def to_dict(self) -> Dict[str, list]:
        """Return all data as {col_name: [values]} dict (pandas-friendly)."""
        return self.read_all_columns()

    def column_stats(self, col_name: str) -> Optional[Dict]:
        """Get column statistics from footer (no data decode needed)."""
        ci = None
        for i, c in enumerate(self.columns):
            if c.name == col_name:
                ci = i
                break
        if ci is None:
            return None
        stats = {"null_count": 0, "min_i64": float("inf"), "max_i64": float("-inf"),
                 "min_str": "", "max_str": ""}
        first = True
        for chunk_idx in range(self.nchunks):
            m = self.col_meta[chunk_idx][ci]
            stats["null_count"] += m.null_count
            if m.min_i64 < stats["min_i64"]: stats["min_i64"] = m.min_i64
            if m.max_i64 > stats["max_i64"]: stats["max_i64"] = m.max_i64
            if first or m.min_str < stats["min_str"]: stats["min_str"] = m.min_str
            if first or m.max_str > stats["max_str"]: stats["max_str"] = m.max_str
            first = False
        return stats


# ── CLI ───────────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    import sys
    if len(sys.argv) < 2:
        print("Usage: python kore_reader.py <file.kore> [column_name]")
        sys.exit(1)

    path = sys.argv[1]
    reader = KoreReader(path)
    print(reader.info())
    print()

    if len(sys.argv) >= 3:
        col_name = sys.argv[2]
        vals = reader.read_column(col_name)
        if vals is None:
            print(f"Column '{col_name}' not found")
        else:
            print(f"{col_name}: {len(vals)} values")
            for v in vals[:20]:
                print(f"  {v}")
            if len(vals) > 20:
                print(f"  ... ({len(vals) - 20} more)")
    else:
        # Show stats for all columns
        for c in reader.columns:
            stats = reader.column_stats(c.name)
            if stats:
                if c.ktype in (KTYPE_INT, KTYPE_FLOAT):
                    print(f"  {c.name}: min={stats['min_i64']}, max={stats['max_i64']}, nulls={stats['null_count']}")
                else:
                    print(f"  {c.name}: min='{stats['min_str']}', max='{stats['max_str']}', nulls={stats['null_count']}")
