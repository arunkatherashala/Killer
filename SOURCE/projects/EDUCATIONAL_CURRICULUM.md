# KILLER EDUCATIONAL MATERIALS GUIDE

**Purpose:** Teaching unsolved $1M mathematics & physics problems  
**Level:** Undergraduate through Graduate  
**Format:** Hands-on with Killer V2 frameworks  
**Updated:** March 17, 2026

---

## COURSE STRUCTURE (16-Week Semester)

### UNIT 1: COMPUTATIONAL COMPLEXITY (Weeks 1-3)

**Topic: P vs NP - The $1M Question**

#### Week 1: Foundations
- **Reading:** P vs NP problem statement (RESULTS.md, sections 1-3)
- **Activity 1:** Run P vs NP framework
  ```bash
  killer projects/computer-science/millennium_prize_1_p_vs_np/FRAMEWORK.killer
  ```
- **Observe:** Factorization verification (easy) vs finding factors (hard)
- **Discussion:** Why is cryptography based on P ≠ NP?

#### Week 2: NP-Complete Problems
- **Learn:** Understand 5 NP-complete problems
  1. Factorization: Factor 143 into 11×13
  2. SAT: Boolean satisfiability
  3. Graph Coloring: K₄ with 2 colors
  4. Clique: Find maximum complete subgraph
  5. Hamiltonian Path: Visit all nodes once

- **Activity 2:** Modify test cases
  - Change factorization input to 323
  - Create new SAT instance (7 variables, 4 clauses)
  - Design different graph for coloring

- **Analysis:** Why are all these problems equivalently hard?

#### Week 3: P vs NP Deep Dive
- **Read:** IMPLEMENTATION.md for P vs NP
- **Challenge 1:** Can you find a faster SAT solver?
- **Challenge 2:** Prove/disprove P = NP (joke assignment!)
- **Project:** Write report: "If P = NP, what breaks?"

---

### UNIT 2: NUMBER THEORY (Weeks 4-5)

**Topic: Riemann Hypothesis - 150-Year Mystery**

#### Week 4: Zeta Function & Prime Distribution
- **Reading:** Riemann Hypothesis RESULTS.md (sections 1-4)
- **Mathematical Background:**
  - ζ(s) = 1 + 1/2^s + 1/3^s + ...
  - Connection to prime distribution
  - Trivial vs non-trivial zeros

- **Activity 3:** Run Riemann framework
  ```bash
  killer projects/mathematics/millennium_prize_2_riemann_hypothesis/FRAMEWORK.killer
  ```
- **Observe:** First 15 zeros all at Re(s) = 0.5

#### Week 5: Computational Evidence
- **Explore:** Why 10¹² zeros verified?
- **Data Analysis:**
  - Zero spacing distribution
  - Growth rate verification
  - GUE theory matching

- **Activity 4:** Create visualization
  - Plot zero locations
  - Analyze gap distribution
  - Compare to random matrices

- **Discussion:** Can computational evidence replace proof?

---

### UNIT 3: ELLIPTIC CURVES (Weeks 6-7)

**Topic: Birch-Swinnerton-Dyer - Algebra meets Analysis**

#### Week 6: Elliptic Curve Basics
- **Reading:** BSD RESULTS.md (sections 1-4)
- **Learn:**
  - Elliptic curves: y² = x³ + ax + b
  - Rational points and rank
  - Elliptic curve addition law

- **Activity 5:** Run BSD framework
  ```bash
  killer projects/mathematics/millennium_prize_4_bsd/FRAMEWORK.killer
  ```
- **Explore:** Point counting on curves mod p

#### Week 7: L-Functions & Rank
- **Deep Dive:** Connection between rank and L-function zeros
- **Challenge:** Can you compute rank for custom curves?
- **Experiment:** Test conjecture on 10 different elliptic curves

- **Activity 6:** Verify BSD prediction
  - Compute rank (algebraic)
  - Find L-function order (analytic)
  - Compare results

---

### UNIT 4: GEOMETRY & TOPOLOGY (Week 8)

**Topic: Hodge Conjecture - Bridging Algebra & Topology**

- **Reading:** Hodge RESULTS.md (all sections)
- **Learn:**
  - Cohomology groups
  - Hodge decomposition
  - Algebraic cycles

- **Activity 7:** Run Hodge framework
  ```bash
  killer projects/mathematics/millennium_prize_6_hodge/FRAMEWORK.killer
  ```

- **Draw:** Hodge diamonds for various surfaces
- **Analyze:** Why dimension 1-2 are solved but 3+ open?

---

### UNIT 5: FLUID DYNAMICS (Weeks 9-10)

**Topic: Navier-Stokes - Equations of Motion**

#### Week 9: Fluid Fundamentals
- **Reading:** Navier-Stokes RESULTS.md
- **Equations:** ∂u/∂t + (u·∇)u = -∇p + ν∇²u + f
- **Learn:**
  - Momentum conservation
  - Incompressibility (div u = 0)
  - Viscosity effects

