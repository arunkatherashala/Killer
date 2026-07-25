# DataQuality - ONLY for `quality` Type

## 🎯 The Critical Distinction

### **Regular Variable (NO quality features)**
```python
email = "alice@example.com"        # Regular string
phone = "555-123-4567"             # Regular string

# You CANNOT do:
email.validate_email()             # ❌ ERROR - regular var has no methods
phone.quality()                    # ❌ ERROR - regular var has no quality()
email.guarantee(Privacy)           # ❌ ERROR - regular var has no guarantee()
```

### **Quality Variable (HAS quality features)**
```python
quality email = "alice@example.com"    # Quality string - HAS tracking
quality phone = "555-123-4567"         # Quality string - HAS tracking

# You CAN do:
email.validate_email()             # ✅ OK - quality var has this method
phone.quality()                    # ✅ OK - quality var has quality()
email.guarantee(Privacy)           # ✅ OK - quality var has guarantee()
```

---

## 📊 Type Comparison

```
┌──────────────────────────────────────────────────────────────┐
│                    REGULAR VARIABLE                          │
├──────────────────────────────────────────────────────────────┤
│  email = "alice@example.com"                                 │
│  └─ Just stores the value                                    │
│  └─ No tracking, no validation, no guarantees                │
│  └─ Behaves like normal string                               │
│                                                               │
│  Can use: print(), concatenate, slice, etc (string methods) │
│  Cannot use: validate_*(), quality(), guarantee()           │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                    QUALITY VARIABLE                          │
├──────────────────────────────────────────────────────────────┤
│  quality email = "alice@example.com"                         │
│  └─ Stores value + tracks quality                           │
│  └─ Has built-in validation methods                         │
│  └─ Has guarantees and audit trail                          │
│  └─ Special DataQuality type                                │
│                                                               │
│  Can use: validate_*(), quality(), guarantee(), audit()     │
│  Cannot use: normal string methods directly                 │
│           (must use .value() to get raw string)             │
└──────────────────────────────────────────────────────────────┘
```

---

## 🔄 Converting Between Types

### **Regular → Quality**
```python
# Start with regular variable
email = get_email_from_form()

# Convert to quality for processing
quality validated_email = email
validated_email.validate_email()

# Now can check quality
if validated_email.quality() >= 0.9:
    save_user(validated_email)
```

### **Quality → Regular (Get Raw Value)**
```python
# Have quality variable
quality email = "alice@example.com"

# Get just the value (converts to regular string)
raw_email = email.value()    # Returns "alice@example.com"

# Now it's regular again - no quality methods
print(raw_email)             # ✓ Works
print(raw_email.quality())   # ❌ ERROR - no longer quality type
```

---

## 💡 Real-World Example: Form Processing

```python
def process_checkout(form_data):
    # form_data is just a dict of regular strings!
    # email = "alice@example.com"
    # phone = "555-1234"
    # etc.
    
    # Convert each to quality type
    quality email = form_data["email"]
    quality phone = form_data["phone"]
    quality amount = form_data["amount"]
    
    # NOW we can validate
    email.validate_email()
    phone.validate_phone()
    amount.validate_positive()
    
    # NOW we can check quality
    if email.quality() >= 0.9 and phone.quality() >= 0.9:
        # Safe to process
        save_order(email, phone, amount)
    else:
        # Show errors from quality variables
        print("Email: " + email.errors())
        print("Phone: " + phone.errors())
```

---

## ✅ Valid Code Examples (Quality Type Only)

### **Example 1: Simple Single Field**
```python
quality user_email = "bob@example.com"
user_email.validate_email()
print("Score: " + str(user_email.quality()))
```

### **Example 2: Multiple Fields**
```python
quality email = "alice@example.com"
quality phone = "555-1234"
quality age = "25"

email.validate_email()
phone.validate_phone()
age.validate_range(18, 120)

if email.quality() >= 0.8 and phone.quality() >= 0.8:
    print("All good!")
```

### **Example 3: With Guarantees**
```python
quality password = hash_password(input)
password.guarantee(Privacy)
password.guarantee(Encryption)

if password.quality() >= 0.95:
    save_password(password)
```

### **Example 4: With Audit**
```python
quality ssn = get_ssn()
ssn.audit("Received from form")
ssn.validate_format("SSN")
ssn.audit("Validated format")
ssn.guarantee(Privacy)
ssn.audit("Encrypted")

save_to_db(ssn)
```

---

## ❌ Invalid Code Examples (Regular Variables)

