# DataQuality - Visual Workflow & Best Practices

## 📌 IMPORTANT: Quality Type Only!

**Only `quality` type variables have DataQuality features:**
```python
# ❌ Regular variable - NO quality methods:
email = "test@test.com"
email.validate_email()          # ERROR
email.quality()                 # ERROR

# ✅ Quality variable - HAS quality methods:
quality email = "test@test.com"
email.validate_email()          # ✓ Works!
email.quality()                 # ✓ Works!
```

---

## 🎯 The Standard Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│                    STEP 1: CREATE VARIABLE                      │
├─────────────────────────────────────────────────────────────────┤
│  quality email = user_input                                    │
│  └─ Internal state created automatically                        │
│     quality_score: 0.34 (incomplete validation)                │
│     status: "Unknown"                                           │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   STEP 2: VALIDATE/ADD RULES                    │
├─────────────────────────────────────────────────────────────────┤
│  email.validate_email()                                         │
│  email.validate_domain("company.com")                          │
│  └─ Quality score recalculated                                 │
│     quality_score: 0.92 (if valid)                            │
│     status: "Valid" or "Invalid"                               │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  STEP 3: ADD GUARANTEES (OPTIONAL)              │
├─────────────────────────────────────────────────────────────────┤
│  email.guarantee(Consistency)                                   │
│  email.guarantee(Durability)                                    │
│  └─ Records what we promise about the data                      │
│     (doesn't change score, just metadata)                       │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 STEP 4: ADD AUDIT TRAIL (OPTIONAL)              │
├─────────────────────────────────────────────────────────────────┤
│  email.audit("Loaded from form")                               │
│  email.audit("Validated by system")                            │
│  └─ Records history for compliance                              │
│     (doesn't change score, just history)                        │
└─────────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   STEP 5: CHECK & DECIDE                        │
├─────────────────────────────────────────────────────────────────┤
│  if email.quality() >= 0.85:                                   │
│      process_safe(email)          # Quality is good            │
│  else:                                                          │
│      manual_review(email)         # Need human review           │
│  └─ Make decision based on quality                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📊 Quality Score Breakdown (How It's Calculated)

### **The 6 Metrics**

```
┌──────────────┬────────────────────────────────┬─────────────────┐
│   Metric     │         Definition             │   Default       │
├──────────────┼────────────────────────────────┼─────────────────┤
│Completeness  │ Is all required data present?  │ 1.0 (has value) │
├──────────────┼────────────────────────────────┼─────────────────┤
│ Accuracy     │ Is the data correct/valid?     │ 0.0 (unknown)   │
├──────────────┼────────────────────────────────┼─────────────────┤
│Consistency   │ Does it follow all rules?      │ 1.0 (ok)        │
├──────────────┼────────────────────────────────┼─────────────────┤
│ Uniqueness   │ Is it unique (no duplicates)?  │ 0.0 (unknown)   │
├──────────────┼────────────────────────────────┼─────────────────┤
│ Timeliness   │ Is the data fresh/current?     │ 1.0 (new)       │
├──────────────┼────────────────────────────────┼─────────────────┤
│  Validity    │ Correct format/schema?         │ 0.0 (unknown)   │
└──────────────┴────────────────────────────────┴─────────────────┘

┌───────────────────────────────────────────────────────────────┐
│  QUALITY_SCORE = Average of all 6 metrics                     │
│  Example: (1.0 + 1.0 + 1.0 + 0.0 + 1.0 + 1.0) / 6 = 0.83    │
└───────────────────────────────────────────────────────────────┘
```

---

## 🎓 Best Practices

### **DO ✅**

```python
# 1. Create quality variable immediately when getting input
quality email = get_user_email()
quality amount = get_payment_amount()
quality address = get_shipping_address()

# 2. Validate right away
email.validate_email()
amount.validate_positive()
amount.validate_range(0, 10000)
address.validate_length(1, 500)

# 3. Check quality before using the data
if email.quality() >= 0.90:
    send_confirmation_email(email)
else:
    ask_user_to_verify(email)

# 4. Use guarantees for important data
quality password = hash_user_password(input)
password.guarantee(Privacy)
password.guarantee(Encryption)

# 5. Add audit trail for compliance
quality user_ssn = get_ssn()
user_ssn.audit("Received from form")
user_ssn.audit("Validated format")
user_ssn.audit("Encrypted before storage")

# 6. Multiple validations are fine
quality phone = get_phone()
phone.validate_phone()
phone.validate_country(country)
phone.validate_not_blocked()
```

### **DON'T ❌**

```python
# 1. DON'T treat quality variable like normal variable
quality x = 5
bad_result = x + 10           # ❌ Don't do this
better_result = x.value() + 10  # ✓ Do this instead

# 2. DON'T ignore quality score
quality email = get_email()
send_confirmation(email)      # ❌ Never do this
if email.quality() > 0.8:     # ✓ Always check first
    send_confirmation(email)

# 3. DON'T abuse guarantees
quality regular_data = "test"
regular_data.guarantee(Privacy)           # ❌ Don't lie
regular_data.guarantee(Encryption)        # ❌ Unless actually true

quality sensitive_data = encrypt(data)
sensitive_data.guarantee(Encryption)      # ✓ Only if it's true

# 4. DON'T forget to validate
quality credit_card = get_card()
save_to_database(credit_card)  # ❌ MISSING VALIDATION!
credit_card.validate_card()    # ✓ Always validate first
if credit_card.is_valid():
    save_to_database(credit_card)

# 5. DON'T mix old/new styles
x = get_data()               # ❌ Old style
if is_valid(x):              # ❌ Old style validation
    process(x)

quality x = get_data()       # ✓ New style
x.validate_format()          # ✓ New style validation
if x.is_valid():             # ✓ Built-in checking
    process(x)
```

---

## 🏗️ Common Patterns

### **Pattern 1: Form Validation**

```python
def validate_signup_form(email, password, phone):
    # Wrap all inputs
    quality form_email = email
    quality form_password = password
    quality form_phone = phone
    
    # Validate each
    form_email.validate_email()
    form_email.validate_not_disposable()
    
    form_password.validate_length(8, 128)
    form_password.validate_has_uppercase()
    form_password.validate_has_numbers()
    form_password.validate_has_special()
    
    form_phone.validate_phone()
    form_phone.validate_country("US")
    
    # Check all together
    fields = [form_email, form_password, form_phone]
    scores = [field.quality() for field in fields]
    
    if all(score >= 0.90 for score in scores):
        return "success"
    else:
        errors = []
        if form_email.quality() < 0.90:
            errors.append("Email issues: " + form_email.errors())
        if form_password.quality() < 0.90:
            errors.append("Password issues: " + form_password.errors())
        if form_phone.quality() < 0.90:
            errors.append("Phone issues: " + form_phone.errors())
        return "error: " + errors.join(", ")
```

### **Pattern 2: Database Save Decision**

```python
def save_user_to_db(user_data):
    # Wrap critical fields
    quality email = user_data["email"]
    quality phone = user_data["phone"]
    quality username = user_data["username"]
    
    email.validate_email()
    phone.validate_phone()
    username.validate_length(3, 30)
    username.validate_alphanumeric()
    
    email.guarantee(Consistency)    # Prevent duplicates
    username.guarantee(Consistency) # Prevent duplicates
    
    # Check quality thresholds
    if email.quality() < 0.85 or phone.quality() < 0.85:
        return "error: data not good enough for database"
    
    # Safe to save
    return db.save_user(email, phone, username)
```

### **Pattern 3: API Request Validation**

```python
def process_api_request(api_key, request_body, timestamp):
    quality key = api_key
    quality body = request_body
    quality time = timestamp
    
    key.validate_format("api_key_format")
    key.validate_not_expired()
    key.validate_has_permission("write")
    key.guarantee(Privacy)  # Token is sensitive
    
    body.validate_json()
    body.validate_schema()
    body.validate_size(1, 10000)  # Not too big
    
    time.validate_timestamp()
    time.validate_within_seconds(300)  # Within 5 min
    
    # All good?
    if key.quality() >= 0.95 and body.quality() >= 0.90:
        return process_request(body)
    else:
        return "Unauthorized or malformed request"
```

---

## 📈 Quality Thresholds (Recommended)

```
┌──────────────┬──────────┬─────────────────────────────────────┐
│  Quality     │ Decision │        Recommended Action           │
├──────────────┼──────────┼─────────────────────────────────────┤
│ 0.95 - 1.0   │ Go       │ ✅ Process immediately              │
│ Excellent    │          │    No review needed                 │
├──────────────┼──────────┼─────────────────────────────────────┤
│ 0.85 - 0.95  │ Go       │ ✓ Process with monitoring           │
│ Good         │          │   Log for review, but proceed       │
├──────────────┼──────────┼─────────────────────────────────────┤
│ 0.75 - 0.85  │ Caution  │ ⚠️  Manual review required          │
│ Acceptable   │          │    Have human check data            │
├──────────────┼──────────┼─────────────────────────────────────┤
│ 0.60 - 0.75  │ Warning  │ ❌ High risk, notify admin          │
│ Fair         │          │    Potential fraud or error         │
├──────────────┼──────────┼─────────────────────────────────────┤
│ < 0.60       │ Stop     │ 🔴 Reject outright                  │
│ Poor         │          │    Do not process                   │
└──────────────┴──────────┴─────────────────────────────────────┘
```

---

## 🔍 Debugging Quality Issues

### **Problem 1: Score Is Lower Than Expected**

```python
quality email = "test@example.com"
email.validate_email()

print("Score: " + str(email.quality()))
# Output: 0.67 (lower than expected!)

# Debug: Check each metric
print("Completeness: " + str(email._completeness))    # 1.0 ✓
print("Accuracy: " + str(email._accuracy))             # 1.0 ✓
print("Consistency: " + str(email._consistency))       # 1.0 ✓
print("Uniqueness: " + str(email._uniqueness))         # 0.0 ✗
print("Timeliness: " + str(email._timeliness))         # 1.0 ✓
print("Validity: " + str(email._validity))             # 1.0 ✓

# Ah! uniqueness is 0 because we haven't checked for duplicates
email.validate_unique()  # Check if email exists
# Now score should be higher: 0.83
```

### **Problem 2: is_valid() Returns False But I Can't See Errors**

```python
quality x = "some data"
x.validate_something()

if not x.is_valid():
    print("Not valid, but why?")
    print("Errors: " + str(x.errors()))
    print("Status: " + str(x.status()))
    print("All metrics: " + str(x.all_metrics()))
```

### **Problem 3: Data Is Valid But Quality Is Low**

```python
quality data = valid_data

# It passes all validations but score is 0.6?
# This means some metrics are unknown (0.0)

print("Metrics:")
print("  Completeness: " + data._completeness)
print("  Accuracy: " + data._accuracy)
print("  Consistency: " + data._consistency)
print("  Uniqueness: " + data._uniqueness)      # Maybe this?
print("  Timeliness: " + data._timeliness)
print("  Validity: " + data._validity)

# Solution: Add more validations to fill in unknown metrics
data.validate_unique()        # Sets uniqueness to 1.0
data.validate_timeliness()    # Confirms timeliness
# Score increases to 0.92
```

---

## 📋 Quick Reference Card

```
╔═══════════════════════════════════════════════════════════════╗
║                CREATE QUALITY VARIABLE                         ║
╠═══════════════════════════════════════════════════════════════╣
║  quality variable_name = value                                 ║
║  └─ Automatically initializes quality tracking                 ║
╠═══════════════════════════════════════════════════════════════╣
║                VALIDATION METHODS                              ║
╠═══════════════════════════════════════════════════════════════╣
║  variable.validate_email()                                    ║
║  variable.validate_phone()                                    ║
║  variable.validate_format(type)                               ║
║  variable.validate_range(min, max)                            ║
║  variable.validate_length(min, max)                           ║
║  variable.validate_unique()                                   ║
║  variable.validate_not_null()                                 ║
║  variable.validate_regex(pattern)                             ║
║  └─ Each updates quality_score automatically                  ║
╠═══════════════════════════════════════════════════════════════╣
║                INFORMATION METHODS                             ║
╠═══════════════════════════════════════════════════════════════╣
║  variable.quality()        → returns 0.0 to 1.0               ║
║  variable.level()          → returns "Excellent"/"Good"/etc   ║
║  variable.is_valid()       → returns true/false               ║
║  variable.errors()         → returns list of errors           ║
║  variable.status()         → returns "Valid"/"Invalid"/etc    ║
║  variable.guarantees()     → returns list of guarantees       ║
║  variable.audit_trail()    → returns history log              ║
╠═══════════════════════════════════════════════════════════════╣
║                METADATA METHODS                                ║
╠═══════════════════════════════════════════════════════════════╣
║  variable.guarantee(type)              # Add guarantee        ║
║  variable.audit(message)               # Add audit entry      ║
║  └─ These don't change quality score, just metadata           ║
╠═══════════════════════════════════════════════════════════════╣
║                DECISION MAKING                                 ║
╠═══════════════════════════════════════════════════════════════╣
║  if variable.quality() >= 0.95:  # Excellent                  ║
║      process_immediately(variable)                            ║
║  elif variable.quality() >= 0.85:  # Good                     ║
║      process_with_logging(variable)                           ║
║  elif variable.quality() >= 0.75:  # Acceptable               ║
║      require_manual_review(variable)                          ║
║  else:  # Poor/Fair                                           ║
║      reject_outright(variable)                                ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## ✨ Why This Design Works

### **Simple** ✅
- One action per line
- No brackets, no builders, no fancy syntax
- Python-like and intuitive

### **Trackable** ✅
- Always know the quality score
- Audit trail built in
- Decisions clear and explainable

### **Safe** ✅
- Can't forget validation (data clearly marked as quality/untrusted)
- Audit trail for compliance
- Guarantees promise what data IS

### **Automatic** ✅
- Quality score recalculates automatically
- No manual score updates
- Metrics always consistent

---

## 🎯 Next Steps?

These working examples show exactly how DataQuality should work. Do you want to:

1. **Implement** this design now? (Start coding Phase 8.1)
2. **Refine** something about the API?
3. **Add** more validators?
4. **Change** how quality scores are calculated?
5. **Something else**?

Let me know! 👍
