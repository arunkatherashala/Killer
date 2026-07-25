# Weeks 12-14: Contract Programming Weekly Schedules
## Detailed study plans with daily tasks and progress tracking

---

## WEEK 12: PRECONDITIONS AND POSTCONDITIONS

### Learning Objectives
- Understand preconditions and postconditions
- Write effective contracts for functions
- Verify contracts with tests
- Handle contract violations gracefully

### Weekly Time Allocation
- **Lectures/Reading:** 10 hours
- **Coding Exercises:** 25 hours
- **Problem Solving:** 20 hours
- **Projects/Integration:** 15 hours
- **Review/Reinforcement:** 5 hours
**Total: 75 hours**

---

## WEEK 12 - DAILY SCHEDULE

### Monday: Preconditions Fundamentals (12 hours)

**Morning Session (4 hours) - Lecture & Conceptual Understanding**
```
0900-0930: Lecture - What are Preconditions? (30 min)
  Topics:
  - Definition: conditions that must be true BEFORE execution
  - Purpose: catch invalid inputs early
  - Examples: x > 0, array.len() > 0, file is open
  - vs. just checking in implementation
  
0930-1000: Activity - Precondition Identification (30 min)
  Task: Given 10 functions, identify preconditions
  Example: sqrt(x) -> precondition: x >= 0
  Submit: list of preconditions with explanations
  
1000-1030: Discussion - Precondition Design (30 min)
  Why certain preconditions?
  Cost/benefit of checking preconditions
  When NOT to use preconditions
  
1030-1100: Break
1100-1200: Lecture - Implementing Preconditions (1 hour)
  Patterns:
  - if-check-return-error
  - assert! macro
  - custom validation functions
  - exception handling
  
1200-1300: Coding Example - Temperature Converter (1 hour)
  Live coding: safe_celsius_to_fahrenheit
  Show: precondition check, error handling, testing
```

**Afternoon Session (4 hours) - Guided Practice**
```
1400-1500: Guided Exercise 1 - Array Head (1 hour)
  Problem: safe_head(array) with precondition len > 0
  Instructor shows solution step by step
  Student follows along, then implements variant
  
1500-1530: Break/Q&A
1530-1700: Guided Exercise 2 - Safe Division (1.5 hours)
  Problem: divide(a, b) with:
    - Precondition: b != 0
    - Precondition: operands are valid numbers
    - Postcondition: quotient * divisor ≈ dividend
  Student must:
    - Identify all conditions
    - Implement checks
    - Write test cases
    
1700-1800: Exercise 3 - Student Choice (1 hour)
  Pick from:
  - parse_int(string) - valid digit string
  - find_element(array, index) - valid index
  - create_rectangle(w, h) - positive dimensions
  Implement with preconditions + tests
```

**Evening Session (4 hours) - Independent Work**
```
1900-2000: Code Review Session 1 (1 hour)
  Review 3 peer solutions
  Identify: good preconditions, missing checks, clear errors
  
2000-2100: Problem Set 12.1 (1 hour)
  Solve 5 problems from Problem Bank
  Each requires precondition identification + implementation
  
2100-2230: Refinement & Testing (1.5 hours)
  Improve today's code
  Add edge cases
  Improve error messages
  
2230-2300: Reflection & Summary (30 min)
  Verify: can explain preconditions
  Verify: can identify in functions
  Verify: can implement checking
  Prepare questions for tomorrow
```

### Tuesday: Postconditions Fundamentals (12 hours)

**Morning Session (4 hours) - Lecture & Conceptual Understanding**
```
0900-0930: Lecture - What are Postconditions? (30 min)
  Definition: conditions that must be true AFTER execution
  Purpose: guarantee output properties
  Examples: result >= 0, array is sorted, size increased by 1
  vs. just returning any result
  
0930-1000: Activity - Postcondition Identification (30 min)
  Task: Given 10 functions, identify postconditions
  Example: sort(array) -> postcondition: array is sorted
  
1000-1030: Discussion - Postcondition Strategy (30 min)
  How to verify postconditions
  Testing strategies
  Practical vs. complete verification
  
1030-1100: Break
1100-1200: Lecture - Implementing Postconditions (1 hour)
  Verification patterns:
  - Check properties of output
  - Verify transformations
  - Validate invariants maintained
  - Assertion-based verification
  
1200-1300: Coding Example - Sorting with Verification (1 hour)
  Live code: verified_sort function
  Verify: sorted property, length preserved, elements same
```

