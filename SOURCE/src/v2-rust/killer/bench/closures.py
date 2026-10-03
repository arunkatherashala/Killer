def make_adder(k):
    def add(x):
        return x + k
    return add


add5 = make_adder(5)
s = 0
for i in range(1000000):
    s = add5(s)
print(s)


def compose(f, g):
    return lambda x: f(g(x))


h = compose(add5, make_adder(3))
t = 0
for i in range(300000):
    t = t + h(i) % 7
print(t)
