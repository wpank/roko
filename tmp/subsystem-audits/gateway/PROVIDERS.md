# Provider Matrix: Current + Planned

## Current State

### Bardo Gateway (5 providers)

| Provider | Protocol | Models | Key Feature |
|---|---|---|---|
| Venice | Custom REST | Llama 3.3-70B, DeepSeek R1 | TEE-attested, zero-retention |
| Anthropic | Messages API | claude-opus-4-6, sonnet-4, haiku-4-5 | Key rotation (up to 10 keys) |
| OpenAI | Chat Completions | gpt-4o, o1, o3, o4-mini | Standard |
| Bankr | Custom REST | gemini-2.5-flash, claude-* (via agent wallet) | Self-funding inference |
| OpenRouter | OpenAI-compat | 300+ models (catch-all) | Fallback aggregator |

### Roko Current (6 protocol families)

| ProviderKind | Protocol | Configured In roko.toml |
|---|---|---|
| `ClaudeCli` | Claude CLI subprocess (stream-json) | `anthropic` (command=`claude`) |
| `AnthropicApi` | Anthropic Messages HTTP | (available but not default) |
| `OpenAiCompat` | OpenAI Chat Completions | `openai`, `moonshot` (Kimi), `zhipu` (GLM), `gemini`, `ollama` |
| `CursorAcp` | Cursor Agent Client Protocol | (available) |
| `PerplexityApi` | Perplexity Sonar HTTP | `perplexity` |
| `GeminiApi` | Gemini native + OpenAI compat | `gemini` |

### Roko Production Models (from roko.toml — verify against live config)

**Note**: Model slugs and pricing drift as roko.toml is updated. The values below are from the last verified read. Check `roko.toml` for current truth.

| Key | Slug | Provider (kind) | Cost In/Out per M |
|---|---|---|---|
| `opus` | claude-opus-4-6 | anthropic (`claude_cli`) | $5.00 / $25.00 |
| `sonnet` | claude-sonnet-4-6 | anthropic (`claude_cli`) | $3.00 / $15.00 |
| `haiku` | claude-haiku-4-5 | anthropic (`claude_cli`) | $0.80 / $4.00 |
| `gpt41` | gpt-4.1 | openai (`openai_compat`) | $2.00 / $8.00 |
| `kimi26` | kimi-k2.6 | moonshot (`openai_compat`) | $0.60 / $3.00 |
| `glm51` | glm-5.1 | zhipu (`openai_compat`) | $1.40 / $4.40 |
| `glm5` | glm-5 | zhipu (`openai_compat`) | $1.00 / $3.20 |
| `sonar` | sonar-pro | perplexity (`perplexity_api`) | (not set in config) |
| `sonar-huge` | sonar-reasoning-pro | perplexity (`perplexity_api`) | (not set in config) |
| `gemini-pro` | gemini-2.5-pro | gemini (`openai_compat`) | $1.25 / $10.00 |
| `gemini-flash` | gemini-2.5-flash | gemini (`openai_compat`) | $0.15 / $0.60 |
| (local) | llama3.1 | ollama (`openai_compat`) | free |

**Important**: The `anthropic` provider in production uses `kind = "claude_cli"` (Claude CLI subprocess with stream-json protocol), NOT `kind = "anthropic_api"` (direct HTTP). The `gemini` provider uses `kind = "openai_compat"` through Google's OpenAI-compatible endpoint, not `kind = "gemini_api"`. Both API adapters exist in code but are not the production defaults.

**CostTable defaults vs roko.toml**: `roko-learn/src/cost_table.rs` has hardcoded fallback prices that are stale (e.g., opus listed at $15/$75 there vs $5/$25 in roko.toml). The roko.toml `cost_input_per_m`/`cost_output_per_m` fields on model profiles are the source of truth when set.

---

## Planned Provider Matrix for `roko-gateway`

### Tier 1: Direct API (highest quality, lowest latency)

| Provider | Protocol | Models | API Key Env | Key Features |
|---|---|---|---|---|
| **Anthropic** | Messages API | claude-opus-4-6, sonnet-4-6, haiku-4-5 | `ANTHROPIC_API_KEY` (+ `_2` thru `_N`) | Key rotation, batch API (50% off), prompt caching (90% off), extended thinking |
| **OpenAI** | Chat Completions | gpt-5.x, o3, o4-mini | `OPENAI_API_KEY` | Batch API, automatic caching (50% off), function calling |
| **Google** | Gemini API (native + OpenAI compat) | gemini-2.5-pro, 2.5-flash | `GOOGLE_API_KEY` | Grounding, code execution, massive context (1M+ tokens) |
| **DeepSeek** | OpenAI-compat | DeepSeek-V3.2, DeepSeek-R1 | `DEEPSEEK_API_KEY` | Cheapest reasoning ($0.14/$0.28 per M), off-peak 50-75% discount |
| **Mistral** | OpenAI-compat | Mistral Large, Codestral, Ministral | `MISTRAL_API_KEY` | Code-specialized models, function calling |

