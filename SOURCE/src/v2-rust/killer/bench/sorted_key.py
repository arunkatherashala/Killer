a = []
x = 7
for i in range(150000):
    x = (x * 1664525 + 1013904223) % 4294967296
    a.append(x % 1000003)
b = sorted(a, key=lambda v: (v * 31) % 1000003)
print(b[0])
print(b[1000])
print(b[len(b) - 1])
