x = 12345
counts = {}
for i in range(400000):
    x = (x * 1664525 + 1013904223) % 4294967296
    w = "word" + str(x % 5000)
    if w in counts:
        counts[w] = counts[w] + 1
    else:
        counts[w] = 1
best = 0
total = 0
for k in counts.keys():
    total = total + counts[k]
    if counts[k] > best:
        best = counts[k]
print(len(counts))
print(total)
print(best)
