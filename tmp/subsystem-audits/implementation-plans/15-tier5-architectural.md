# 15 — Tier 5: Architectural Extraction (8 items, all OPEN)

Restructure on a clean, smaller codebase. ~3-5 sessions.

**Source**: doc 41 backlog T5-35..T5-42, doc 36 (orchestrate.rs decomposition),
docs 23 / 24 (dispatch / runtime ledger redesign plans).

These items overlap with the deeper subsystem plans (20: orchestrate.rs
extraction, 22: dispatch streaming, 24: runtime ledger). Read those for
the full architectural context; this file gives the per-item mechanical
sequence.

---

## Cross-Cutting Notes

### Anti-patterns to enforce (architectural)

1. **No new "shadow" runtime / dispatcher / state machine.** If you need
   different behavior, extend the existing one or add an adapter.
2. **Pure mechanical moves first; behavior changes second.** When extracting
   a function, the first commit moves code with no logic change. Later
   commits refactor.
3. **No "incremental" public API leaks.** A new `pub fn` introduced as part
   of an extraction must be the *intended* permanent surface, not a
   transient shim. If transient, prefix with `pub(crate)` or
   `pub(super)`.
4. **Compatibility adapters are the deletion list.** Every "compat shim"
   landed must have a tracked deletion follow-up. No permanent compat.
5. **One slice per commit.** Each architectural slice = one commit. If
   touching > 1 file substantially, those go in their own commits.

### Bisect-friendly slicing

For T5-35 and T5-40 specifically, structure each slice so that **between
commits**, the workspace builds and tests pass. This means:

- Step 1: Add the new module and its tests; the original code path is
  unchanged. (`cargo test` green; new code unused except by tests.)
- Step 2: Switch the original site to delegate to the new module. (`cargo
  test` green; new code now in product path.)
- Step 3: Delete the inlined version. (`cargo test` green; only the new
  module remains.)

Three commits per slice. If a regression appears, `git bisect` lands on
the exact slice that introduced it.

---

## [ ] T5-35: Extract `dispatch_agent_with` into composable units

**Why**: `dispatch_agent_with` is ~2,059 lines (`crates/roko-cli/src/orchestrate.rs:14575+`).
It mixes model selection, prompt assembly, agent invocation, and outcome
recording. Adding a new branch means appending another 50-100 lines to
the same function. Every recent runner batch contributed to its growth.

This task extracts it into 4 focused units, one per commit. After
completion, `dispatch_agent_with` is ~50 lines that calls into the four
modules.

**See** `20-orchestrate-rs-extraction.md` **for the full plan**, which
covers naming, module placement (`orchestrate/dispatch/<unit>.rs`), and
the dependency order. Highlights:

| Slice | Source lines (approx) | New module | Owner concept |
|---|---|---|---|
| 1 | 14575-14910 (~335) | `orchestrate/dispatch/select_model.rs` | Model + provider resolution |
| 2 | 14910-15260 (~350) | `orchestrate/dispatch/build_prompt.rs` | 9-layer prompt assembly + safety contract injection |
| 3 | 15260-15590 (~330) | `orchestrate/dispatch/launch_agent.rs` | Spawn / stream / collect |
| 4 | 15590-15885 (~295) | `orchestrate/dispatch/record_outcome.rs` | Episode write + feedback emission + ledger entry |

(Approximate line ranges; recompute when starting because earlier landed
work shifted offsets.)

**Per-slice procedure** (repeat 4 times):

1. Identify the contiguous block in `dispatch_agent_with`.
2. Determine its inputs (mostly local `let`s above the block) and outputs
   (mostly local `let`s and side effects after the block).
3. Define a typed request/response struct in the new module:
   ```rust
   pub(crate) struct SelectModelReq<'a> {
       pub task: &'a Task,
       pub config: &'a RokoConfig,
       pub router: &'a CascadeRouter,
       pub override: Option<&'a str>,
   }

   pub(crate) struct SelectModelRes {
       pub model: ResolvedModel,
       pub provider: ResolvedProvider,
       pub source: ModelChoiceSource,
   }

   pub(crate) async fn select_model(req: SelectModelReq<'_>) -> Result<SelectModelRes, DispatchError> {
       // 335 lines moved verbatim
   }
   ```
