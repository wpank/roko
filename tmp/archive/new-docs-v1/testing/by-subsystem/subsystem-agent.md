# roko-agent — Test Coverage

> 346 tests for the agent layer: 5 LLM backends, CascadeRouter, MCP tool integration, 7-step safety pipeline.

**Status**: Shipping
**Crate**: `roko-agent`
**Section**: 02 — Agents
**Last reviewed**: 2026-04-19

---

## Test Count: 346

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `backends` | ~80 | 5 LLM backends × API contract |
| `cascade_router` | ~70 | Static → Confidence → UCB routing |
| `safety_pipeline` | ~60 | 7-step safety checks, auth, pre/post-call verification |
| `mcp_client` | ~40 | MCP protocol, tool dispatch, error handling |
| `context_manager` | ~30 | Context window management, token counting |
| `retry_logic` | ~30 | Backoff, circuit breakers, fallback |
| `tool_dispatch` | ~36 | Tool selection, argument validation, result parsing |

---

## Key Test Focus Areas

### LLM Backends

All five backends (`ClaudeCLI`, `AnthropicAPI`, `OpenAICompat`, `CursorACP`, `Ollama`) are tested against a mock server that replays pre-recorded responses. Tests verify:
- The backend sends the correct wire format for each provider.
- Authentication headers are set correctly (without exposing real credentials).
- Rate limit responses (429) trigger backoff and retry.
- Context window overflow returns `Err(ContextOverflow)` rather than panicking.
- Streaming responses are assembled correctly from SSE chunks.

### CascadeRouter (3-stage routing)

Stage 1 (Static): task type determines model tier deterministically.
Stage 2 (Confidence): low-confidence outputs from Stage 1 escalate to a more capable model.
Stage 3 (UCB): exploration policy selects among equivalent models to balance cost and quality.

Tests verify:
- Static routing always selects the configured tier for known task types.
- Confidence routing escalates when the confidence signal drops below threshold.
- UCB routing converges to the best model over N iterations (bandit convergence).
- Fallback: if the primary model is unavailable, the cascade falls back to the next tier.
- Cost tracking: each dispatch records token cost for the learning subsystem.

Key property: [../by-property/cascade-router-fallback-ordering.md](../by-property/cascade-router-fallback-ordering.md).

### Safety Pipeline (7 steps)

The safety pipeline runs on every LLM call. Tests verify each step:

| Step | Test focus |
|---|---|
| Role authorization | Unauthorized role returns `Err(Unauthorized)` before any LLM call |
| Pre-call content check | Unsafe input content is rejected pre-call |
| Context injection | Safety context is injected into the prompt |
| Response validation | Malformed or unsafe responses are rejected |
| Post-call content check | Output is checked for policy violations |
| Provenance recording | Every LLM call is recorded in the Engram provenance chain |
| Audit log | Every call produces an audit event |

Key property: [../by-property/safety-pipeline-ordering.md](../by-property/safety-pipeline-ordering.md).

### MCP Client

- Protocol compliance: all MCP message types round-trip correctly.
- Error propagation: MCP server errors surface as typed `AgentError` variants.
- Timeout: MCP calls that exceed timeout return `Err(Timeout)`.
- Retry: transient errors (503) retry with backoff; permanent errors (400) do not.

---

## Property Tests

| Property | Test name |
|---|---|
| Safety pipeline ordering | `safety_pipeline_steps_always_in_order` |
| Cascade router fallback | `cascade_router_always_has_fallback` |
| Token counting determinism | `token_count_deterministic` |
| Backend selection reproducibility | `static_routing_deterministic` |

---

## Known Gaps

- The `CursorACP` backend has fewer tests than the other four backends (the protocol is less documented).
- Safety pipeline step 5 (post-call content check) has no property tests yet — only unit tests with fixed inputs.
- Integration tests for all 5 backends in cascade are limited to 2-backend scenarios.

## See also

- [../by-property/cascade-router-fallback-ordering.md](../by-property/cascade-router-fallback-ordering.md)
- [../by-property/safety-pipeline-ordering.md](../by-property/safety-pipeline-ordering.md)
- [../tools-and-harness/02-mock-llms.md](../tools-and-harness/02-mock-llms.md) — tape replay for backend tests
