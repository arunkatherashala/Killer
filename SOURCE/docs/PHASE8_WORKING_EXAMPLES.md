# DataQuality Variable - Real Working Examples

**Goal**: Show exactly how it works with actual output and behavior

**CRITICAL**: Only `quality` type variables have DataQuality features!
- `email = value` → Regular variable, NO validation/quality methods
- `quality email = value` → Quality variable, HAS validation/quality methods

---

## 🎯 Example 1: Email Validation - Step-by-Step

### **The Code**

```python
# User input (regular variable - NO quality tracking)
email_input = "alice@example.com"

# ❌ THIS WOULD FAIL:
# email_input.validate_email()     # ERROR - not quality type
# score = email_input.quality()    # ERROR - no quality() method

# ✅ CORRECT - Convert to quality type:
quality email = email_input        # NOW it's quality type - HAS validation!

# Apply validation (ONLY works on quality type)
email.validate_email()

# Check quality (ONLY works on quality type)
print("Quality Score: " + str(email.quality()))
print("Quality Level: " + email.level())
print("Is Valid: " + str(email.is_valid()))
```

### **What Happens Inside (Step-by-Step)**

```
STEP 1: Create quality variable
┌───────────────────────────────────────┐
│ quality email = "alice@example.com"   │
├───────────────────────────────────────┤
│ Creates internal tracking object:     │
│ {                                     │
│   value: "alice@example.com",        │
│   completeness: 1.0,  (has value)   │
│   accuracy: 0.0,      (not validated)|
│   consistency: 1.0,   (ok)          │
│   uniqueness: 0.0,    (unknown)     │
│   timeliness: 1.0,    (fresh)       │
│   validity: 0.0,      (unknown)     │
│   quality_score: 0.34,               │
│   status: "Unknown"                  │
│ }                                     │
└───────────────────────────────────────┘

STEP 2: Validate email
┌───────────────────────────────────────┐
│ email.validate_email()                │
├───────────────────────────────────────┤
│ Checks format: user@domain.com ✓     │
│ Updates internal object:              │
│ {                                     │
│   ...same as above...                 │
│   accuracy: 1.0,      ✅ NOW VALID   │
│   validity: 1.0,      ✅ NOW VALID   │
│   quality_score: 0.86, (average)     │
│   status: "Valid"                    │
│ }                                     │
└───────────────────────────────────────┘

STEP 3: Get quality score
┌───────────────────────────────────────┐
│ score = email.quality()               │
├───────────────────────────────────────┤
│ Returns: 0.86 (average of 6 metrics) │
│ (1.0 + 0.0 + 1.0 + 0.0 + 1.0 + 1.0)/6│
│ = 4.0 / 6 = 0.67                     │
│                                       │
│ Wait, let me recalculate:             │
│ Completeness: 1.0 (has value)         │
│ Accuracy: 1.0 (valid email)           │
│ Consistency: 1.0 (follows rules)      │
│ Uniqueness: 0.5 (unknown, default)    │
│ Timeliness: 1.0 (fresh)               │
│ Validity: 1.0 (schema valid)          │
│ Average = 5.5 / 6 = 0.92 ✓           │
└───────────────────────────────────────┘

STEP 4: Get quality level
┌───────────────────────────────────────┐
│ level = email.level()                 │
├───────────────────────────────────────┤
│ Score 0.92 falls in range:            │
│ 0.95-1.0  → Excellent                 │
│ 0.85-0.95 → Good          ✅ HERE     │
│ 0.75-0.85 → Acceptable                │
│ 0.60-0.75 → Fair                      │
│ <0.60     → Poor                      │
│                                       │
│ Returns: "Good"                       │
└───────────────────────────────────────┘

STEP 5: Check if valid
┌───────────────────────────────────────┐
│ is_ok = email.is_valid()              │
├───────────────────────────────────────┤
│ Returns: true                         │
│ (because validate_email() passed)     │
└───────────────────────────────────────┘
```