4. Move the block. The mechanical move:
   - Cut the block from `dispatch_agent_with`.
   - Paste into the new module's function body.
   - Convert local references to use `req.<field>`.
   - Convert the block's "outputs" into the response struct.
5. In `dispatch_agent_with`, replace the block with:
   ```rust
   let res = orchestrate::dispatch::select_model::select_model(SelectModelReq {
       task: &task,
       config: &cfg,
       router: &cascade_router,
       override: cli_model.as_deref(),
   }).await?;
   let model = res.model;
   let provider = res.provider;
   let model_source = res.source;
   ```
6. Run `cargo check`, then `cargo test --workspace`. Both must pass.
7. Commit: `T5-35a: Extract select_model from dispatch_agent_with` (4
   commits total — `a/b/c/d`).

**Verify per slice**:

```bash
wc -l crates/roko-cli/src/orchestrate.rs
wc -l crates/roko-cli/src/orchestrate/dispatch/<new_module>.rs
# Sum should match (within margin) the pre-extraction line count
cargo test --workspace
```

**After all 4 slices**:

```bash
wc -l crates/roko-cli/src/orchestrate.rs
# Should be ~21,000 lines (down from 22,756)
rg 'fn dispatch_agent_with' crates/roko-cli/src/orchestrate.rs
# Function still exists, but body is < 100 lines
```

### Do not

- Refactor *inside* the moved blocks during the move. Move first, refactor
  later.
- Change function signatures of helpers called by the moved code. They
  stay where they are.
- Merge two slices into one commit even if they share a helper. Extract
  the helper separately first.
- Land an extraction without an accompanying test that exercises the new
  module's request struct.
- Extract into a new crate. The 4 modules live under
  `crates/roko-cli/src/orchestrate/dispatch/`.

**Estimated effort**: 6-10 hours per slice (24-40 hours total). This is
the heaviest item in the plan.

**Detailed expansion**: see `20-orchestrate-rs-extraction.md`.

---

## [ ] T5-36: Migrate remaining serve dispatch to `ModelCallService`

**Why**: Some serve routes still construct `reqwest::Client` and hit
provider HTTP directly. After T5-35, `dispatch_agent_with` is the
runner's path; serve has its own remaining paths.

**Files** (one route per commit):

```bash
rg 'reqwest::Client::new|reqwest::ClientBuilder' crates/roko-serve/src/routes/ -l
```

Expected hits include:

- `routes/inference.rs` (if not already migrated)
- `routes/providers.rs::test_provider` (already migrated for one path; check)
- `routes/research.rs`
- `routes/connectors.rs` (some external API calls; only migrate the LLM ones)
- `routes/agents.rs` (provider calls for managed agents)

### Per-route procedure

1. Identify the LLM call site:
   ```rust
   let client = reqwest::Client::new();
   let resp = client.post(url).json(&body).send().await?;
   let parsed: ProviderResponse = resp.json().await?;
   ```
2. Replace with `state.model_call_service`:
   ```rust
   let req = ModelCallRequest {
       model: model_slug.into(),
       provider: provider_id.into(),
       messages: vec![/* ... */],
       max_tokens: cfg.max_tokens,
       // ...
   };
   let resp = state.model_call_service.call(req).await?;
   ```
3. The `ModelCallResponse` carries `UsageObservation`, `text`, and
   provider-native metadata. Adapt the route's response shape if needed.
4. Remove unused `reqwest` import if it was the only use. (Don't remove
   `reqwest` from `roko-serve`'s Cargo.toml; other routes use it for
   non-LLM HTTP.)
5. Tests:
   ```rust
   #[tokio::test]
   async fn route_uses_shared_dispatch() {
       let app = build_test_app().await;
       // Replace state.model_call_service with a stub provider; assert it's called.
   }
   ```

### Per-route notes

- **`routes/inference.rs`**: probably the first to migrate; serves
  `/api/inference/complete`. Check if already done.
- **`routes/providers.rs::test_provider`**: D9 in doc 35 says this is
  done. Verify with `rg 'reqwest::Client' crates/roko-serve/src/routes/providers.rs`.
