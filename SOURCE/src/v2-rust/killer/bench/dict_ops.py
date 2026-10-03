d = {}
for i in range(200000):
    d["key" + str(i)] = i
s = 0
for i in range(200000):
    s = s + d["key" + str(i)]
for i in range(0, 200000, 2):
    del d["key" + str(i)]
print(s)
print(len(d.keys()))
n = {}
for i in range(200000):
    n[i] = i * 2
t = 0
for i in range(200000):
    t = t + n[i]
print(t)
