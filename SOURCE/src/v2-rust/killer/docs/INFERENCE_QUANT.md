# Inference: quantization and loader status

Source: `src/inference/` (`gguf.rs`, `quant.rs`, `engine.rs`, `tokenizer.rs`). Dequantizers were checked against
literal ports of the ggml reference loops (`dequantize_row_*` in `ggml-quants.c`) on pseudo-random blocks; the
reference loops were written from memory of ggml, **not compared with ggml output on real model files** (no
network, no ggml build here).

## Types

| Type | Block bytes / elements | Status |
|---|---|---|
| F32, F16, BF16 | 4 / 2 / 2 | verified |
| Q4_0, Q4_1, Q5_0, Q5_1, Q8_0 | 18, 20, 22, 24, 34 per 32 | existing implementation, partly unit tested |
| Q4_K, Q5_K, Q6_K, Q8_K | 144, 176, 210, 292 per 256 | existing implementation, **not re-verified in this pass** |
| Q2_K | 84 per 256 | layout fixed (scales first, shift-plane order); matches the reference loop port |
| Q3_K | 110 per 256 | layout fixed (scale unpacking, plane order, hmask bit order); matches the reference loop port |
| IQ4_NL | 18 per 32 | size fixed (was 144 per 256); table `kvalues_iq4nl` |
| IQ4_XS | 136 per 256 | scale nibble and quant order fixed; matches the reference loop port |
| TQ2_0 | 66 per 256 | element order fixed; matches the reference loop port |
| IQ2_XXS 66, IQ2_XS 74, IQ2_S 82, IQ3_XXS 98, IQ3_S 110, IQ1_S 50, IQ1_M 56 | per 256 | block sizes corrected, but **dequantization returns an error**: `unsupported quantization IQ...: lookup table not verified`. The grid tables in the old code were invented, and no ggml reference data is available to replace them |
| TQ1_0 | 54 per 256 | error (layout not verified) |
| Q8_1, I8..I64, F64, Q4_0_4x4/4x8/8x8, IQ4_NL_4_x | | error: no dequantizer |

## Loader checks

`KillerInference::load` rejects (with an error, not a panic) a model whose config is inconsistent (zero or
non-divisible head counts, odd head_dim), that lacks a required tensor, whose tensor shapes disagree with the
config, or whose embedding has fewer rows than the tokenizer has tokens. Tensor offsets and sizes are checked
against the file length with overflow-safe arithmetic when the file is opened. Fused-QKV models (for example Phi-3
`attn_qkv`) are rejected because the engine expects separate Q/K/V tensors.

## Tokenizer and RoPE

- GPT-2 style BPE (Qwen2, LLaMA 3) applies `tokenizer.ggml.merges` ranks inside pre-tokens. The pre-tokenizer is
  chosen by `tokenizer.ggml.pre` and implemented as a hand-written scanner (`src/inference/pretokenizer.rs`, no
  regex crate) equivalent to the reference regex:
  - `gpt-2`, `gpt2`, `phi-2`, `mpt`, `olmo`, `roberta-bpe`: GPT-2 regex (contractions, ` ?` letters / numbers /
    symbols, whitespace runs that leave the last space to the next word).
  - `llama3`, `llama-v3`, `llama-bpe`, `falcon3`, `dbrx`, `smaug-bpe`: LLaMA 3 regex (case-insensitive contractions,
    one non-letter prefix before letters, numbers in groups of up to 3, newline handling).
  - `qwen2`, `deepseek-r1-qwen`, `stablelm2`: as LLaMA 3 but every digit is its own pre-token.
  - `starcoder`, `refact`, `command-r`, `smollm`: every digit split off, then GPT-2.
  - Missing or any other name: GPT-2 splitting with a warning on stderr (`KillerTokenizer::pre_warning`); ids can
    differ from the reference tokenizer.

  Character classes: `\s` and `\p{N}` use Rust's `is_whitespace` / `is_numeric` (same sets as the reference);
  `\p{L}` is `is_alphabetic && !is_numeric`, which also counts Other_Alphabetic combining marks (for example
  Devanagari vowel signs) as letters, so text with such marks can split differently. The splits were checked against
  Python `re` with ASCII class approximations on 12000 random ASCII strings (the `regex` module was not available);
  non-ASCII cases are unit tested by hand-derived expectations only. Not compared with a real tokenizer.json.
- RoPE pairs (i, i+d/2) for Qwen2, Phi and Gemma and (i, i+1) for LLaMA/Mistral.
- Architectures: `falcon`, `gptneox`, `gptj`, `gpt2`, `bloom`, `mpt` and `starcoder` are rejected at load with
  `unsupported architecture <name>: ...` (they need fused QKV, parallel residual, LayerNorm with bias, GELU MLP or
  ALiBi / learned positions, none of which the engine implements). Other unrecognised `general.architecture` values
  still run with LLaMA settings and print a warning; the tensor name and shape checks still apply.
