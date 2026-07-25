# ⚡ KILLER v1.0 - QUICK REFERENCE

**Print this page and keep it at your desk!**

---

## 🚀 RUN A PROGRAM

```bash
killer.exe program.killer
```

---

## 📝 HELLO WORLD

```killer
kfn main
    print("Hello, World!")
```

---

## 🔢 VARIABLES

```killer
x = 42              # integer
pi = 3.14           # float
name = "Alice"      # string
active = true       # boolean
nums = [1, 2, 3]    # list
```

---

## ⚙️ FUNCTIONS

```killer
kfn add(a: Int, b: Int) -> Int
    return a + b

kfn main
    result = add(5, 3)
    print(result)    # 8
```

---

## 🔄 CONTROL FLOW

### If/Else
```killer
if x > 0
    print("positive")
elif x < 0
    print("negative")
else
    print("zero")
```

### While Loop
```killer
while x < 10
    print(x)
    x = x + 1
```

### For Loop
```killer
for i in 0..10
    print(i)
```

---

## 📦 LISTS

```killer
nums = [1, 2, 3, 4, 5]
print(nums[0])          # 1
nums.append(6)          # add item
print(len(nums))        # 6

for n in nums
    print(n)
```

---

## 🔤 STRINGS

| Operation | Code |
|-----------|------|
| Concatenate | `"Hello " + "World"` |
| Interpolate | `"Hello $name"` |
| Length | `len("Alice")` |
| Uppercase | `"hello".upper()` |
| Lowercase | `"HELLO".lower()` |

---

## 🎯 OPERATORS

| Operator | Meaning |
|----------|---------|
| `+` | Add |
| `-` | Subtract |
| `*` | Multiply |
| `/` | Divide |
| `%` | Modulo |
| `==` | Equal |
| `!=` | Not equal |
| `>` | Greater |
| `<` | Less |
| `and` | AND |
| `or` | OR |
| `not` | NOT |

---

## 📚 BUILT-IN FUNCTIONS

| Function | Purpose |
|----------|---------|
| `print(x)` | Output |
| `len(x)` | Length |
| `sort(list)` | Sort |
| `reverse(x)` | Reverse |
| `contains(x, y)` | Check exists |
| `append(x, y)` | Add to list |

---

## 💡 COMMON PATTERNS

### Sum a List
```killer
total = 0
for n in numbers
    total = total + n
```

### Count Occurrences
```killer
count = 0
for item in list
    if item == target
        count = count + 1
```

### Find Maximum
```killer
max_val = numbers[0]
for n in numbers
    if n > max_val
        max_val = n
```

---

## 🐛 DEBUGGING

```killer
print("x = " + x)           # Check value
print("Is x > 0? " + (x > 0))  # Check condition
result = function(args)
print("Result: " + result)
```

---

## ❌ COMMON MISTAKES

| Wrong | Correct |
|-------|---------|
| `let x = 5` | `x = 5` |
| `fn main()` | `kfn main` |
| `if x = 5` | `if x == 5` |
| `print x` | `print(x)` |
| Bad indent | Proper indent (4 spaces) |

---

## 📖 COMPLETE EXAMPLES

### Grade Calculator
```killer
kfn get_grade(score: Int) -> String
    if score >= 90
        return "A"
    elif score >= 80
        return "B"
    else
        return "C"

kfn main
    print(get_grade(95))    # A
```

### Fibonacci
```killer
kfn fib(n: Int) -> Int
    if n <= 1
        return n
    return fib(n - 1) + fib(n - 2)

kfn main
    for i in 0..11
        print(fib(i) + " ")
```

### List Operations
```killer
kfn sum_list(nums: List) -> Int
    total = 0
    for n in nums
        total = total + n
    return total

kfn main
    data = [10, 20, 30]
    print(sum_list(data))   # 60
```

---

**Status:** ✅ Ready to Use  
**Print & Keep Handy!** 📌

