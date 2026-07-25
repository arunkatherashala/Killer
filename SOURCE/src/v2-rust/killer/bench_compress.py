import gzip, lzma, zlib, brotli, time, os

koreb = open(r'C:\Users\skathera\Downloads\kore\1st_data_bin.kore','rb').read()
kore  = open(r'C:\Users\skathera\Downloads\kore\1st_data_v2.kore','rb').read()
csv   = open(r'C:\Users\skathera\Downloads\test_data_100records.csv','rb').read()
pq    = open(r'C:\Users\skathera\Downloads\kore\1st_data.parquet','rb').read()

csvlen = len(csv)

def bench(name, compressed, decomp_fn):
    REPS = 2000
    t = time.perf_counter()
    for _ in range(REPS):
        decomp_fn(compressed)
    us = (time.perf_counter() - t) * 1_000_000 / REPS
    sz = len(compressed)
    print(f"  {name:<28} {sz/1024:>6.1f}KB  {sz*100/csvlen:>6.1f}%  {us:>6.0f}us")

print(f"Input: CSV={csvlen/1024:.1f}KB  KORE-B={len(koreb)/1024:.1f}KB  KORE-K={len(kore)/1024:.1f}KB  Parquet={len(pq)/1024:.1f}KB")
print()
print(f"  {'Format':<28} {'Size':>7}  {'%CSV':>6}  DecompSpeed")
print("-"*62)
print(f"  {'CSV raw':<28} {csvlen/1024:>6.1f}KB  100.0%  baseline")
print(f"  {'Parquet+Snappy':<28} {len(pq)/1024:>6.1f}KB  {len(pq)*100/csvlen:>5.1f}%  fast(C)")
print()
print("--- KORE-K (text) ---")
for lvl in [1, 6, 9]:
    c = gzip.compress(kore, compresslevel=lvl)
    bench(f"KORE-K + gzip-{lvl}", c, gzip.decompress)
for lvl in [1, 6, 9]:
    c = zlib.compress(kore, lvl)
    bench(f"KORE-K + zlib-{lvl}", c, zlib.decompress)
for q in [1, 5, 11]:
    c = brotli.compress(kore, quality=q)
    bench(f"KORE-K + brotli-{q}", c, brotli.decompress)
for preset in [0, 6, 9]:
    c = lzma.compress(kore, preset=preset)
    bench(f"KORE-K + lzma-{preset}", c, lzma.decompress)
print()
print("--- KORE-B (binary) ---")
for lvl in [1, 6, 9]:
    c = gzip.compress(koreb, compresslevel=lvl)
    bench(f"KORE-B + gzip-{lvl}", c, gzip.decompress)
for lvl in [1, 6, 9]:
    c = zlib.compress(koreb, lvl)
    bench(f"KORE-B + zlib-{lvl}", c, zlib.decompress)
for q in [1, 5, 11]:
    c = brotli.compress(koreb, quality=q)
    bench(f"KORE-B + brotli-{q}", c, brotli.decompress)
for preset in [0, 6, 9]:
    c = lzma.compress(koreb, preset=preset)
    bench(f"KORE-B + lzma-{preset}", c, lzma.decompress)