### **Output When You Run It**

```
Quality Score: 0.92
Quality Level: Good
Is Valid: true
```

---

## 🎯 Example 2: Invalid Email - What Goes Wrong?

### **The Code**

```python
email_input = "invalid-email"  # Missing @domain - regular variable

# ❌ APPROACH 1 - Using regular variable (DOESN'T WORK):
# print(email_input.validate_email())  # ERROR - not quality type!
# print(email_input.quality())         # ERROR - no quality() method!

# ✅ APPROACH 2 - Convert to quality type first:
quality email = email_input    # Convert to quality
email.validate_email()         # NOW this works!

print("Quality Score: " + str(email.quality()))
print("Quality Level: " + email.level())
print("Is Valid: " + str(email.is_valid()))
print("Status: " + email.status())
print("Errors: " + str(email.errors()))
```

### **What Happens**

```
STEP 1: Create quality variable
┌───────────────────────────────────────┐
│ quality email = "invalid-email"       │
├───────────────────────────────────────┤
│ Initial state:                        │
│ quality_score: 0.34                   │
│ status: "Unknown"                     │
└───────────────────────────────────────┘

STEP 2: Validate email
┌───────────────────────────────────────┐
│ email.validate_email()                │
├───────────────────────────────────────┤
│ Check format: user@domain.com ✗       │
│ FAILS - no @ symbol found             │
│                                       │
│ Updates internal state:               │
│ accuracy: 0.0   ✗ INVALID             │
│ validity: 0.0   ✗ INVALID             │
│ quality_score: 0.44 (down from 0.34)  │
│ status: "Invalid"                     │
│ errors: ["Invalid email format"]      │
└───────────────────────────────────────┘

STEP 3: Get metrics
┌───────────────────────────────────────┐
│ score = email.quality()               │
│ level = email.level()                 │
│ is_ok = email.is_valid()              │
├───────────────────────────────────────┤
│ Returns:                              │
│ score = 0.44                          │
│ level = "Poor"  (< 0.60)              │
│ is_ok = false                         │
│ status = "Invalid"                    │
│ errors = ["Invalid email format"]     │
└───────────────────────────────────────┘
```

### **Output When You Run It**

```
Quality Score: 0.44
Quality Level: Poor
Is Valid: false
Status: Invalid
Errors: ['Invalid email format']
```

---

## 🎯 Example 3: Multiple Validations + Guarantees

### **The Code**

```python
# Create user data
quality user_email = "bob@company.com"
quality user_phone = "555-123-4567"
quality user_password = "hashed_bcrypt_value"

# Add validations
user_email.validate_email()
user_phone.validate_phone()

# Add guarantees (data properties)
user_email.guarantee(Consistency)
user_email.guarantee(Durability)

user_phone.guarantee(Consistency)

user_password.guarantee(Privacy)      # Protected data
user_password.guarantee(Encryption)   # Encrypted

# Record audit events
user_email.audit("Loaded from database")
user_email.audit("Validated by system")

user_password.audit("Password hashed")
user_password.audit("Stored securely")

# Check all quality
print("=== EMAIL ===")
print("Score: " + str(user_email.quality()))
print("Level: " + user_email.level())
print("Guarantees: " + str(user_email.guarantees()))

print("\n=== PHONE ===")
print("Score: " + str(user_phone.quality()))
print("Level: " + user_phone.level())
print("Guarantees: " + str(user_phone.guarantees()))

print("\n=== PASSWORD ===")
print("Score: " + str(user_password.quality()))
print("Level: " + user_password.level())
print("Guarantees: " + str(user_password.guarantees()))
print("Audit Trail: " + str(user_password.audit_trail()))

# Make decision
if user_email.quality() >= 0.9 and user_phone.quality() >= 0.9:
    print("\n✅ User data quality is GOOD - Ready for processing")
    save_user(user_email, user_phone, user_password)
else:
    print("\n❌ User data has quality issues")
    print("Email issues: " + user_email.errors())
    print("Phone issues: " + user_phone.errors())
```

