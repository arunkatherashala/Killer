# 🎯 Reusable Agent Templates - Quick Reference

## Available Templates

### **1. 4-Agent Template** (`agent_template_4.killer`)
**Scale**: Small projects  
**Time**: 30 mins - 1 hour  
**Confidence**: 85%+  
**Use For**:
- Quick validation
- Business decisions
- Small proofs

**Agents**:
- Validator
- Benchmark Runner
- Doc Generator
- Quality Auditor

---

### **2. 24-Agent Template** (`agent_template_24.killer`) ⭐
**Scale**: Professional  
**Time**: 3-5 hours  
**Confidence**: 95%+  
**Use For**:
- Millennium Problems
- Mathematical theorems
- Major research papers
- Critical decisions

**Agents**:
- 5 Attack Agents
- 19 Support Agents (validators, specialists, auditors)

---

### **3. 72-Agent Template** (`agent_template_72.killer`)
**Scale**: Enterprise  
**Time**: 1-2 days  
**Confidence**: 98%+  
**Use For**:
- Complex proofs
- Large research projects
- Institutional validation

**Agents**:
- Tier 1: 24 agents (core debate)
- Tier 2: 48 agents (specialized domains)

---

### **4. 108-Agent Template** (`agent_template_108.killer`)
**Scale**: Institutional  
**Time**: 1-2 weeks  
**Confidence**: 99%+  
**Use For**:
- Breakthrough discoveries
- Multi-institutional validation
- Critical decisions worth $1M+

**Agents**:
- Tier 1: 24 agents (core)
- Tier 2: 48 agents (expansion)
- Tier 3: 36 agents (enterprise)

---

## Quick Start

### Copy a Template
```bash
cp agent_template_24.killer my_problem.killer
```

### Customize
1. Update `problem_statement`
2. Modify attack strategies
3. Customize validators
4. Run!

### Run
```bash
killer.exe my_problem.killer
```

---

## Example Problems Ready-to-Deploy

See `examples/` folder:

- `collatz_consensus.killer` - 72-agent Collatz validation
- `navier_stokes_validation.killer` - Navier-Stokes consensus
- `business_risk_assessment.killer` - 24-agent business decision
- `research_validation.killer` - 108-agent paper validation
- `security_audit_consensus.killer` - 24-agent security audit

---

## Documentation

- **AGENT_FRAMEWORK.md** - Overview and architecture
- **REUSE_PATTERNS.md** - Customization patterns for each domain
- **API_REFERENCE.md** - Framework APIs and agents

---

**Start with 24-agent template for most problems.** 🚀