- **`routes/research.rs`**: research uses Perplexity Sonar. The Perplexity
  parser already uses `UsageObservation`; the dispatch path should also
  share `ModelCallService`.
- **Non-LLM HTTP**: Some routes (e.g. `connectors.rs` for GitHub /
  Linear / Slack) hit non-LLM APIs. **Do not migrate those**. The
  scope is LLM dispatch only.

### Verify

```bash
# Per migrated route, confirm no raw client construction
rg 'reqwest::Client::new|reqwest::ClientBuilder' crates/roko-serve/src/routes/<route>.rs
# Empty for migrated routes

# Confirm shared dispatch is used
rg 'model_call_service\.(call|stream)' crates/roko-serve/src/routes/
```

After all routes migrated:

```bash
rg 'reqwest::Client::new|reqwest::ClientBuilder' crates/roko-serve/src/routes/ \
  | rg -v '(connectors|webhooks|integrations|chain|deploy|relay)'
# Should be empty (the listed routes do non-LLM HTTP)
```

### Do not

- Migrate non-LLM HTTP. GitHub/Linear/Slack connectors use `octocrab`,
  `linear-client`, etc., not `ModelCallService`.
- Migrate all routes in one PR. One per commit.
- Change the route's response shape "while you're there." Migration is
  mechanical.
- Skip the auth check. `state.model_call_service` enforces provider auth;
  if a route currently bypasses, that's a separate fix in
  `DispatchResolver`.

**Estimated effort**: 2 hours per route (6-12 hours total).

---

## [ ] T5-37: Remove or quarantine `dispatch_direct`

**Depends on**: T5-36 (mostly).

**Why**: `crates/roko-cli/src/dispatch_direct.rs` is the legacy "shell out
to the provider CLI directly" path. It bypasses `ModelCallService`, the
safety layer, and the streaming contract. Several tests and runtime paths
still use it (chat_inline, lib, unified, marketplace).

**Approach**: feature-gate behind `legacy-direct-dispatch`. Production
code never sets this feature; tests can.

### Step 1: Make `dispatch_direct` feature-gated

In `crates/roko-cli/Cargo.toml`:

```toml
[features]
default = []
legacy-direct-dispatch = []
```

In `crates/roko-cli/src/lib.rs`:

```rust
#[cfg(feature = "legacy-direct-dispatch")]
pub mod dispatch_direct;

// (keep the public re-export under the same feature gate)
```

In every site that imports `dispatch_direct`:

```rust
#[cfg(feature = "legacy-direct-dispatch")]
use crate::dispatch_direct::*;
```

Build with `--features legacy-direct-dispatch` for tests that need it;
default build excludes the module entirely.

### Step 2: Identify production callers

```bash
rg 'dispatch_direct' crates/ -g '*.rs'
```

Expected hits (verified 2026-05-01):

- `crates/roko-cli/src/dispatch_direct.rs` (definition)
- `crates/roko-cli/src/lib.rs` (re-export)
- `crates/roko-cli/src/chat_inline.rs` (production caller)
- `crates/roko-cli/src/unified.rs` (production caller)
- `crates/roko-chain/src/marketplace.rs` (production caller)
- `crates/roko-chain/src/identity_economy_markets.rs` (production caller)

For each production caller:

1. Replace `dispatch_direct::run(...)` with the equivalent
   `ModelCallService::call(...)` or `stream(...)`.
2. If the caller is itself legacy and not actively used, mark it
   `#[cfg(feature = "legacy-direct-dispatch")]` too — but **only** if
   confirmed unused in production.
3. **Do not** preserve a fallback path that calls `dispatch_direct` when
   `ModelCallService` errors. That's the silent-fallback anti-pattern.

### Step 3: Confirm production builds without the feature

```bash
cargo build --workspace
cargo build --workspace --release
# These run with default features; dispatch_direct is excluded.
cargo test --workspace
# Tests that need it pass --features legacy-direct-dispatch internally.
```

If a production build fails because a non-test caller still references
`dispatch_direct`, that caller hasn't been migrated; finish migration
before flipping the feature gate.

### Step 4: Static check

Add to `scripts/roko-fitness-checks.sh` (plan 27):