- **Activity 8:** Run Navier-Stokes framework
  ```bash
  killer projects/physics/millennium_prize_3_navier_stokes/FRAMEWORK.killer
  ```

#### Week 10: 2D vs 3D & Smoothness
- **Explore:** Why 2D is solved but 3D is open
- **Analyze:** Poiseuille flow in framework
- **Experiment:** Add perturbations, watch stability
- **Challenge:** Can you induce blow-up?

- **Activity 9:** Create different flows
  - Backward-step flow
  - Shear flow
  - Rotating fluid

---

### UNIT 6: QUANTUM FIELD THEORY (Weeks 11-12)

**Topic: Yang-Mills - Gauge Theory**

#### Week 11: Gauge Fields & Confinement
- **Reading:** Yang-Mills RESULTS.md
- **Physics:**
  - Gauge symmetry SU(2), SU(3)
  - Field strength tensor F_μν
  - Gluon interactions

- **Activity 10:** Run Yang-Mills framework
  ```bash
  killer projects/physics/millennium_prize_5_yang_mills/FRAMEWORK.killer
  ```

#### Week 12: Lattice Gauge Theory & Mass Gap
- **Explore:** Wilson loops and area law
- **Analyze:** Gluon propagator structure
- **Verify:** Critical lattice parameters for confinement

- **Activity 11:** Test lattice configurations
  - Vary coupling constant
  - Change lattice size
  - Observe confinement persistence

---

### UNIT 7: ADVANCED TOPICS & RESEARCH (Weeks 13-14)

**Cross-Field Connections**

#### Week 13: Comparison & Synthesis
- **Read:** MILLENNIUM_COMPARISON_ANALYSIS.md
- **Analyze:** Relationships between problems
  - P vs NP → Cryptography → Elliptic curves
  - Riemann → Prime distribution → L-functions
  - Yang-Mills → QCD → Physical experiments
  - Navier-Stokes → Climate modeling → Turbulence

- **Activity 12:** Create concept maps
  - How do these 6 problems relate?
  - Which might be solved first?
  - What techniques transfer between problems?

#### Week 14: Research Extensions
- **Choose one problem for research project**
- **Options:**
  1. Extend framework with new test cases
  2. Improve algorithm implementation
  3. Create visualization/simulation
  4. Write literature review
  5. Propose new attack on problem

---

### UNIT 8: CAPSTONE (Weeks 15-16)

**Final Project & Presentation**

#### Week 15: Project Development
- **Students choose:**
  - Deepen one problem investigation
  - Compare multiple problems
  - Create educational materials
  - Develop new framework features

- **Deliverables:**
  - Code (extended framework)
  - Documentation (1000+ words)
  - Presentation slides

#### Week 16: Presentations & Closing
- **Each student presents:**
  - Problem selected
  - Work completed
  - Findings & insights
  - Future directions

- **Closing:** Discussion of where breakthroughs might come

---

## LEARNING OUTCOMES

By end of course, students will:

### Knowledge
✓ Understand all 6 Millennium Prize problems  
✓ Know current proof status  
✓ Appreciate computational evidence  
✓ See connections across disciplines  

### Skills
✓ Run & analyze complex frameworks  
✓ Implement mathematical algorithms  
✓ Read advanced mathematical literature  
✓ Present research findings  

### Research Awareness
✓ Frontier of unsolved problems  
✓ Proof techniques & limitations  
✓ Computational methodology  
✓ Career paths in research  

---

## ASSESSMENT RUBRIC

### Participation (20%)
- Activity completion (10 activities)
- Class discussions
- Peer feedback

### Assignments (30%)
- Weekly reading responses
- Framework modifications
- Case studies

### Midterm Project (20%)
- Deepen understanding of 2-3 problems
- 10-page report
- 10-minute presentation

### Final Project (30%)
- Extended research/implementation
- 15-page report or code documentation
- 15-minute presentation

---

## SUGGESTED BACKGROUND

### Minimum Requirements
- Linear algebra (matrices, eigenvalues)
- Calculus (multivariable, partial derivatives)
- Discrete mathematics (basic complexity)
- Computer science fundamentals

### Helpful (Not Required)
- Abstract algebra
- Real/complex analysis
- Differential geometry
- Numerical methods

---

## TEXTBOOK & REFERENCE MATERIALS

### Problem-Specific Resources
- **P vs NP:** "Computers and Intractability" by Garey & Johnson
- **Riemann:** "The Riemann Hypothesis" by Rockmore
- **BSD:** "Arithmetic of Elliptic Curves" by Silverman
- **Hodge:** "Hodge Theory and Complex Algebraic Geometry" by Voisin
- **Navier-Stokes:** "Partial Differential Equations" by Evans
- **Yang-Mills:** "Quantum Field Theory in a Nutshell" by Zee

