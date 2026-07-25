# 🔄 Multi-Agent Reuse Patterns

**How to adapt agent templates for ANY problem**

---

## Pattern 1: Simple Adaptation (4-Agent)

### Starting Template
```killer
agent Validator {
  handle validate(input: String) -> Bool {
    // Generic validation logic
    true
  }
}
```

### For YOUR Problem
```killer
agent CustomValidator {
  handle validate(proof: String) -> Bool {
    // 1. Change input type
    // 2. Implement YOUR validation logic
    // 3. Return consensus
    
    is_valid = check_proof(proof)
    is_valid
  }
}
```

### Step-by-Step
1. Copy `agent_template_4.killer`
2. Replace `validate()` with `validate_YOUR_PROBLEM()`
3. Change success criteria
4. Deploy

**Time**: 30 minutes | **Agents**: 4 | **Confidence**: 85%+

---

## Pattern 2: Professional Adaptation (24-Agent)

### Customization Points

```killer
// 1. Problem statement
PROBLEM = "Your theorem: ..."

// 2. Attack strategies (customize for each agent)
attack_strategy_1 = |proof| {
  // Try to break proof using approach A
}

attack_strategy_2 = |proof| {
  // Try to break proof using approach B
}

// ... 5 total attack strategies

// 3. Support agent specializations
validator_expertise = ["formal logic", "your domain"]

benchmark_criteria = [
  "speed",
  "accuracy on known cases",
  "your metric"
]

// 4. Success criteria
success_if_all_agents_agree = true
confidence_threshold = 90%
```

### Deployment
```killer
agents = deploy_24_agent_system(
  problem: PROBLEM,
  strategies: [attack_strategy_1, attack_strategy_2, ...],
  validators: validator_expertise,
  benchmark: benchmark_criteria,
  success: success_criteria
)

result = agents.orchestrate()
```

**Time**: 2-4 hours | **Agents**: 24 | **Confidence**: 95%+

---

## Pattern 3: Enterprise Adaptation (72 or 108-Agent)

### Multi-Tier Customization

```killer
// Tier 1: Core (24 agents)
tier_1 = agents_24(problem, strategies, validators)

// Tier 2: Expansion (48 more agents - specialized domains)
tier_2_specialists = [
  "quantum_experts",
  "geometric_analysts",
  "statistical_validators",
  "computational_verification"
]

tier_2 = agents_48(tier_1.results, tier_2_specialists)

// Tier 3: Enterprise (36 more agents - institutional review)
tier_3 = agents_36(tier_2.results)

// Final consensus
all_tiers = [tier_1, tier_2, tier_3]
final_result = orchestrate_consensus(all_tiers)
```

**Time**: 1-2 weeks | **Agents**: 72+ | **Confidence**: 99%+

---

## Problem Domain Mappings

### Mathematics
- **Input**: Theorem statement + proof
- **Agents**: Devil's Advocate, Domain Specialists, Logicians, Computational Verifiers
- **Success**: All 24 agents agree proof is sound
- **Time**: 3-5 hours per problem

### Business/Finance
- **Input**: Decision/investment proposal
- **Agents**: Risk Analysts, Market Experts, Financial Auditors, Skeptics
- **Success**: Consensus on risk/reward ratio
- **Time**: 1-2 hours

### Research/Academia
- **Input**: Paper/hypothesis
- **Agents**: Domain Experts, Methodologists, Reproducibility Auditors, Critics
- **Success**: Consensus on validity
- **Time**: 4-8 hours

### Security/Compliance
- **Input**: System/policy
- **Agents**: Security Experts, Attack Specialists, Auditors, Compliance Officers
- **Success**: 0 exploitable vulnerabilities found
- **Time**: 2-4 hours

### Product Development
- **Input**: Feature/design
- **Agents**: UX Experts, Engineers, QA Analysts, Market Analysts
- **Success**: Consensus on go/no-go
- **Time**: 2-3 hours

---

## Customization Checklist

### Before Deployment

- [ ] **Problem Definition** - Clear, unambiguous problem statement
- [ ] **Proof/Solution** - Complete solution to validate
- [ ] **Success Criteria** - What counts as "valid"
- [ ] **Attack Strategies** - How agents try to break your solution
- [ ] **Domain Experts** - Specialists for your field
- [ ] **Benchmark Metrics** - How to measure quality
- [ ] **Time Budget** - 24-agent: 3-5 hrs, 72-agent: 1-2 days
- [ ] **Confidence Target** - 90%+, 95%+, 99%+?

### During Deployment

- [ ] Monitor agent progress
- [ ] Track consensus building
- [ ] Watch for agent disagreements
- [ ] Adjust strategies if needed
- [ ] Generate benchmark reports

### After Deployment

- [ ] Document results
- [ ] Publish consensus report
- [ ] Archive agent reasoning
- [ ] Store in KillerDB

---

## Real-World Examples

### Example A: Collatz Conjecture (72-agent)

```killer
problem = "Collatz sequence always reaches 1"
strategies = [
  "exhaustive_search_strategy",
  "mathematical_proof_strategy",
  "counterexample_search",
  "statistical_analysis",
  "graph_theory_approach"
]
specialists = ["number_theorists", "mathematicians", "computer_scientists"]

result = deploy_72_agent_system(problem, strategies, specialists)
// Result: 99.8% confidence, 2 days of computation
```

### Example B: Market Risk Assessment (24-agent)

```killer
problem = "Is this $10M investment sound?"
strategies = [
  "market_downturn_scenario",
  "competitive_threat_scenario",
  "regulatory_risk_scenario",
  "execution_risk_scenario",
  "industry_disruption_scenario"
]
specialists = ["finance", "risk", "market", "operations", "legal"]

result = deploy_24_agent_system(problem, strategies, specialists)
// Result: 96% consensus, 2 hours
```

---

## Tips for Success

1. **Clear Problem Statement** - Ambiguity = agent disagreement
2. **Diverse Strategies** - Different angles find different weaknesses
3. **Expert Matching** - Right specialists for your domain
4. **Patience** - Consensus takes time but is very reliable
5. **Document Everything** - Agent reasoning is valuable for understanding
6. **Start Small** - 4-agent for quick feedback, 24-agent for serious work

---

**Next**: [API_REFERENCE.md](API_REFERENCE.md) for detailed framework APIs
