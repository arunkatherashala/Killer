def risky(i):
    if i % 10 == 0:
        raise Exception("bad" + str(i))
    return i


ok = 0
bad = 0
for i in range(300000):
    try:
        ok = ok + risky(i)
    except Exception as e:
        bad = bad + 1
print(ok)
print(bad)
