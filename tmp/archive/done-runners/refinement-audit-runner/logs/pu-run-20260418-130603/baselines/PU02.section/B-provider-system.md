# B — Provider System (Docs 01, 02, 14)

Parity analysis of `docs/02-agents/01-provider-registry.md`, `02-provider-adapters.md`, `14-provider-integrations.md` vs actual codebase.

---

## B.01 — ProviderConfig Struct (Doc 01)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`ProviderConfig` at `crates/roko-core/src/config/schema.rs:717` with 8 fields: `kind`, `base_url`, `api_key_env`, `command`, `args`, `timeout_ms`, `extra_headers`, `max_concurrent`.

### What exists
`ProviderConfig` at `crates/roko-core/src/config/schema.rs:981` with 10 fields. All 8 documented fields are present, plus two additional timeout fields not in the doc:
- `ttft_timeout_ms` (time-to-first-token timeout) at line 1007
- `connect_timeout_ms` (TCP connection timeout) at line 1013

The struct also uses serde defaults for the timeout fields (`default_provider_timeout_ms`, `default_provider_ttft_timeout_ms`, `default_provider_connect_timeout_ms`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.01a | Doc says line 717, actual is line 981 | `docs/02-agents/01-provider-registry.md:119` | Low — doc line number stale |
| B.01b | Doc omits `ttft_timeout_ms` field | `schema.rs:1007` | Low — doc incomplete but code is richer |
| B.01c | Doc omits `connect_timeout_ms` field | `schema.rs:1013` | Low — doc incomplete but code is richer |

### Verify
```bash
grep -n 'pub struct ProviderConfig' crates/roko-core/src/config/schema.rs
```

---

## B.02 — ProviderKind Enum (Doc 01)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/01-provider-registry.md`

### What the doc says
`ProviderKind` at `crates/roko-core/src/agent.rs:34` with 4 variants: `AnthropicApi`, `ClaudeCli`, `OpenAiCompat`, `CursorAcp`. States that `OpenAiCompat` handles most providers including Perplexity and Gemini.

### What exists
`ProviderKind` at `crates/roko-core/src/agent.rs:35` with **6 variants**:
- `AnthropicApi` (line 37)
- `ClaudeCli` (line 39)
- `OpenAiCompat` (line 41, with `alias = "open_ai_compat"`)
- `CursorAcp` (line 44)
- `PerplexityApi` (line 46) — **not in doc**
- `GeminiApi` (line 48) — **not in doc**

The doc explicitly says Perplexity and Gemini are handled by `OpenAiCompat`. That was the original design, but the code has since promoted them to dedicated `ProviderKind` variants with their own adapters (`PerplexityAdapter`, `GeminiAdapter`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.02a | Doc lists 4 variants, code has 6 | `agent.rs:35-49` | Medium — doc misleads about routing |
| B.02b | Doc says Perplexity/Gemini use `OpenAiCompat` | `docs/02-agents/01-provider-registry.md:202-206` | Medium — contradicts `PerplexityApi` and `GeminiApi` variants |

### Verify
```bash
grep -n 'pub enum ProviderKind' -A 15 crates/roko-core/src/agent.rs
```

---

## B.03 — ModelProfile Struct (Doc 01)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/01-provider-registry.md`

### What the doc says
`ModelProfile` at `crates/roko-core/src/config/schema.rs:819` with ~23 fields including `supports_tools`, `supports_thinking`, `supports_vision`, `supports_web_search`, `supports_mcp_tools`, `supports_partial`, `provider_routing`, `tool_format`, cost fields, `max_tools`, `tokenizer_ratio`, Perplexity fields (`supports_search`, `supports_citations`, `supports_async`, `is_embedding_model`, `search_context_size`, `cost_per_request`).

### What exists
`ModelProfile` at `crates/roko-core/src/config/schema.rs:1110` with **31 fields**. All documented fields are present, plus additional fields not in the doc:
- `supports_grounding` (line 1141) — Google Search grounding
- `supports_code_execution` (line 1143) — built-in code execution
- `supports_caching` (line 1146) — provider-side context caching
- `cost_input_per_m_high` (line 1162) — high-context pricing tier
- `cost_output_per_m_high` (line 1164) — high-context pricing tier
- `thinking_level` (line 1174) — provider-specific reasoning depth label

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.03a | Doc says line 819, actual is line 1110 | `docs/02-agents/01-provider-registry.md:222` | Low — stale line ref |
| B.03b | Doc omits `supports_grounding` field | `schema.rs:1141` | Low — doc incomplete |
| B.03c | Doc omits `supports_code_execution` field | `schema.rs:1143` | Low — doc incomplete |
| B.03d | Doc omits `supports_caching` field | `schema.rs:1146` | Low — doc incomplete |
| B.03e | Doc omits `cost_input_per_m_high` / `cost_output_per_m_high` | `schema.rs:1162-1165` | Low — doc incomplete |
| B.03f | Doc omits `thinking_level` field | `schema.rs:1174` | Low — doc incomplete |

### Verify
```bash
grep -n 'pub struct ModelProfile' -A 95 crates/roko-core/src/config/schema.rs | head -100
```

---

## B.04 — resolve_model Two-Phase Resolution (Doc 01)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`resolve_model` at `crates/roko-core/src/agent.rs:239`. Phase 1: config registry lookup. Phase 2: slug heuristic fallback. Returns `ResolvedModel` with `model_key`, `slug`, `provider_kind`, `provider_config`, `profile`, `backend`.

### What exists
`resolve_model` at `crates/roko-core/src/agent.rs:253`. Matches the documented behavior exactly:
- Phase 1 (line 254-270): looks up `config.models.get(model_key)`, resolves provider config and backend, returns `ResolvedModel`.
- Phase 2 (line 272-281): falls back to `AgentBackend::from_model(model_key)`.

`ResolvedModel` at line 234 has exactly the documented fields: `model_key`, `slug`, `provider_kind`, `provider_config`, `profile`, `backend`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.04a | Doc says line 239, actual is line 253 | `docs/02-agents/01-provider-registry.md:288` | Low — stale line ref |

### Verify
```bash
grep -n 'pub fn resolve_model' crates/roko-core/src/agent.rs
grep -n 'pub struct ResolvedModel' crates/roko-core/src/agent.rs
```

---

## B.05 — Config Merge (effective_providers / effective_models) (Doc 01)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`RokoConfig` provides `effective_providers()` and `effective_models()` methods that merge built-in defaults with user config. Priority: user `[providers.*]` > built-in profiles > slug heuristic fallback.

### What exists
- `effective_providers()` at `crates/roko-core/src/config/schema.rs:452` — Returns user-specified providers if non-empty, otherwise synthesizes from `[agent]` section (creates a `claude_cli` entry from `agent.command`). Also checks for `ANTHROPIC_BASE_URL` env var to synthesize an Anthropic API provider.
- `effective_models()` at `crates/roko-core/src/config/schema.rs:510` — Merges `agent.tier_models`, `agent.default_model`, and explicit `self.models` entries. Explicit models take highest priority.
- Both methods have tests: `effective_providers_backwards_compat` (line 2991), `effective_providers_synthesized_from_agent_section` (line 3005), `effective_models_backwards_compat` (line 3078).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'fn effective_providers\|fn effective_models' crates/roko-core/src/config/schema.rs
```

---

## B.06 — ProviderRouting (OpenRouter) (Doc 01)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`ProviderRouting` struct with fields: `sort`, `order`, `allow_fallbacks`, `max_price`, `require_parameters`. When set, the `OpenAiCompatAdapter` injects these as OpenRouter-specific body extensions.

### What exists
`ProviderRouting` at `crates/roko-core/src/config/schema.rs:1090` with exactly the documented 5 fields:
- `sort` (line 1093)
- `order` (line 1095)
- `allow_fallbacks` (line 1099)
- `max_price` (line 1102)
- `require_parameters` (line 1105)

Injection is handled by `inject_provider_routing()` at `crates/roko-agent/src/provider/openai_compat.rs:76`. This function checks if the provider is OpenRouter (`is_openrouter()` at line 53), then serializes the routing struct into a `"provider"` key in the request body (line 92-93).

Test at `openai_compat.rs:1216` verifies the full routing injection with `sort`, `order`, `allow_fallbacks`, `max_price`, `require_parameters`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub struct ProviderRouting' crates/roko-core/src/config/schema.rs
grep -n 'inject_provider_routing' crates/roko-agent/src/provider/openai_compat.rs
```

---

## B.07 — ProviderAdapter Trait (Doc 02)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`ProviderAdapter` trait at `crates/roko-agent/src/provider/mod.rs:113` with 3 methods: `kind()`, `create_agent()`, `classify_error()`.

### What exists
`ProviderAdapter` trait at `crates/roko-agent/src/provider/mod.rs:314` with exactly the documented 3 methods:
- `fn kind(&self) -> ProviderKind` (line 316)
- `fn create_agent(&self, provider: &ProviderConfig, model: &ModelProfile, options: &AgentOptions) -> Result<Box<dyn Agent>, AgentCreationError>` (lines 319-324)
- `fn classify_error(&self, status: u16, body: &Value) -> ProviderError` (line 328)

Signatures match the doc exactly.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.07a | Doc says line 113, actual is line 314 | `docs/02-agents/02-provider-adapters.md:17` | Low — stale line ref |

### Verify
```bash
grep -n 'pub trait ProviderAdapter' crates/roko-agent/src/provider/mod.rs
```

---

## B.08 — Four Static Adapters (Doc 02)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: B.02
- **Files to modify**: `docs/02-agents/02-provider-adapters.md`

### What the doc says
4 static adapter instances: `ANTHROPIC_API_ADAPTER`, `CLAUDE_CLI_ADAPTER`, `CURSOR_ACP_ADAPTER`, `OPENAI_COMPAT_ADAPTER`. The `adapter_for_kind` function dispatches to these 4.

### What exists
**6 static adapter instances** at `crates/roko-agent/src/provider/mod.rs:72-77`:
- `ANTHROPIC_API_ADAPTER: AnthropicApiAdapter` (line 72)
- `CLAUDE_CLI_ADAPTER: ClaudeCliAdapter` (line 73)
- `CURSOR_ACP_ADAPTER: CursorAcpAdapter` (line 74)
- `OPENAI_COMPAT_ADAPTER: OpenAiCompatAdapter` (line 75)
- `PERPLEXITY_ADAPTER: PerplexityAdapter` (line 76) — **not in doc**
- `GEMINI_ADAPTER: GeminiAdapter` (line 77) — **not in doc**

`adapter_for_kind` at line 87 dispatches to all 6:
```rust
ProviderKind::PerplexityApi => &PERPLEXITY_ADAPTER,
ProviderKind::GeminiApi => &GEMINI_ADAPTER,
```

Adapter implementations:
- `AnthropicApiAdapter` at `crates/roko-agent/src/provider/anthropic_api.rs:11` (440 lines, with tool-loop support via submodule `tool_loop`)
- `ClaudeCliAdapter` at `crates/roko-agent/src/provider/claude_cli.rs:10` (399 lines)
- `CursorAcpAdapter` at `crates/roko-agent/src/provider/cursor_acp.rs:9` (257 lines)
- `OpenAiCompatAdapter` at `crates/roko-agent/src/provider/openai_compat.rs` (1387 lines, handles GLM, Kimi, OpenRouter params)
- `PerplexityAdapter` at `crates/roko-agent/src/perplexity/adapter.rs:167` (518 lines, routes to chat/embed/deep-research/tool-loop)
- `GeminiAdapter` at `crates/roko-agent/src/gemini/adapter.rs:117` (428 lines, routes to compat/native/embed/tool-loop)

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.08a | Doc says 4 adapters, code has 6 | `mod.rs:72-77` | Medium — doc undercounts |
| B.08b | Doc says `adapter_for_kind` is 4-arm match, code is 6-arm | `mod.rs:87-96` | Medium — doc undercounts |
| B.08c | Doc does not describe PerplexityAdapter routing logic | `perplexity/adapter.rs:168-223` | Medium — substantial logic undocumented |
| B.08d | Doc does not describe GeminiAdapter routing logic | `gemini/adapter.rs:119-182` | Medium — substantial logic undocumented |

### Verify
```bash
grep -n 'static.*ADAPTER' crates/roko-agent/src/provider/mod.rs
grep -c 'impl ProviderAdapter for' crates/roko-agent/src/provider/*.rs crates/roko-agent/src/perplexity/adapter.rs crates/roko-agent/src/gemini/adapter.rs
```

---

## B.09 — create_agent_for_model Factory (Doc 02)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/02-provider-adapters.md`

### What the doc says
`create_agent_for_model` at `crates/roko-agent/src/provider/mod.rs:82`. 5-step resolution: resolve_model -> profile fallback -> provider_config fallback -> adapter_for_kind -> create_agent. Simple flow, ~15 lines.

### What exists
`create_agent_for_model` at `crates/roko-agent/src/provider/mod.rs:102`. The function is significantly more complex than documented (~95 lines vs ~15):

1. **Safety layer integration** (line 108): obtains current safety layer or defaults.
2. **resolve_model** (line 109): as documented.
3. **Profile/provider fallback** (lines 110-118): as documented.
4. **Legacy command detection** (lines 119-122): checks `options.command` and `config.agent.command`.
5. **Known protocol command synthesis** (lines 129-157): when no explicit config exists but the command is a known CLI (`claude`, `codex`, `cursor-agent`), synthesizes default `ProviderConfig` and `ModelProfile`. This is a significant undocumented path.
6. **ExecAgent fallback** (lines 158-177): when nothing matches, creates an `ExecAgent` with safety layer. Not documented.
7. **Provider semaphore initialization** (lines 187-190): auto-creates `ProviderSemaphores` if not provided.
8. **Safety-scoped adapter dispatch** (lines 192-195): calls `with_safety_layer` wrapping the adapter.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.09a | Doc says line 82, actual is line 102 | `docs/02-agents/02-provider-adapters.md:144` | Low — stale line ref |
| B.09b | Doc omits safety layer integration | `mod.rs:108,193-195` | Medium — safety is a core feature |
| B.09c | Doc omits known-protocol-command synthesis path | `mod.rs:129-157` | Medium — this is what makes `roko init` work out of the box |
| B.09d | Doc omits ExecAgent fallback path | `mod.rs:158-177` | Low — edge case but safety-critical |
| B.09e | Doc omits provider semaphore auto-init | `mod.rs:187-190` | Low — concurrency control undocumented |

### Verify
```bash
grep -n 'pub fn create_agent_for_model' crates/roko-agent/src/provider/mod.rs
```

---

## B.10 — AgentOptions Struct (Doc 02)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/02-provider-adapters.md`

### What the doc says
`AgentOptions` at `crates/roko-agent/src/provider/mod.rs:132` with 10 fields: `timeout_ms`, `system_prompt`, `tools`, `mcp_config`, `env`, `extra_args`, `effort`, `bare_mode`, `dangerously_skip_permissions`, `name`.

### What exists
`AgentOptions` at `crates/roko-agent/src/provider/mod.rs:333` with **13 fields**:
- `command` (line 334) — **not in doc**
- `timeout_ms` (line 335)
- `system_prompt` (line 336)
- `cached_content` (line 337) — **not in doc** (used for Gemini context caching)
- `tools` (line 338)
- `mcp_config` (line 339)
- `working_dir` (line 340) — **not in doc**
- `provider_semaphores` (line 341) — **not in doc**
- `env` (line 342)
- `extra_args` (line 343)
- `effort` (line 344)
- `bare_mode` (line 345)
- `dangerously_skip_permissions` (line 346)
- `name` (line 347)

Also includes `with_working_dir()` builder (line 353) and `with_perplexity_search_options()` builder (line 360).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.10a | Doc says line 132, actual is line 333 | `docs/02-agents/02-provider-adapters.md:211` | Low — stale line ref |
| B.10b | Doc omits `command` field | `mod.rs:334` | Low — used for legacy command passthrough |
| B.10c | Doc omits `cached_content` field | `mod.rs:337` | Low — Gemini-specific |
| B.10d | Doc omits `working_dir` field | `mod.rs:340` | Medium — important for worktree support |
| B.10e | Doc omits `provider_semaphores` field | `mod.rs:341` | Low — internal concurrency plumbing |

### Verify
```bash
grep -n 'pub struct AgentOptions' crates/roko-agent/src/provider/mod.rs
```

---

## B.11 — Error Classification (ProviderError + RetryAction) (Doc 02)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`ProviderError` enum with 8 variants: `RateLimit`, `AuthFailure`, `Timeout`, `ServerError`, `ContentPolicy`, `ContextOverflow`, `ModelNotFound`, `Other`. `RetryAction` enum with 4 variants: `WaitAndRetry`, `TryFallback`, `TryWithSmallerContext`, `Skip`. `should_retry` maps errors to actions.

### What exists
- `ProviderError` at `crates/roko-agent/src/provider/mod.rs:370` — exactly the documented 8 variants. Also implements `Display` (line 381) and `Error` (line 399).
- `RetryAction` at `crates/roko-agent/src/provider/mod.rs:403` — exactly the documented 4 variants.
- `should_retry` at `crates/roko-agent/src/provider/mod.rs:416` — exact match for the documented mapping.
- Test at line 839 (`retry_policy_maps_error_classes`) verifies all 8 error variants map to the correct retry actions.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub enum ProviderError' crates/roko-agent/src/provider/mod.rs
grep -n 'pub enum RetryAction' crates/roko-agent/src/provider/mod.rs
grep -n 'pub fn should_retry' crates/roko-agent/src/provider/mod.rs
```

---

## B.12 — AgentCreationError (Doc 02)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`AgentCreationError` with 3 variants: `MissingApiKey`, `MissingConfig`, `InvalidKind`.

### What exists
`AgentCreationError` at `crates/roko-agent/src/provider/mod.rs:440` — exactly 3 variants with `thiserror` derives:
- `MissingApiKey(String)` (line 442)
- `MissingConfig(String)` (line 444)
- `InvalidKind(ProviderKind)` (line 446)

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub enum AgentCreationError' crates/roko-agent/src/provider/mod.rs
```

---

## B.13 — TaskRequirements + Automatic Model Selection (Doc 02)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`TaskRequirements` struct with 8 fields. `score_model_for_task` scores profiles against requirements. `select_model_for_task` selects the best model with CascadeRouter bonus integration.

### What exists
- `TaskRequirements` at `crates/roko-core/src/agent.rs:285` with exactly the documented 8 fields: `needs_web_search`, `needs_code_execution`, `needs_thinking`, `needs_vision`, `needs_structured_output`, `min_context_window`, `max_cost_output_per_m`, `max_latency_ms`.
- `score_model_for_task` at `crates/roko-core/src/agent.rs:308` — richer than documented. Hard requirements check is similar, but scoring is more nuanced:
  - Checks `supports_web_search || supports_search || supports_grounding` for web search (line 313).
  - Checks `supports_tools || supports_partial` for structured output (line 314).
  - Adds bonus for `supports_caching` (line 359).
  - Context window ratio bonus (lines 363-366).
  - Cost efficiency scoring with and without budget caps (lines 368-379).
  - Latency bonus for non-thinking models under tight latency (lines 381-384).
- `select_model_for_task` at `crates/roko-core/src/agent.rs:392` — delegates to `select_model_for_task_with_bonus` (line 401), which accepts a learned bonus function closure instead of requiring a `CascadeRouter` reference directly. Sorting includes tiebreakers on context window, cost, and model name (lines 423-431).
- Used by orchestrator at `crates/roko-cli/src/orchestrate.rs:9752`.
- Tests at `agent.rs:1239` and `agent.rs:1281`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.13a | Doc shows `select_model_for_task` taking `CascadeRouter` param, code uses a generic bonus closure | `agent.rs:401-407` | Low — code is more flexible than doc shows |

### Verify
```bash
grep -n 'pub fn score_model_for_task\|pub fn select_model_for_task' crates/roko-core/src/agent.rs
grep -n 'TaskRequirements' crates/roko-cli/src/orchestrate.rs | head -5
```

---

## B.14 — ProviderOptimizations + StreamingMode (Doc 02)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~40
- **Dependencies**: None
- **Files to modify**: `crates/roko-agent/src/provider/mod.rs` (if implementing)

### What the doc says
`ProviderOptimizations` struct with 5 fields: `use_batch_api`, `enable_prompt_caching`, `enable_efficient_tools`, `max_concurrent`, `streaming_mode`. `StreamingMode` enum with 4 variants: `Sse`, `StreamJson`, `JsonRpc`, `None`. Intended for per-adapter optimization hints.

### What exists
Neither `ProviderOptimizations` nor `StreamingMode` exists anywhere in the codebase. Zero matches in `crates/`.

Prompt caching behavior is partially handled:
- Anthropic caching: token-efficient tools and caching headers are set in `anthropic_api/tool_loop.rs`.
- Gemini caching: `cached_content` parameter in `AgentOptions` (line 337) is injected by `GeminiAdapter` (at `gemini/adapter.rs:44`).
- Batch API: not implemented.
- Streaming mode: determined implicitly by each adapter, not via a centralized enum.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.14a | `ProviderOptimizations` struct not implemented | Not in codebase | Low — doc describes aspirational design |
| B.14b | `StreamingMode` enum not implemented | Not in codebase | Low — streaming is implicit per-adapter |

### Verify
```bash
grep -rn 'ProviderOptimizations\|StreamingMode' crates/ --include='*.rs'
```

---

## B.15 — Perplexity Integration (Doc 14)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/14-provider-integrations.md`

### What the doc says
Perplexity exposes 4 API surfaces (chat, agent/responses, search, embeddings). `ModelProfile` includes Perplexity fields: `supports_search`, `supports_citations`, `supports_async`, `search_context_size`, `cost_per_request`. Doc says routing goes via `OpenAiCompat` for chat/search. Status: "Config ready, via shared factory/tool-loop for chat/search."

### What exists
**Substantially richer** than documented. Perplexity has a dedicated `ProviderKind::PerplexityApi` variant and a full adapter:

- `PerplexityAdapter` at `crates/roko-agent/src/perplexity/adapter.rs:167` routes to 4 agent types:
  1. `PerplexityEmbedAgent` (line 194) — for `is_embedding_model` models
  2. `PerplexityDeepResearchAgent` (line 203) — for `supports_async` models
  3. `PerplexityToolLoopAgent` (line 213) — for `supports_tools` models (tool-loop with search options)
  4. `PerplexityChatAgent` (line 219) — for standard chat models with search

- `SearchOptions` struct with 11 fields: `search_domain_filter`, `search_recency_filter`, `search_after_date_filter`, `search_before_date_filter`, `last_updated_after_filter`, `last_updated_before_filter`, `search_context_size`, `search_mode`, `return_images`, `return_related_questions`, `user_location`.
- Perplexity search options are passed via `AgentOptions::with_perplexity_search_options()` (at `mod.rs:360`).
- Full test coverage: 13 tests in `perplexity/adapter.rs` covering all routing paths, search option merging, missing API key errors, and error classification.
- End-to-end factory test in `provider/mod.rs:767` (`create_agent_for_model_routes_perplexity_search_grounded_chat`).

All documented `ModelProfile` Perplexity fields exist in `schema.rs:1182-1198`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.15a | Doc says Perplexity goes via `OpenAiCompat`; code uses dedicated `PerplexityApi` adapter | `perplexity/adapter.rs:168` | Medium — doc misleads about architecture |
| B.15b | Doc does not describe `SearchOptions` struct or merge logic | `perplexity/adapter.rs:76-125` | Medium — substantial undocumented config |
| B.15c | Doc does not describe `PerplexityToolLoopAgent` path | `perplexity/tool_loop.rs` | Medium — agent-driven tool loop is important |

### Verify
```bash
grep -n 'impl ProviderAdapter for PerplexityAdapter' crates/roko-agent/src/perplexity/adapter.rs
grep -rn 'PerplexityApi' crates/roko-core/src/agent.rs
```

---

## B.16 — Gemini Integration (Doc 14)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/14-provider-integrations.md`

### What the doc says
Gemini via OpenAI-compatible endpoint for initial integration. Grounding and code execution are native-API only and "stay adapter-specific." Status: "Config ready, via shared factory/tool-loop for compat models."

### What exists
**Substantially richer** than documented. Gemini has a dedicated `ProviderKind::GeminiApi` variant and a full adapter:

- `GeminiAdapter` at `crates/roko-agent/src/gemini/adapter.rs:117` routes to 5 agent types:
  1. `GeminiEmbedAgent` (line 149) — for `is_embedding_model` models
  2. `GeminiNativeAgent` (line 164) — for `supports_grounding` or `supports_code_execution` models
  3. Native tool-loop via `gemini_native_tool_loop_agent` (line 171) — for `tool_format == "gemini_native"` models
  4. Compat tool-loop via `gemini_tool_loop_agent` (line 173) — for standard tool-capable models
  5. `GeminiCompatAgent` (line 175) — for simple chat models

- 8 source files under `crates/roko-agent/src/gemini/`: `adapter.rs`, `cache.rs`, `compat.rs`, `embed.rs`, `mod.rs`, `native.rs`, `types.rs`, `wire.rs`.
- Context caching support via `options.cached_content` passed through `AgentOptions` and injected as `cached_content` body param (line 44-48).
- Test coverage: 9 tests in `gemini/adapter.rs` covering routing to compat, native, embedding, tool-loop, custom names, missing API key, and error classification.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.16a | Doc says Gemini goes via `OpenAiCompat`; code uses dedicated `GeminiApi` adapter | `gemini/adapter.rs:119` | Medium — doc misleads about architecture |
| B.16b | Doc does not describe native Gemini tool-loop agent path | `gemini/adapter.rs:82-114` | Medium — separate from compat |
| B.16c | Doc does not describe `GeminiEmbedAgent` | `gemini/embed.rs` | Low |
| B.16d | Doc does not describe context caching integration | `gemini/adapter.rs:43-48` | Low |

### Verify
```bash
grep -n 'impl ProviderAdapter for GeminiAdapter' crates/roko-agent/src/gemini/adapter.rs
grep -rn 'GeminiApi' crates/roko-core/src/agent.rs
ls crates/roko-agent/src/gemini/
```

---

## B.17 — ZhipuAI (GLM) Integration (Doc 14)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
GLM models use `OpenAiCompatAdapter`. Config example with `zai` provider. Integration test verifies the factory path with a mock ZhipuAI server. Finish reason normalization handles GLM-specific values.

### What exists
- GLM models route through `OpenAiCompatAdapter` via `ProviderKind::OpenAiCompat`.
- Provider-specific param injection: `inject_glm_params` at `openai_compat.rs:57` injects GLM thinking params (`thinking.type = "enabled"`, `thinking.clear_thinking = true`, `tool_stream = true`) when `model.supports_thinking` and the provider is ZhipuAI.
- Factory test at `provider/mod.rs:712` (`create_agent_for_model_returns_configured_agent`) uses the documented `zai`/`glm-5-1` config.
- Integration test at `crates/roko-agent/tests/provider_integration.rs:275` (`glm_zai_direct`) with mock HTTP poster.
- Live integration test at `provider_integration.rs:536` (`glm_zai_direct_live_http`) behind `integration` feature flag.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'inject_glm_params' crates/roko-agent/src/provider/openai_compat.rs
grep -n 'glm.*zai' crates/roko-agent/tests/provider_integration.rs
```

---

## B.18 — Moonshot (Kimi) Integration (Doc 14)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/14-provider-integrations.md`

### What the doc says
Moonshot Kimi models use `OpenAiCompatAdapter`. Config example. Status: "Config ready, via OpenAiCompat, not yet tested."

### What exists
- Kimi models route through `OpenAiCompatAdapter`.
- Provider-specific param injection: `inject_kimi_params` at `openai_compat.rs:169` injects Kimi thinking params (`thinking.type = "enabled"`) when `model.supports_thinking` and the model slug starts with `kimi-`.
- Integration test at `crates/roko-agent/tests/provider_integration.rs:408` (`kimi_moonshot_direct`) — **doc says "not yet tested" but a test exists**.
- Kimi-specific constraints documented in `openai_compat.rs:1-18` module doc: base64 images only, fixed temperature/top_p with thinking, tool_choice limited to auto/none, max 128 tools, 2-hour timeout, `reasoning_content` must be carried forward.
- Kimi partial continuation test at `openai_compat.rs:1293` verifies partial response handling.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| B.18a | Doc says "Needs testing" but integration test exists | `provider_integration.rs:408` | Medium — doc underreports status |

### Verify
```bash
grep -n 'kimi_moonshot' crates/roko-agent/tests/provider_integration.rs
grep -n 'inject_kimi_params' crates/roko-agent/src/provider/openai_compat.rs
```

---

## B.19 — OpenRouter Integration (Doc 14)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
OpenRouter uses `OpenAiCompatAdapter` with routing extensions. `ProviderRouting` injected into request body. `openrouter_meta` module at `crates/roko-agent/src/provider/openrouter_meta.rs` provides `fetch_model_metadata` for querying the model catalog. Status: "Done, partial tests."

### What exists
- OpenRouter routes through `OpenAiCompatAdapter` with `inject_provider_routing` (at `openai_compat.rs:76`).
- `openrouter_meta.rs` at 387 lines provides `fetch_model_metadata` (line 21) which:
  - Queries `{base_url}/models` with `Authorization: Bearer {key}`, `HTTP-Referer`, `X-Title` headers.
  - Parses `OpenRouterModelsResponse` (wrapped or direct model).
  - Maps `OpenRouterModel` fields to `ModelProfile` including: `context_length`, `max_completion_tokens`, `supported_parameters` (mapped to `supports_tools`, `supports_thinking`, `supports_vision`, `supports_web_search`, `supports_mcp_tools`, `supports_partial`), `pricing` (converted to per-million costs), and `architecture` (for vision detection).
- Integration test at `openrouter_meta.rs:308` (`openrouter_meta_fetch`) with mock HTTP server.
- Integration test at `provider_integration.rs:343` (`glm_openrouter`) for end-to-end OpenRouter routing.
- Provider routing injection test at `openai_compat.rs:1199` verifying all 5 routing fields.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'pub async fn fetch_model_metadata' crates/roko-agent/src/provider/openrouter_meta.rs
grep -n 'openrouter' crates/roko-agent/tests/provider_integration.rs
```

---

## B.20 — resolve_api_key Method (Doc 01)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`ProviderConfig::resolve_api_key()` reads the named environment variable at runtime.

### What exists
`resolve_api_key` at `crates/roko-core/src/config/schema.rs:1037`:
```rust
pub fn resolve_api_key(&self) -> Option<String> {
    self.api_key_env
        .as_ref()
        .and_then(|env_name| std::env::var(env_name).ok())
}
```

Exact match with the documented implementation. Used by all HTTP adapters: `AnthropicApiAdapter` (at `anthropic_api.rs:40`), `CursorAcpAdapter` (at `cursor_acp.rs:38`), `PerplexityAdapter` (at `perplexity/adapter.rs:179`), `GeminiAdapter` (at `gemini/adapter.rs:130`), and the `OpenAiCompatAdapter` (at `openai_compat.rs:182`).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -n 'resolve_api_key' crates/roko-core/src/config/schema.rs
grep -rn 'resolve_api_key' crates/roko-agent/src/provider/ crates/roko-agent/src/perplexity/adapter.rs crates/roko-agent/src/gemini/adapter.rs
```

---

## B.21 — Test Coverage (Doc 02)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Integration tests exercise the full factory path with a mock HTTP server. Shows a test for `create_agent_for_model` with a `glm-5-1` config.

### What exists
Extensive test coverage across multiple files:

**`provider/mod.rs` unit tests** (lines 451-1051):
- `adapter_for_kind_returns_expected_adapter` — all 6 adapter kinds
- `build_tool_dispatcher_attaches_scoped_safety_layer`
- `with_scoped_safety_layer_defaults_when_unscoped`
- `with_scoped_safety_layer_preserves_existing_scope`
- `create_agent_for_model_returns_configured_agent` — GLM via mock HTTP server
- `create_agent_for_model_routes_perplexity_search_grounded_chat` — Perplexity via mock HTTP server
- `create_agent_for_model_routes_perplexity_async_models_to_deep_research`
- `retry_policy_maps_error_classes` — all 8 error variants
- `exec_agent_fallback_for_unknown_model_key`
- `exec_agent_fallback_defaults_safety_layer_when_unscoped`
- `exec_agent_fallback_uses_scoped_safety_layer_when_active`
- `known_protocol_command_detection_handles_paths`
- `create_agent_for_model_uses_command_kind_for_ambiguous_claude_model_key`
- `provider_semaphore_blocks_fourth_request_when_limit_is_three`

**Per-adapter tests**:
- `anthropic_api.rs`: 2 tests (plain agent, tool-loop routing)
- `claude_cli.rs`: 3 tests (all options, working dir, timeout)
- `cursor_acp.rs`: 1 test (ACP request format)
- `openai_compat.rs`: multiple tests (GLM, Kimi, OpenRouter routing, partial continuation, etc.)
- `perplexity/adapter.rs`: 13 tests
- `gemini/adapter.rs`: 9 tests

**Integration test file** (`crates/roko-agent/tests/provider_integration.rs`): 575 lines, 6 tests covering GLM direct, GLM via OpenRouter, Kimi Moonshot, Ollama local via factory, plus live integration test behind feature flag.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| — | None | — | — |

### Verify
```bash
grep -c '#\[test\]\|#\[tokio::test\]' crates/roko-agent/src/provider/mod.rs crates/roko-agent/src/provider/anthropic_api.rs crates/roko-agent/src/provider/claude_cli.rs crates/roko-agent/src/provider/cursor_acp.rs crates/roko-agent/src/provider/openrouter_meta.rs crates/roko-agent/src/perplexity/adapter.rs crates/roko-agent/src/gemini/adapter.rs crates/roko-agent/tests/provider_integration.rs
```

---

## Summary

| ID | Item | Status | Key Finding |
|----|------|--------|-------------|
| B.01 | ProviderConfig | DONE | Code has 2 extra timeout fields; line numbers stale |
| B.02 | ProviderKind | PARTIAL | Code has 6 variants (doc says 4); PerplexityApi + GeminiApi undocumented |
| B.03 | ModelProfile | PARTIAL | Code has 31 fields (doc shows ~23); 6 fields undocumented |
| B.04 | resolve_model | DONE | Exact match; line numbers stale |
| B.05 | Config merge | DONE | Full match |
| B.06 | ProviderRouting | DONE | Full match with test |
| B.07 | ProviderAdapter trait | DONE | Exact match; line number stale |
| B.08 | Static adapters | PARTIAL | Code has 6 adapters (doc says 4); Perplexity + Gemini undocumented |
| B.09 | create_agent_for_model | PARTIAL | Code is ~6x longer than documented; safety, command synthesis, semaphores missing |
| B.10 | AgentOptions | PARTIAL | Code has 13 fields (doc shows 10); working_dir, cached_content missing |
| B.11 | Error classification | DONE | Exact match |
| B.12 | AgentCreationError | DONE | Exact match |
| B.13 | TaskRequirements + selection | DONE | Code is richer than doc; bonus closure vs CascadeRouter |
| B.14 | ProviderOptimizations | NOT DONE | Never implemented; aspirational design in doc only |
| B.15 | Perplexity | DONE | Far beyond doc; dedicated adapter with 4 agent types |
| B.16 | Gemini | DONE | Far beyond doc; dedicated adapter with 5 agent types |
| B.17 | ZhipuAI (GLM) | DONE | Full match with GLM-specific params |
| B.18 | Moonshot (Kimi) | DONE | Doc says untested but test exists; Kimi-specific params implemented |
| B.19 | OpenRouter | DONE | Full match with metadata fetching |
| B.20 | resolve_api_key | DONE | Exact match |
| B.21 | Test coverage | DONE | 40+ tests across 8 files |

**Overall**: The code is **ahead of the docs** in nearly every area. The docs describe the original 4-adapter architecture but the code has evolved to 6 adapters with dedicated `PerplexityApi` and `GeminiApi` variants. The primary action needed is doc updates to reflect the richer reality, not code changes. The only item that exists in docs but not in code is `ProviderOptimizations`/`StreamingMode` (B.14), which is P3 aspirational design.

---

## Agent Execution Notes

### What Batch 02 Should Actually Own Here

Most of this file is doc drift, not runtime debt.

Good batch-`02` ownership in this section is limited to:

- `B.09` where provider creation semantics affect agent runtime behavior,
- backend coverage work that unblocks tool-loop universality,
- any shared-type fallout from response-surface ownership changes.

Do not spend a code batch on:

- updating every stale provider count,
- renaming things just to match the old docs,
- building `ProviderOptimizations` or `StreamingMode`.

### B.09 — `create_agent_for_model`

This is the main provider-system execution seam.

Recommended slice:

1. keep the factory as the single construction hub,
2. route more production paths through it,
3. avoid creating new side-entry agent builders.

Acceptance criteria:

- new runtime behavior still flows through `create_agent_for_model`,
- safety/dispatcher wiring is added through existing helper layers,
- the batch does not fork provider creation logic.

### B.14 — ProviderOptimizations

Treat as deferred. This is not parity-critical unattended work.
