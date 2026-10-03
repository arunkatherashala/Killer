s = ""
for i in range(200000):
    s = s + "x"
t = ""
for i in range(100000):
    t += str(i) + ","
print(len(s))
print(len(t))
