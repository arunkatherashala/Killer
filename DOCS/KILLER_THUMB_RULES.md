# KILLER THUMB RULES
## Timeless Principles That Never Become Obsolete

**Created**: March 18, 2026  
**Version**: 1.0  
**Status**: PERMANENT - Never becomes outdated  
**Purpose**: Guide all future development decisions

---

## WHAT ARE THUMB RULES?

**Thumb Rules** = General principles that are:
- ✅ Simple (easy to remember)
- ✅ Practical (work in real situations)
- ✅ Universal (apply across all contexts)
- ✅ Timeless (don't become outdated)
- ✅ Correct (proven by experience)

**They guide decisions, not mandate them.**

---

## SECTION 1: TESTING THUMB RULES

### Rule 1.1: Never Deploy Untested Code
**What**: ALWAYS test before releasing  
**Why**: Untested = unknown risks  
**When**: Every single time, no exceptions  
**How**: Follow ARU process (test 100%)  

**Good** ✅:
```
Build → Test → Document → Deploy
```

**Bad** ❌:
```
Build → Hope → Deploy
```

---

### Rule 1.2: Test Under Real Conditions
**What**: Test with real data, real load, real scenarios  
**Why**: Lab tests ≠ Production problems  
**When**: Before deployment  
**How**: Load tests, chaos tests, real user scenarios  

**Good** ✅:
```
Test with 1000 concurrent users
Test with corrupted database
Test with network latency
```

**Bad** ❌:
```
Test with single request
Test with perfect data
Test on developer laptop
```

---

### Rule 1.3: Test Edge Cases First
**What**: Test failures BEFORE success cases  
**Why**: Edge cases hide bugs that tests miss  
**When**: During development  
**How**: What if null? What if empty? What if corrupted?  

**Good** ✅:
```
Test null input
Test empty response
Test timeout after 10ms
Test network disconnect
```

**Bad** ❌:
```
Test happy path only
Skip error handling tests
Assume failures won't happen
```

---

### Rule 1.4: Regression Tests Are Your Insurance
**What**: Automated tests that catch regressions  
**Why**: Don't repeat same bugs  
**When**: Run every single build  
**How**: Baseline → Compare → Alert  

**Good** ✅:
```
Test Suite PASS
All metrics within baseline
No regressions detected
→ SAFE TO DEPLOY
```

**Bad** ❌:
```
No regression tests
Randomly break things
Fix same bug 3 times
```

---

## SECTION 2: CODE QUALITY THUMB RULES

### Rule 2.1: Simple is Better Than Complex
**What**: Choose simple solution over complex one  
**Why**: Simple code has fewer bugs  
**When**: Every decision  
**How**: Ask "Can this be simpler?"  

**Good** ✅:
```
if x == 0:
    return 0
```

**Bad** ❌:
```
if (x != undefined && x != null && x.value != NaN):
    if (x.value >= -epsilon && x.value <= epsilon):
        return 0
```

---

### Rule 2.2: Make it Work First, Optimize Later
**What**: Don't over-engineer early  
**Why**: Works > Not works, always  
**When**: Initial implementation  
**How**: Get it working, then optimize  

**Good** ✅:
```
1. Make feature work completely
2. Test all scenarios
3. Measure performance
4. Optimize bottleneck
```

**Bad** ❌:
```
1. Optimize before testing
2. Add features that don't exist yet
3. Guess what's slow
4. Rewrite when performance bad
```

---

### Rule 2.3: Code is Read More Than Written
**What**: Write code for humans, not computers  
**Why**: Maintainability > any optimization  
**When**: Always  
**How**: Clear names, good comments, simple logic  

**Good** ✅:
```
def get_agent_response(question: String) -> Response {
    // Search killer_db for similar questions
    // Find best matching pattern
    // Return synthesized response
}
```

**Bad** ❌:
```
def gr(q) {
    // Magic happens here
    x = db.s(q)
    return x[0]
}
```

---

## SECTION 3: DEPLOYMENT THUMB RULES

### Rule 3.1: Never Skip Testing to Ship Faster
**What**: Speed without testing = reckless  
**Why**: One bug costs more than testing time  
**When**: NEVER skip  
**How**: Faster tests, not fewer tests  

**Good** ✅:
```
Friday 5pm:      All tests pass
Saturday 10am:   Deploy with confidence
```

**Bad** ❌:
```
Friday 5pm:      "Testing is slow, ship it"
Saturday 3am:    Production meltdown
Sunday 10am:     "Why didn't we test?"
```

---

### Rule 3.2: Always Have a Rollback Plan
**What**: Know how to undo everything  
**Why**: Deployments sometimes fail  
**When**: Before deploying  
**How**: Tested rollback procedure  

**Good** ✅:
```
Deploy plan:  Version 1.1 → 1.2
Rollback plan: Version 1.2 → 1.1 (automatic)
Time to rollback: < 5 minutes
```

**Bad** ❌:
```
We can't rollback
Hope nothing breaks
Spend 6 hours debugging
```

---

### Rule 3.3: Monitor Everything After Deploy
**What**: Track system health constantly  
**Why**: Catch problems before users do  
**When**: Immediately after deploy  
**How**: Metrics, logs, alerts  

**Good** ✅:
```
Deploy → Monitor latency (catches spike in 5 sec)
Deploy → Monitor errors (catches crash in 1 sec)
Deploy → Track users (ensures they're not hit)
```

**Bad** ❌:
```
Deploy → Check email
Deploy → Go to lunch
Deploy → Get call 6 hours later "System is slow"
```

---

## SECTION 4: DOCUMENTATION THUMB RULES

### Rule 4.1: Document WHY, Not What
**What**: Explain decision reasoning, not code steps  
**Why**: "What" is obvious from code, "why" is not  
**When**: Always when writing comments  
**How**: Tell the story  

**Good** ✅:
```
# Using allocation pooling here because
# memory fragmentation was causing p99 latency spikes
# from 50ms to 200ms. Tests show pooling reduces to <60ms.
allocator = PoolAllocator()
```

**Bad** ❌:
```
# Create allocator
allocator = PoolAllocator()

# Use it
result = allocator.allocate(size)
```

---

### Rule 4.2: Example > Explanation
**What**: Show working code, not just theory  
**Why**: Developers learn faster from examples  
**When**: Always in documentation  
**How**: Real code, not pseudo-code  

**Good** ✅:
```
Question: How do I use KillerAgent?
Answer: 
  target/release/killer-native.exe agent.killer
  
  Then in agent.killer:
  let agent = KillerAgent()
  let response = agent.process("What is X?")
  print(response)
```

**Bad** ❌:
```
Question: How do I use KillerAgent?
Answer: Instantiate the agent, call process method
```

---

### Rule 4.3: Keep Documentation Close to Code
**What**: Don't separate docs from code  
**Why**: Separated docs become outdated quickly  
**When**: Always  
**How**: Comments in code, README in repo, docs in folder  

**Good** ✅:
```
src/killer_agent.rs         (code)
├── /// Documentation in code
├── tests/

README.md                   (quick start)
docs/AGENT_API.md          (detailed)
```

**Bad** ❌:
```
src/killer_agent.rs         (code - maybe outdated)
docs/Killer_v2_guide.docx  (probably outdated)
wiki/agent_docs.html       (definitely outdated)
```

---

## SECTION 5: TEAM COLLABORATION THUMB RULES

### Rule 5.1: Automate Everything Repeatable
**What**: If done twice, automate it  
**Why**: Humans make mistakes, computers don't  
**When**: After doing it manually once  
**How**: Scripts, CI/CD, tests  

**Good** ✅:
```
Manual Task:  Running regression tests
1 round: 30 minutes
Automated:    Push button → 5 minutes
Repeatable:   Every single time
```

**Bad** ❌:
```
Manual Task:  Running regression tests
1 round: 30 minutes
100 rounds:   50 hours
Team frustration: Maximum
```

---

### Rule 5.2: When in Doubt, Ask
**What**: Questions are free, mistakes are expensive  
**Why**: Asking prevents big problems  
**When**: Before doing uncertain work  
**How**: Clear communication  

**Good** ✅:
```
Developer: "Should I optimize latency or throughput first?"
Lead: "Test both, then optimize what hurts most"
Result: Right decision made
```

**Bad** ❌:
```
Developer: (guesses, optimizes wrong thing)
Lead: (finds out later)
Result: Wasted 2 weeks of work
```

---

### Rule 5.3: Share Knowledge Constantly
**What**: Document what you learn  
**Why**: Future you thanks present you  
**When**: As you learn  
**How**: Quick notes, team meetings, peer reviews  

**Good** ✅:
```
Tomorrow's problem = Yesterday's learning
Knowledge shared = Problem solved instantly
```

**Bad** ❌:
```
Same problem solved 3 times (different people)
Knowledge lost = Repeated work
```

---

## SECTION 6: PERFORMANCE THUMB RULES

### Rule 6.1: Measure Before Optimizing
**What**: Know what's slow BEFORE fixing  
**Why**: Guessing wastes time  
**When**: Before optimization  
**How**: Profiling, benchmarking, metrics  

**Good** ✅:
```
Measure: Database queries = 80% of time
Fix: Add caching
Result: Now 60% total time ✅
```

**Bad** ❌:
```
Guess: "Allocations are slow"
Fix: Add allocator
Measure: Still slow ❌
Reason: Database was the problem, we missed it
```

---

### Rule 6.2: Premature Optimization is Evil
**What**: Don't optimize before proving need  
**Why**: Optimization = complexity = bugs  
**When**: NEVER optimize first  
**How**: Build correct, then optimize bottlenecks  

**Good** ✅:
```
1. Correct algorithm
2. Tests pass
3. Measure performance
4. Find bottleneck
5. Optimize bottleneck
6. Verify improvement
```

**Bad** ❌:
```
1. Assume what's slow
2. Over-engineer
3. Create bugs
4. Tests fail
5. Discard everything
6. Start over
```

---

### Rule 6.3: Latency Matters More Than Throughput
**What**: Real-time impact > raw throughput  
**Why**: User feels latency, not throughput  
**When**: Setting priorities  
**How**: Measure p50/p99 latency  

**Good** ✅:
```
Target: p99 latency < 100ms
Ignore: Can do 50,000 req/sec if latency is 50ms each
Focus: Make 1000 req/sec feel instant
```

**Bad** ❌:
```
"We can handle 50,000 req/sec!"
(But each takes 500ms)
User: "This is slow! I don't care about throughput"
```

---

## SECTION 7: PROBLEM-SOLVING THUMB RULES

### Rule 7.1: Reproduce the Problem First
**What**: Can't fix what you can't see  
**Why**: Reproducing reveals root cause  
**When**: First step of debugging  
**How**: Test, isolate, repeat  

**Good** ✅:
```
Problem: "Sometimes latency spikes"
Reproduce: Run 1000 concurrent requests
See: Latency spikes from 50ms to 200ms
Root cause: Memory allocation under load
Fix: Add pooling
Verify: Latency stays < 60ms
```

**Bad** ❌:
```
Problem: "Sometimes latency spikes"
(Can't reproduce)
Guess: "Maybe cache is wrong?"
Fix: Disable cache
Test: Different behavior, hard to tell
Spin for days: Can't figure it out
```

---

### Rule 7.2: Simplest Explanation Usually Wins
**What**: Don't assume complex causes  
**Why**: Simple bugs are most common  
**When**: Debugging  
**How**: Start simple, get complex  

**Good** ✅:
```
Problem: Service crashes every night at midnight
Guess 1: Complex race condition
Guess 2: Memory leak
Guess 3: Scheduled task runs
Check: Yes! Scheduled task runs at midnight
Fix: Disable scheduled task
Result: Problem solved! (It was simple)
```

**Bad** ❌:
```
Problem: Service crashes every night
Assume: Complex race condition
Spend weeks refactoring code
Problem still happens
Finally notice: Scheduled task at midnight
(Should have checked first)
```

---

### Rule 7.3: Fix Root Cause, Not Symptoms
**What**: Solve the real problem, not the visible one  
**Why**: Symptom fixes come back  
**When**: Always  
**How**: Keep asking "Why?"  

**Good** ✅:
```
Symptom: High latency
Root cause: Slow database query
Fix: Optimize query
Result: Persistent improvement
```

**Bad** ❌:
```
Symptom: High latency
"Fix": Increase timeout
Result: Latency still high, users still unhappy
```

---

## SECTION 8: ARU STRATEGY THUMB RULES

### Rule 8.1: 100% Testing > 99% Confidence
**What**: Never say "probably works"  
**Why**: 1% failure is 100% problem for users  
**When**: Always  
**How**: Test everything, document gaps  

**Good** ✅:
```
"We tested 100%, deployment confident"
Deploy → Zero outages
Customer: Happy
```

**Bad** ❌:
```
"Probably works, we tested 99%"
Deploy → That 1% fails
Customer: Unhappy
```

---

### Rule 8.2: Documentation = Part of Code
**What**: Docs aren't optional, they're required  
**Why**: Undocumented = unsupported  
**When**: Finish code = Write docs  
**How**: Inline + Runbook + Examples  

**Good** ✅:
```
Code: 100 lines
Documentation: 100 lines
Ratio: 1:1 (Ideal)
Result: Anyone can use it
```

**Bad** ❌:
```
Code: 1000 lines
Documentation: 10 lines
Ratio: 100:1 (Abandoned)
Result: No one can use it
```

---

### Rule 8.3: Repeatable Process > Ad-Hoc Heroics
**What**: Process that works every time > one-time hero fix  
**Why**: Process scales, heroes don't  
**When**: Always  
**How**: Document, automate, verify  

**Good** ✅:
```
Phase 7 testing used process: ✅ Works perfectly
Phase 8 testing uses same process: ✅ Works perfectly
Phase 9 testing uses same process: ✅ Works perfectly
```

**Bad** ❌:
```
Phase 7: Hero fixes everything
Phase 8: Heroes not available
Phase 9: No process, chaos
```

---

## SECTION 9: SCALABILITY THUMB RULES

### Rule 9.1: Vertical Scaling Limit = Reality Check
**What**: Single machine has limits  
**Why**: Physics limits, not just code  
**When**: Planning architecture  
**How**: Know your limits  

**Good** ✅:
```
Single machine: 10,000 req/sec max
Architecture: Distribute across 10 machines
Result: 100,000 req/sec
Realistic: Handles growth
```

**Bad** ❌:
```
"Single machine can handle anything"
Assume: Unlimited scale
Reality: Crashes at 5000 req/sec
Discovery: Too late, users affected
```

---

### Rule 9.2: Caching Is Your Best Friend
**What**: Use caching aggressively  
**Why**: Memory is 1000x faster than disk  
**When**: Always when reading repeatedly  
**How**: Cache layers, TTL, invalidation  

**Good** ✅:
```
Database query: 50ms
Cached result: 0.5ms
Speedup: 100x
Users: Happy
```

**Bad** ❌:
```
No caching: "Database is powerful"
Database overloads: 500ms per query
Users: Frustrated
```

---

## SECTION 10: SECURITY THUMB RULES

### Rule 10.1: Never Trust User Input
**What**: Assume all input is malicious  
**Why**: Even good users make mistakes  
**When**: Always at boundaries  
**How**: Validate, sanitize, escape  

**Good** ✅:
```
Input: user_input = "<script>alert('hacked')</script>"
Validate: Is this valid JSON? No → Reject
Result: Safe
```

**Bad** ❌:
```
Input: user_input = "<script>alert('hacked')</script>"
No validation: Execute directly
Result: Hacked
```

---

### Rule 10.2: Principle of Least Privilege
**What**: Give minimum permissions needed  
**Why**: Limits damage if compromised  
**When**: Always  
**How**: Role-based access, tight permissions  

**Good** ✅:
```
User: Can read user profile (own only)
User: Cannot modify user profile
User: Cannot access admin features
Result: Limited damage if account hacked
```

**Bad** ❌:
```
User: Can do anything
Result: Catastrophic if hacked
```

---

## FINAL THUMB RULES: GOLDEN RULES

### Golden Rule 1: ALWAYS BUILD IS GOOD (See Section Below)

### Golden Rule 2: If Unsure, Ask the Users
**What**: User feedback > Internal debate  
**Why**: Users know what they need  
**When**: Design decisions  
**How**: Ask, listen, implement  

### Golden Rule 3: Broken Pipe Isn't Always Your Bug
**What**: Investigate deeply before assuming  
**Why**: Obvious cause is often not real cause  
**When**: Debugging production issues  
**How**: Check logs, metrics, traces  

### Golden Rule 4: Document Your Assumptions
**What**: Write down what you assumed  
**Why**: Future you won't remember  
**When**: When making decisions  
**How**: Comments, specs, ADRs  

### Golden Rule 5: Test Your Tests
**What**: Verify tests catch failures  
**Why**: Test can pass even if wrong  
**When**: When writing tests  
**How**: Introduce known bug, test should fail  

---

## USAGE GUIDE

**These rules guide decisions, they don't mandate them.**

### When to Apply:
- Uncertain about approach → Pick a rule
- Making similar mistake twice → Apply rule
- Onboarding new developer → Share rules
- Design review → Reference rules
- Post-mortem → Learn from broken rule

### When NOT to Apply:
- Rule conflicts with explicit requirement
- Rule has been superseded by new learning
- Context is very different from rule assumption

### How to Update:
- Found exception to rule? Document it
- Found new thumb rule? Add to document
- Proven rule wrong? Update or remove
- Version increases when rules change

---

## THUMB RULES PHILOSOPHY

**These rules are:**
- ✅ Defaults, not absolutes
- ✅ Wisdom, not dogma
- ✅ Guides, not mandates
- ✅ Timeless, not trendy
- ✅ Proven, not theoretical

**Apply with judgment.**
**Question when needed.**
**Update when proven wrong.**
**Share with team.**

---

**Document**: KILLER_THUMB_RULES.md  
**Version**: 1.0  
**Status**: PERMANENT (Never obsolete)  
**Last Updated**: March 18, 2026  
**Maintenance**: Update only when proven wrong or new wisdom gained

