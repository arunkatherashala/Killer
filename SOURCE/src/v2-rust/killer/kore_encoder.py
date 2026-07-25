"""
kore_encoder.py  —  KORE Binary format (KORE-B)
Layout:
  Header  : magic(4='KORE') mode(1='B') ncols(2) nrows(4)
  Columns : [name_len(1) name(N) type(1) algo(1) data_len(3) data(N)]

  Algo encodings:
    RLE   (r): [zigzag_varint(count), value, ...]   for int/float
               [varint(count), uint8_len, utf8 ...]  for str
    Delta (d): zigzag_varint(base), zigzag_varint(diff) × (nrows-1)
    Dict  (t): uint16(nentries), uint8_len+utf8 × entries, uint8/uint16 indices
    Plain (p): float32 × nrows  (for random floats)

  Types  : 0=int  1=float  2=str  3=bool
  Algos  : 0=rle  1=delta  2=dict  3=plain

  KORE format family:
    KORE-K  (text)   : readable, starts with 'KORE-K' line, extension .kore
    KORE-B  (binary) : compact, starts with magic 'KORE'+0x42, extension .kore
"""

import struct, os, re, gzip, lzma
import pandas as pd

CSV_PATH   = r"C:\Users\skathera\Downloads\test_data_100records.csv"
KOREB_PATH = r"C:\Users\skathera\Downloads\kore\1st_data_bin.kore"
KOREBZ_PATH= r"C:\Users\skathera\Downloads\kore\1st_data_bin.kore.gz"

MAGIC = b'KORE'
MODE  = b'B'   # 'B' = binary mode  (text mode files start with 'KORE-K')

# ── varint helpers ──────────────────────────────────────────────────────────

def zigzag(n: int) -> int:
    return (n << 1) ^ (n >> 63)

def write_varint(buf: bytearray, n: int):
    n = n & 0xFFFFFFFFFFFFFFFF
    while n >= 0x80:
        buf.append((n & 0x7F) | 0x80)
        n >>= 7
    buf.append(n)

def write_zvarint(buf: bytearray, n: int):
    write_varint(buf, zigzag(n))

def read_varint(data: bytes, pos: int):
    n = 0; shift = 0
    while True:
        b = data[pos]; pos += 1
        n |= (b & 0x7F) << shift
        if not (b & 0x80): break
        shift += 7
    return n, pos

def read_zvarint(data: bytes, pos: int):
    n, pos = read_varint(data, pos)
    return (n >> 1) ^ -(n & 1), pos

# ── pack str ────────────────────────────────────────────────────────────────

def pack_str(s: str) -> bytes:
    b = s.encode('utf-8')
    return bytes([len(b)]) + b

# ── normalise helpers (same as v2) ──────────────────────────────────────────

DATE_RE = re.compile(r'^\d{4}-\d{2}-\d{2} 00:00:00\.0+$')
DT_RE   = re.compile(r'^(\d{4}-\d{2}-\d{2}) \d{2}:\d{2}:\d{2}\.\d+')

def norm_str(v):
    v = v.strip()
    if not v: return "EMPTY"
    if DATE_RE.match(v): return v[:10]
    m = DT_RE.match(v)
    if m and v.endswith('.000'): return v[:19]
    return v.replace('^','-').replace('*','x').replace('~','-')

def trim_float(v):
    s = str(v).rstrip('0').rstrip('.')
    return s if s and s != '-' else '0'

# ── encoders ────────────────────────────────────────────────────────────────

def encode_rle_int(nums, buf):
    if not nums: return
    runs = []
    cur = nums[0]; cnt = 1
    for n in nums[1:]:
        if n == cur: cnt += 1
        else: runs.append((cnt, cur)); cur = n; cnt = 1
    runs.append((cnt, cur))
    write_varint(buf, len(runs))
    for cnt, val in runs:
        write_zvarint(buf, cnt)
        write_zvarint(buf, val)

def encode_rle_str(vals, buf):
    runs = []
    cur = vals[0]; cnt = 1
    for v in vals[1:]:
        if v == cur: cnt += 1
        else: runs.append((cnt, cur)); cur = v; cnt = 1
    runs.append((cnt, cur))
    write_varint(buf, len(runs))
    for cnt, val in runs:
        write_zvarint(buf, cnt)
        b = val.encode('utf-8')
        write_varint(buf, len(b))
        buf.extend(b)

def encode_delta_int(nums, buf):
    write_zvarint(buf, nums[0])
    for i in range(1, len(nums)):
        write_zvarint(buf, nums[i] - nums[i-1])

def encode_dict_str(vals, buf):
    seen = {}; order = []; idxs = []
    for v in vals:
        if v not in seen: seen[v] = len(order); order.append(v)
        idxs.append(seen[v])
    # Dictionary entries
    write_varint(buf, len(order))
    for s in order:
        b = s.encode('utf-8')
        write_varint(buf, len(b)); buf.extend(b)
    # Indices: uint8 if ≤256 entries else uint16
    if len(order) <= 256:
        for idx in idxs: buf.append(idx)
    else:
        for idx in idxs: buf.extend(struct.pack('<H', idx))

def encode_plain_float(vals, buf):
    for v in vals:
        buf.extend(struct.pack('<f', v))

# ── type detection ──────────────────────────────────────────────────────────

def detect_type(col_vals):
    all_int = True; all_float = True
    for v in col_vals:
        sv = str(v).strip()
        if not sv or sv.lower() in ('nan','null',''): continue
        try: float(sv)
        except: all_int = False; all_float = False; break
        try: int(sv)
        except: all_int = False
    # bool heuristic: all 0/1
    if all_int:
        nums = []
        for v in col_vals:
            sv = str(v).strip()
            if sv.lower() in ('nan','null',''): continue
            try: nums.append(int(sv))
            except: pass
        if nums and all(n in (0,1) for n in nums): return 3  # bool→rle
        return 0  # int
    if all_float: return 1
    return 2  # str

