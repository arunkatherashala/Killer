parts = []
for i in range(300000):
    parts.append("w" + str(i % 1000))
big = ",".join(parts)
words = big.split(",")
print(len(big))
print(len(words))
j = "-".join(words)
r = j.replace("w1", "Q")
print(len(j))
print(len(r))
u = big.upper()
print(len(u))