```bash
# Reject any non-feature-gated dispatch_direct usage.
violations=$(rg 'dispatch_direct' crates/ -g '*.rs' \
  | rg -v '#\[cfg\(feature = "legacy-direct-dispatch"\)\]' \
  | rg -v 'crates/roko-cli/src/dispatch_direct\.rs')
if [ -n "$violations" ]; then
    echo "FAIL: dispatch_direct used without feature gate:"
    echo "$violations"
    exit 1
fi
```

### Step 5: Plan deletion

Once the static check is green for 30 days, follow up with a "delete
`dispatch_direct.rs` entirely" commit. Track that as T5-37b.

### Verify

```bash
# Default build excludes the module
! cargo build --workspace 2>&1 | rg 'dispatch_direct'
cargo build --workspace --features legacy-direct-dispatch
# Should still work
cargo test --workspace
```

### Do not

- Delete `dispatch_direct.rs` immediately. The module probably has tests
  that need it. Feature-gate first; delete later.
- Make `legacy-direct-dispatch` a default feature. The whole point is to
  prevent accidental use.
- Quietly migrate production callers without confirming the new path
  exists (e.g. `chat_inline.rs` may have specific behavior the migration
  must preserve — verify).

**Estimated effort**: 4-8 hours, plus 1-2 hours per production caller.

---

## [ ] T5-38: Collapse config into validated model

**Why**: Config loading today returns `RokoConfig` with no provenance, no
versioning, and no semantic validation beyond the strict-TOML check
(T1-12). Adding a `ResolvedConfig` / `ValidatedConfig` wrapper makes
provenance and source-tracking first-class.

**File**: `crates/roko-core/src/config/`

**See** `23-config-validation-pipeline.md` **for the full plan**.

Highlights:

1. Add `ResolvedConfig` (where each field came from: shared / local / env /
   CLI).
2. Add `ValidatedConfig` (semantic checks: provider auth resolves,
   model slugs are unique, gate thresholds are in [0,1], etc.).
3. `load_config()` returns `ValidatedConfig` (newtype around
   `Arc<RokoConfig>` with a `provenance: ConfigProvenance` companion).
4. Migrate consumers (one crate per commit): `roko-cli`, `roko-serve`,
   `roko-acp`, `roko-runtime`, `roko-agent`.

**Estimated effort**: 8-15 hours. See plan 23.

---

## [ ] T5-39: Add budget guardrail to Ollama dispatch path

**Why**: Most dispatch paths wrap their `ToolLoop` (the agent's iteration
loop) in `TaskRunner` which enforces `RunnerBudgetGuardrail` — a hard cap
on iterations, time, and cost. The Ollama path (~`orchestrate.rs:15910-16011`)
does not.

A misconfigured local Ollama agent can loop indefinitely, consuming local
GPU until OOM.

**File**: `crates/roko-cli/src/orchestrate.rs:15910-16011`

### Step 1: Identify the Ollama loop

```bash
rg 'ollama|Ollama' crates/roko-cli/src/orchestrate.rs -n | head -20
```

The block starts around line 15910 and ends around 16011. Read carefully;
some Ollama-specific options may not map to `TaskRunner`'s expectations.

### Step 2: Wrap in `TaskRunner` (if practical)

```rust
let runner = TaskRunner::new(
    OllamaToolLoop::new(...),
    RunnerBudgetGuardrail::from_config(&task_runner_config),
);
let outcome = runner.run().await?;
```

