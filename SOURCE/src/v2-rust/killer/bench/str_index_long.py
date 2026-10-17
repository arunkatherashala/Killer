base = "abcdefghijklmnopqrstuvwxyz"
s = base
for k in range(13):
    s = s + s
c = 0
i = 0
while i < len(s):
    if s[i] == "e":
        c = c + 1
    i = i + 1
print(len(s))
print(c)
print(s[-1])