### Killer V2 Documentation
- FRAMEWORK.killer files (working code)
- RESULTS.md files (problem analysis)
- IMPLEMENTATION.md files (technical details)

---

## CLASSROOM ACTIVITIES (DETAILED)

### Activity 1: Run P vs NP Framework
**Time:** 20 minutes
**Process:**
1. Execute framework (5 min)
2. Observe output (5 min)
3. Discuss implications (10 min)

**Discussion Questions:**
- Why is verification easier than finding?
- How does cryptography depend on P ≠ NP?
- What would happen if P = NP?

---

### Activity 2: Modify NP Test Cases
**Time:** 30 minutes
**Challenges:**
1. Create new SAT instance (harder than given)
2. Change factorization to 3-digit number
3. Design different graph for coloring

**Learning:** Understand complexity by experimentation

---

### Activity 3: Zero Distribution Analysis
**Time:** 45 minutes
**Explore:**
1. Run Riemann framework
2. Extract zero data
3. Plot distribution
4. Compare to GUE theory
5. Discuss implications

**Outcome:** Understand role of computational evidence

---

### Activity 4: Elliptic Curve Explorer
**Time:** 45 minutes
**Process:**
1. Run BSD framework
2. Modify test curves
3. Compute ranks
4. Verify conjecture
5. Report findings

**Learning:** See conjecture in action

---

### Activity 5: Hodge Diamond Construction
**Time:** 30 minutes
**Task:**
1. Run Hodge framework
2. Draw 3 Hodge diamonds (varying dimensions)
3. Identify symmetries
4. Discuss missing pieces for dim 3+

---

### Activity 6: Navier-Stokes Stability
**Time:** 45 minutes
**Experiment:**
1. Run framework (simple flow)
2. Add perturbations
3. Track energy decay
4. Vary viscosity
5. Explore parameters

**Challenge:** Can you cause instability?

---

### Activity 7: Yang-Mills Confinement
**Time:** 45 minutes
**Analyze:**
1. Run framework
2. Examine Wilson loops
3. Verify area law
4. Compute string tension
5. Interpret results

**Understanding:** What confinement means physically

---

## GRADING GUIDELINES

### Framework Activities (70%)
- Successful execution: 10 points
- Understanding output: 10 points
- Thoughtful modification: 10 points
- Written explanation: 10 points

### Research Projects (30%)
- Code quality: 10 points
- Documentation: 10 points
- Insight/depth: 10 points

---

## ADVANCED EXTENSIONS

### For Top Students
1. **Prove a partial result** (e.g., P ≠ NP for restricted class)
2. **Extend a framework** (add new algorithm, test cases)
3. **Compare multiple approaches** (different solvers)
4. **Write research paper** (survey of problem)
5. **Create visualization** (interactive learning tool)

### Research Projects
- Investigate connections between problems
- Test computational methods
- Explore specific mathematician's work
- Develop teaching materials

---

## TIME ESTIMATES

### Shorter Course (8 weeks)
- Select 3 problems
- Reduce to 1-2 activities per topic
- Simpler final projects
- Estimated effort: 3 hours/week

### Full Course (16 weeks)
- All 6 problems
- Full activity set
- Major research project
- Estimated effort: 6-8 hours/week

### Seminar (4 weeks, intensive)
- 6 problems in rapid sequence
- Focus on understanding, not implementation
- Presentations instead of projects
- Estimated effort: 10-12 hours/week

---

## INCLUSIVE CLASSROOM PRACTICES

**Accessibility:**
- Provide framework output transcripts
- Record all demonstrations
- Alternative activities for coding-averse students
- Pair programming encouraged

**Diversity of Learning Styles:**
- Visual: Diagrams, Hodge diamonds, plots
- Kinesthetic: Run frameworks, modify code
- Auditory: Discussions, presentations
- Reading: Papers, documentation

**Level Adjustment:**
- Beginner: Run frameworks as-is
- Intermediate: Modify test cases
- Advanced: Extend algorithms

---

## EVALUATION & FEEDBACK

### Student Assessment
- Weekly feedback on activities
- Midpoint progress check
- Final project rubric
- Peer review component

### Course Evaluation
- Student survey on effectiveness
- Problem clarity feedback
- Activity difficulty assessment
- Teaching materials improvement

---

## RECOMMENDED READING: THE PROBLEMS THEMSELVES

**Best sources:**
- Clay Mathematics Institute official descriptions
- Individual RESULTS.md files (700+ lines each)
- Popular books by mathematicians
- Recent survey articles

---

## CONCLUSION

This course provides:
✅ Access to unsolved $1M problems  
✅ Hands-on experience with frameworks  
✅ Research foundation for future work  
✅ Inspiration from frontiers of mathematics  

**Bottom line:** Students will understand deeply why these problems are hard, see computational progress, and learn modern approaches to mathematical research.

