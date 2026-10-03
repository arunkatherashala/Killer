a = []
x = 7
for i in range(500000):
    x = (x * 1664525 + 1013904223) % 4294967296
    a.append(x)
s = 0
for i in range(500000):
    s = s + a[i] % 10
for i in range(100000):
    a.pop()
b = sorted(a)
print(len(a))
print(s)
print(b[0])
print(b[len(b) - 1])
print(b[200000])