**Afternoon Session (4 hours) - Guided Practice**
```
1400-1500: Exercise 1 - List Reversal (1 hour)
  Postcondition: array[i] == result[n-1-i]
  Implement + verify all properties
  
1500-1530: Break
1530-1700: Exercise 2 - Square Root (1.5 hours)
  Precondition: x >= 0
  Postcondition: result >= 0 AND result² ≈ x
  Compare with Monday's exercise
  
1700-1800: Exercise 3 - Unique Elements (1 hour)
  Postcondition: no duplicates in result
  Postcondition: same elements as input
  Implement dual verification
```

**Evening Session (4 hours) - Independent Work**
```
1900-2000: Problem Set 12.2 (1 hour)
  Solve 5 problems from postcondition section
  Focus on verification implementation
  
2000-2100: Refinement (1 hour)
  Improve error messages
  Handle edge cases
  Optimize verification
  
2100-2200: Integration Mini-Project (1 hour)
  Write function with 2+ pre + 2+ post conditions
  Implement complete verification
  Write comprehensive tests
  
2200-2300: Review & Reflection (1 hour)
  Compare Day 1 & Day 2 understanding
  Create: Contract Specification Template
  Prepare: Day 3 mixed challenges
```

### Wednesday: Combined Pre + Post Conditions (12 hours)

**Morning Session (4 hours) - Advanced Concepts**
```
0900-1000: Lecture - Combining Contracts (1 hour)
  How preconditions enable postconditions
  Example: sort requires proper input, guarantees output
  Relationship between pre and post
  
1000-1100: Case Study - Integer Arithmetic (1 hour)
  multiply(a, b):
    - Precondition: both valid numbers
    - Postcondition: result == a * b (math)
  divide(a, b):
    - Precondition: b != 0
    - Postcondition: quotient * b + remainder == a
  
1100-1130: Break
1130-1230: Workshop - Contract Design (1 hour)
  Group: design contracts for complex operations
  Map merge, find closest pair, compute statistics
  
1230-1300: Debrief & Patterns (30 min)
```

**Afternoon Session (4 hours) - Guided Practice**
```
1400-1530: Major Exercise - String Operations (1.5 hours)
  Implement 3 string functions with contracts:
  - substring(s, start, end)
  - split_by(s, delimiter)
  - parse_csv_line(line)
  
1530-1600: Break & Peer Review (30 min)
1600-1730: Major Exercise - Collection Operations (1.5 hours)
  Implement 3 collection functions with contracts:
  - binary_search(array, target)
  - merge_sorted_lists(list1, list2)
  - group_by(items, key_fn)
  
1730-1800: Consolidation (30 min)
  Review: all contracts from today
  Identify: patterns
```

**Evening Session (4 hours) - Independent Work**
```
1900-2000: Problem Set 12.3 (1 hour)
  Solve 5 combined pre+post problems
  Real-world scenarios
  
2000-2100: Code Refactoring (1 hour)
  Take Monday+Tuesday code
  Add missing contracts
  Improve verification
  
2100-2200: Contract Library (1 hour)
  Create reusable contract helpers
  Common preconditions (non-empty, in range, etc.)
  Common postconditions (sorted, unique, etc.)
  
2200-2300: Integration & Testing (1 hour)
  Use new library in functions
  Verify: consistent contract checking
  Test: edge cases
```

### Thursday: Contract Violation Detection (12 hours)