### Tier 2: Aggregators (breadth, fallback)

| Provider | Protocol | Models | API Key Env | Key Features |
|---|---|---|---|---|
| **OpenRouter** | OpenAI-compat | 300+ models | `OPENROUTER_API_KEY` | Catch-all fallback, provider routing policies, model metadata API |
| **HuggingFace Inference** | OpenAI-compat | All HF-hosted models | `HF_TOKEN` | 18+ backend providers, routing policies (`:fastest`, `:cheapest`), free tier |
| **Amazon Bedrock** | Bedrock SDK / OpenAI-compat | Claude, Llama, Titan, Mistral | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | Enterprise compliance, VPC, batch |
| **Azure OpenAI** | OpenAI-compat (Azure endpoint) | GPT-5.x, o-series | `AZURE_OPENAI_API_KEY` + endpoint | Enterprise, HIPAA, regional deployment |

### Tier 3: Specialty / Cost-Optimized

| Provider | Protocol | Models | API Key Env | Key Features |
|---|---|---|---|---|
| **Groq** | OpenAI-compat | Llama 3.x, Mixtral, Gemma | `GROQ_API_KEY` | Fastest inference (LPU), great for T1/cheap tasks |
| **Together** | OpenAI-compat | Llama, Code Llama, Qwen, DeepSeek | `TOGETHER_API_KEY` | Fine-tuning, dedicated endpoints |
| **Fireworks** | OpenAI-compat | Llama, Mistral, custom | `FIREWORKS_API_KEY` | Fast inference, function calling, JSON mode |
| **Cerebras** | OpenAI-compat | Llama 3.x | `CEREBRAS_API_KEY` | Wafer-scale inference, extremely fast |
| **SambaNova** | OpenAI-compat | Llama, custom | `SAMBANOVA_API_KEY` | Reconfigurable dataflow, long context |
| **Venice** | Custom REST | Llama, DeepSeek (TEE) | `VENICE_API_KEY` | Zero-retention, TEE-attested privacy |

### Tier 4: Local / Self-Hosted

| Provider | Protocol | Models | Config | Key Features |
|---|---|---|---|---|
| **Ollama** | OpenAI-compat | Any GGUF model | `base_url` | Local, free, private |
| **vLLM** | OpenAI-compat | Any HF model | `base_url` | Production serving, PagedAttention |
| **llama.cpp server** | OpenAI-compat | Any GGUF | `base_url` | Lightweight local serving |
| **TGI** (HF Text Gen Inference) | OpenAI-compat | Any HF model | `base_url` | HF official serving stack |
| **HF Inference Endpoints** | OpenAI-compat | Any HF model | `HF_TOKEN` + endpoint URL | Managed GPU (scale-to-zero), pay-per-minute |

### Tier 5: Research / CLI

| Provider | Protocol | Interface | Key Features |
|---|---|---|---|
| **Perplexity** | OpenAI-compat + Sonar extensions | HTTP | Web-grounded search + reasoning |
| **Moonshot (Kimi)** | OpenAI-compat | HTTP | Chinese language strength, long context |
| **ZhipuAI (GLM)** | OpenAI-compat | HTTP | Chinese language, thinking mode |
| **Claude CLI** | stream-json subprocess | CLI | MCP tools, computer use, project knowledge |
| **Cursor ACP** | Agent Client Protocol | JSON-RPC | Editor integration, file context |

---

## HuggingFace Integration (5 Layers)

From `02-huggingface-integration.md` — HuggingFace provides 5 complementary API surfaces:

### Layer 1: Inference Providers (immediate)
- **Base URL**: `https://router.huggingface.co/v1`
- OpenAI-compatible chat completions across 18+ backend providers
- Routing policies: `:fastest`, `:cheapest`, `:preferred`, `:provider-name`
- No HF markup — pass-through provider costs
- **Gateway integration**: `kind = "openai_compat"` works today. Dedicated `HuggingFaceApi` kind adds routing policy metadata.

