# KILLER v2.0: QUICK START GUIDE
## Run the 4 Core Examples Right Now

**This guide shows how to run and understand each example.**

---

## BEFORE YOU START

These examples use Killer v4.2+ (the current production version).

```bash
# Verify Killer is installed
killer --version
# Expected output: killer 4.2.0
```

---

## EXAMPLE #1: REGRESSION ENSEMBLE (5 minutes)

**File**: `EDUCATIONAL_TRACK/EXAMPLE_01_REGRESSION_ENSEMBLE.killer`

**What it does**:
- Trains 3 regression models in parallel (Feature #1: Async)
- Combines predictions via ensemble voting
- Simulates GPU batch inference
- Shows 3-5x speedup from parallelism

**Run it**:
```bash
cd EDUCATIONAL_TRACK
killer EXAMPLE_01_REGRESSION_ENSEMBLE.killer
```

**Expected output**:
```
=== EXAMPLE #1: REGRESSION ENSEMBLE ===

PHASE 1: Data generation...
✓ 1000 house samples generated

PHASE 2: Training models in parallel...
Training ensemle (3 models async)...
Time: 50ms

PHASE 3: Single prediction...
Input: 2500 sqft, 3 bedrooms, 10 years
Predicted price: $450,000
R²: 0.85, RMSE: $45,000

PHASE 4: GPU batch inference...
100 predictions: 10ms (GPU)
1000 predictions: 50ms (GPU)
10,000 predictions: 150ms (GPU)

PHASE 5: Streaming...
✓ Real-time streaming mode
```

**Key takeaway**: 5x speedup from training 3 models in parallel using Feature #1 (Async)

---

## EXAMPLE #2: NLP PIPELINE (5 minutes)

**File**: `EDUCATIONAL_TRACK/EXAMPLE_02_NLP_PIPELINE.killer`

**What it does**:
- Creates vector embeddings for documents (Stage 2 math)
- Performs semantic search (cosine similarity)
- Simulates LLM-powered Q&A (Feature #2 ready)
- Runs concurrent queries in parallel (Feature #1)

**Run it**:
```bash
cd EDUCATIONAL_TRACK
killer EXAMPLE_02_NLP_PIPELINE.killer
```

**Expected output**:
```
=== EXAMPLE #2: NLP PIPELINE ===

PHASE 1: Initializing components...
✓ EmbeddingEngine initialized
✓ VectorStore initialized
✓ NLPAgent ready

PHASE 2: Loading knowledge base...
✓ 4 documents loaded

PHASE 3: Semantic search...
Query: "What is machine learning?"
Top results:
  #1: Introduction to ML (similarity: 85%)
  #2: Neural Networks (similarity: 72%)

Query: "How do neural networks work?"
Top results:
  #1: Neural Networks (similarity: 91%)
  #2: Deep Learning (similarity: 78%)

PHASE 4: Q&A with LLM...
User: What is machine learning?
Retrieved docs: 3
Agent: ML is a subset of AI...

PHASE 5: Concurrent queries...
Asking 4 questions in parallel...
✓ All responses received (4x speedup)
```

**Key takeaway**: Stage 2 math + semantic search + ready for real LLM APIs

**To use with real LLM** (after Week 6):
```kill
config = ModelConfig {
  provider: "openai",
  model_name: "gpt-4",
  api_key: "sk-...",
  // ... rest of config
}
```

---

## EXAMPLE #3: COMPUTER VISION - CNN (10 minutes)

**File**: `EDUCATIONAL_TRACK/EXAMPLE_03_COMPUTER_VISION.killer`

**What it does**:
- Builds CNN (2 conv layers, 2 fully connected layers)
- Classifies images (8 classes: cat, dog, bird, etc.)
- Demonstrates GPU acceleration (Feature #10)
- Shows batch processing efficiency

**Run it**:
```bash
cd EDUCATIONAL_TRACK
killer EXAMPLE_03_COMPUTER_VISION.killer
```

**Expected output**:
```
=== EXAMPLE #3: COMPUTER VISION - CNN ===

PHASE 1: Initializing CNN...
✓ Conv1 (32 filters) + MaxPool
✓ Conv2 (64 filters) + MaxPool
✓ FC (128 neurons)
✓ Output (8 classes)

PHASE 2: CPU Inference
Image: 32×32×3 pixels
Prediction: cat
Confidence: 85%
CPU Latency: 50ms

PHASE 3: GPU Batch inference...
10 images with GPU acceleration
GPU Results:
  Image 1: dog (5ms GPU)
  Image 2: bird (5ms GPU)
  ...
SPEEDUP: 10x

PHASE 4: Parallel GPU Inference...
Processing 5 images in parallel
✓ All predictions complete

PHASE 5: Scaling...
Batch 1: 50ms CPU → 5ms GPU (10x)
Batch 10: 500ms CPU → 50ms GPU (10x)
Batch 100: 5s CPU → 500ms GPU (10x)
Batch 1000: 50s CPU → 5s GPU (10-100x)
```

**Key takeaway**: Feature #10 (GPU) provides 10-100x speedup on large batches

---

## EXAMPLE #4: AUTONOMOUS AGENTS (10 minutes)

**File**: `EDUCATIONAL_TRACK/EXAMPLE_04_AUTONOMOUS_AGENTS.killer`

**What it does**:
- Creates 3-agent team (Researcher, Analyst, Executor)
- Each agent has memory (Feature #6) and tools (Feature #3)
- Team votes on decisions (Feature #7: Coordination)
- Agents learn from experience
- Everything parallelized (Feature #1)

**Run it**:
```bash
cd EDUCATIONAL_TRACK
killer EXAMPLE_04_AUTONOMOUS_AGENTS.killer
```

**Expected output**:
```
=== EXAMPLE #4: AUTONOMOUS AGENTS ===

PHASE 1: Team assembly...
✓ Alice (Researcher)
✓ Bob (Analyst)
✓ Charlie (Executor)

PHASE 2: Individual agent reasoning...
Alice's thought: Analyzing research task...
  Decision: search_knowledge
Bob's thought: Data analysis required...
  Decision: analyze_data
Charlie's thought: Ready to execute...
  Decision: execute_action

PHASE 3: Parallel tool execution...
Executing tools in parallel:
  - search_knowledge: Results for: AI topics
  - analyze_data: Analysis: findings data
  - execute_action: Executed: solution

PHASE 4: Team consensus voting...
Proposal: Should we deploy?
Team Decision: APPROVED (82% confidence)

PHASE 5: Learning & memory...
Alice learns from experience:
  - Research task 1: Success (90% reward)
  - Research task 2: Success (80% reward)
  - Research task 3: Failed (30% reward)
Success rate: 73%

PHASE 6: Workflow execution...
Executing: research → analyze → implement
✓ Workflow complete

PHASE 7: Team report...
Alice: 3 tasks completed
Bob: 2 tasks completed
Charlie: 2 tasks completed
```

**Key takeaway**: Multi-agent systems with reasoning, tools, memory, coordination, and learning

---

## FEATURE #2: LLM INTEGRATION (Testing)

**File**: `AI_FEATURES/llm_implementation.killer`

**What it does**:
- Multi-provider abstraction (OpenAI, Claude, Ollama)
- Automatic retries + exponential backoff
- Conv history management
- Concurrent queries (Feature #1)

**Run it**:
```bash
cd AI_FEATURES
killer llm_implementation.killer
```

**Expected output**:
```
=== FEATURE #2: LLM INTEGRATION ===

DEMO 1: OpenAI Provider
User: What is machine learning?
Assistant (GPT-4): Machine learning is...
Tokens: 100
Latency: 150ms

DEMO 2: Claude Provider
User: Explain neural networks
Assistant (Claude-opus): Neural networks...
Tokens: 120
Latency: 180ms

DEMO 3: Ollama (Local)
User: What is deep learning?
Assistant (Mistral): Deep learning...
Tokens: 80
Latency: 200ms

DEMO 4: Resilient with retries
Rate limited, retrying in 2000ms...
Success after 2 retries

DEMO 5: Concurrent queries
Sending 5 queries in parallel...
✓ All responses received (5x parallelism)
```

---

## STAGE 2: MATH LIBRARY (Reference)

**File**: `EDUCATIONAL_TRACK/STAGE_02_MATH_FOR_ML.md`

Reference library (not runnable - it's a guide). Shows:
- Linear algebra (vectors, matrices)
- Calculus (derivatives, gradients)
- Probability (distributions, entropy)
- Optimization (gradient descent, Adam)
- Metrics (MSE, accuracy, F1)

**Use it**: Copy functions into your projects

---

## HOW TO USE THESE EXAMPLES

### Option 1: Run standalone
```bash
cd EDUCATIONAL_TRACK
killer EXAMPLE_01_REGRESSION_ENSEMBLE.killer
killer EXAMPLE_02_NLP_PIPELINE.killer
killer EXAMPLE_03_COMPUTER_VISION.killer
killer EXAMPLE_04_AUTONOMOUS_AGENTS.killer
```

### Option 2: Integrate into your project
```killer
// In your code
include "STAGE_02_MATH_FOR_ML.killer"    // Math operations
include "EXAMPLE_01_REGRESSION_ENSEMBLE.killer"  // ML models
include "EXAMPLE_02_NLP_PIPELINE.killer"  // NLP

// Now use the components
model = CNNModel::spawn()
agent = NLPAgent::spawn()
```

### Option 3: Extend the examples
```killer
// Copy Example #1, modify for your use case
// e.g., use for stock price prediction instead of house prices

dataset = load_stock_data()
model = LinearRegressor::spawn()
results = await model.fit(dataset)
```

---

## TROUBLESHOOTING

### Issue: "Feature #X not implemented"
**Reason**: You're trying to use a feature that's still in development  
**Solution**: Stick to Features #1-2 until full v2.0 release (Week 26)

### Issue: "Actor spawning failed"
**Reason**: Killer version is too old  
**Solution**: Update to Killer v4.2+

### Issue: "Memory allocation failed"
**Reason**: Too many concurrent tasks  
**Solution**: Reduce batch size or number of parallel tasks

### Issue: "LLM API call timed out"
**Reason**: Network issue or API rate limiting  
**Solution**: Implemented auto-retry logic handles this. Check API key is valid.

---

## WHAT'S NEXT?

### Immediate (This week):
- Run all 4 examples
- Understand how each works
- Try modifying Example #1 (regression) for your own dataset

### Week 2-3:
- Connect Example #2 to real OpenAI API
- Test document Q&A with your own documents
- Build a small project using the components

### Week 4-6:
- Features #3-#10 rollout
- More advanced examples
- Full curriculum guides (Stages 3-12)
- Student projects

---

## QUICK REFERENCE

| Example | Time | Features | Key Output |
|---------|------|----------|-----------|
| #1 Regression | 5 min | #1, #4, #10 | House price prediction (5x speedup) |
| #2 NLP | 5 min | #2, #3, #5, #1 | Document Q&A (4x concurrent) |
| #3 Computer Vision | 10 min | #10, #1 | Image classification (10-100x speedup) |
| #4 Agents | 10 min | #2, #3, #6, #7, #1 | Multi-agent team reasoning |

---

## LEARNING PATH

Recommended order:
1. **Start**: Example #1 (understand parallelism)
2. **Then**: Example #3 (understand deep learning)
3. **Then**: Example #2 (understand NLP)
4. **Finally**: Example #4 (multi-agent systems)

This goes from simple (regression) → complex (agents) → realistic.

---

Ready to run? Start with:
```bash
cd EDUCATIONAL_TRACK
killer EXAMPLE_01_REGRESSION_ENSEMBLE.killer
```

Welcome to Killer v2.0! 🚀
