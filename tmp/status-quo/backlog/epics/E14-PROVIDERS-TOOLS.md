# E14 — Providers & Tools  *(correctness of the dispatch path every self-run flows through)*

> **Depends on E01.** Once `plan run` defaults to Runner v2 and spawns real agents
> (E01-T01), *every* self-hosted task dispatches through the provider/tool path this
> epic hardens. A retry that aborts on one 429, a tool allow-list that strips every
> tool on non-Claude models, or a builtin the agent is offered but can't execute are
> all silent-correctness bugs on the hottest path in the system.

## Goal

Make the agent dispatch path **honest and complete**: retries actually retry, every
provider keeps its tools, every advertised builtin is executable (or is not
advertised), Gemini streams, and the provider-kind registry is not missing a family.

## Why

`38-AGENT-PROVIDERS-TOOLS` + `99-TRACE-AGENT-TURN` traced the live turn and found six
defects on the path all self-execution runs on. Four have partial plans (P13/P27/P28/P09);
three root causes are uncovered. The worst is structural: the builtin registry advertises
**37** tools to models but only **16** have executable handlers — an agent that calls any
of the 21 definition-only tools gets a silent `None` from `handler_for`, not a result.

## Source docs

`38-AGENT-PROVIDERS-TOOLS.md` · `99-TRACE-AGENT-TURN.md` ·
supporting: `36-ORCHESTRATION-RUNNERS.md` (dispatch), `98-TRACE-TOOL-DISPATCH.md`.

## Findings covered

| # | Finding | Evidence (re-verified @ HEAD `5852c93c05`) | Prio | State |
|---|---|---|---|---|
| a | Rate-limit retry broken: 429 mapped to `LlmError::Network`; retry loop only retries `LlmError::Provider` → one 429 aborts the turn. `classify_error(429→RateLimit)` exists but is never called on the send path. | `openai_compat_backend.rs:459` `.map_err(\|e\| LlmError::Network(self.decorate_error(&e)))`; `classify_error` at `provider/openai_compat.rs:437` (only called in tests) | **P0** | plan exists (P13) |
| b | Tool-alias bug: PascalCase allow-list (`"Read,Write"`) vs snake_case registry (`read_file`) → filter strips **all** tools on non-Claude providers. `canonical_of_claude` exists but is never called on this path. | `provider/openai_compat.rs:252` `parse_allowed_tools_csv` (raw names), `:348` filter; `canonical_of_claude` at `roko-core/tool/aliases.rs` | **P0** | plan exists (P09) |
| c | Only **16 of 37** builtins have executable handlers; the 17 chain + 4 ISFR tools are definition-only. `handler_for` returns `None` for them → agent offered a tool it cannot run. | `roko-std/tool/handlers.rs:26-45` (16 arms, `_ => None`); `builtin/mod.rs:44` `TOOL_COUNT = 37`; `CHAIN_TOOL_NAMES` (17), `isfr::ISFR_TOOL_NAMES` (4) | **P0** | **uncovered** |
| d | Gemini native backend has no streaming — `GeminiNativeBackend` implements only `send_turn`, no `stream_turn`/`send_turn_streaming`. | `tool_loop/backends/gemini_native.rs:162-163` `impl LlmBackend` has `async fn send_turn` only | **P1** | **uncovered** |
| e | Image/vision unsupported: non-Anthropic translators drop image blocks; ACP hardcodes `image: false`. | `translate/gemini.rs` + `translate/mod.rs` no image handling; ACP `handler.rs` `image: false` | **P1** | plan partial (P28) |
| f | `ProviderKind::GeminiCli` absent — enum has `GeminiApi` but no CLI-subprocess family (cf. `CursorCli`). | `roko-core/agent.rs:35-59` `enum ProviderKind` (10 variants, no `GeminiCli`) | **P2** | **uncovered** |

## Reconciliation with existing plans

