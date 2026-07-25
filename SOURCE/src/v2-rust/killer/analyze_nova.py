import os, struct, zlib, gzip, math
from collections import Counter

files = {
    'CSV original':    r'C:\Users\skathera\Downloads\test_data_100records.csv',
    'KORE-K v1':       r'C:\Users\skathera\Downloads\kore\1st_data.kore',
    'KORE-K v2':       r'C:\Users\skathera\Downloads\kore\1st_data_v2.kore',
    'KORE-B binary':   r'C:\Users\skathera\Downloads\kore\1st_data_bin.kore',
    'Nova current':    r'C:\Users\skathera\Downloads\kore\1st_data.nova',
    'Parquet+Snappy':  r'C:\Users\skathera\Downloads\kore\1st_data.parquet',
}
csv_sz = os.path.getsize(files['CSV original'])
print("  Format                  Bytes    pct_CSV")
print("-"*50)
for name, path in files.items():
    sz = os.path.getsize(path)
    print(f"  {name:<22} {sz:>8}   {sz*100/csv_sz:>6.1f}%")

nova = open(r'C:\Users\skathera\Downloads\kore\1st_data.nova', 'rb').read()
zc   = zlib.compress(nova, 9)

print()
print(f"Nova current:     {len(nova)} bytes")
print(f"Nova + zlib9:     {len(zc)} bytes  ({len(zc)*100/csv_sz:.1f}% csv)")

# Entropy analysis
counts = Counter(nova)
total  = len(nova)
entropy = -sum((c/total)*math.log2(c/total) for c in counts.values())
theoretical = total * entropy / 8
print()
print(f"Entropy:          {entropy:.3f} bits/byte  (8.0=random noise)")
print(f"Theoretical min:  {theoretical:.0f} bytes  ({theoretical*100/csv_sz:.1f}% csv)")
print(f"Gap remaining:    {len(nova)-theoretical:.0f} bytes still squeezable")

# Analyze byte distribution
zeros  = nova.count(0)
ones   = nova.count(1)
small  = sum(1 for b in nova if b < 16)
print()
print(f"Zero bytes:       {zeros}/{total} = {zeros*100/total:.0f}%")
print(f"Byte=1:           {ones}/{total}")
print(f"Bytes < 16:       {small}/{total} = {small*100/total:.0f}%  (varint wins here)")

# What does the CSV actually contain (unique values)
csv_text = open(r'C:\Users\skathera\Downloads\test_data_100records.csv', encoding='utf-8').read()
csv_lines = csv_text.splitlines()
header = csv_lines[0].split(',')
nrows  = len(csv_lines) - 1
rows   = [l.split(',') for l in csv_lines[1:]]
cols   = [[rows[r][c] if c < len(rows[r]) else '' for r in range(nrows)] for c in range(len(header))]

total_uniq = sum(len(set(c)) for c in cols)
const_cols = sum(1 for c in cols if len(set(c)) == 1)
print()
print(f"CSV columns:      {len(header)}")
print(f"Constant cols:    {const_cols}  (single value, RLE = 3 bytes)")
print(f"Total uniq vals:  {total_uniq} across all cols")
ideal = const_cols * 3 + (len(header) - const_cols) * 10
print(f"Ideal lower bound approx: ~{ideal} bytes payload")
