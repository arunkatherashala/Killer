import sys
sys.setrecursionlimit(100000)
step = 1
def deep(n):
    if n == 0:
        return 0
    return step + deep(n - 1)
print(deep(20000))