**Morning Session (4 hours)**
```
0900-1000: Lecture - Violation Scenarios (1 hour)
  What happens when contract violated
  Prevention: check before execution
  Detection: identify when violation occurs
  Recovery: graceful error handling
  
1000-1100: Case Studies - Real Violations (1 hour)
  Example: SQL injection (precondition violation)
  Example: Buffer overflow (postcondition violation)
  Example: Race condition (invariant violation)
  
1100-1130: Break
1130-1230: Error Design (1 hour)
  Designing meaningful error messages
  Context: what failed
  Cause: why it failed
  Resolution: how to fix
  
1230-1300: Debrief
```

**Afternoon Session (4 hours) - Exercises**
```
1400-1530: Exercise 1 - Bug Hunting (1.5 hours)
  Given: implementations with contract violations
  Task: identify violations, explain error
  Fix: implement corrections
  
1530-1600: Break
1600-1700: Exercise 2 - Error Message Design (1 hour)
  For various contracts, design error messages
  Criteria: clear, helpful, actionable
  
1700-1800: Exercise 3 - Defensive Programming (1 hour)
  Rewrite functions to be more robust
  Validate all preconditions thoroughly
  Verify postconditions extensively
```

**Evening Session (4 hours) - Projects**
```
1900-2000: Problem Set 12.4 (1 hour)
  Solve violation identification problems
  
2000-2200: Week 12 Integration Project (2 hours)
  Implement: URL Parser
  Preconditions:
    - valid URL format
    - valid scheme
    - non-empty host
  Postconditions:
    - can extract components
    - can reconstruct URL
    - components valid
  
2200-2300: Testing & Documentation (1 hour)
  Comprehensive test suite
  Document all contracts
```

### Friday: Advanced Contracts & Real-World Patterns (12 hours)

**Morning Session (3 hours) - Advanced Topics**
```
0900-1000: Lecture - Complex Contracts (1 hour)
  Dependent preconditions: if A then B
  Parameterized contracts: contracts with variables
  Assume/Guarantee contracts
  
1000-1100: Workshop - Real-World Scenarios (1 hour)
  Contracts for: file operations, network requests, database
  Thread-safe contracts
  Resource management contracts
  
1100-1130: Debrief
```

**Afternoon Session (4 hours) - Capstone Project**
```
1400-1530: Design Phase (1.5 hours)
  Define contracts for: Shopping Cart System
  Preconditions, postconditions for:
  - add_item(cart, product) 
  - remove_item(cart, item)
  - apply_discount(cart, code)
  - compute_total(cart)
  
1530-1600: Break
1600-1800: Implementation Phase (2 hours)
  Implement all contract logic
  Comprehensive testing
```

**Evening Session (5 hours) - Week 12 Wrap-up**
```
1900-2000: Testing & Refinement (1 hour)
2000-2100: Code Review (1 hour)
2100-2200: Documentation (1 hour)
  Document: all contracts
  Create: contract reference
  Lessons learned
  
2200-2300: Week 12 Reflection (1 hour)
  Self-assessment:
  - Can identify preconditions? [5/5]
  - Can implement checks? [5/5]
  - Can design postconditions? [5/5]
  - Understand contracts deeply? [5/5]
  Create: Week 12 Summary (1 page)
```

**Weekend: Review & Preparation**
```
Saturday: 5 hours
- Review all Week 12 code
- Solve additional 10 problems
- Practice: contract design for new problems
- Prepare: Week 13 pre-reading

Sunday: 4 hours
- Refactor Week 12 code
- Polish documentation
- Create: reusable contract patterns library
- Review: Week 13 topics
```

---

## WEEK 13: CLASS INVARIANTS & OBJECT CONTRACTS
### (Similar detailed breakdown with daily tasks)

### Key Daily Themes
- **Monday:** Simple invariants (counter, queue)
- **Tuesday:** Invariants with methods (contracts per method)
- **Wednesday:** Complex invariants (trees, dynamic structures)
- **Thursday:** Inheritance and LSP (derived classes)
- **Friday-Weekend:** Capstone project + review

---

## WEEK 14: AUTOMATED VERIFICATION & TESTING
### (Similar detailed breakdown)

### Key Daily Themes
- **Monday:** Test case generation from contracts
- **Tuesday:** Verification framework implementation
- **Wednesday:** Property-based testing
- **Thursday:** Mutation testing + advanced
- **Friday-Weekend:** Integration project + mastery demo

