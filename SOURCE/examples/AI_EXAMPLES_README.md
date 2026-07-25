<!-- Killer AI Examples Guide -->

# Killer AI Examples

This directory contains comprehensive examples demonstrating all AI capabilities in Killer V3.0.

## Examples Overview

### 1. **ai_01_generate.killer** - Text Generation
Learn how to use `ai_generate()` to create text with AI.

**Key Features:**
- Simple text generation with prompts
- Custom options (model, max_tokens, temperature)
- Batch generation of multiple texts
- Different model selection

**Use Cases:**
- Business copywriting
- Content creation
- Marketing text generation
- Code commenting

---

### 2. **ai_02_embed.killer** - Embeddings & Semantic Search
Learn how to use `ai_embed()` to generate embeddings and find similar items.

**Key Features:**
- Embedding generation for documents
- Semantic similarity search
- Custom model selection (BERT, etc.)
- Vector-based document lookup

**Use Cases:**
- Similarity search engines
- Document recommendation
- Semantic clustering
- Content discovery

---

### 3. **ai_03_classify.killer** - Classification & Sentiment Analysis
Learn how to use `ai_classify()` for zero-shot classification.

**Key Features:**
- Sentiment analysis (positive/negative/neutral)
- Topic classification
- Intent detection
- Confidence scoring
- Custom models

**Use Cases:**
- Sentiment analysis
- Content moderation
- Intent recognition
- Topic categorization
- Review classification

---

### 4. **ai_04_extract.killer** - Information Extraction
Learn how to use `ai_extract()` to pull structured data from unstructured text.

**Key Features:**
- Contact information extraction
- Product details extraction
- Event information extraction
- Schema-based extraction
- Batch processing

**Use Cases:**
- Named entity recognition
- Data structuring
- Invoice processing
- Document parsing
- Information cleanup

---

### 5. **ai_05_local_infer.killer** - Local AI Model Inference
Learn how to use `ai_local_infer()` for on-device inference.

**Key Features:**
- BERT text classification
- Local embedding generation
- Named entity recognition (NER)
- Batch processing
- Latency tracking
- Privacy-preserving inference

**Use Cases:**
- Privacy-critical applications
- Offline AI processing
- Low-latency requirements
- On-device classification
- Custom model deployment

---

### 6. **ai_06_advanced.killer** - Advanced Multi-AI Applications
Learn how to combine multiple AI operations to build practical systems.

**Key Features:**
- Document Intelligence Pipeline
- Question-Answering System
- Content Recommendation Engine
- Automated Data Processing
- Combined operations workflow

**Use Cases:**
- Intelligent document processing
- Q&A chatbots
- Recommendation systems
- Automated data pipelines
- Complex AI workflows

---

### 7. **ai_07_providers.killer** - AI Provider Configuration
Learn how to configure and switch between different AI providers.

**Key Features:**
- OpenAI provider setup (cloud-based)
- Local ONNX provider setup (on-device)
- Cache configuration
- Provider comparison
- Context-based provider switching
- Configuration profiles (dev/prod/privacy)

**Use Cases:**
- Development vs production setups
- Privacy-first applications
- Cost optimization
- Performance tuning
- Provider fallback strategies

---

## Running the Examples

### Basic Execution
```bash
killer ai_01_generate.killer
killer ai_02_embed.killer
killer ai_03_classify.killer
killer ai_04_extract.killer
killer ai_05_local_infer.killer
killer ai_06_advanced.killer
killer ai_07_providers.killer
```

### With Configuration
```bash
# Set to use local models
KILLER_AI_PROVIDER=local killer ai_05_local_infer.killer

# Set API key for OpenAI
KILLER_OPENAI_KEY=sk-... killer ai_01_generate.killer
```

## AI Functions Quick Reference

### Core AI Functions

| Function | Parameters | Returns | Purpose |
|----------|-----------|---------|---------|
| `ai_generate()` | prompt, options? | string | Generate text |
| `ai_embed()` | text, model? | vector | Create embeddings |
| `ai_classify()` | text, categories, model? | {category, confidence, scores} | Classify text |
| `ai_extract()` | text, schema, model? | object | Extract structured data |
| `ai_local_infer()` | model_path, input | {model, output, latency} | Local model inference |

### Configuration Functions

| Function | Parameters | Returns | Purpose |
|----------|-----------|---------|---------|
| `ai_provider_set()` | provider, config | bool | Set provider config |
| `ai_provider_get()` | provider | dict | Get provider info |
| `ai_cache_enable()` | cache_type | bool | Enable caching |
| `ai_cache_clear()` | - | null | Clear cache |

## Common Patterns

### Pattern 1: Simple Classification
```killer
let categories = ["positive", "negative", "neutral"];
let result = ai_classify("Great product!", categories);
print(result["category"]);  // Output: positive
```

### Pattern 2: Semantic Search
```killer
let docs = ["doc1", "doc2", "doc3"];
let query_embedding = ai_embed("search query");
// Compare embeddings for similarity
```

### Pattern 3: Extract and Classify
```killer
let extracted = ai_extract(text, schema);
let classification = ai_classify(text, categories);
// Combine results
```

### Pattern 4: Generate with Context
```killer
let context = "Killer is a programming language";
let prompt = "Based on " + context + " write about its benefits";
let response = ai_generate(prompt);
```

### Pattern 5: Local-First with Fallback
```killer
ai_provider_set("local", {});
let result = ai_local_infer("./model", input);
if (result == null) {
    ai_provider_set("openai", {});
    result = ai_generate(input);
}
```

## Configuration Examples

### Development Setup
```killer
ai_provider_set("local", {
    "model_path": "./models",
    "debug": true
});
ai_cache_enable("lru");
```

### Production Setup
```killer
ai_provider_set("openai", {
    "api_key": env("OPENAI_KEY"),
    "model": "gpt-4",
    "temperature": 0.7
});
ai_cache_enable("redis");
```

### Privacy-First Setup
```killer
ai_provider_set("local", {
    "model_path": "./models/private"
});
ai_cache_enable("memory");  // Never persist data
```

## Performance Tips

1. **Use Caching**: Enable caching for repeated queries
2. **Batch Operations**: Process multiple items together when possible
3. **Choose Right Provider**: Use local for speed, OpenAI for accuracy
4. **Profile Latency**: Use `ai_local_infer()` latency tracking to optimize
5. **Optimize Prompts**: Well-written prompts get better results faster

## Error Handling

All AI functions return results or errors. Example:
```killer
let result = ai_generate(prompt);
if (result == null) {
    print("Generation failed");
} else {
    // Use result
}
```

## Advanced Topics

- See `ai_06_advanced.killer` for building multi-AI applications
- See `ai_07_providers.killer` for advanced provider configuration
- See individual example files for detailed usage patterns

## Next Steps

1. Run each example to understand the capabilities
2. Combine patterns to build your own AI applications
3. Refer to the API reference for detailed parameters
4. Check performance characteristics for your use case
5. Build end-to-end AI-powered applications with Killer

## Resources

- **API Reference**: See main documentation
- **Tutorial**: Run through examples in order
- **Best Practices**: See `ai_07_providers.killer` for configuration strategies
- **Troubleshooting**: Check error codes in returned results

---

*Last Updated: Killer V3.2 - AI Release*
