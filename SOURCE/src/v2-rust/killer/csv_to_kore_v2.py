"""
csv_to_kore_v2.py  —  Improved CSV → KORE-K encoder
Optimizations over v1:
  1. Float precision trim   : 1.00000000 → 1,  -2489.7600 → -2489.76
  2. Auto-RLE for constants : all-same column → N~value  (5 bytes vs hundreds)
  3. Auto-RLE for ints      : low-diversity int (≤8 unique) → RLE instead of delta
  4. Date normalization      : strip trailing  00:00:00.000 from ISO datetimes
  5. Schema short codes      : int→i, float→f, str→s, bool→b  |  delta→d, plain→p, rle→r, dict→t
     (saves ~400 bytes on schema line; decoder must be updated to read short codes)
  NOTE: schema short codes are written but kore_pure.killer v2 decoder handles them.
"""

import re, os, sys

CSV_PATH  = r"C:\Users\skathera\Downloads\test_data_100records.csv"
OUT_PATH  = r"C:\Users\skathera\Downloads\kore\1st_data_v2.kore"

# ─── helpers ─────────────────────────────────────────────────────────────────

DATE_RE = re.compile(r'^\d{4}-\d{2}-\d{2} 00:00:00\.0+$')
DT_RE   = re.compile(r'^(\d{4}-\d{2}-\d{2}) \d{2}:\d{2}:\d{2}\.\d+')

def norm_str(v):
    """Normalize a string value before encoding."""
    v = v.strip()
    if not v:
        return "EMPTY"
    # Strip useless midnight time from date strings
    if DATE_RE.match(v):
        return v[:10]
    # Keep non-midnight datetimes but strip sub-seconds if 000
    m = DT_RE.match(v)
    if m and v.endswith('.000'):
        return v[:19]  # keep HH:MM:SS, drop .000
    # Sanitize KORE-K separator chars
    return v.replace('^', '-').replace('*', 'x').replace('~', '-').replace('|', '/')

def trim_float(v):
    """1.00000000 → 1   |   -2489.7600 → -2489.76   |   0.0 → 0"""
    v = str(v).rstrip('0').rstrip('.')
    return v if v and v != '-' else '0'

# ─── encoders ────────────────────────────────────────────────────────────────

def rle_encode(vals):
    """count~value,count~value — works for any type"""
    if not vals:
        return ""
    result = []
    cur = vals[0]; cnt = 1
    for v in vals[1:]:
        if v == cur:
            cnt += 1
        else:
            result.append(f"{cnt}~{cur}")
            cur = v; cnt = 1
    result.append(f"{cnt}~{cur}")
    return ",".join(result)

def delta_encode(nums):
    if not nums:
        return ""
    parts = [str(nums[0])]
    for i in range(1, len(nums)):
        parts.append(str(nums[i] - nums[i-1]))
    return ",".join(parts)

def dict_encode(vals):
    seen = {}; order = []; idxs = []
    for v in vals:
        if v not in seen:
            seen[v] = len(order)
            order.append(v)
        idxs.append(seen[v])
    return "^".join(order) + "*" + ",".join(map(str, idxs))

def plain_encode(vals):
    return ",".join(vals)

def choose_encode(col_type, raw_vals):
    """
    Choose best algorithm and produce (algo_short, encoded_str).
    algo codes: d=delta  p=plain  r=rle  t=dict
    """
    if col_type == 's':
        vals = [norm_str(v) for v in raw_vals]
        return 't', dict_encode(vals)

    elif col_type == 'b':
        vals = ['true' if str(v).strip().lower() in ('1','true','yes') else 'false'
                for v in raw_vals]
        return 'r', rle_encode(vals)

    elif col_type == 'i':
        nums = []
        for v in raw_vals:
            sv = str(v).strip()
            try:    nums.append(int(sv))
            except: nums.append(0)

        uniq = len(set(nums))
        # Constant → RLE (single entry N~v)
        if uniq == 1:
            return 'r', rle_encode([str(n) for n in nums])
        # Low diversity flag-style → RLE
        if uniq <= 8:
            return 'r', rle_encode([str(n) for n in nums])
        # Sequential-ish ids → delta
        return 'd', delta_encode(nums)

    else:  # float
        trimmed = []
        for v in raw_vals:
            sv = str(v).strip()
            try:    trimmed.append(trim_float(float(sv)))
            except: trimmed.append('0')

        uniq = len(set(trimmed))
        if uniq == 1:
            return 'r', rle_encode(trimmed)
        if uniq <= 8:
            return 'r', rle_encode(trimmed)
        return 'p', plain_encode(trimmed)