```python
# ❌ WRONG - regular variable trying to use quality methods
email = "alice@example.com"
email.validate_email()        # ERROR - email is not quality type
print(email.quality())        # ERROR - no quality() method
email.guarantee(Privacy)      # ERROR - no guarantee() method

# ❌ WRONG - forgetting the quality keyword
validated = "test@test.com"   # This is regular, NOT quality!
validated.validate_email()    # ERROR - not a quality variable

# ❌ WRONG - mixing regular and quality
x = "value"                        # Regular variable
quality y = "value"               # Quality variable
x.validate_format()              # ERROR - x is not quality
if x.quality() >= 0.8:           # ERROR - x is not quality
    do_something()
```

---

## 🎓 Decision Flow: When to Use Quality Type

```
┌─────────────────────────────────┐
│  Do I need to validate data?    │
├─────────────────────────────────┤
│         YES                      │
│           ▼                      │
│  Use: quality type              │
│  quality x = value              │
│  x.validate_*()                 │
│  Check quality before using     │
│                                  │
│         NO                       │
│           ▼                      │
│  Use: regular variable          │
│  x = value                      │
│  Just use normal operations     │
└─────────────────────────────────┘
```

---

## 📋 Complete Example: Before & After

### **BEFORE (Dangerous - No Quality Checking)**
```python
def save_user(form_data):
    email = form_data["email"]      # Just a string - UNTRUSTED
    phone = form_data["phone"]      # Just a string - UNTRUSTED
    
    # Saving directly without validation!
    db.insert({
        "email": email,
        "phone": phone
    })
    # PROBLEM: What if email is invalid? Could save bad data!
```

### **AFTER (Safe - Using Quality Type)**
```python
def save_user(form_data):
    # Convert to quality type
    quality email = form_data["email"]      # Now TRACKED
    quality phone = form_data["phone"]      # Now TRACKED
    
    # Validate
    email.validate_email()
    phone.validate_phone()
    
    # Record guarantees
    email.guarantee(Consistency)    # Can only have one per account
    phone.guarantee(Consistency)    # Can only have one per account
    
    # Record audit
    email.audit("From checkout form")
    phone.audit("From checkout form")
    
    # Check quality before saving
    if email.quality() >= 0.90 and phone.quality() >= 0.90:
        db.insert({
            "email": email.value(),    # Get raw value
            "phone": phone.value(),    # Get raw value
            "audit": {
                "email_trail": email.audit_trail(),
                "phone_trail": phone.audit_trail()
            }
        })
        print("✅ User saved successfully")
    else:
        print("❌ validation failed:")
        print("Email issues: " + email.errors())
        print("Phone issues: " + phone.errors())
```

---

## 🔑 Key Rules

| Rule | Regular Variable | Quality Variable |
|------|-----------------|-----------------|
| **Creation** | `x = value` | `quality x = value` |
| **Validation** | ❌ No methods | ✅ `validate_*()` |
| **Quality Score** | ❌ No tracking | ✅ `quality()` returns score |
| **Guarantees** | ❌ Cannot set | ✅ `guarantee(Type)` |
| **Audit Trail** | ❌ No history | ✅ `audit(message)` |
| **Get Raw Value** | `x` directly | `x.value()` |
| **Get Quality Info** | N/A | `x.quality()`, `x.level()`, `x.errors()` |
| **Use Case** | Simple values | Important/trusted data |

---

## 🚀 Summary

**Quality type is ONLY for:**
- ✅ Form inputs (need validation)
- ✅ Database data (track quality)
- ✅ User inputs (verify correctness)
- ✅ Sensitive data (need guarantees)
- ✅ Compliance data (need audit trail)

**DO NOT use quality for:**
- ❌ Simple calculations (just use regular variables)
- ❌ Temporary loop variables (overhead not needed)
- ❌ Math operations (regular types are faster)
- ❌ Internal state you fully control (no validation needed)

---

## 📝 Template: How to Write Code

```python
# Step 1: Get input (regular variables)
form_input = get_form_data()     # Regular dict of strings
email_str = form_input["email"]  # Regular string

# Step 2: Convert to quality (if need to validate)
quality email = email_str        # Now tracked!

# Step 3: Validate (quality methods)
email.validate_email()           # Validation happens

# Step 4: Check (quality methods)
if email.quality() >= 0.9:      # Quality check
    save_safe(email)            # Safe to save
else:
    show_errors(email)          # Show what's wrong
```

That's it! 🎉

---

**Bottom Line**: 
- `email = "test@test.com"` → Regular, no quality tracking
- `quality email = "test@test.com"` → Quality tracked, can validate, can check score

Only `quality` type has the validation, guarantees, audit, and quality checking features!