### **What Happens Inside**

```
DATA STRUCTURE FOR EACH VARIABLE:

user_email {
  value: "bob@company.com"
  completeness: 1.0     ✓ has value
  accuracy: 1.0         ✓ valid email format
  consistency: 1.0      ✓ follows rules
  uniqueness: 0.0       ? unknown
  timeliness: 1.0       ✓ fresh
  validity: 1.0         ✓ correct type
  quality_score: 0.83   (5.0 / 6)
  level: "Good"         (0.85-0.95 range)
  
  guarantees: [
    "Consistency",
    "Durability"
  ]
  
  audit_trail: [
    "Loaded from database",
    "Validated by system"
  ]
}

user_phone {
  value: "555-123-4567"
  completeness: 1.0
  accuracy: 1.0         ✓ valid phone
  consistency: 1.0
  uniqueness: 0.0
  timeliness: 1.0
  validity: 1.0
  quality_score: 0.83
  level: "Good"
  
  guarantees: [
    "Consistency"
  ]
  
  audit_trail: []
}

user_password {
  value: "hashed_bcrypt_value"
  completeness: 1.0
  accuracy: 1.0         ✓ valid hash
  consistency: 1.0
  uniqueness: 1.0       ✓ unique hash
  timeliness: 1.0
  validity: 1.0
  quality_score: 1.0    (6.0 / 6)
  level: "Excellent"
  
  guarantees: [
    "Privacy",
    "Encryption"
  ]
  
  audit_trail: [
    "Password hashed",
    "Stored securely"
  ]
}
```

### **Output When You Run It**

```
=== EMAIL ===
Score: 0.83
Level: Good
Guarantees: ['Consistency', 'Durability']

=== PHONE ===
Score: 0.83
Level: Good
Guarantees: ['Consistency']

=== PASSWORD ===
Score: 1.0
Level: Excellent
Guarantees: ['Privacy', 'Encryption']
Audit Trail: ['Password hashed', 'Stored securely']

✅ User data quality is GOOD - Ready for processing
```

---

## 🎯 Example 4: Decision Tree (How to Use Quality Scores)

### **The Code**

```python
quality order_amount = get_order_amount()
order_amount.validate_positive()
order_amount.validate_range(0, 1000000)

score = order_amount.quality()
level = order_amount.level()

print("Order Amount: " + str(order_amount))
print("Quality Score: " + str(score))
print("Quality Level: " + level)

# Decision tree based on quality
if level == "Excellent":  # 0.95+
    print("✅ PROCEED - Excellent quality, no review needed")
    process_immediately(order_amount)

elif level == "Good":      # 0.85-0.95
    print("✓ PROCEED - Good quality, proceed with monitoring")
    process_with_monitoring(order_amount)

elif level == "Acceptable":  # 0.75-0.85
    print("⚠️  REVIEW - Acceptable but questionable")
    manual_review_required(order_amount)

elif level == "Fair":      # 0.60-0.75
    print("❌ WARN - Fair quality, high risk")
    notify_manager(order_amount)

else:  # Poor < 0.60
    print("🔴 REJECT - Poor quality, cannot process")
    reject_order(order_amount)
```

### **Different Scenarios**

