class Node:
    def __init__(self, v, l, r):
        self.v = v
        self.l = l
        self.r = r


def build(d, v):
    if d == 0:
        return None
    return Node(v, build(d - 1, v * 2), build(d - 1, v * 2 + 1))


def total(n):
    if n is None:
        return 0
    return n.v + total(n.l) + total(n.r)


def depth(n):
    if n is None:
        return 0
    return 1 + max(depth(n.l), depth(n.r))


t = build(17, 1)
print(total(t) % 1000003)
print(depth(t))