If the Ollama loop's signature doesn't match `TaskRunner`'s `ToolLoop`
trait, **add an adapter** (don't fork `TaskRunner`):

```rust
struct OllamaToolLoopAdapter { /* ... */ }
impl ToolLoop for OllamaToolLoopAdapter {
    async fn step(&mut self) -> Result<StepOutcome, ToolLoopError> {
        // delegate
    }
}
```

### Step 3: Alternatively, add a guardrail directly

If `TaskRunner` integration is too invasive, add `RunnerBudgetGuardrail`
inline:

```rust
let mut guardrail = RunnerBudgetGuardrail::from_config(&cfg.runner);
loop {
    guardrail.tick()?;     // returns Err if budget exceeded
    let step = ollama_step().await?;
    if step.is_done() { break; }
}
```

### Step 4: Tests

```rust
#[tokio::test]
async fn ollama_dispatch_respects_iteration_cap() {
    let cfg = make_config_with_max_iterations(3);
    let outcome = dispatch_ollama_loop(&cfg, /* runaway-stub */).await;
    assert!(matches!(outcome.unwrap_err(), DispatchError::IterationBudgetExceeded));
}
```

### Verify

```bash
rg 'RunnerBudgetGuardrail' crates/roko-cli/src/orchestrate.rs
# Should appear in the Ollama section
cargo test -p roko-cli ollama_budget --lib
```

### Do not

- Remove `OllamaToolLoop` or rename it.
- Add a separate guardrail config — reuse `runner.budget` config.
- Skip this for "local-only" runs. Local Ollama can still consume the
  whole machine.

**Estimated effort**: 2-3 hours.

---

## [ ] T5-40: Replace event-replay reports with `RunLedger`

**Why**: Workflow reports today are inferred by replaying events from a
ledger of strings. Bugs include: missed events produce missing report
fields; events received twice produce duplicates; ordering issues produce
wrong status. The `RunLedger` (typed entries) was designed to fix this.

**File**: `crates/roko-runtime/src/run_ledger.rs` and consumers in
`roko-cli/src/orchestrate.rs`, `roko-runtime/src/workflow_engine.rs`.

**See** `24-runtime-ledger-migration.md` **for the full plan**. Highlights:

| Slice (one commit each) | Migrated source | Affected ledger entries |
|---|---|---|
| 1 | Gate verdicts | `RunLedger::Gate { rung, status, duration }` |
| 2 | Artifact validity | `RunLedger::Artifact { id, outcome }` |
| 3 | Event log | `RunLedger::Event { kind, payload }` |
| 4 | Resume / checkpoint | `RunLedger::Checkpoint { state }` |

After all 4 slices, the workflow report is constructed from
`RunLedger::derive_report()` — a single function that walks typed entries.
The string-event replay path is removed.

**Estimated effort**: 6-10 hours per slice (24-40 hours total). See plan 24.

---

## [ ] T5-41: Migrate demo automation off prompt scraping

**Why**: `demo/demo-app/src/lib/scenario-runners/` uses regex on PTY
output to detect command success. The terminal subsystem now emits typed
`CommandEvent` lifecycle events (`Started`, `Output`, `Exited`,
`SpawnFailed`, `Cancelled`). The scenario runners should subscribe to
those, not parse output.

**File**: `demo/demo-app/src/lib/scenario-runners/*.ts`

**See** `26-terminal-demo-truth.md` **for the full plan**. Highlights:

- Subscribe to `/api/terminal/sessions/{id}/events` (SSE) for typed events.
- Replace `if (line.match(/\$ $/))` with `if (event.type === 'exited' && event.code === 0)`.
- The scenario success/failure result is the `Exited.code`, not a regex
  on output.

**Estimated effort**: 4-6 hours.

---

## [ ] T5-42: Provider-native structured history for all adapters

**Why**: Some provider adapters still build a flat string ("user: foo\n
assistant: bar\n...") instead of using the provider's structured message
format. This loses role boundaries and tool-call structure on
multi-turn conversations.

**Files** (one adapter per commit):

- `crates/roko-agent/src/translate/openai.rs` — already does it; reference.
- `crates/roko-agent/src/providers/anthropic*.rs`
- `crates/roko-agent/src/providers/gemini*.rs`
- `crates/roko-agent/src/providers/cerebras*.rs` (OpenAI-compatible; verify)
- `crates/roko-agent/src/providers/cursor*.rs`
- `crates/roko-agent/src/providers/ollama*.rs` (chat endpoint, not generate)

### Per-adapter procedure

1. Verify the adapter accepts `Vec<Message>` where `Message::role: Role`
   and `Message::content: ContentBlock`.
2. Convert to provider-native:

   - Anthropic: `[{role: "user", content: [{type: "text", text: ...}]}]`.
     Tool use blocks: `{type: "tool_use", id, name, input}` and
     `{type: "tool_result", tool_use_id, content}`.
   - Gemini: `contents: [{role: "user", parts: [{text: ...}]}]`. Tool
     use: `parts: [{functionCall: {name, args}}]` and
     `parts: [{functionResponse: {name, response}}]`.
   - OpenAI-compatible (Cerebras, Cursor, others): same as
     `roko-agent/src/translate/openai.rs`.
   - Ollama chat: `[{role, content}]`, similar to OpenAI.

3. Tests:
   ```rust
   #[test]
   fn anthropic_translates_multi_turn_history() {
       let msgs = vec![
           Message::user("hello"),
           Message::assistant("hi back"),
           Message::user("how are you"),
       ];
       let translated = anthropic::translate_messages(&msgs);
       assert_eq!(translated.len(), 3);
       assert_eq!(translated[0]["role"], "user");
       assert_eq!(translated[1]["role"], "assistant");
   }

   #[test]
   fn anthropic_translates_tool_call() {
       let msgs = vec![
           Message::user("call the calc"),
           Message::tool_call("calc-1", "calc", json!({"x": 1})),
           Message::tool_result("calc-1", "2"),
       ];
       let translated = anthropic::translate_messages(&msgs);
       assert!(translated[1]["content"][0]["type"] == "tool_use");
       assert!(translated[2]["content"][0]["type"] == "tool_result");
   }
   ```

### Verify

```bash
cargo test -p roko-agent translate --lib
# Per provider:
cargo test -p roko-agent providers::anthropic --lib
```

### Do not

- Bundle multiple providers per commit.
- Change the `Message` / `ContentBlock` types in `roko-core` during
  these PRs.
- Drop the prompt-rendering fallback (the flat-string path) in this PR.
  It stays as the fallback when a provider doesn't support structured
  history — which should be no provider, but keep it as defense in depth.

**Estimated effort**: 2-4 hours per adapter (10-20 hours total).

---

## Combined Verification

After all of T5-35..T5-42:

```bash
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Architectural extraction
wc -l crates/roko-cli/src/orchestrate.rs              # << 22,756 (target: ~21,000)
ls crates/roko-cli/src/orchestrate/dispatch/          # 4 new modules

# Dispatch consolidation
rg 'reqwest::Client::new' crates/roko-serve/src/routes/ | wc -l   # decreased
rg 'dispatch_direct' crates/roko-cli/src/ | rg -v 'cfg\(feature = "legacy-direct-dispatch"\)' \
  | rg -v 'crates/roko-cli/src/dispatch_direct\.rs'    # 0 matches
ls crates/roko-cli/src/dispatch_direct.rs             # still exists, but feature-gated

# Config validation
rg 'ValidatedConfig' crates/roko-core/src/config/    # type exists in API
rg 'load_config\(\)' crates/ -g '*.rs' | wc -l        # all callers receive ValidatedConfig

# Budget guardrail
rg 'RunnerBudgetGuardrail' crates/roko-cli/src/orchestrate.rs   # >1 occurrence

# Run ledger
rg 'RunLedger::derive_report' crates/                # consumed by report builders

# Demo
rg 'CommandEvent' demo/demo-app/src/lib/scenario-runners/   # subscribes to events
rg '\.match\(/\$' demo/demo-app/src/lib/scenario-runners/   # 0 matches

# Provider-native history
rg 'translate_messages' crates/roko-agent/src/providers/   # per-provider helper
```

---

## Status

- [ ] T5-35 — Extract `dispatch_agent_with` into composable units (4 sub-commits)
- [ ] T5-36 — Migrate remaining serve dispatch to `ModelCallService`
- [ ] T5-37 — Remove or quarantine `dispatch_direct`
- [ ] T5-38 — Collapse config into validated model
- [ ] T5-39 — Add budget guardrail to Ollama dispatch path
- [ ] T5-40 — Replace event-replay reports with `RunLedger` (4 sub-commits)
- [ ] T5-41 — Migrate demo automation off prompt scraping
- [ ] T5-42 — Provider-native structured history for all adapters

**After completion**: orchestrate.rs is ~16-18K lines (the four extracted
units pulled out + the runtime ledger migrations also stripped lines), no
serve route does raw provider HTTP, dispatch_direct is feature-gated,
config flows through `ValidatedConfig`, the runtime ledger is the single
source of report truth, and demo automation uses typed events.

The codebase is then ready for forward-looking work in plans 40-42.