---

## DAILY HABIT CHECKLIST

Every day, verify you have:

### Morning Preparation (15 min)
- [ ] Review learning objectives
- [ ] Read relevant lecture material
- [ ] Prepare development environment
- [ ] Set daily goal

### Coding Practice (3+ hours)
- [ ] Complete watched exercises
- [ ] Solve problems from problem bank
- [ ] Implement practice projects
- [ ] Test comprehensively

### Review & Reflection (30 min)
- [ ] Verify contract understanding
- [ ] Document learnings
- [ ] Identify gaps
- [ ] Plan next day

---

## PROGRESS TRACKING TEMPLATE

### Weekly Progress Sheet

```
WEEK: [12/13/14]
DATE: Start _____ End _____

LEARNING OBJECTIVES:
[ ] Understand concept A
[ ] Can implement pattern B
[ ] Can design contracts for C
[ ] Can verify/test D

PROBLEMS COMPLETED:
- Category 1: ___ / 30 problems
- Category 2: ___ / 30 problems
- Category 3: ___ / 30 problems
- Category 4: ___ / 30 problems
TOTAL: ___ / 120 problems

PROJECTS COMPLETED:
[ ] Day 1-2 exercises
[ ] Day 3-4 exercises
[ ] Day 5 capstone
[ ] Extra credit projects

SELF-ASSESSMENT:
Contract Design:     [1] [2] [3] [4] [5]
Contract Verification: [1] [2] [3] [4] [5]
Error Handling:      [1] [2] [3] [4] [5]
Real-World Application: [1] [2] [3] [4] [5]

QUESTIONS/GAPS:
1. ...
2. ...
3. ...

NEXT WEEK PREP:
- [ ] Read materials
- [ ] Review this week's highlights
- [ ] Prepare questions
```

---

## ASSESSMENT & MILESTONES

### Week 12 Milestone
- [ ] Complete 120 precondition+postcondition problems
- [ ] Design contracts for 5 new functions
- [ ] Implement comprehensive error handling
- [ ] Explain contracts clearly

### Week 13 Milestone
- [ ] Design 10+ invariants
- [ ] Build 3 invariant-based classes
- [ ] Implement class contracts
- [ ] Understand inheritance + contracts

### Week 14 Milestone
- [ ] Generate test suites from contracts
- [ ] Build verification framework
- [ ] Implement property-based testing
- [ ] Apply contracts in real systems

---

## Success Criteria

By end of Weeks 12-14, you should:

✓ **Week 12 Goals (Preconditions & Postconditions)**
  1. Identify pre/postconditions in any function
  2. Design contracts for new functions
  3. Implement robust checking
  4. Handle violations gracefully
  5. Explain trade-offs

✓ **Week 13 Goals (Class Invariants)**
  1. Design appropriate invariants
  2. Enforce in all methods
  3. Handle inheritance correctly
  4. Build complex invariant systems
  5. Optimize verification

✓ **Week 14 Goals (Automated Verification)**
  1. Generate test cases from contracts
  2. Build verification frameworks
  3. Apply property-based testing
  4. Detect contract violations
  5. Integrate with development

---

## Week-by-Week Problem Targets

| Week | Target | Progress | Status |
|------|--------|----------|--------|
| 12   | 120 problems | ___/120 | |
| 13   | 140 problems | ___/140 | |
| 14   | 140 problems | ___/140 | |
| **Total** | **400 problems** | ___/400 | ✓ Master when complete |

---

## Resource Links

### Lecture Videos
- [Preconditions Fundamentals](#)
- [Postconditions Verification](#)
- [Class Invariants](#)
- etc.

### Read Required
- CONTRACT_PROGRAMMING_WEEKS_12_14.md
- PROBLEM_BANK_WEEKS_12_14.killer
- Eiffel documentation on DbC

### Practice Materials
- contract_programming.rs (framework)
- contract_exercises.rs (hands-on)
- Weekly problem sets (120+140+140)