# ─── type detection ──────────────────────────────────────────────────────────

def detect_type(col_vals):
    """Returns 'i', 'f', 's', 'b'"""
    all_int = True; all_float = True
    bool_words = {'true','false','True','False','1','0','yes','no','TRUE','FALSE'}
    for v in col_vals:
        sv = str(v).strip()
        if not sv or sv.lower() in ('nan','null','empty',''):
            continue
        try:
            float(sv)
        except:
            all_int = False; all_float = False; break
        try:
            int(sv)
        except:
            all_int = False
    if all_int:   return 'i'
    if all_float: return 'f'
    return 's'

# ─── main ────────────────────────────────────────────────────────────────────

with open(CSV_PATH, encoding='utf-8') as f:
    csv_lines = f.read().splitlines()

header = csv_lines[0].split(',')
ncols  = len(header)
nrows  = len(csv_lines) - 1
rows   = [line.split(',') for line in csv_lines[1:]]

print(f"CSV: {nrows} rows x {ncols} cols")

# Extract columns
cols = []
for ci in range(ncols):
    vals = [row[ci] if ci < len(row) else '' for row in rows]
    cols.append(vals)

# Detect types
types = [detect_type(c) for c in cols]

# Encode
print("Encoding...")
schema_parts = []
col_lines    = []

for ci, (col_name, col_type, raw_vals) in enumerate(zip(header, types, cols)):
    algo, encoded = choose_encode(col_type, raw_vals)
    schema_parts.append(f"{col_name}:{col_type}:{algo}")
    col_lines.append(f"COL {col_name} {encoded}")
    if ci % 10 == 0:
        print(f"  col {ci}/{ncols}")

schema_line = "SCHEMA " + " ".join(schema_parts)
all_lines   = ["KORE-K2", schema_line, f"ROWS {nrows}"] + col_lines + ["END"]
content     = "\n".join(all_lines)

with open(OUT_PATH, 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

# ─── stats ───────────────────────────────────────────────────────────────────

csv_sz  = os.path.getsize(CSV_PATH)
v1_sz   = os.path.getsize(r"C:\Users\skathera\Downloads\kore\1st_data.kore")
v2_sz   = os.path.getsize(OUT_PATH)

print(f"\n{'='*50}")
print(f"CSV original:   {csv_sz/1024:.1f} KB  (100%)")
print(f"KORE v1:        {v1_sz/1024:.1f} KB  ({v1_sz*100/csv_sz:.1f}%)")
print(f"KORE v2:        {v2_sz/1024:.1f} KB  ({v2_sz*100/csv_sz:.1f}%)")
print(f"v2 vs v1:       {(v2_sz-v1_sz)/1024:+.1f} KB  ({(v2_sz-v1_sz)*100/v1_sz:.1f}% change)")
print(f"{'='*50}")

# Per-column breakdown of biggest savers
print("\nTop 10 columns by size reduction:")
import json
v1_lines = open(r"C:\Users\skathera\Downloads\kore\1st_data.kore", encoding='utf-8').read().splitlines()
v1_data = {}
for ln in v1_lines[3:-1]:
    parts = ln.split(' ', 2)
    v1_data[parts[1]] = len(parts[2]) if len(parts) > 2 else 0

savings = []
for ln in col_lines:
    parts = ln.split(' ', 2)
    name = parts[1]; sz2 = len(parts[2]) if len(parts)>2 else 0
    sz1 = v1_data.get(name, sz2)
    savings.append((name, sz1, sz2, sz1-sz2))

for name, sz1, sz2, saved in sorted(savings, key=lambda x: -x[3])[:10]:
    print(f"  {name:<30}  {sz1:>5} → {sz2:>5}  saved {saved:>5} bytes")

print("\nAlgo summary:")
algo_counts = {}
for sp in schema_parts:
    algo = sp.split(':')[2]
    algo_counts[algo] = algo_counts.get(algo, 0) + 1
for algo, cnt in sorted(algo_counts.items()):
    print(f"  {algo}: {cnt} cols")
