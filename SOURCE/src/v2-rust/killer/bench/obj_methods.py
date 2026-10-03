class Counter:
    def __init__(self, step):
        self.n = 0
        self.step = step

    def inc(self):
        self.n = self.n + self.step

    def get(self):
        return self.n


c = Counter(2)
for i in range(1000000):
    c.inc()
print(c.get())
objs = []
for i in range(100000):
    objs.append(Counter(i))
t = 0
for o in objs:
    o.inc()
    t = t + o.get()
print(t)
