# KILLER v3.0 - QUICK TEST GUIDE (One Page)

## Installation (Choose One)

### Windows
```
cd C:\Users\skathera\Downloads\killer
.\killer-standalone-installer.bat
```

### macOS/Linux
```
cd ~/Downloads/killer
sudo bash killer-standalone-installer.sh
```

---

## 5 Minute Test

### Test 1: Say Hello
```bash
killer examples/01_hello.killer
```
✅ Expected: Sees welcome message with "Hello World"

### Test 2: Try Variables
```bash
killer examples/02_variables.killer
```
✅ Expected: Sees arithmetic results (10 + 5 = 15, etc.)

### Test 3: Create Your Own
**Save as: `mytest.killer`**
```killer
print("Testing Killer!");
x = 100;
y = 50;
z = x - y;
print(z);
```

Run: `killer mytest.killer`  
✅ Expected: Shows "Testing Killer!" and "50"

---

## Full Feature Test

### Test Arrays
```bash
killer examples/04_arrays.killer
```
✅ Expected: Shows array operations and indexing

### Test Functions
```bash
killer examples/05_functions.killer
```
✅ Expected: Shows function calls and returns

### Test Classes
```bash
killer examples/06_classes.killer
```
✅ Expected: Shows object creation and methods

---

## Success Criteria ✅

Your test passed if:
- [ ] Installation completes without errors
- [ ] `killer` command works from any folder
- [ ] Example programs run and show output
- [ ] Custom .killer files execute successfully
- [ ] No Python windows/prompts appear
- [ ] Output is correct and readable
- [ ] Program completes without crashing

---

## Syntax Reference

```killer
# Variables
name = "Killer";
version = 3.0;
ready = true;

# Arrays
arr = [1, 2, 3];
print(arr[0]);  # Output: 1

# Objects
person = {name: "Alice", age: 30};
print(person.name);  # Output: Alice

# Functions
fn add(a, b) {
    return a + b;
}
print(add(5, 3));  # Output: 8

# Loops
i = 0;
while (i < 3) {
    print(i);
    i = i + 1;
}  # Output: 0 1 2

# Conditionals
if (version >= 3) {
    print("Latest version!");
}

# String concat
msg = "Hello" + " " + "World";
print(msg);  # Output: Hello World
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "killer: command not found" | Run installer again or add to PATH |
| "File not found" | Check file path and .killer extension |
| Syntax error | Check Killer syntax (use `==` not `===`) |
| Python window appears | Normal - that's the interpreter |

---

## Report Template

**If something fails, provide:**

```
WHAT HAPPENED:
Command: killer myprogram.killer

ERROR MESSAGE:
[paste exact error]

EXPECTED:
[what should happen]

SYSTEM:
OS: Windows/macOS/Linux
Installation Path: C:\...\Killer or /usr/local/bin
```

---

**All tests passed? ✅ Killer is working perfectly!**

Need more help? See TESTING_GUIDE.md for detailed instructions.
