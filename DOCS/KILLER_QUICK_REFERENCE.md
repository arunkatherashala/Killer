# ⚡ KILLER v1.0 - QUICK REFERENCE (One Page)

**Print this page and keep it handy!**

---

## 🚀 BASIC SETUP

```bash
killer.exe program.killer    # Run a program
killer.exe --version         # Check version
```

---

## 📝 HELLO WORLD

```killer
kfn main
    print("Hello, World!")
```

---

## 🔢 VARIABLES & TYPES

| Type | Example |
|------|---------|
| Integer | `x = 42` |
| Float | `pi = 3.14` |
| String | `name = "Alice"` |
| Boolean | `active = true` |
| List | `nums = [1, 2, 3]` |
| Map | `person = {"name": "Alice"}` |

---

## ⚙️ FUNCTIONS

```killer
kfn name(param: Type) -> ReturnType
    return value

kfn main
    result = name(argument)
```

---

## 🔄 CONTROL FLOW

### If/Else
```killer
if condition
    # do something
elif another_condition
    # do something else
else
    # default
```

### While Loop
```killer
while condition
    # code
    condition = condition - 1
```

### For Loop
```killer
for i in 0..10
    print(i)

for item in list
    print(item)
```

---

## 📦 COLLECTIONS

### Lists
```killer
nums = [1, 2, 3]
nums.append(4)
print(nums[0])          # 1
print(len(nums))        # 4
for n in nums
    print(n)
```

### Maps
```killer
person = {"name": "Alice", "age": 30}
print(person["name"])   # Alice
person["email"] = "alice@example.com"

for key in keys(person)
    print(person[key])
```

---

## 🎨 STRINGS

| Operation | Example |
|-----------|---------|
| Concatenate | `"Hello " + "World"` |
| Interpolate | `"Hello $name"` |
| Length | `len("Alice")` |
| Uppercase | `"hello".upper()` |
| Lowercase | `"HELLO".lower()` |
| Reverse | `reverse("hello")` |

---

## 📚 BUILT-IN FUNCTIONS

| Function | Purpose |
|----------|---------|
| `print(x)` | Output value |
| `len(x)` | Get length |
| `sort(list)` | Sort list |
| `reverse(x)` | Reverse |
| `contains(x, y)` | Check if exists |
| `append(x, y)` | Add to list |
| `split(x, y)` | Split string |

---

## 🔗 OPERATORS

| Operator | Meaning |
|----------|---------|
| `+` | Add / Concatenate |
| `-` | Subtract |
| `*` | Multiply |
| `/` | Divide |
| `%` | Modulo |
| `==` | Equal |
| `!=` | Not equal |
| `>` | Greater than |
| `<` | Less than |
| `>=` | Greater or equal |
| `<=` | Less or equal |
| `and` | Logical AND |
| `or` | Logical OR |
| `not` | Logical NOT |

---

## 💡 COMMON PATTERNS

### Sum a List
```killer
total = 0
for n in numbers
    total = total + n
print(total)
```

### Count Item in List
```killer
count = 0
for item in list
    if item == target
        count = count + 1
print(count)
```

### Find Maximum
```killer
max_val = numbers[0]
for n in numbers
    if n > max_val
        max_val = n
print(max_val)
```

### Print List With Numbers
```killer
for i in 0..len(items)
    print(i + ": " + items[i])
```

---

## 🐛 DEBUGGING TIPS

```killer
# Print intermediate values
print("x = " + x)

# Check conditions
print("Is x > 0? " + (x > 0))

# Verify function calls
result = function_name(args)
print("Result: " + result)
```

---

## 🎯 PERFORMANCE TIPS

1. **Avoid Deep Recursion:** Use loops when possible
2. **Pre-calculate:** Don't repeat calculations
3. **Use Right Data Type:** Lists for sequences, Maps for lookup
4. **Minimize Operations:** Combine operations when safe

---

## ❌ COMMON MISTAKES

| Mistake | Correct |
|---------|---------|
| `let x = 5` | `x = 5` |
| `fn main() {}` | `kfn main` |
| `for i in 0 to 10` | `for i in 0..10` |
| `if x = 5` | `if x == 5` |
| `print x` | `print(x)` |
| Wrong indent | Proper indent (4 spaces) |

---

## 📖 LEARNING ORDER

1. **Step 1:** Basics (variables, print, types)
2. **Step 2:** Functions (create and call)
3. **Step 3:** Control flow (if, loops)
4. **Step 4:** Collections (lists, maps)
5. **Step 5:** Advanced (recursion, higher-order)

---

## 🎓 EXAMPLE PROGRAM

```killer
# Simple grade calculator
kfn get_grade(score: Int) -> String
    if score >= 90
        return "A"
    elif score >= 80
        return "B"
    elif score >= 70
        return "C"
    else
        return "F"

kfn main
    scores = [95, 87, 73]
    for score in scores
        grade = get_grade(score)
        print("Score: " + score + " Grade: " + grade)
```

---

**Status:** ✅ Ready  
**Keep this handy for quick reference!** 📌