| Plan | Finding | Verdict | Root cause? |
|---|---|---|---|
| **P13-rate-limit-retry** (4 tasks) | (a) | **Fully covers.** T1 classifies `send_turn` (non-streaming), T2 `stream_turn`, T3 `send_turn_streaming`, T4 unit test. All three send paths map 429→`ProviderError::RateLimit`→`LlmError::Provider` (the retryable variant), 5xx→`ServerError`. | **Yes** — fixes the exact `map_err` sites the retry loop keys on. No gap. |
| **P09-tool-alias-fix** (3 tasks) | (b) | **Fully covers.** T1 rewrites `parse_allowed_tools_csv` to run `canonical_of_claude` on each name (fixes both the `:252` parse and the `:348` filter via `HashSet<String>`), T2 tests, T3 audits other backends for the same gap. | **Yes** — fixes the parse site so any naming convention resolves. No gap. |
| **P28-image-support** (5 tasks) | (e) | **Partial.** T1 ACP capability from `supports_vision`, T2 log placeholder, T3 image-injection helper on both dispatch paths, T4 `anthropic_api` passthrough, T5 test. Covers ACP + Anthropic path. | **Partial** — does **not** wire image blocks through the **non-Anthropic translators** (`translate/gemini.rs`, openai_compat), which still drop images. → **E14-T06**. |
| **P27-provider-error-ux** (4 tasks) | (adjacent) | Doctor/auth error-message UX (conditional key checks, provider-agnostic messages). Complementary to E14; not one of the six findings. | n/a |

**Net gaps for this epic:** finding (c) the 21 unimplemented handlers (**P0**, biggest hole),
(d) Gemini streaming, (f) `GeminiCli` kind, and the non-Anthropic image-translator tail of (e).

## Task list

Each task: id · title · tier · files · depends_on · acceptance · verify.

### E14-T01 — Stop advertising tools that have no handler  *(P0 correctness gate)*
- **tier** focused · **files** `crates/roko-std/src/tool/handlers.rs`, `crates/roko-std/src/tool/registry.rs`, `crates/roko-std/src/tool/builtin/mod.rs` · **depends_on** [] (soft: E01-T01 for a live path to exercise it)
- The registry offers 37 tools but `handler_for` (`handlers.rs:26`) resolves only 16; the other 21 (17 chain + 4 ISFR) fall through to `_ => None`. Either (a) filter the default agent registry to executable tools only, **or** (b) return a structured "tool not implemented" error at call time instead of silent `None`. Do not offer a model a tool that cannot run.
- **Acceptance:**
  1. A model is never advertised a builtin whose `handler_for` returns `None`, **or** calling such a tool yields a typed `ToolResult` error naming the tool as unimplemented (never a silent drop/panic).
  2. The set of advertised builtins == the set with handlers, verifiable by a test.
  3. Existing 16-tool behavior is unchanged for implemented tools.
- **verify:** `structural` count-match test (below) · `compile` `cargo check -p roko-std` · `test` `cargo test -p roko-std tool::`

### E14-T02 — Implement the 4 ISFR tool handlers
- **tier** integrative · **files** `crates/roko-std/src/tool/builtin/isfr.rs`, `crates/roko-std/src/tool/handlers.rs` · **depends_on** ["E14-T01"]
- The 4 ISFR tools (`isfr::ISFR_TOOL_NAMES`, `builtin/isfr.rs:143`) are definition-only. Add a `ToolHandler` (`fn name`, `async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult`) for each and register them in `handler_for`.
- **Acceptance:**
  1. Each of the 4 ISFR names resolves to a `Some(handler)` from `handler_for`.
  2. Each handler executes and returns a well-formed `ToolResult` (success or typed error), covered by a unit test.
  3. `handler_for` handler count rises from 16 to 20.
- **verify:** `structural` `grep -c 'ISFR' crates/roko-std/src/tool/handlers.rs` > 0 · `compile` `cargo check -p roko-std` · `test` `cargo test -p roko-std isfr`