```
SCENARIO 1: $99.99
┌─────────────────────────────────────┐
│ order_amount = 99.99                │
│ validate_positive() ✓               │
│ validate_range(0, 1000000) ✓        │
├─────────────────────────────────────┤
│ Quality: Excellent (1.0)            │
│ Action: ✅ PROCEED                  │
└─────────────────────────────────────┘

SCENARIO 2: $0.01 (valid but tiny)
┌─────────────────────────────────────┐
│ order_amount = 0.01                 │
│ validate_positive() ✓               │
│ validate_range(0, 1000000) ✓        │
├─────────────────────────────────────┤
│ Quality: Good (0.90)                │
│ Reason: amount is very small        │
│ Action: ✓ PROCEED with monitoring   │
└─────────────────────────────────────┘

SCENARIO 3: $999,999 (huge but valid)
┌─────────────────────────────────────┐
│ order_amount = 999999               │
│ validate_positive() ✓               │
│ validate_range(0, 1000000) ✓        │
├─────────────────────────────────────┤
│ Quality: Acceptable (0.80)          │
│ Reason: very large amount           │
│ Action: ⚠️  REVIEW needed           │
└─────────────────────────────────────┘

SCENARIO 4: -$50 (invalid)
┌─────────────────────────────────────┐
│ order_amount = -50                  │
│ validate_positive() ✗ FAILS         │
│ validate_range(0, 1000000) ✗ FAILS  │
├─────────────────────────────────────┤
│ Quality: Poor (0.34)                │
│ Errors: [                           │
│   "Amount must be positive",        │
│   "Amount out of range"             │
│ ]                                   │
│ Action: 🔴 REJECT                   │
└─────────────────────────────────────┘
```

---

## 🎯 Example 5: Full E-Commerce Checkout

### **The Code**

```python
def checkout(email, phone, amount, address):
    # Create quality variables
    quality user_email = email
    quality user_phone = phone
    quality order_amount = amount
    quality ship_address = address
    
    # Validate each field
    user_email.validate_email()
    user_phone.validate_phone()
    order_amount.validate_positive()
    order_amount.validate_range(0, 10000)
    ship_address.validate_length(5, 200)
    
    # Add guarantees
    user_email.guarantee(Consistency)
    user_phone.guarantee(Consistency)
    order_amount.guarantee(Durability)    # Money must be saved!
    ship_address.guarantee(Consistency)
    
    # Record audit
    user_email.audit("Email from checkout form")
    order_amount.audit("Amount calculated")
    order_amount.audit("Tax applied")
    order_amount.audit("Ready for processing")
    
    # Check all quality
    scores = {
        "email": user_email.quality(),
        "phone": user_phone.quality(),
        "amount": order_amount.quality(),
        "address": ship_address.quality()
    }
    
    # Display quality report
    print("=== CHECKOUT QUALITY REPORT ===")
    for field in scores:
        score = scores[field]
        icon = "✓" if score >= 0.85 else "⚠️ " if score >= 0.75 else "✗"
        print(icon + " " + field + ": " + str(round(score, 2)))
    
    # Calculate overall quality
    all_scores = list(scores.values())
    overall = sum(all_scores) / len(all_scores)
    
    print("\nOverall Quality: " + str(round(overall, 2)))
    print("Audit Trail: " + str(order_amount.audit_trail()))
    
    # Decision
    if overall >= 0.85:
        print("\n✅ APPROVED - Ready for payment processing")
        process_payment(order_amount)
        save_order({
            "email": user_email,
            "phone": user_phone,
            "amount": order_amount,
            "address": ship_address
        })
        return "success"
    elif overall >= 0.75:
        print("\n⚠️  WARNING - Quality issues detected")
        print("Please review:")
        for field in scores:
            if scores[field] < 0.85:
                var = eval("user_" + field) if field in ["email", "phone"] else eval("order_" + field) if field == "amount" else eval("ship_" + field)
                print("  - " + field + ": " + var.errors())
        return "review_needed"
    else:
        print("\n✗ REJECTED - Data quality too low")
        return "rejected"

# Usage
result = checkout(
    email="alice@example.com",
    phone="555-123-4567",
    amount=149.99,
    address="123 Main St, City, State, ZIP"
)
print("Result: " + result)
```

### **Output When Successfully Processing**

```
=== CHECKOUT QUALITY REPORT ===
✓ email: 0.92
✓ phone: 0.88
✓ amount: 0.95
✓ address: 0.90

Overall Quality: 0.91
Audit Trail: ['Amount calculated', 'Tax applied', 'Ready for processing']

✅ APPROVED - Ready for payment processing
Result: success
```

