a = [i * 2 for i in range(500000)]
b = [x + 1 for x in a if x % 3 == 0]
c = [str(x) for x in b]
d = [[i, i * i] for i in range(200000)]
print(len(a))
print(len(b))
print(len(c))
print(d[1999][1])
print(sum(b))