### Layer 2: Hub API (model discovery)
- **Base URL**: `https://huggingface.co/api`
- `GET /api/models?search=...` → dynamic model discovery for CascadeRouter
- `GET /api/models/{id}` → auto-populate ModelProfile from model cards
- Webhooks → react to new model releases, auto-benchmark

### Layer 3: Dataset Viewer (benchmark data)
- **Base URL**: `https://datasets-server.huggingface.co`
- Stream dataset rows via REST, no Python dependency
- Parquet bulk access for SWE-bench, MBPP, HumanEval, etc.
- Replaces Python `datasets` library entirely

### Layer 4: Inference Endpoints (elastic compute)
- Dedicated GPU instances (A10G/A100/H200)
- Scale to zero between runs (no billing when idle)
- Pay-by-the-minute ($0.50/hour GPU)
- Use case: batch benchmark runs — spin up, run 200 instances, tear down

### Layer 5: AutoTrain (fine-tuning loop)
- SFT, ORPO, DPO, KTO for LLMs
- Push fine-tuned model back to Hub
- **The exponential loop**: episodes → training data → fine-tune → Hub → CascadeRouter arm → explore → if wins → more data → ...

---

## Provider Capability Matrix

| Provider | Chat | Stream | Tools | Vision | Thinking | Batch | Cache | Embed | JSON Mode |
|---|---|---|---|---|---|---|---|---|---|
| Anthropic | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| OpenAI | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (auto) | ✓ | ✓ |
| Google Gemini | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| DeepSeek | ✓ | ✓ | ✓ | ✗ | ✓ | ✗ | ✗ | ✗ | ✓ |
| Mistral | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ | ✓ |
| OpenRouter | ✓ | ✓ | varies | varies | varies | ✗ | ✗ | varies | varies |
| HuggingFace | ✓ | ✓ | ✓ | varies | varies | ✗ | ✗ | ✓ | ✓ |
| Groq | ✓ | ✓ | ✓ | varies | ✗ | ✗ | ✗ | ✗ | ✓ |
| Together | ✓ | ✓ | ✓ | varies | ✗ | ✗ | ✗ | ✓ | ✓ |
| Fireworks | ✓ | ✓ | ✓ | varies | ✗ | ✗ | ✗ | ✓ | ✓ |
| Ollama | ✓ | ✓ | ✓ | varies | ✗ | ✗ | ✗ | ✓ | ✓ |
| Perplexity | ✓ | ✓ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ |
| AWS Bedrock | ✓ | ✓ | ✓ | varies | varies | ✓ | ✗ | ✓ | varies |
| Azure OpenAI | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

---

## Pricing Comparison (per M tokens, April 2026)

### Premium Tier (best quality)

| Model | Input | Output | Cached | Provider |
|---|---|---|---|---|
| claude-opus-4-6 | $5.00 | $25.00 | $0.50 | Anthropic |
| gpt-5.4 | $3.00 | $12.00 | $1.50 | OpenAI |
| o3 | $10.00 | $40.00 | $5.00 | OpenAI |
| gemini-2.5-pro | $1.25 | $10.00 | $0.315 | Google |
| sonar-reasoning-pro | $5.00 | $10.00 | — | Perplexity |

### Mid Tier (quality/cost sweet spot)

| Model | Input | Output | Cached | Provider |
|---|---|---|---|---|
| claude-sonnet-4-6 | $3.00 | $15.00 | $0.30 | Anthropic |
| gpt-5.2 | $2.00 | $8.00 | $1.00 | OpenAI |
| glm-5.1 | $1.40 | $4.40 | — | ZhipuAI |
| kimi-k2.5 | $0.60 | $3.00 | — | Moonshot |
| Mistral Large | $2.00 | $6.00 | — | Mistral |

### Economy Tier (cheapest per token)

| Model | Input | Output | Cached | Provider |
|---|---|---|---|---|
| claude-haiku-4-5 | $0.80 | $4.00 | $0.08 | Anthropic |
| gemini-2.5-flash | $0.15 | $0.60 | $0.0375 | Google |
| DeepSeek-V3.2 | $0.14 | $0.28 | — | DeepSeek |
| o4-mini | $1.10 | $4.40 | $0.55 | OpenAI |
| glm-5 | $1.00 | $3.20 | — | ZhipuAI |
| Groq Llama 3.3-70B | $0.59 | $0.79 | — | Groq |

### Free / Local Tier

| Model | Cost | Provider |
|---|---|---|
| Any GGUF model | $0 | Ollama |
| Any HF model | $0 | vLLM / TGI (self-hosted) |
| Llama 3.x (free tier) | $0 (rate-limited) | Groq / HuggingFace |
