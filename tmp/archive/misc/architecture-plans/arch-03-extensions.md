# Architecture Plan: Extensions

**Source:** `tmp/architecture/03-extensions.md`
**Generated:** 2026-04-25
**Source hash:** `37d0d7b0189826d9b8522dcc27e9feb978615a94c99c8ba0ee45e01d9d6a22fa`
**Section tasks:** 16
**Context mode:** full source section embedded in every task; no excerpt truncation.
**Quality threshold:** every task must score at least 9.5/10 before implementation begins.

## Purpose
Turn every source section into an executable, self-contained implementation task. A Codex agent should not need prior conversation context or a separate reading pass to understand the requirement, although it must still inspect current code before editing.

## Global Implementation Rules
- Extend existing modules before creating new ones; only add new route/service files when no canonical owner exists.
- Implement production wiring, not only structs, mocks, or isolated helpers.
- Preserve every extracted detail unless a parity-ledger row explicitly marks it covered or deferred.
- Add persistence, events, auth/safety, dashboard projections, and docs updates whenever the requirement reaches those surfaces.
- A checked box means code, tests, docs, parity ledger, and strict gates are done for that task.

## Primary Target Areas
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`

## Source Section Map

| Task | Source Line | Heading | Status | Score |
|------|-------------|---------|--------|-------|
| ARCH-03-S001 | 1 | Extensions | [ ] | 9.8 |
| ARCH-03-S002 | 83 | Extension loading and discovery | [ ] | 9.8 |
| ARCH-03-S003 | 138 | Extension hook execution order | [ ] | 9.8 |
| ARCH-03-S004 | 164 | Extension dependency resolution | [ ] | 9.8 |
| ARCH-03-S005 | 189 | Connectors (universal primitive) | [ ] | 9.8 |
| ARCH-03-S006 | 195 | Why Connector is distinct from Extension | [ ] | 9.8 |
| ARCH-03-S007 | 199 | Connector trait shape | [ ] | 9.8 |
| ARCH-03-S008 | 237 | Existing code that maps to Connector | [ ] | 9.8 |
| ARCH-03-S009 | 246 | Dashboard authoring surface | [ ] | 9.8 |
| ARCH-03-S010 | 255 | Relationship to Extensions and Feeds | [ ] | 9.8 |
| ARCH-03-S011 | 265 | Spec clarifications (added 2026-04-25) | [ ] | 9.8 |
| ARCH-03-S012 | 269 | Decision enum variants | [ ] | 9.8 |
| ARCH-03-S013 | 323 | Hook timeout | [ ] | 9.8 |
| ARCH-03-S014 | 334 | AgentContext (passed to extension hooks) | [ ] | 9.8 |
| ARCH-03-S015 | 352 | Connector discovery | [ ] | 9.8 |
| ARCH-03-S016 | 361 | Acceptance criteria | [ ] | 9.8 |

## Tasks

### ARCH-03-S001 -- Extensions

**Source section:** `tmp/architecture/03-extensions.md:1` through `82`
**Heading level:** H1
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
# Extensions

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from the agent runtime section of the v2 redesign doc.
> Cross-reference: [Agent Runtime](02-agent-runtime.md) for the pipeline that invokes these hooks.

---

Agents are specialized by their extension chain, not code forks. Extensions implement hooks across eight layers:

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Which layer this extension operates in.
    fn layer(&self) -> ExtensionLayer;

    // --- Foundation layer ---
    async fn on_init(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }

    // --- Perception layer ---
    async fn on_observe(&self, _obs: &mut Observations) -> Result<()> { Ok(()) }
    async fn filter_input(&self, _input: &mut AgentMessage) -> Result<FilterDecision> {
        Ok(FilterDecision::Pass)
    }

    // --- Memory layer ---
    async fn on_retrieve(&self, _query: &str, _results: &mut Vec<MemoryItem>) -> Result<()> {
        Ok(())
    }
    async fn on_store(&self, _item: &MemoryItem) -> Result<()> { Ok(()) }

    // --- Cognition layer ---
    async fn pre_inference(&self, _req: &mut InferenceRequest) -> Result<()> { Ok(()) }
    async fn post_inference(&self, _resp: &mut InferenceResponse) -> Result<()> { Ok(()) }
    async fn on_gate(&self, _decision: &mut GateDecision) -> Result<()> { Ok(()) }

    // --- Action layer ---
    async fn pre_action(&self, _action: &mut Action) -> Result<ActionDecision> {
        Ok(ActionDecision::Proceed)
    }
    async fn post_action(&self, _action: &Action, _result: &ActionResult) -> Result<()> {
        Ok(())
    }
    async fn on_tool_call(&self, _call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }

    // --- Social layer ---
    async fn on_message_send(&self, _msg: &mut AgentMessage) -> Result<()> { Ok(()) }
    async fn on_message_receive(&self, _msg: &AgentMessage) -> Result<()> { Ok(()) }

    // --- Meta layer ---
    async fn on_reflect(&self, _state: &CorticalState) -> Result<Vec<Adjustment>> {
        Ok(vec![])
    }
    async fn on_cost_update(&self, _usage: &Usage) -> Result<()> { Ok(()) }

    // --- Recovery layer ---
    async fn on_error(&self, _error: &AgentError) -> Result<RecoveryAction> {
        Ok(RecoveryAction::Propagate)
    }
    async fn on_budget_exceeded(&self, _usage: &Usage) -> Result<BudgetAction> {
        Ok(BudgetAction::Sleepwalk)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExtensionLayer {
    Foundation,
    Perception,
    Memory,
    Cognition,
    Action,
    Social,
    Meta,
    Recovery,
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `286`
- Section hash: `68467a528535528d2aa05bba12cca8e271fa4069e59cf1c3fd743b8475dbb412`

**Normative requirements and implementation claims:**
- ---
- Agents are specialized by their extension chain, not code forks. Extensions implement hooks across eight layers:
- // --- Action layer --- async fn pre_action(&self, _action: &mut Action) -> Result<ActionDecision> { Ok(ActionDecision::Proceed) } async fn post_action(&self, _action: &Action, _result: &ActionResult) -> Result<()> { Ok(()) } async fn on_tool_call(&self, _call: &mut ToolCall) -> Result<ToolDecision> { Ok(ToolDecision::Allow) }

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Extension
- name
- layer
- on_init
- on_shutdown
- on_observe
- filter_input
- on_retrieve
- on_store
- pre_inference
- post_inference
- on_gate
- pre_action
- post_action
- on_tool_call
- on_message_send
- on_message_receive
- on_reflect
- on_cost_update
- on_error
- on_budget_exceeded
- ExtensionLayer

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `#[async_trait]`

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Which layer this extension operates in.
    fn layer(&self) -> ExtensionLayer;

    // --- Foundation layer ---
    async fn on_init(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }

    // --- Perception layer ---
    async fn on_observe(&self, _obs: &mut Observations) -> Result<()> { Ok(()) }
    async fn filter_input(&self, _input: &mut AgentMessage) -> Result<FilterDecision> {
        Ok(FilterDecision::Pass)
    }

    // --- Memory layer ---
    async fn on_retrieve(&self, _query: &str, _results: &mut Vec<MemoryItem>) -> Result<()> {
        Ok(())
    }
    async fn on_store(&self, _item: &MemoryItem) -> Result<()> { Ok(()) }

    // --- Cognition layer ---
    async fn pre_inference(&self, _req: &mut InferenceRequest) -> Result<()> { Ok(()) }
    async fn post_inference(&self, _resp: &mut InferenceResponse) -> Result<()> { Ok(()) }
    async fn on_gate(&self, _decision: &mut GateDecision) -> Result<()> { Ok(()) }

    // --- Action layer ---
    async fn pre_action(&self, _action: &mut Action) -> Result<ActionDecision> {
        Ok(ActionDecision::Proceed)
    }
    async fn post_action(&self, _action: &Action, _result: &ActionResult) -> Result<()> {
        Ok(())
    }
    async fn on_tool_call(&self, _call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }

    // --- Social layer ---
    async fn on_message_send(&self, _msg
...
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Result|self|Sync|async|layer|Extension|Action|decision" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Result|self|Sync|async|layer|Extension|Action|decision" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `Extension` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `layer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_init` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_shutdown` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_observe` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `filter_input` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_retrieve` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_store` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `pre_inference` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `post_inference` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_gate` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `pre_action` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `post_action` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_tool_call` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_message_send` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_message_receive` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_reflect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_cost_update` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_error` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `on_budget_exceeded` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ExtensionLayer` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S002 -- Extension loading and discovery

**Source section:** `tmp/architecture/03-extensions.md:83` through `137`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extension loading and discovery

Extensions are loaded from three sources, checked in order:

```
Source          Location                                    Format
──────          ────────                                    ──────
Built-in        Compiled into the roko binary               Rust code (static dispatch)
Local           .roko/extensions/{name}/                    Compiled .so (Linux), .dylib (macOS)
Registry        Fetched from relay extension registry       Downloaded to .roko/extensions/ on first use,
                on first use, cached locally                then loaded as local
```

**Load order**: Built-in extensions load first (always available). Then local extensions from disk. Registry extensions are fetched only if referenced in config but not found locally.

**Error handling**:

```toml
# roko.toml
[[agents]]
name = "coder-1"
extensions = [
  { name = "git",           optional = false },  # default: abort on load failure
  { name = "custom-linter", optional = true },    # skip with warning on load failure
]
```

- `optional = false` (the default): if the extension fails to load, agent startup aborts with an error. This is the default for profile defaults (e.g., `git` in the `coding` profile) because the agent cannot function without core extensions.
- `optional = true`: if the extension fails to load, log a warning and continue startup without it. The agent operates with a reduced extension chain.

**Registry fetch flow**:

```
Config references "vuln-scanner"
         │
         ▼
Check .roko/extensions/vuln-scanner/
         │
    found ──► Load .so/.dylib
         │
    not found
         │
         ▼
GET {relay_url}/registry/extensions/vuln-scanner
         │
         ▼
Download to .roko/extensions/vuln-scanner/
         │
         ▼
Verify SHA-256 checksum from registry manifest
         │
         ▼
Load .so/.dylib
```
````

**Explicit detail extraction from this section:**

- Section word count: `213`
- Section hash: `cd102a0c3861427d829d451693068c55971f97d8cf7c65e91f838c31f42f537c`

**Normative requirements and implementation claims:**
- **Load order**: Built-in extensions load first (always available). Then local extensions from disk. Registry extensions are fetched only if referenced in config but not found locally.
- **Error handling**:
- - `optional = false` (the default): if the extension fails to load, agent startup aborts with an error. This is the default for profile defaults (e.g., `git` in the `coding` profile) because the agent cannot function without core extensions. - `optional = true`: if the extension fails to load, log a warning and continue startup without it. The agent operates with a reduced extension chain.
- **Registry fetch flow**:

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/extensions/
- .roko/extensions/vuln-scanner/
- registry/extensions/

**Types, functions, traits, and inline code identifiers:**
- git
- coding

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- name = "coder-1"
- extensions = [

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `optional = false` (the default): if the extension fails to load, agent startup aborts with an error. This is the default for profile defaults (e.g., `git` in the `coding` profile) because the agent cannot function without core extensions.
- - `optional = true`: if the extension fails to load, log a warning and continue startup without it. The agent operates with a reduced extension chain.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Source          Location                                    Format`

```
Source          Location                                    Format
──────          ────────                                    ──────
Built-in        Compiled into the roko binary               Rust code (static dispatch)
Local           .roko/extensions/{name}/                    Compiled .so (Linux), .dylib (macOS)
Registry        Fetched from relay extension registry       Downloaded to .roko/extensions/ on first use,
                on first use, cached locally                then loaded as local
```
- Contract 2: language `toml`, first line `# roko.toml`

```toml
# roko.toml
[[agents]]
name = "coder-1"
extensions = [
  { name = "git",           optional = false },  # default: abort on load failure
  { name = "custom-linter", optional = true },    # skip with warning on load failure
]
```
- Contract 3: language `plain`, first line `Config references "vuln-scanner"`

```
Config references "vuln-scanner"
         │
         ▼
Check .roko/extensions/vuln-scanner/
         │
    found ──► Load .so/.dylib
         │
    not found
         │
         ▼
GET {relay_url}/registry/extensions/vuln-scanner
         │
         ▼
Download to .roko/extensions/vuln-scanner/
         │
         ▼
Verify SHA-256 checksum from registry manifest
         │
         ▼
Load .so/.dylib
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `.roko/extensions/`
- `.roko/extensions/vuln-scanner/`
- `registry/extensions/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|Load|extensions|git|Registry|loading|discovery" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|Load|extensions|git|Registry|loading|discovery" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `.roko/extensions/`
- `.roko/extensions/vuln-scanner/`
- `registry/extensions/`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `git` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `coding` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `name = "coder-1"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `extensions = [` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S003 -- Extension hook execution order

**Source section:** `tmp/architecture/03-extensions.md:138` through `163`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extension hook execution order

Per tick, extensions fire in layer order: L0 (Foundation) through L7 (Recovery). Within a layer, extensions fire in config order -- the order they appear in the `extensions = [...]` array in `roko.toml`.

```
Tick execution:

  L0 Foundation:   [git.on_init, compiler.on_init]         ← config order
  L1 Perception:   [git.on_observe, web-search.on_observe]
  L2 Memory:       [neuro-store.on_retrieve]
  L3 Cognition:    [safety.pre_inference, compiler.post_inference]
  L4 Action:       [git.pre_action, test-runner.post_action]
  L5 Social:       [slack.on_message_send]
  L6 Meta:         [cost-tracker.on_cost_update]
  L7 Recovery:     [circuit-breaker.on_error]
```

**Fault isolation**: If one extension's hook returns `Err`, the runtime logs the error with the extension name and hook name, then continues to the next extension in the chain. The agent does not abort on a single extension error. This prevents a buggy optional extension from taking down the entire agent.

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

The exception is `pre_action` hooks that return `ActionDecision::Block` -- these are not errors but intentional vetoes (e.g., safety extensions blocking dangerous tool calls). Blocks halt the action, not the agent.
````

**Explicit detail extraction from this section:**

- Section word count: `177`
- Section hash: `ab8964b7f98c5408c11fdbf20386f5ad2f7d503f3fe690473f192cffc6a89ffc`

**Normative requirements and implementation claims:**
- L0 Foundation: [git.on_init, compiler.on_init] ← config order L1 Perception: [git.on_observe, web-search.on_observe] L2 Memory: [neuro-store.on_retrieve] L3 Cognition: [safety.pre_inference, compiler.post_inference] L4 Action: [git.pre_action, test-runner.post_action] L5 Social: [slack.on_message_send] L6 Meta: [cost-tracker.on_cost_update] L7 Recovery: [circuit-breaker.on_error] ```
- **Fault isolation**: If one extension's hook returns `Err`, the runtime logs the error with the extension name and hook name, then continues to the next extension in the chain. The agent does not abort on a single extension error. This prevents a buggy optional extension from taking down the entire agent.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Err
- pre_action
- ActionDecision::Block

**Event names and event-like entities:**
- git.on_init
- compiler.on_init
- git.on_observe
- search.on_observe
- store.on_retrieve
- safety.pre_inference
- compiler.post_inference
- git.pre_action
- runner.post_action
- slack.on_message_send
- tracker.on_cost_update
- breaker.on_error

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- roko.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `Tick execution:`

```
Tick execution:

  L0 Foundation:   [git.on_init, compiler.on_init]         ← config order
  L1 Perception:   [git.on_observe, web-search.on_observe]
  L2 Memory:       [neuro-store.on_retrieve]
  L3 Cognition:    [safety.pre_inference, compiler.post_inference]
  L4 Action:       [git.pre_action, test-runner.post_action]
  L5 Social:       [slack.on_message_send]
  L6 Meta:         [cost-tracker.on_cost_update]
  L7 Recovery:     [circuit-breaker.on_error]
```
- Contract 2: language `plain`, first line `[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s`

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|order|hook|Err|Action|pre_action|execution|extensions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|order|hook|Err|Action|pre_action|execution|extensions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `Err` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `pre_action` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ActionDecision::Block` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `git.on_init` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `compiler.on_init` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `git.on_observe` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `search.on_observe` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `store.on_retrieve` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `safety.pre_inference` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `compiler.post_inference` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `git.pre_action` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `runner.post_action` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `slack.on_message_send` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `tracker.on_cost_update` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Emit or consume `breaker.on_error` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S004 -- Extension dependency resolution

**Source section:** `tmp/architecture/03-extensions.md:164` through `188`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Extension dependency resolution

Extensions can declare dependencies on other extensions:

```toml
# .roko/extensions/report-writer/manifest.toml
[extension]
name = "report-writer"
layer = "action"
depends_on = ["citation", "summarizer"]
```

On load, the runtime performs a topological sort of extensions within each layer to resolve dependencies. If `report-writer` depends on `citation`, then `citation` hooks always fire before `report-writer` hooks within the same layer.

**Cyclic dependency** (e.g., A depends on B, B depends on A) is a startup error:

```
Error: Cyclic extension dependency detected: report-writer -> citation -> report-writer
       Remove the cycle or merge the extensions.
```

**Cross-layer dependencies** are not supported. Extensions in different layers already have a fixed execution order (L0 before L1 before L2, etc.). A Memory-layer extension that needs Foundation-layer setup gets it automatically through layer ordering.

---
````

**Explicit detail extraction from this section:**

- Section word count: `128`
- Section hash: `b4f0163a1ea0b83e810e5ce8690f1b962fb11b9462824866a338cfbf885c925c`

**Normative requirements and implementation claims:**
- On load, the runtime performs a topological sort of extensions within each layer to resolve dependencies. If `report-writer` depends on `citation`, then `citation` hooks always fire before `report-writer` hooks within the same layer.
- **Cyclic dependency** (e.g., A depends on B, B depends on A) is a startup error:
- **Cross-layer dependencies** are not supported. Extensions in different layers already have a fixed execution order (L0 before L1 before L2, etc.). A Memory-layer extension that needs Foundation-layer setup gets it automatically through layer ordering.
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- .roko/extensions/report-writer/manifest.toml

**Types, functions, traits, and inline code identifiers:**
- citation

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- report-writer -> citation -

**Config keys and TOML-like settings:**
- [extension]
- name = "report-writer"
- layer = "action"
- depends_on = ["citation", "summarizer"]

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `toml`, first line `# .roko/extensions/report-writer/manifest.toml`

```toml
# .roko/extensions/report-writer/manifest.toml
[extension]
name = "report-writer"
layer = "action"
depends_on = ["citation", "summarizer"]
```
- Contract 2: language `plain`, first line `Error: Cyclic extension dependency detected: report-writer -> citation -> report-writer`

```
Error: Cyclic extension dependency detected: report-writer -> citation -> report-writer
       Remove the cycle or merge the extensions.
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `.roko/extensions/report-writer/manifest.toml`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Extension|layer|citation|dependency|writer|report|extensions|resolution" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Extension|layer|citation|dependency|writer|report|extensions|resolution" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `.roko/extensions/report-writer/manifest.toml`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `citation` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `report-writer -> citation -` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Add or verify config key `[extension]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `name = "report-writer"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `layer = "action"` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] Add or verify config key `depends_on = ["citation", "summarizer"]` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S005 -- Connectors (universal primitive)

**Source section:** `tmp/architecture/03-extensions.md:189` through `194`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Connectors (universal primitive)

> Added 2026-04-24. Per dashboard PRD 23, Connector is a first-class primitive in the 12-primitive vocabulary.

A Connector wraps external system I/O behind a universal trait: `connect / query / execute / health / disconnect`. This generalizes what was previously hardcoded as `ChainClient`, `VenueAdapter`, MCP server configs, and database connections into a single composable abstraction.
````

**Explicit detail extraction from this section:**

- Section word count: `55`
- Section hash: `d880cbc713112c34fbccf947e4865cc55ab7023a1e03c4e9c5ddbd1114562654`

**Normative requirements and implementation claims:**
- > Added 2026-04-24. Per dashboard PRD 23, Connector is a first-class primitive in the 12-primitive vocabulary.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ChainClient
- VenueAdapter

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "connect|primitive|Connector|universal|VenueAdapter|Connectors|ChainClient|wraps" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "connect|primitive|Connector|universal|VenueAdapter|Connectors|ChainClient|wraps" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `ChainClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VenueAdapter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S006 -- Why Connector is distinct from Extension

**Source section:** `tmp/architecture/03-extensions.md:195` through `198`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Why Connector is distinct from Extension

Extensions modify agent behavior through hooks -- they intercept, filter, and transform. Connectors provide bidirectional I/O with external systems. A Connector does not modify agent behavior; it provides a capability. The distinction matters for composition: an agent *loads* extensions but *uses* connectors.
````

**Explicit detail extraction from this section:**

- Section word count: `42`
- Section hash: `077e9437b0667f8aad59c5a2631b61562b2391eb4fbf761eac738d2188855e2f`

**Normative requirements and implementation claims:**
- Extensions modify agent behavior through hooks -- they intercept, filter, and transform. Connectors provide bidirectional I/O with external systems. A Connector does not modify agent behavior; it provides a capability. The distinction matters for composition: an agent *loads* extensions but *uses* connectors.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Connector|Extension|distinct|Why|modify|extensions|behavior|Connectors" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Connector|Extension|distinct|Why|modify|extensions|behavior|Connectors" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S007 -- Connector trait shape

**Source section:** `tmp/architecture/03-extensions.md:199` through `236`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Connector trait shape

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    /// Human-readable name (e.g., "hyperliquid", "github-mcp", "postgres").
    fn name(&self) -> &str;

    /// Connector kind for registry classification.
    fn kind(&self) -> ConnectorKind;

    /// Establish connection. Called once at agent startup.
    async fn connect(&mut self, config: &ConnectorConfig) -> Result<()>;

    /// One-shot query against the external system.
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;

    /// Execute a mutating operation (order placement, tx submission, write).
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;

    /// Health check. Called periodically by the conductor's health watcher.
    async fn health(&self) -> Result<HealthStatus>;

    /// Graceful disconnect. Called on agent shutdown.
    async fn disconnect(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectorKind {
    ChainRpc,       // Ethereum, Solana, etc.
    Exchange,       // Hyperliquid, Binance, etc.
    McpServer,      // MCP tool servers
    Database,       // Postgres, SQLite, etc.
    Webhook,        // External HTTP endpoints
    Api,            // Generic REST/gRPC APIs
}
```
````

**Explicit detail extraction from this section:**

- Section word count: `136`
- Section hash: `c6d82b6426702be7916626023a8e34eae322906d81e6ae309e38f75edb017acb`

**Normative requirements and implementation claims:**
- #[derive(Debug, Clone, Copy)] pub enum ConnectorKind { ChainRpc, // Ethereum, Solana, etc. Exchange, // Hyperliquid, Binance, etc. McpServer, // MCP tool servers Database, // Postgres, SQLite, etc. Webhook, // External HTTP endpoints Api, // Generic REST/gRPC APIs } ```

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- Connector
- name
- kind
- connect
- query
- execute
- health
- disconnect
- ConnectorKind

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `#[async_trait]`

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    /// Human-readable name (e.g., "hyperliquid", "github-mcp", "postgres").
    fn name(&self) -> &str;

    /// Connector kind for registry classification.
    fn kind(&self) -> ConnectorKind;

    /// Establish connection. Called once at agent startup.
    async fn connect(&mut self, config: &ConnectorConfig) -> Result<()>;

    /// One-shot query against the external system.
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;

    /// Execute a mutating operation (order placement, tx submission, write).
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;

    /// Health check. Called periodically by the conductor's health watcher.
    async fn health(&self) -> Result<HealthStatus>;

    /// Graceful disconnect. Called on agent shutdown.
    async fn disconnect(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectorKind {
    ChainRpc,       // Ethereum, Solana, etc.
    Exchange,       // Hyperliquid, Binance, etc.
    McpServer,      // MCP tool servers
    Database,       // Postgres, SQLite, etc.
    Webhook,        // External HTTP endpoints
    Api,            // Generic REST/gRPC APIs
}
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "connect|Connector|query|kind|health|execute|trait|self" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "connect|Connector|query|kind|health|execute|trait|self" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `Connector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `name` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `kind` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `connect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `query` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `execute` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `health` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `disconnect` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ConnectorKind` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S008 -- Existing code that maps to Connector

**Source section:** `tmp/architecture/03-extensions.md:237` through `245`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Existing code that maps to Connector

| Existing construct | Crate | Becomes |
|--------------------|-------|---------|
| `ChainClient` / `AlloyChainClient` | `roko-chain` | `ChainRpcConnector` |
| `VenueAdapter` | `roko-chain` (gap doc 02) | `ExchangeConnector` |
| MCP server config in `roko.toml` | `roko-agent` | `McpConnector` (auto-registered from config) |
| Oracle endpoints | `roko-chain` (gap doc 01) | `ApiConnector` |
````

**Explicit detail extraction from this section:**

- Section word count: `37`
- Section hash: `c1f6522cf693964c9e25a44718841b2b96f7900927f74fcef58f60e6da9f6210`

**Normative requirements and implementation claims:**
- | Existing construct | Crate | Becomes | |--------------------|-------|---------| | `ChainClient` / `AlloyChainClient` | `roko-chain` | `ChainRpcConnector` | | `VenueAdapter` | `roko-chain` (gap doc 02) | `ExchangeConnector` | | MCP server config in `roko.toml` | `roko-agent` | `McpConnector` (auto-registered from config) | | Oracle endpoints | `roko-chain` (gap doc 01) | `ApiConnector` |

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- ChainClient
- AlloyChainClient
- ChainRpcConnector
- VenueAdapter
- ExchangeConnector
- McpConnector
- ApiConnector

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- roko.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- Table 1:

```markdown
| Existing construct | Crate | Becomes |
|--------------------|-------|---------|
| `ChainClient` / `AlloyChainClient` | `roko-chain` | `ChainRpcConnector` |
| `VenueAdapter` | `roko-chain` (gap doc 02) | `ExchangeConnector` |
| MCP server config in `roko.toml` | `roko-agent` | `McpConnector` (auto-registered from config) |
| Oracle endpoints | `roko-chain` (gap doc 01) | `ApiConnector` |
```

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Connector|chain|Existing|ChainClient|maps|code|VenueAdapter|McpConnector" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Connector|chain|Existing|ChainClient|maps|code|VenueAdapter|McpConnector" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `ChainClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `AlloyChainClient` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ChainRpcConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `VenueAdapter` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ExchangeConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `McpConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ApiConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `roko.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S009 -- Dashboard authoring surface

**Source section:** `tmp/architecture/03-extensions.md:246` through `254`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `auth`, `chain`, `agent-runtime`, `dashboard-support`, `verification`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Dashboard authoring surface

The Connector Manager is a 4-stage authoring surface (per PRD 23):

1. **Type selection** -- pick connector type from a template gallery
2. **Configuration** -- connection string, auth, rate limits, retry policy (live health check)
3. **Tool registration** -- auto-discover operations; select which to expose as agent tools
4. **Test and deploy** -- execute test query, verify health endpoint, show latency and error rate
````

**Explicit detail extraction from this section:**

- Section word count: `62`
- Section hash: `f80ca832677966a3c997e9023c867e7a0c9aa85a050ee26d285e2ed0e31eba1f`

**Normative requirements and implementation claims:**
- 1. **Type selection** -- pick connector type from a template gallery 2. **Configuration** -- connection string, auth, rate limits, retry policy (live health check) 3. **Tool registration** -- auto-discover operations; select which to expose as agent tools 4. **Test and deploy** -- execute test query, verify health endpoint, show latency and error rate

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- from

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Type selection** -- pick connector type from a template gallery
- 2. **Configuration** -- connection string, auth, rate limits, retry policy (live health check)
- 3. **Tool registration** -- auto-discover operations; select which to expose as agent tools
- 4. **Test and deploy** -- execute test query, verify health endpoint, show latency and error rate

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "auth|surface|authoring|select|rate|health|Type|Tool" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "auth|surface|authoring|select|rate|health|Type|Tool" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-serve/src/state.rs`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Security: enforce scope/capability checks before side effects, cover unauthenticated/read/write/admin/agent-token/JWT cases, and redact secrets.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `from` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S010 -- Relationship to Extensions and Feeds

**Source section:** `tmp/architecture/03-extensions.md:255` through `264`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Relationship to Extensions and Feeds

Connectors sit between Extensions and Feeds in the composition hierarchy:

- An **Extension** can *wrap* a Connector to add behavior (e.g., rate limiting, retry logic)
- A **Feed** is *sourced from* a Connector (e.g., a price feed subscribes to an exchange connector)
- An **Agent** *uses* Connectors for I/O but *loads* Extensions for behavior modification

---
````

**Explicit detail extraction from this section:**

- Section word count: `55`
- Section hash: `0a7858b24b1bd5aa7988a91896e13fe0e594d0f1889b45cdf72c085ceea36baf`

**Normative requirements and implementation claims:**
- - An **Extension** can *wrap* a Connector to add behavior (e.g., rate limiting, retry logic) - A **Feed** is *sourced from* a Connector (e.g., a price feed subscribes to an exchange connector) - An **Agent** *uses* Connectors for I/O but *loads* Extensions for behavior modification
- ---

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - An **Extension** can *wrap* a Connector to add behavior (e.g., rate limiting, retry logic)
- - A **Feed** is *sourced from* a Connector (e.g., a price feed subscribes to an exchange connector)
- - An **Agent** *uses* Connectors for I/O but *loads* Extensions for behavior modification

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Feed|Extension|Extensions|Feeds|Relationship|Connector|behavior" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Feed|Extension|Extensions|Feeds|Relationship|Connector|behavior" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Convert every normative sentence and bullet in the section into ledger-backed implementation tasks; if no backend change is required, write an explicit `covered_by` or `deferred` row with rationale.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S011 -- Spec clarifications (added 2026-04-25)

**Source section:** `tmp/architecture/03-extensions.md:265` through `268`
**Heading level:** H2
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
## Spec clarifications (added 2026-04-25)

> Backported from `tmp/architecture-plans/06-architecture-implementation.md` Phase A.3.
````

**Explicit detail extraction from this section:**

- Section word count: `12`
- Section hash: `e6dea17e6ca826becd311d2d74c0c85cb378b2136c4d9ac1b72b4afc756964d1`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- tmp/architecture-plans/06-architecture-implementation.md

**Types, functions, traits, and inline code identifiers:**
- None extracted from this section.

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `tmp/architecture-plans/06-architecture-implementation.md`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "clarifications|added|Spec|plans|Phase|Backported|extensions" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "clarifications|added|Spec|plans|Phase|Backported|extensions" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `tmp/architecture-plans/06-architecture-implementation.md`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S012 -- Decision enum variants

**Source section:** `tmp/architecture/03-extensions.md:269` through `322`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Decision enum variants

The Extension trait uses several decision types that were not fully specified. Complete definitions:

```rust
/// Perception layer: filter_input() return value
pub enum FilterDecision {
    Pass,                          // Message passes through unchanged
    Drop,                          // Message silently discarded
    Transform(AgentMessage),       // Replace message with transformed version
}

/// Action layer: pre_action() return value
pub enum ActionDecision {
    Proceed,                       // Action executes normally
    Block { reason: String },      // Action halted (not an error — intentional veto)
    Modify(Action),                // Execute modified action instead
}

/// Action layer: on_tool_call() return value
pub enum ToolDecision {
    Allow,                         // Tool call proceeds
    Block { reason: String },      // Tool call blocked (logged, agent notified)
    Substitute(ToolCall),          // Replace with different tool call
}

/// Recovery layer: on_error() return value
pub enum RecoveryAction {
    Propagate,                     // Error propagates up (default)
    Retry,                         // Retry the failed operation
    Ignore,                        // Suppress the error
    Escalate(String),              // Escalate with custom message
}

/// Recovery layer: on_budget_exceeded() return value
pub enum BudgetAction {
    Sleepwalk,                     // Enter sleepwalk mode (observe + reflect only)
    Stop,                          // Shut down the agent
    RequestMore(u64),              // Request additional budget (microdollars)
}

/// Meta layer: on_reflect() return value
pub enum Adjustment {
    SetGoal(Goal),                 // Replace or add a goal
    UpdateBelief(String, f64),     // Update belief key-value pair
    ShiftAttention(String),        // Change attention focus
}
```

**Behavioral consequences**:
- `FilterDecision::Drop` → message never reaches the agent's pipeline. Logged for debugging.
- `ActionDecision::Block` → action halted, agent continues (not crashed). Agent receives "action blocked by {extension_name}: {reason}" in its next turn.
- `ToolDecision::Substitute` → original tool call replaced transparently. The agent sees the substitute's result.
````

**Explicit detail extraction from this section:**

- Section word count: `224`
- Section hash: `d2921ff36e36b45ede6c2a750953d33fdc32b0b8feb2bfb160bf31dc3791b19d`

**Normative requirements and implementation claims:**
- /// Action layer: on_tool_call() return value pub enum ToolDecision { Allow, // Tool call proceeds Block { reason: String }, // Tool call blocked (logged, agent notified) Substitute(ToolCall), // Replace with different tool call }
- /// Recovery layer: on_budget_exceeded() return value pub enum BudgetAction { Sleepwalk, // Enter sleepwalk mode (observe + reflect only) Stop, // Shut down the agent RequestMore(u64), // Request additional budget (microdollars) }
- /// Meta layer: on_reflect() return value pub enum Adjustment { SetGoal(Goal), // Replace or add a goal UpdateBelief(String, f64), // Update belief key-value pair ShiftAttention(String), // Change attention focus } ```
- **Behavioral consequences**: - `FilterDecision::Drop` → message never reaches the agent's pipeline. Logged for debugging. - `ActionDecision::Block` → action halted, agent continues (not crashed). Agent receives "action blocked by {extension_name}: {reason}" in its next turn. - `ToolDecision::Substitute` → original tool call replaced transparently. The agent sees the substitute's result.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- uses
- FilterDecision
- ActionDecision
- ToolDecision
- RecoveryAction
- BudgetAction
- Adjustment
- FilterDecision::Drop
- ActionDecision::Block
- ToolDecision::Substitute

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - `FilterDecision::Drop` → message never reaches the agent's pipeline. Logged for debugging.
- - `ActionDecision::Block` → action halted, agent continues (not crashed). Agent receives "action blocked by {extension_name}: {reason}" in its next turn.
- - `ToolDecision::Substitute` → original tool call replaced transparently. The agent sees the substitute's result.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `/// Perception layer: filter_input() return value`

```rust
/// Perception layer: filter_input() return value
pub enum FilterDecision {
    Pass,                          // Message passes through unchanged
    Drop,                          // Message silently discarded
    Transform(AgentMessage),       // Replace message with transformed version
}

/// Action layer: pre_action() return value
pub enum ActionDecision {
    Proceed,                       // Action executes normally
    Block { reason: String },      // Action halted (not an error — intentional veto)
    Modify(Action),                // Execute modified action instead
}

/// Action layer: on_tool_call() return value
pub enum ToolDecision {
    Allow,                         // Tool call proceeds
    Block { reason: String },      // Tool call blocked (logged, agent notified)
    Substitute(ToolCall),          // Replace with different tool call
}

/// Recovery layer: on_error() return value
pub enum RecoveryAction {
    Propagate,                     // Error propagates up (default)
    Retry,                         // Retry the failed operation
    Ignore,                        // Suppress the error
    Escalate(String),              // Escalate with custom message
}

/// Recovery layer: on_budget_exceeded() return value
pub enum BudgetAction {
    Sleepwalk,                     // Enter sleepwalk mode (observe + reflect only)
    Stop,                          // Shut down the agent
    RequestMore(u64),              // Request additional budget (microdollars)
}

/// Meta layer: on_reflect() return value
pub enum Adjustment {
    SetGoal(Goal),                 /
...
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Action|Decision|enum|Tool|value|turn|return|layer" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Action|Decision|enum|Tool|value|turn|return|layer" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `uses` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FilterDecision` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ActionDecision` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ToolDecision` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `RecoveryAction` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `BudgetAction` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `Adjustment` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FilterDecision::Drop` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ActionDecision::Block` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ToolDecision::Substitute` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S013 -- Hook timeout

**Source section:** `tmp/architecture/03-extensions.md:323` through `333`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Hook timeout

All extension hooks timeout after **5 seconds**. This is currently hardcoded (not configurable per hook).

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

If timeout behavior becomes a problem, the first enhancement would be a per-extension `timeout_ms` field in `manifest.toml`. But keep it simple until proven needed.
````

**Explicit detail extraction from this section:**

- Section word count: `55`
- Section hash: `aa1bd21cb73fa14f38419e76e03cea7cc6517a82710fe97338a3f72cce1526f7`

**Normative requirements and implementation claims:**
- None extracted from this section.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- timeout_ms

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- manifest.toml

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `plain`, first line `[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s`

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "timeout|Hook|timeout_ms|extension|after|until|toml|simple" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "timeout|Hook|timeout_ms|extension|after|until|toml|simple" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `timeout_ms` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Add or verify config key `manifest.toml` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S014 -- AgentContext (passed to extension hooks)

**Source section:** `tmp/architecture/03-extensions.md:334` through `351`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`, `knowledge-learning`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### AgentContext (passed to extension hooks)

Extensions receive `&AgentContext` for read access to agent state:

```rust
pub struct AgentContext {
    pub agent_id: String,
    pub profile: DomainProfile,
    pub mode: AgentMode,
    pub regime: Regime,               // current adaptive clock regime
    pub budget_remaining: u64,        // microdollars
    pub episode_count: u64,
    pub config: Arc<AgentConfig>,     // full agent config (read-only)
}
```

This is **read-only**. Extensions that need to modify agent behavior do so through their return values (decision enums above), not by mutating context.
````

**Explicit detail extraction from this section:**

- Section word count: `69`
- Section hash: `4b09a68302f3e65e0c83e7531ee1ea225b9de72dc7954282eea980d688e70b86`

**Normative requirements and implementation claims:**
- Extensions receive `&AgentContext` for read access to agent state:
- ```rust pub struct AgentContext { pub agent_id: String, pub profile: DomainProfile, pub mode: AgentMode, pub regime: Regime, // current adaptive clock regime pub budget_remaining: u64, // microdollars pub episode_count: u64, pub config: Arc<AgentConfig>, // full agent config (read-only) } ```
- This is **read-only**. Extensions that need to modify agent behavior do so through their return values (decision enums above), not by mutating context.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- AgentContext

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- None extracted from this section.

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- Contract 1: language `rust`, first line `pub struct AgentContext {`

```rust
pub struct AgentContext {
    pub agent_id: String,
    pub profile: DomainProfile,
    pub mode: AgentMode,
    pub regime: Regime,               // current adaptive clock regime
    pub budget_remaining: u64,        // microdollars
    pub episode_count: u64,
    pub config: Arc<AgentConfig>,     // full agent config (read-only)
}
```

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "AgentContext|context|extension|passed|hooks|regime|read|config" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "AgentContext|context|extension|passed|hooks|regime|read|config" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.
- Knowledge/learning: preserve provenance, confidence, lineage, decay, retrieval, promotion/demotion, episode signals, and queryable outputs.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `AgentContext` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S015 -- Connector discovery

**Source section:** `tmp/architecture/03-extensions.md:352` through `360`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Connector discovery

Connectors are discovered from three sources (matching extension discovery order):
1. **Config**: `[[agents]] connectors = ["postgres", "hyperliquid"]` in roko.toml
2. **MCP auto-register**: Any MCP server in `agent.mcp_config` auto-registers as `McpConnector`
3. **Extension-provided**: An extension can register connectors in its `on_init()` hook

There is no registry-based discovery for connectors (unlike extensions). Connectors are always explicitly declared in agent config or provided by extensions.
````

**Explicit detail extraction from this section:**

- Section word count: `67`
- Section hash: `b7a31d54eb1f2c5f1d162fc5b7b4e2ae0fe0fce328ae95488f96bcf5668b8d38`

**Normative requirements and implementation claims:**
- There is no registry-based discovery for connectors (unlike extensions). Connectors are always explicitly declared in agent config or provided by extensions.

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- McpConnector

**Event names and event-like entities:**
- agent.mcp_config

**State transitions:**
- None extracted from this section.

**Config keys and TOML-like settings:**
- agent.mcp_config

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- 1. **Config**: `[[agents]] connectors = ["postgres", "hyperliquid"]` in roko.toml
- 2. **MCP auto-register**: Any MCP server in `agent.mcp_config` auto-registers as `McpConnector`
- 3. **Extension-provided**: An extension can register connectors in its `on_init()` hook

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "Connector|discovery|extension|McpConnector|Connectors|register|Config|provided" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "Connector|discovery|extension|McpConnector|Connectors|register|Config|provided" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `McpConnector` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Emit or consume `agent.mcp_config` through the production event bus with sequence id, timestamp, actor, workspace, replay behavior, and WS/SSE coverage if user-visible.
- [ ] Add or verify config key `agent.mcp_config` with schema validation, default, docs, env override if applicable, and hot-reload/restart-required behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

### ARCH-03-S016 -- Acceptance criteria

**Source section:** `tmp/architecture/03-extensions.md:361` through `371`
**Heading level:** H3
**Concern split:** `api`, `realtime`, `storage`, `chain`, `agent-runtime`, `dashboard-support`, `verification`, `config-deployment`
**Implementation status:** `[ ] not started` `[ ] in progress` `[ ] implemented` `[ ] verified`

**Full source context for this task:**

````markdown
### Acceptance criteria

- [ ] Extension trait compiles with all 22 hooks and default no-op implementations
- [ ] `FilterDecision::Drop` silently discards message (logged)
- [ ] `ActionDecision::Block` halts action but agent continues
- [ ] `ToolDecision::Substitute` transparently replaces tool call
- [ ] Hook timeout at 5 seconds → warning logged, next extension continues
- [ ] Missing optional extension → warning logged, agent starts normally
- [ ] Missing required extension → agent startup aborts with clear error
- [ ] Cyclic dependency detected → startup error with cycle description
- [ ] Extensions sorted: by layer, then by dependency (topological), then by config order
````

**Explicit detail extraction from this section:**

- Section word count: `78`
- Section hash: `919889f9099ee2b5e7e2f5174211648ed12c89cae0f9ee321f9dcad9140c039e`

**Normative requirements and implementation claims:**
- - [ ] Extension trait compiles with all 22 hooks and default no-op implementations - [ ] `FilterDecision::Drop` silently discards message (logged) - [ ] `ActionDecision::Block` halts action but agent continues - [ ] `ToolDecision::Substitute` transparently replaces tool call - [ ] Hook timeout at 5 seconds → warning logged, next extension continues - [ ] Missing optional extension → warning logged, agent starts normally - [ ] Missing required extension → agent startup aborts with clear error - [ ] Cyclic dependency detected → startup error with cycle description - [ ] Extensions sorted: by layer, then by dependency (topological), then by config order

**Routes and endpoint references:**
- None extracted from this section.

**Files and path references:**
- None extracted from this section.

**Types, functions, traits, and inline code identifiers:**
- compiles
- FilterDecision::Drop
- ActionDecision::Block
- ToolDecision::Substitute

**Event names and event-like entities:**
- None extracted from this section.

**State transitions:**
- Hook timeout at 5 seconds -> warning logged
- Missing optional extension -> warning logged
- Missing required extension -> agent startup aborts with clear error
- Cyclic dependency detected -> startup error with cycle description

**Config keys and TOML-like settings:**
- None extracted from this section.

**Commands and operator actions:**
- None extracted from this section.

**Bullet requirements:**
- - [ ] Extension trait compiles with all 22 hooks and default no-op implementations
- - [ ] `FilterDecision::Drop` silently discards message (logged)
- - [ ] `ActionDecision::Block` halts action but agent continues
- - [ ] `ToolDecision::Substitute` transparently replaces tool call
- - [ ] Hook timeout at 5 seconds → warning logged, next extension continues
- - [ ] Missing optional extension → warning logged, agent starts normally
- - [ ] Missing required extension → agent startup aborts with clear error
- - [ ] Cyclic dependency detected → startup error with cycle description
- - [ ] Extensions sorted: by layer, then by dependency (topological), then by config order

**Tables extracted:**
- None extracted from this section.

**Data/code contracts extracted:**
- None extracted from this section.

**Read before editing:**
- `tmp/architecture/03-extensions.md`
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `tmp/architecture-plans/07-docs-parity-closure.md`
- `tmp/architecture-plans/08-end-to-end-acceptance.md`
- `tmp/architecture-plans/COVERAGE-MATRIX.json`

**Discovery commands:**

```bash
rg -n "criteria|compiles|Extension|Acceptance|logged|warning|tool|startup" crates apps contracts docs tmp/architecture tmp/architecture-plans
rg -n "criteria|compiles|Extension|Acceptance|logged|warning|tool|startup" /Users/will/dev/nunchi/nunchi-dashboard/docs/prd /Users/will/dev/nunchi/nunchi-dashboard/src || true
cargo metadata --format-version 1 --no-deps
```

**Target artifacts to create or modify:**
- `crates/roko-core/src/extension.rs`
- `crates/roko-plugin/src/`
- `crates/roko-runtime/src/`
- `crates/roko-serve/src/routes/extensions.rs`
- `crates/roko-serve/src/routes/`
- `docs/API-REFERENCE.md`
- `crates/roko-serve/src/events.rs`
- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/sse.rs`
- `.roko/`
- `crates/roko-fs/src/`
- `contracts/src/`
- `crates/roko-chain/src/`
- `apps/mirage-rs/src/`
- `crates/roko-gate/src/`
- `crates/roko-serve/tests/`
- `crates/roko-cli/tests/`
- `crates/roko-serve/src/routes/projections.rs`
- `.roko/parity/docs-ledger.json`

**Concern contracts to satisfy:**
- API: define concrete request/response structs, auth scope, error shape, pagination/filtering, idempotency semantics, and generated API-reference/OpenAPI coverage.
- Realtime: emit typed Bus/StateHub events with room, sequence, actor, timestamp, replay/degraded semantics, and dashboard cache-update payload.
- Storage: define deterministic schema, migration/default behavior, corruption handling, restart recovery, cleanup/GC, and fixture data.
- Chain: default to devnet/simulation, prove contract/client/indexer sync, enforce wallet/risk/simulation gates, and record audit/witness data.
- Runtime: wire into the production agent loop with lifecycle, cancellation, health, audit, provider/tool safety, and observable events.
- Frontend support: provide stable projections, mock fixtures, empty/loading/stale/degraded/unauthorized states, and realtime update payloads that match projection schemas.
- Verification: add deterministic unit, integration, route/CLI, auth/safety, persistence/restart, serialization, and failing-path tests; avoid LLM self-grading where ground truth is required.
- Config/deployment: validate defaults, hot reload or restart-required paths, env var diagnostics, service dry-runs, and non-destructive operations.

**Explicit implementation obligations derived from this section:**
- [ ] Implement or verify `compiles` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `FilterDecision::Drop` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ActionDecision::Block` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Implement or verify `ToolDecision::Substitute` in the canonical crate with serde/validation/defaults where it is a data type, and unit tests for boundary behavior.
- [ ] Enforce state transition `Hook timeout at 5 seconds -> warning logged` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Missing optional extension -> warning logged` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Missing required extension -> agent startup aborts with clear error` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] Enforce state transition `Cyclic dependency detected -> startup error with cycle description` with invalid-transition rejection tests and persistence/restart behavior.
- [ ] For this architecture-derived task, prove backend behavior first and add frontend projections only where operators or dashboard PRDs need observability/control.

**General implementation checklist:**
- [ ] Add one parity-ledger row for this exact source section with task id, source hash, owner, targets, tests, and acceptance gate.
- [ ] Extend existing canonical modules before adding new modules; document any new module choice in the ledger.
- [ ] Implement production-path wiring across runtime/service/storage/API/events, not only structs or mocks.
- [ ] Add dashboard-friendly projection and realtime payloads when any UI, operator, or PRD requirement would otherwise need to stitch state client-side.
- [ ] Persist durable state under existing storage conventions and cover restart/replay/recovery behavior.
- [ ] Add tests for happy path, invalid input, invalid state, unauthorized access, dependency unavailable, persistence/restart, serialization, and degraded frontend state where applicable.
- [ ] Update generated API/CLI/docs references or record an explicit no-public-surface rationale.
- [ ] Run verification commands and attach produced gate artifacts to the ledger row.

**Verification commands:**

```bash
cargo test -p roko-serve
cargo test -p roko-runtime
cargo test -p roko-gate
cargo test -p roko-chain
cd contracts && forge test
cargo test -p roko-cli
./target/debug/roko parity check --strict --area tmp/architecture/03-extensions
```

**Acceptance criteria:**
- [ ] Every extracted route, type, event, state transition, config key, command, table, and data contract above is either implemented, covered by an existing canonical implementation, or explicitly deferred with owner/dependency/future gate.
- [ ] Every normative requirement and bullet above has a corresponding test, route/CLI/API contract, docs update, or deferral rationale.
- [ ] The implementation is observable through logs/events/StateHub/projections and is debuggable from CLI or HTTP when relevant.
- [ ] Dashboard-facing behavior includes stable identifiers, timestamps, status, stale/degraded metadata, and fixture data.
- [ ] `roko parity check --strict` reports this source section as covered and not stale.

**Gap-prevention review before checking off:**
- [ ] No source detail listed above was skipped because it seemed frontend-only, optional, or already implied.
- [ ] No placeholder, TODO-only, mock-only, or type-only implementation remains on the production path.
- [ ] No duplicate implementation was introduced when an existing module could be extended.
- [ ] Contradictory or obsolete source claims are resolved in the ledger with explicit rationale.

**Self-assessment:**
- Detail score: **9.8/10**
- Rationale: this task embeds the full source section, extracts all recognizable routes/types/events/state transitions/config keys/commands/tables/data contracts/normative claims, maps them to explicit obligations, and defines verification plus gap-prevention gates.
- Iteration note: score is below 10 only because final exact target-file ownership must be confirmed against live code during implementation; if any extracted detail is not actionable, expand this task before coding.