### E14-T03 — Implement (or feature-gate) the 17 chain-domain tool handlers
- **tier** architectural · **files** `crates/roko-std/src/tool/handlers.rs`, `crates/roko-chain/src/tools.rs` · **depends_on** ["E14-T01"]
- The 17 chain tools (`roko_chain::tools::CHAIN_TOOL_NAMES`) are definition-only; chain runtime is Phase 2+ (needs a blockchain backend for witness anchoring — see CLAUDE.md). Either implement handlers backed by `roko-chain` primitives, **or** feature-gate the chain tools out of the default registry (`builtin/mod.rs:69`) behind a `chain` cargo feature so they are only advertised when a witness backend is wired.
- **Acceptance:**
  1. With chain disabled/absent, the 17 chain tools are **not** advertised (registry excludes them; `TOOL_COUNT` reflects the active set).
  2. With chain enabled, each chain tool resolves to a handler and executes against `roko-chain`.
  3. No path leaves a chain tool advertised-but-unimplemented.
- **verify:** `compile` `cargo check -p roko-std --features chain` and default · `test` `cargo test -p roko-std` (registry vs handler parity holds in both feature states)

### E14-T04 — Add streaming to the Gemini native backend
- **tier** integrative · **files** `crates/roko-agent/src/tool_loop/backends/gemini_native.rs` · **depends_on** []
- `GeminiNativeBackend` (`gemini_native.rs:162`) implements only `send_turn`. Add `stream_turn` / `send_turn_streaming` (matching the `LlmBackend` trait used by `cursor_agent.rs:652` and the streaming call sites in `hermes/http_adapter.rs:409`) using Gemini's `streamGenerateContent` SSE endpoint, emitting incremental events on `event_tx`.
- **Acceptance:**
  1. `GeminiNativeBackend` implements the streaming method(s) required by `LlmBackend`.
  2. A streamed Gemini turn emits ≥1 incremental token event, then a terminal event, over `event_tx`.
  3. Non-streaming `send_turn` behavior is unchanged.
- **verify:** `compile` `cargo check -p roko-agent` · `test` `cargo test -p roko-agent gemini` (streaming event test, mockable)

### E14-T05 — Add `ProviderKind::GeminiCli` and wire its dispatch
- **tier** focused · **files** `crates/roko-core/src/agent.rs`, `crates/roko-agent/src/provider/mod.rs`, `crates/roko-agent/src/provider/pre_flight.rs` · **depends_on** ["E14-T04"]
- `enum ProviderKind` (`agent.rs:35`) has `GeminiApi` but no CLI-subprocess family (cf. `CursorCli`). Add a `GeminiCli` variant with `label()` ("gemini_cli"), serde alias, `AgentBackend` derivation, and registry/pre-flight wiring so a `gemini` CLI subprocess provider is selectable from `roko.toml`.
- **Acceptance:**
  1. `ProviderKind::GeminiCli` exists with a `label()` arm and serde round-trips.
  2. A `roko.toml` provider with `kind = "gemini_cli"` resolves to a working backend (or a clear pre-flight error if the `gemini` CLI is absent — no silent fallback).
  3. All existing `match ProviderKind` sites compile (exhaustive).
- **verify:** `structural` `grep -q 'GeminiCli' crates/roko-core/src/agent.rs` · `compile` `cargo check -p roko-core -p roko-agent` · `test` `cargo test -p roko-core agent`

### E14-T06 — Pass image blocks through non-Anthropic translators  *(closes P28 tail of finding e)*
- **tier** integrative · **files** `crates/roko-agent/src/translate/gemini.rs`, `crates/roko-agent/src/translate/mod.rs`, `crates/roko-agent/src/provider/openai_compat.rs` · **depends_on** [] (soft: P28-T3 for the shared injection helper)
- P28 wires images through ACP + `anthropic_api` but the Gemini/openai-compat translators still drop `ContentBlock::Image`. Translate image blocks to each wire format (Gemini `inlineData`, OpenAI `image_url`) for vision-capable models; keep dropping (with the P28 log placeholder) for non-vision models.
- **Acceptance:**
  1. An image content block survives translation to Gemini `inlineData` and OpenAI `image_url` when the model `supports_vision`.
  2. Non-vision models still drop images cleanly (no wire error), logging the P28 placeholder.
  3. Round-trip translation test covers both providers.
