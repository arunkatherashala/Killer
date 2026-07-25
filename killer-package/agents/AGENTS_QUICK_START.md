# 🤖 Multi-Agent Framework - Quick Start

**For the complete framework, see**: `SOURCE/agents/`

---

## Two Ways to Use Agents

### **Option 1: Quick Start (this package)**
- Templates: 4-agent, 24-agent
- Examples: Collatz validation
- Time: 30 mins to 3 hours
- Best for: First-time users

### **Option 2: Full Framework**
- All templates: 4, 24, 72, 108-agent
- Complete documentation
- Multiple examples
- Location: `SOURCE/agents/` in Killer repository

---

## Templates Included Here

### **agent_template_4.killer**
- 4 agents
- 30 minutes
- Quick business decisions
- See: `templates/agent_template_4.killer`

### **agent_template_24.killer** ⭐
- 24 agents
- 3-5 hours
- Proofs, research, decisions worth $10K+
- Best choice for most problems
- See: `templates/agent_template_24.killer`

---

## Quick Example

```bash
# Run example
killer.exe agents/examples/collatz_consensus.killer

# Copy template for your problem
cp agents/templates/agent_template_24.killer my_problem.killer

# Customize my_problem.killer with:
# - Your problem statement
# - Your validation logic
# - Your attack strategies

# Deploy
killer.exe my_problem.killer
```

---

## For More Agents (72, 108)

Visit: `SOURCE/agents/` in main Killer repository

Contains:
- Full documentation
- All 4 templates (4, 24, 72, 108-agent)
- Advanced examples
- Complete API reference

---

## Customization Quick Guide

1. **Copy template**: `cp templates/agent_template_24.killer my_porblem.killer`
2. **Edit problem_statement**: What are you trying to solve?
3. **Edit attack strategies**: How could this fail?
4. **Edit validators**: How do you validate success?
5. **Run**: `killer.exe my_problem.killer`

---

See [../QUICK_START.md](../QUICK_START.md) for 5-minute tutorial.
See [../README.md](../README.md) for full Killer language reference.