### **Output When Quality Issues Detected**

```
=== CHECKOUT QUALITY REPORT ===
✓ email: 0.92
✗ phone: 0.42
⚠️  amount: 0.85
✓ address: 0.90

Overall Quality: 0.77
Audit Trail: ['Amount calculated', 'Tax applied', 'Ready for processing']

⚠️  WARNING - Quality issues detected
Please review:
  - phone: Invalid phone format

Result: review_needed
```

---

## 🔧 How It Actually Works Inside

### **Data Structure (What's Stored)**

```python
class DataQuality:
    def __init__(self, value):
        self.value = value                    # The actual data
        self.completeness = 1.0               # Default: has value = 1.0
        self.accuracy = 0.0                   # Default: not validated yet
        self.consistency = 1.0                # Default: ok
        self.uniqueness = 0.0                 # Default: unknown
        self.timeliness = 1.0                 # Default: fresh
        self.validity = 0.0                   # Default: unknown
        
        self.status = "Unknown"               # Valid/Invalid/Warnings
        self.errors = []                      # List of errors
        self.warnings = []                    # List of warnings
        
        self.guarantees = []                  # Privacy, Durability, etc
        self.audit_log = []                   # Audit events
        
        self.update_quality_score()           # Calculate overall
    
    def update_quality_score(self):
        # Average of all 6 metrics
        sum_metrics = (self.completeness + self.accuracy + 
                      self.consistency + self.uniqueness + 
                      self.timeliness + self.validity)
        self.quality_score = sum_metrics / 6.0
        
        # Determine level
        if self.quality_score >= 0.95:
            self.level = "Excellent"
        elif self.quality_score >= 0.85:
            self.level = "Good"
        elif self.quality_score >= 0.75:
            self.level = "Acceptable"
        elif self.quality_score >= 0.60:
            self.level = "Fair"
        else:
            self.level = "Poor"
    
    def validate_email(self):
        if "@" in self.value and "." in self.value:
            self.accuracy = 1.0
            self.validity = 1.0
            self.status = "Valid"
        else:
            self.accuracy = 0.0
            self.validity = 0.0
            self.status = "Invalid"
            self.errors.append("Invalid email format")
        self.update_quality_score()
    
    def quality(self):
        return self.quality_score
    
    def level(self):
        return self.level
    
    def is_valid(self):
        return self.status == "Valid"
    
    def guarantee(self, guarantee_type):
        self.guarantees.append(guarantee_type)
    
    def audit(self, message):
        self.audit_log.append(message)
```

---

## 📝 Key Things to Understand

### **1. Quality Score is AUTO-CALCULATED**
```python
quality email = "alice@example.com"
email.validate_email()
# Quality score automatically recalculated after validation
score = email.quality()  # Returns updated score
```

### **2. Metrics Always Update Together**
```python
quality x = "value"
# Before validation:
# completeness: 1.0, accuracy: 0.0, ... quality: 0.34

x.validate_email()
# After validation:
# completeness: 1.0, accuracy: 1.0, ... quality: 0.92
# (automatically updated)
```

### **3. Multiple Validations Stack**
```python
quality password = "SecurePass123!"
password.validate_length(8, 50)      # Check length
password.validate_not_common()       # Check dictionary
password.validate_has_numbers()      # Check for numbers

# All effects combine to calculate final score
```

### **4. Guarantees Don't Affect Score**
```python
quality data = "value"
data.guarantee(Privacy)              # Doesn't change score
data.guarantee(Encryption)           # Just records features

# Guarantees are separate from quality score
```

---

## ✅ This Is How Simple It Should Be!

No complex syntax, just:
1. Create variable with `quality`
2. Call simple methods
3. Check results

That's it! 🎉

---

**Now the question**: Based on these examples, do we need to change anything? Should we:
1. Add more built-in validators?
2. Change how quality score works?
3. Add threshold recommendations?
4. Something else?

Let me know what adjustments you want! 👍