def choose_algo_encode(col_type, raw_vals, buf):
    """Returns algo byte 0..3"""
    if col_type == 2:  # str
        vals = [norm_str(v) for v in raw_vals]
        uniq = len(set(vals))
        if uniq == 1:
            encode_rle_str(vals, buf); return 0   # rle
        encode_dict_str(vals, buf);   return 2    # dict

    elif col_type == 3:  # bool (0/1)
        nums = [int(str(v).strip() in ('1','true','True','yes')) for v in raw_vals]
        encode_rle_int(nums, buf); return 0        # rle

    else:  # int (0) or float (1)
        if col_type == 0:  # int
            nums = []
            for v in raw_vals:
                sv = str(v).strip()
                try: nums.append(int(float(sv)))
                except: nums.append(0)
        else:  # float
            nums_f = []
            for v in raw_vals:
                sv = str(v).strip()
                try: nums_f.append(float(sv))
                except: nums_f.append(0.0)
            # Try scaling to int (if all values × 10000 are exact ints)
            scaled = [round(x * 10000) for x in nums_f]
            # Store as scaled int (× 10000)
            nums = scaled

        uniq = len(set(nums))
        if uniq == 1:
            encode_rle_int(nums, buf); return 0        # rle
        if uniq <= 12:
            encode_rle_int(nums, buf); return 0        # rle
        diffs = [abs(nums[i]-nums[i-1]) for i in range(1,len(nums))]
        avg_diff = sum(diffs)/len(diffs) if diffs else 0
        avg_val  = sum(abs(n) for n in nums)/len(nums) if nums else 1
        if avg_val == 0 or avg_diff / avg_val < 0.9:
            encode_delta_int(nums, buf); return 1      # delta
        else:
            encode_rle_int(nums, buf); return 0        # rle (fallback)

# ── build binary ────────────────────────────────────────────────────────────

df    = pd.read_csv(CSV_PATH, dtype=str, keep_default_na=False)
ncols = len(df.columns)
nrows = len(df)
print(f"Building KORE binary: {nrows} rows × {ncols} cols")

# Header: 'KORE' + 'B' + ncols(2) + nrows(4)
out = bytearray(MAGIC)
out += MODE
out += struct.pack('<H', ncols)
out += struct.pack('<I', nrows)

TYPE_MAP = {0:'i', 1:'f', 2:'s', 3:'b'}

algo_counts = {0:0,1:0,2:0,3:0}
for ci, col_name in enumerate(df.columns):
    raw_vals = df[col_name].tolist()
    col_type = detect_type(raw_vals)

    col_buf = bytearray()
    algo = choose_algo_encode(col_type, raw_vals, col_buf)
    algo_counts[algo] += 1

    name_b = col_name.encode('utf-8')
    out += bytes([len(name_b)]) + name_b
    out += bytes([col_type, algo])
    # 3-byte data length
    dlen = len(col_buf)
    out += bytes([dlen & 0xFF, (dlen >> 8) & 0xFF, (dlen >> 16) & 0xFF])
    out += col_buf

    if ci % 10 == 0: print(f"  col {ci}/{ncols}")

# Write raw KORE binary
with open(KOREB_PATH, 'wb') as f:
    f.write(out)

# Write gzip compressed KORE binary
with gzip.open(KOREBZ_PATH, 'wb', compresslevel=9) as f:
    f.write(out)

# Also write lzma on text v2
v2_text = open(r'C:\Users\skathera\Downloads\kore\1st_data_v2.kore','rb').read()
lzma_path = r'C:\Users\skathera\Downloads\kore\1st_data_v2.kore.xz'
with lzma.open(lzma_path, 'wb', preset=6) as f:
    f.write(v2_text)

# Stats
csv_sz  = os.path.getsize(CSV_PATH)
v1_sz   = os.path.getsize(r'C:\Users\skathera\Downloads\kore\1st_data.kore')
v2_sz   = os.path.getsize(r'C:\Users\skathera\Downloads\kore\1st_data_v2.kore')
pq_sz   = os.path.getsize(r'C:\Users\skathera\Downloads\kore\1st_data.parquet')
koreb_sz = os.path.getsize(KOREB_PATH)
korebz_sz= os.path.getsize(KOREBZ_PATH)
v2xz_sz  = os.path.getsize(lzma_path)

print(f"\n{'Format':<26} {'Size':>8}  {'% of CSV':>9}  Notes")
print("-"*66)
def row(name, sz, note=''):
    print(f"{name:<26} {sz/1024:>7.1f}KB  {sz*100/csv_sz:>8.1f}%  {note}")

row("CSV original",        csv_sz,   "plain text, 100 rows × 71 cols")
row("Parquet (snappy)",    pq_sz,    "binary columnar")
row("KORE-K v1 (text)",    v1_sz,    "basic delta/dict/rle")
row("KORE-K v2 (text)",    v2_sz,    "+ float trim, const-RLE, dates")
row("KORE-K v2 + lzma",    v2xz_sz,  "text + compress")
row("KORE-B (binary)",     koreb_sz, "binary varint, scaled floats")
row("KORE-B + gzip",       korebz_sz,"binary + compress")

print(f"\nKORE-B algo distribution:")
names = {0:'rle', 1:'delta', 2:'dict', 3:'plain'}
for k,v in algo_counts.items(): print(f"  {names[k]}: {v} cols")
print(f"\nKORE-B avg bytes/col: {koreb_sz/ncols:.0f}")
print(f"KORE-K avg bytes/col: {v2_sz/ncols:.0f}")