- **verify:** `compile` `cargo check -p roko-agent` · `test` `cargo test -p roko-agent translate` (image passthrough per provider)

### E14-T07 — Regression test: advertised builtins == executable handlers
- **tier** focused · **files** `crates/roko-std/src/tool/registry.rs` (or `tests/`) · **depends_on** ["E14-T01","E14-T02","E14-T03"]
- Add a test asserting that for every tool in the default agent registry, `handler_for(name).is_some()` — so finding (c) can never silently regress. Reuses the `BUILTIN_TOOL_NAMES` / `TOOL_COUNT` scaffolding at `registry.rs:112-121`.
- **Acceptance:**
  1. Test enumerates the default advertised builtins and asserts a handler exists for each.
  2. Test fails if a definition-only tool is re-added to the default registry without a handler.
  3. Passes on the post-T01/T02/T03 tree.
- **verify:** `test` `cargo test -p roko-std registry` · `compile` `cargo check -p roko-std`

## First three tasks (native schema — `tasks.toml`)

```toml
[meta]
plan = "E14-providers-tools"
total = 7
done = 0
status = "ready"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────
# E14-T01: Stop advertising tools that have no handler (P0 correctness)
# ─────────────────────────────────────────────────────────────────────
[[task]]
id = "E14-T01"
title = "Stop advertising builtins that have no executable handler"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 90
files = [
    "crates/roko-std/src/tool/handlers.rs",
    "crates/roko-std/src/tool/registry.rs",
    "crates/roko-std/src/tool/builtin/mod.rs",
]
role = "implementer"
depends_on = []
depends_on_plan = ["E01-EXECUTION-ENGINE"]

[task.context]
read_files = [
    { path = "crates/roko-std/src/tool/handlers.rs", lines = "26-45", why = "handler_for: 16 match arms + `_ => None` for the other 21 tools" },
    { path = "crates/roko-std/src/tool/builtin/mod.rs", lines = "44-97", why = "TOOL_COUNT=37, ROKO_BUILTIN_TOOLS (16 std + chain + isfr), BUILTIN_TOOL_NAMES" },
    { path = "crates/roko-std/src/tool/registry.rs", lines = "108-160", why = "registry construction + BUILTIN_TOOL_NAMES/TOOL_COUNT test scaffolding" },
]
symbols = [
    "handler_for(name: &str) -> Option<Arc<dyn ToolHandler>> — handlers.rs:26",
    "ROKO_BUILTIN_TOOLS: LazyLock<Vec<ToolDef>> — builtin/mod.rs:50",
    "TOOL_COUNT: usize = 37 — builtin/mod.rs:44",
]
anti_patterns = [
    "Do NOT return Some(no-op handler) that silently succeeds — a called-but-unimplemented tool must surface a typed ToolResult error or not be advertised at all.",
    "Do NOT delete the chain/ISFR ToolDefs — they are needed once handlers land (E14-T02/T03); only gate what the DEFAULT agent registry advertises.",
    "Do NOT hardcode a new count literal that will drift — derive the advertised set from handler_for coverage.",
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-std 2>&1"
fail_msg = "roko-std must compile after gating unadvertised tools"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-std tool:: 2>&1"
fail_msg = "Every advertised builtin must resolve to a handler; no silent None on the dispatch path"

acceptance = [
    "No model is advertised a builtin whose handler_for returns None, OR calling such a tool yields a typed unimplemented-tool ToolResult error.",
    "Advertised-builtin set equals handler-backed set, asserted by a test.",
    "The 16 implemented tools behave exactly as before.",
]

# ─────────────────────────────────────────────────────────────────────
# E14-T02: Implement the 4 ISFR tool handlers
# ─────────────────────────────────────────────────────────────────────
[[task]]
id = "E14-T02"
title = "Implement executable handlers for the 4 ISFR tools"
status = "ready"
tier = "integrative"
model_hint = "claude-sonnet-4-20250514"
max_loc = 220
files = [
    "crates/roko-std/src/tool/builtin/isfr.rs",
    "crates/roko-std/src/tool/handlers.rs",
]
role = "implementer"
depends_on = ["E14-T01"]

[task.context]
read_files = [
    { path = "crates/roko-std/src/tool/builtin/isfr.rs", lines = "130-160", why = "all_tool_defs() + ISFR_TOOL_NAMES [4] — the definition-only tools needing handlers" },
    { path = "crates/roko-std/src/tool/builtin/glob.rs", lines = "60-90", why = "reference ToolHandler impl: fn name + async fn execute(&self, call, ctx) -> ToolResult" },
    { path = "crates/roko-std/src/tool/handlers.rs", lines = "26-45", why = "handler_for match block to register the 4 new handlers into" },
]
symbols = [
    "ISFR_TOOL_NAMES: [&str; 4] — isfr.rs:143",
    "trait ToolHandler { fn name(&self)->&str; async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult }",
    "handler_for(name) -> Option<Arc<dyn ToolHandler>> — handlers.rs:26",
]
anti_patterns = [
    "Do NOT stub execute() to return an empty success — implement the real ISFR behavior or a typed error for the unsupported branch.",
    "Do NOT change ISFR_TOOL_NAMES or the ToolDefs — only add handlers + registration.",
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-std 2>&1"
fail_msg = "roko-std must compile with the 4 ISFR handlers"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-std isfr 2>&1"
fail_msg = "Each ISFR tool must resolve to a handler and execute to a well-formed ToolResult"

acceptance = [
    "Each of the 4 ISFR names resolves to Some(handler) from handler_for.",
    "Each handler executes and returns a well-formed ToolResult, covered by a unit test.",
    "handler_for handler count rises from 16 to 20.",
]

# ─────────────────────────────────────────────────────────────────────
# E14-T03: Implement or feature-gate the 17 chain-domain tool handlers
# ─────────────────────────────────────────────────────────────────────
[[task]]
id = "E14-T03"
title = "Implement or feature-gate the 17 chain-domain tool handlers"
status = "ready"
tier = "architectural"
model_hint = "claude-opus-4-20250514"
max_loc = 320
files = [
    "crates/roko-std/src/tool/handlers.rs",
    "crates/roko-std/src/tool/builtin/mod.rs",
    "crates/roko-chain/src/tools.rs",
]
role = "implementer"
depends_on = ["E14-T01"]

[task.context]
read_files = [
    { path = "crates/roko-std/src/tool/builtin/mod.rs", lines = "19-72", why = "CHAIN_DOMAIN_TOOLS/CHAIN_TOOL_NAMES import + extend into ROKO_BUILTIN_TOOLS at line 69" },
    { path = "crates/roko-chain/src/tools.rs", lines = "1-60", why = "CHAIN_DOMAIN_TOOLS + CHAIN_TOOL_NAMES definitions and available chain primitives" },
    { path = "crates/roko-std/src/tool/handlers.rs", lines = "26-45", why = "handler_for — where chain handlers register or stay excluded under feature gate" },
]
symbols = [
    "CHAIN_TOOL_NAMES — 17 names, roko_chain::tools",
    "CHAIN_DOMAIN_TOOLS — Vec<ToolDef>, roko_chain::tools",
    "handler_for(name) -> Option<Arc<dyn ToolHandler>> — handlers.rs:26",
]
anti_patterns = [
    "Do NOT advertise chain tools when no witness/blockchain backend is wired (Phase 2+) — gate them behind a `chain` cargo feature instead of leaving them handler-less.",
    "Do NOT couple roko-std unconditionally to a live chain backend; keep the default build green with chain disabled.",
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-std 2>&1 && cargo check -p roko-std --features chain 2>&1"
fail_msg = "roko-std must compile with chain both disabled (default) and enabled"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-std 2>&1"
fail_msg = "Registry vs handler parity must hold in both feature states — no advertised-but-unimplemented chain tool"

acceptance = [
    "With chain disabled, the 17 chain tools are not advertised and the active TOOL_COUNT reflects that.",
    "With chain enabled, each chain tool resolves to a handler backed by roko-chain and executes.",
    "No path leaves a chain tool advertised-but-unimplemented.",
]
```
