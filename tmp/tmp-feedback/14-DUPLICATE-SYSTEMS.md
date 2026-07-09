# Duplicate Systems -- Consolidation Plan

## Summary Table

| # | Duplication | Copies | Crate(s) | Priority | Effort | Risk |
|---|-------------|--------|----------|----------|--------|------|
| 1 | Config loading | 8+ entry points (2 real paths) | roko-core, roko-cli, roko-serve, roko-acp | **P0** | L | High -- env behavior differs |
| 2 | RetryPolicy | 2 independent impls | roko-core, roko-agent | **P1** | M | Low -- core has zero callers |
| 3 | ContextBidder trait + ContextCandidate | 3 incompatible definitions | roko-compose, roko-runtime, roko-neuro | **P1** | L | Medium -- trait unification |
| 4 | ErrorPattern + ConductorState | 2 identical copies (one subset) | roko-learn, roko-agent | **P2** | S | Low -- mechanical |
| 5 | GitOps types (GitOpsConfig, GitOpsRetryPolicy, ConfigDrift) | 2 identical copies | roko-runtime, roko-agent | **P2** | S | Low -- mechanical |
| 6 | ErrorClass | 2 near-identical copies | roko-agent, roko-learn | **P2** | S | Low |
| 7 | CircuitBreaker | 2 fundamentally different impls | roko-core, roko-conductor | **P3** | S | Low -- different purposes |
| 8 | CapabilityError | 2 different enums, same name | roko-agent, roko-orchestrator | **P3** | S | Low -- different domains |
| 9 | DispatchError | 3 different enums, same name | roko-core, roko-cli, roko-agent-server | **P3** | S | Low -- different domains |
| 10 | Signal/Engram alias | 1 struct + alias layer | roko-core | **P4** | M | Medium -- 144+ files |

**Effort**: S = < 1 hour, M = 1-4 hours, L = 4-8 hours

---

## 1. Config Loading -- 8+ Entry Points, 2 Real Paths

### Locations

| Function | File:Line | What It Does |
|----------|-----------|--------------|
| `load_config_unified()` | `crates/roko-core/src/config/loader.rs:105` | Hierarchical merge (global + project + env), returns `RokoConfig` |
| `load_config_with_options()` | `crates/roko-core/src/config/loader.rs:110` | Same with `LoadOptions` param |
| `load_config_file()` | `crates/roko-core/src/config/loader.rs:123` | Single-file load with options |
| `load_config_validated()` | `crates/roko-core/src/config/loader.rs:130` | Wraps `load_config_unified` + validation, returns `ValidatedConfig` |
| `load_config()` | `crates/roko-core/src/config/mod.rs:118` | Delegates to `load_config_validated()` |
| `load_config_strict()` | `crates/roko-core/src/config/mod.rs:131` | Like `load_config()` but with strict validation |
| `load_resolved_config()` | `crates/roko-cli/src/config.rs:2895` | CLI-local: `ConfigLayer` with manual TOML parsing, own env vars |
| `load_config_or_defaults()` | `crates/roko-cli/src/unified.rs:250` | Calls `load_resolved_config()` with fallback |
| `load_roko_config()` | `crates/roko-cli/src/orchestrate.rs:863` | OnceLock cache wrapping `load_config_unified` |
| `load_roko_config_models()` | `crates/roko-cli/src/run.rs:3048` | Inline call to `load_config_unified`, extracts model list |
| `load_roko_config_file()` | `crates/roko-cli/src/serve_runtime.rs:490` | Another wrapper, returns `Option<RokoConfig>` |
| `AppState::load_roko_config()` | `crates/roko-serve/src/state.rs:783` | Method wrapper for serve state |
| `load_roko_config_with_warning()` | `crates/roko-acp/src/config.rs:120` | With startup warnings |
| `load_roko_config()` | `crates/roko-acp/src/config.rs:209` | Separate ACP wrapper |

### Comparison: The Two Real Paths

**Path A: `roko-core::config::loader::load_config_unified()`** (50 callsites across 31 files)
- Hierarchical env overrides: `ROKO__SECTION__FIELD` (double-underscore separated)
- Global config at `~/.config/roko/roko.toml` merged with project `roko.toml`
- Returns `RokoConfig` (the schema type)
- No provenance tracking of which file contributed which value
- Has optional `LoadOptions` for customization

**Path B: `roko-cli::config::load_resolved_config()`** (45 callsites across 18 files)
- Own `ConfigLayer` struct with manual TOML parsing
- Flat env vars: `ROKO_*` (single underscore)
- Returns `ResolvedConfig` wrapping a `Config` (a CLI-specific type)
- Provenance tracking via `ConfigPaths` (tells caller which files were loaded)
- 30+ CLI callsites in config_cmd.rs, doctor.rs, chat_inline.rs, daemon.rs, etc.

### What's The Same
Both read from the same `roko.toml` file. Both support global + project layering. Both return roughly the same configuration data.

### What's Different
- **Env var format**: `ROKO__AGENT__MODEL` vs `ROKO_AGENT_MODEL` -- different override behavior
- **Return type**: `RokoConfig` (core schema) vs `ResolvedConfig` (CLI wrapper)
- **Validation**: Path A has `load_config_validated()` with schema validation; Path B has none
- **Provenance**: Path B tracks which files contributed; Path A does not
- **Config type**: Path B uses a CLI-local `Config` struct that wraps/mirrors `RokoConfig`

### Which Is Better
**Path A** (`load_config_unified`) is the better implementation:
- Lives in the right crate (roko-core, not roko-cli)
- Has proper env override syntax
- Has validation via `load_config_validated()`
- Already has more callsites (50 vs 45)

Path B has one feature worth keeping: provenance tracking (`ConfigPaths`).

### Consolidation Design

**Target location**: `crates/roko-core/src/config/loader.rs`

**Unified API**:
```rust
// crates/roko-core/src/config/loader.rs

/// The single entry point. All other functions are thin wrappers.
pub fn load_config_unified(workdir: &Path) -> Result<RokoConfig, LoadConfigError>

/// With explicit options (custom paths, strictness, etc.)
pub fn load_config_with_options(workdir: &Path, opts: &LoadOptions) -> Result<RokoConfig, LoadConfigError>

/// With validation.
pub fn load_config_validated(workdir: &Path) -> Result<ValidatedConfig, LoadConfigError>

/// With provenance tracking (absorb Path B's feature).
pub fn load_config_with_provenance(workdir: &Path) -> Result<(RokoConfig, ConfigProvenance), LoadConfigError>

/// ConfigProvenance tells the caller which files contributed.
pub struct ConfigProvenance {
    pub global_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    pub env_overrides: Vec<String>,
}
```

**Features to keep from each copy**:
- From Path A: hierarchical env overrides, schema validation, `LoadOptions`
- From Path B: provenance tracking (as `ConfigProvenance`)

**Migration path**:
1. Add `ConfigProvenance` to roko-core's loader
2. Add `load_config_with_provenance()` to roko-core
3. Make `roko-cli::config::load_resolved_config()` delegate to `load_config_with_provenance()`
4. Migrate CLI callsites one file at a time from `load_resolved_config()` to core's loader
5. Deprecate and eventually delete `load_resolved_config()`

**How to prevent re-duplication**:
- Mark `load_resolved_config` as `#[deprecated]` immediately
- Add a module-level doc comment in `roko-core/src/config/mod.rs` stating "ALL config loading goes through `loader::load_config_unified`"
- Lint: `clippy::disallowed_methods` could flag the deprecated path

### Code-Level Plan

**Files to modify**:
- `crates/roko-core/src/config/loader.rs` -- add `ConfigProvenance` + `load_config_with_provenance()`
- `crates/roko-cli/src/config.rs` -- deprecate `load_resolved_config()`, delegate to core
- `crates/roko-cli/src/config_cmd.rs` (5 callsites) -- switch to core loader
- `crates/roko-cli/src/doctor.rs` (2 callsites)
- `crates/roko-cli/src/chat_inline.rs` (2 callsites)
- `crates/roko-cli/src/unified.rs` (1 callsite)
- `crates/roko-cli/src/daemon.rs` (2 callsites)
- `crates/roko-cli/src/commands/agent.rs` (1 callsite)
- `crates/roko-cli/src/commands/job.rs` (2 callsites)
- `crates/roko-cli/src/commands/plan.rs` (1 callsite)
- `crates/roko-cli/src/commands/server.rs` (1 callsite)
- `crates/roko-cli/src/commands/do_cmd.rs` (2 callsites)
- `crates/roko-cli/src/bench_demo.rs` (2 callsites)
- `crates/roko-cli/src/prd.rs` (2 callsites)
- `crates/roko-cli/src/dispatch_v2.rs` (1 callsite)
- `crates/roko-cli/src/run.rs` (1 callsite)
- `crates/roko-cli/src/main.rs` (2 callsites)
- `crates/roko-cli/src/lib.rs` (4 callsites)

**Files to delete**: None (deprecate, don't delete yet)

**Import changes**: All `use crate::config::load_resolved_config` -> `use roko_core::config::loader::load_config_unified` (or `load_config_with_provenance` where provenance is needed)

**Test updates**: `crates/roko-cli/src/config.rs` has 4 tests for `load_resolved_config` -- keep them but point at the new delegation path.

### Risk Analysis
- **High risk**: any callsite that depends on `ROKO_*` flat env vars will silently break when switched to `ROKO__*` double-underscore format. Audit every env var reference first.
- **Medium risk**: `ResolvedConfig.config` vs `RokoConfig` are structurally similar but may differ in optional fields. Run the full test suite after each batch of migrations.
- **Low risk**: the `AppState::load_roko_config()` wrapper in roko-serve is harmless (thin method on state).

---

## 2. RetryPolicy -- Two Independent Implementations

### Locations

| Copy | File:Line | Callers |
|------|-----------|---------|
| Core | `crates/roko-core/src/error/retry.rs:27` | **0 direct callers** (only roko-core error methods reference it) |
| Agent | `crates/roko-agent/src/retry.rs:45` | `tool_loop/mod.rs:250,280,325,1560` (the actual retry loop) |

### Comparison

| Feature | Core | Agent |
|---------|------|-------|
| Fields | `max_attempts`, `base_delay_ms`, `max_delay_ms`, `jitter` (bool) | `max_attempts`, `base_delay_ms`, `max_delay_ms`, `retryable_errors` (Vec) |
| `should_retry()` | `const fn(attempt: u32) -> bool` (budget check only) | `fn(&ProviderError, u32) -> bool` (error classification) |
| `delay_for()` | Deterministic jitter via xorshift hash | True random jitter via `rand::thread_rng()` |
| `execute()` | Generic async executor: `FnMut() -> Fut` | None (caller implements loop manually in tool_loop) |
| `Retry-After` | Not supported | `delay_with_retry_after(attempt, retry_after_ms)` |
| Error classification | None | `ErrorClass` enum (8 variants) + `From<&ProviderError>` |
| Defaults | None (all params required) | `Default` impl using `roko_core::defaults::*` constants |
| Tests | 11 tests | 3 tests |
| Dependencies | None (no `rand`) | `rand`, `roko_core::defaults` |

### Which Is Better
**Neither is complete alone**. The agent's is the load-bearing one (it actually runs). The core's has two things the agent's lacks: a generic `execute()` async executor and deterministic jitter (no `rand` dep). But the agent's has provider-aware error classification and `Retry-After` support, which are essential for LLM APIs.

### Consolidation Design

**Target location**: `crates/roko-core/src/error/retry.rs`

**Unified API**:
```rust
// crates/roko-core/src/error/retry.rs

/// Core retry policy with exponential backoff.
/// Supports both deterministic and random jitter.
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    jitter: JitterMode,
}

pub enum JitterMode {
    /// No jitter.
    None,
    /// Deterministic jitter from xorshift hash (no rand dependency).
    Deterministic,
    /// Full-jitter via provided RNG (requires rand feature).
    #[cfg(feature = "rand-jitter")]
    FullRandom,
}

impl RetryPolicy {
    pub fn should_retry(&self, attempt: u32) -> bool { ... }
    pub fn delay_for(&self, attempt: u32) -> Duration { ... }
    pub fn delay_with_retry_after(&self, attempt: u32, retry_after: Option<Duration>) -> Duration { ... }
    pub async fn execute<F, Fut, T, E>(&self, f: F) -> Result<T, E> { ... }
}

/// Error classification trait -- implemented by consumers
/// to teach RetryPolicy which errors are retryable.
pub trait RetryClassifier {
    fn is_retryable(&self, attempt: u32) -> bool;
}
```

**Features to keep from each copy**:
- From Core: generic `execute()` async helper, deterministic jitter, `CircuitBreaker`
- From Agent: `ErrorClass` enum, `Retry-After` support, `Default` impl, error classification logic

**Migration path**:
1. Merge agent's `ErrorClass`, `should_retry(&ProviderError, u32)`, and `delay_with_retry_after` into core's `RetryPolicy`
2. Add `RetryClassifier` trait to core so the agent can implement provider-specific classification without core depending on `ProviderError`
3. Make `roko-agent/src/retry.rs` a thin wrapper: `pub use roko_core::error::retry::RetryPolicy;` + implement `RetryClassifier for ProviderError`
4. Update `tool_loop/mod.rs` imports

### Code-Level Plan

**Files to modify**:
- `crates/roko-core/src/error/retry.rs` -- add `delay_with_retry_after()`, `JitterMode`, `RetryClassifier` trait
- `crates/roko-agent/src/retry.rs` -- replace with re-export + `impl RetryClassifier for ProviderError`
- `crates/roko-agent/src/tool_loop/mod.rs:32,250,280,325` -- update imports
- `crates/roko-core/src/error/mod.rs:321,450-473,815-833` -- update `retry_policy()` methods

**Files to delete**: None (`roko-agent/src/retry.rs` becomes a thin adapter)

**Test updates**:
- Core: keep all 11 existing tests, add tests for `delay_with_retry_after()`
- Agent: keep 3 existing tests, update to use the unified type

### Risk Analysis
- **Low risk**: core's `RetryPolicy` has zero external callers, so changing its API breaks nothing
- **Low risk**: agent's callers are all in `tool_loop/mod.rs` (one file)
- **Watch out**: the `rand` dependency. Core currently avoids `rand`; the unified version should use `cfg(feature)` to keep it optional

---

## 3. ContextBidder Trait + ContextCandidate -- Three Incompatible Definitions

### Locations

| Copy | File:Line | Status |
|------|-----------|--------|
| Compose-time | `crates/roko-compose/src/context_provider.rs:682` (trait), `:376` (struct) | **WIRED** -- used in SystemPromptBuilder VCG auction |
| Runtime | `crates/roko-runtime/src/heartbeat_attention.rs:665` (trait), `:108` (struct) | **NOT WIRED** -- 8 bidder impls but never called from dispatch |
| Neuro-internal | `crates/roko-neuro/src/context.rs:1202` (struct only, private) | **INTERNAL** -- only used inside neuro context assembly |

### Comparison: The Two Traits

| Feature | Compose (context_provider.rs) | Runtime (heartbeat_attention.rs) |
|---------|-------------------------------|----------------------------------|
| Method | `propose_context(&self, &PromptInput) -> Vec<ContextCandidate>` | `generate_candidates(&self, &BidderContext) -> Vec<ContextCandidate>` |
| Identifier | `bidder_id(&self) -> &str` | `subsystem_id(&self) -> SubsystemId` |
| Input | `PromptInput` (prompt-level: role, task desc, etc.) | `BidderContext` (runtime-level: PAD vector, retry count, deadlines, build rates) |
| Candidate type | `{ section: ContextSection, relevance: f32, bidder: AttentionBidder }` | `{ subsystem_id, category, token_count, expected_value, urgency, content_summary }` |
| Implementations | `LearningContextBidder` | `NeuroBidder`, `DaimonBidder`, `IterationMemoryBidder`, `CodeIntelligenceBidder`, `PlaybookRulesBidder`, `ResearchBidder`, `PredictionBidder`, `SafetyBidder` |
| Auction | VCG allocator in SystemPromptBuilder | `run_attention_auction()` in heartbeat_attention.rs (NOT wired) |

### Comparison: The Three ContextCandidate Structs

| Field | Compose | Runtime | Neuro |
|-------|---------|---------|-------|
| Content | `section: ContextSection` | `content_summary: String` | `chunk: ContextChunk`, `full_content: String`, `summary_content: String` |
| Relevance | `relevance: f32` | `expected_value: f64` | `base_bid: f64` |
| Source | `bidder: AttentionBidder` | `subsystem_id: SubsystemId` | `family: SourceFamily` |
| Tokens | (from section) | `token_count: usize` | `full_tokens: usize`, `summary_tokens: usize` |
| Urgency | (not present) | `urgency: f64` | (not present) |
| Category | (not present) | `category: ContextCategory` | `priority: AttentionPriority` |
| Visibility | `pub` | `pub` | `pub(crate)` (private to neuro) |

### Which Is Better
The **runtime** version is architecturally richer (urgency, affect modulation, 8 bidder implementations with real logic). But the **compose** version is the one that actually runs at dispatch time. The neuro-internal one is fine as-is (private, different purpose).

### Consolidation Design

**Target location**: `crates/roko-core/src/context_bid.rs` (new file in core)

**Unified API**:
```rust
// crates/roko-core/src/context_bid.rs

/// Subsystem that can propose context for the attention auction.
pub trait ContextBidder: Send + Sync {
    /// Unique identifier for this bidder.
    fn bidder_id(&self) -> &str;

    /// Generate context candidates given the current context.
    fn generate_candidates(&self, ctx: &BidContext) -> Vec<ContextCandidate>;
}

/// Merged input context (compose-time + runtime signals).
pub struct BidContext {
    // From compose-time PromptInput:
    pub role: String,
    pub task_description: String,
    // From runtime BidderContext:
    pub pad: PadVector,
    pub consecutive_failures: u32,
    pub deadline_pressure: f64,
    pub safety_relevance: f64,
    // ... all fields from both
}

/// Unified context candidate.
pub struct ContextCandidate {
    pub bidder_id: String,
    pub category: ContextCategory,
    pub content: ContextContent,    // enum: Section(ContextSection) | Text(String)
    pub token_count: usize,
    pub relevance: f64,             // normalized [0.0, 1.0]
    pub urgency: f64,               // multiplier, default 1.0
}
```

**Features to keep from each copy**:
- From Compose: `ContextSection` support (structured prompt sections), VCG auction integration
- From Runtime: `BidderContext` signals (PAD, deadlines, retry counts), `ContextCategory` affect modulation, all 8 bidder implementations

**Migration path**:
1. Define the unified `ContextBidder` trait and `ContextCandidate` in roko-core
2. Adapt compose-time `LearningContextBidder` to implement the new trait
3. Adapt runtime's 8 bidders to implement the new trait
4. Wire the runtime bidders into the compose-time VCG auction (this is the actual win -- the 8 subsystem bidders can finally participate)
5. Leave neuro's private `ContextCandidate` as-is (it's internal, different purpose)

### Code-Level Plan

**Files to create**:
- `crates/roko-core/src/context_bid.rs` -- unified trait + types

**Files to modify**:
- `crates/roko-core/src/lib.rs` -- add `pub mod context_bid;`
- `crates/roko-compose/src/context_provider.rs:376,682` -- adapt `ContextCandidate` and `ContextBidder` to use core's
- `crates/roko-compose/src/system_prompt_builder.rs` -- update VCG auction to accept `Vec<Box<dyn core::ContextBidder>>`
- `crates/roko-runtime/src/heartbeat_attention.rs:108,665` -- adapt structs and trait to implement core's
- All 8 bidder impls in `heartbeat_attention.rs:675-790+` -- implement new trait

**Files to delete**: None (the compose and runtime modules keep their local adapter logic)

**Test updates**: Tests in both `context_provider.rs` and `heartbeat_attention.rs` need signature updates.

### Risk Analysis
- **Medium risk**: The compose-time VCG auction expects `ContextSection` (rich structured data). The runtime bidders produce plain text summaries. The `ContextContent` enum bridges this, but the auction scoring logic may need adjustment.
- **Low risk**: The neuro-internal `ContextCandidate` is private and untouched.
- **Dependency concern**: roko-compose and roko-runtime must both depend on roko-core (they already do).

---

## 4. ErrorPattern + ConductorState -- Two Identical Copies (One Subset)

### Locations

| Copy | File:Line | Variants |
|------|-----------|----------|
| roko-learn | `crates/roko-learn/src/conductor.rs:59` (ErrorPattern), `:40` (ConductorState) | 10 variants: Unknown, Compile, Test, ToolCall, Timeout, RateLimit, ContextOverflow, Refusal, LoopDetected, Infrastructure |
| roko-agent | `crates/roko-agent/src/task_runner.rs:302` (ErrorPattern), `:319` (ConductorState) | 6 variants: Unknown, Compile, Test, ToolCall, Timeout, Infrastructure |

### Comparison

The roko-agent copy is a **strict subset** of the roko-learn copy. The roko-learn version has 4 additional variants: `RateLimit`, `ContextOverflow`, `Refusal`, `LoopDetected`. Both `ConductorState` structs are field-identical (same 7 fields, same types, same names). The roko-learn copy derives `Debug, Clone`; the roko-agent copy derives `Debug, Clone, PartialEq`.

### Which Is Better
**roko-learn's** -- it's the superset with all variants.

### Consolidation Design

**Target location**: `crates/roko-learn/src/conductor.rs` (keep the superset in place)

**Unified API**: No change needed to the learn version. Add `PartialEq` derive to match the agent version.

**Migration path**:
1. Add `PartialEq` derive to `ErrorPattern` and `ConductorState` in `roko-learn/src/conductor.rs`
2. Export both from `roko-learn` crate root
3. In `roko-agent/src/task_runner.rs`, delete the local `ErrorPattern` and `ConductorState` definitions
4. Add `use roko_learn::conductor::{ErrorPattern, ConductorState};`

### Code-Level Plan

**Files to modify**:
- `crates/roko-learn/src/conductor.rs:39-40` -- add `PartialEq` to derives
- `crates/roko-learn/src/lib.rs` -- ensure `pub use conductor::{ErrorPattern, ConductorState};`
- `crates/roko-agent/src/task_runner.rs:299-334` -- delete both definitions, add import
- `crates/roko-agent/Cargo.toml` -- add `roko-learn` dependency (if not already present)

**Files to delete**: None (inline deletion)

**Test updates**: None expected (the types are structurally identical).

### Risk Analysis
- **Low risk**: the agent copy is a strict subset. All existing agent-side code uses only the 6 variants that exist in the learn copy.
- **Dependency concern**: roko-agent may not currently depend on roko-learn. Adding a learn -> agent dependency could create a cycle. **Check dependency graph first.** If a cycle exists, move `ErrorPattern` + `ConductorState` to `roko-core` instead.

---

## 5. GitOps Types -- Two Identical Copies

### Locations

| Copy | File:Line | Types |
|------|-----------|-------|
| roko-runtime | `crates/roko-runtime/src/lifecycle.rs:310` (GitOpsConfig), `:349` (GitOpsRetryPolicy), `:374` (ConfigDrift) | Canonical home |
| roko-agent | `crates/roko-agent/src/lifecycle.rs:1813` (GitOpsConfig), `:1852` (GitOpsRetryPolicy), `:1877` (ConfigDrift) | Copy |

### Comparison
These are **byte-for-byte identical** copies (same field names, same types, same derives, same defaults, same doc comments). The roko-runtime copy is already re-exported via `roko_runtime::lib.rs:73`.

### Which Is Better
**roko-runtime's** -- it's the canonical home and already re-exported.

### Consolidation Design

**Target location**: `crates/roko-runtime/src/lifecycle.rs` (already there)

**Migration path**:
1. In `roko-agent/src/lifecycle.rs`, delete the `GitOpsConfig`, `GitOpsRetryPolicy`, and `ConfigDrift` definitions (lines ~1810-1897)
2. Add `use roko_runtime::{GitOpsConfig, GitOpsRetryPolicy, ConfigDrift};`
3. Re-export from `roko_runtime` if not already (it is: `lib.rs:73`)

### Code-Level Plan

**Files to modify**:
- `crates/roko-agent/src/lifecycle.rs` -- delete ~87 lines (three struct/enum defs + impls), add import
- `crates/roko-agent/Cargo.toml` -- ensure `roko-runtime` is in `[dependencies]`

**Files to delete**: None

**Test updates**: None (types are identical).

### Risk Analysis
- **Low risk**: identical types. Serde compatibility is guaranteed because field names and `rename_all` are the same.
- **Dependency**: roko-agent likely already depends on roko-runtime. Verify with `Cargo.toml`.

---

## 6. ErrorClass -- Two Near-Identical Copies

### Locations

| Copy | File:Line | Variants |
|------|-----------|----------|
| roko-agent | `crates/roko-agent/src/retry.rs:9` | RateLimit, AuthFailure, Timeout, ServerError, ContentPolicy, ContextOverflow, ModelNotFound, Unknown |
| roko-learn | `crates/roko-learn/src/provider_health.rs:54` | RateLimit, AuthFailure, Timeout, ServerError, ContentPolicy, ContextOverflow, ModelNotFound, Unknown |

### Comparison
**Identical variant names.** The roko-learn copy has `Serialize, Deserialize` derives; the roko-agent copy does not. The roko-agent copy has `From<&ProviderError>` impl; the roko-learn copy does not.

### Which Is Better
**Neither alone**. Merge: take the roko-learn derives (for persistence) + the roko-agent `From<&ProviderError>` impl.

### Consolidation Design

**Target location**: `crates/roko-core/src/error/mod.rs` or new `crates/roko-core/src/error/classification.rs`

Both roko-agent and roko-learn depend on roko-core, so placing `ErrorClass` in core avoids cycles.

**Unified API**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
    RateLimit,
    AuthFailure,
    Timeout,
    ServerError,
    ContentPolicy,
    ContextOverflow,
    ModelNotFound,
    Unknown,
}
```

The `From<&ProviderError>` impl stays in roko-agent (since `ProviderError` lives there).

**Migration path**:
1. Add `ErrorClass` to `roko-core/src/error/mod.rs`
2. In `roko-agent/src/retry.rs`, delete enum, add `use roko_core::error::ErrorClass;`
3. In `roko-learn/src/provider_health.rs`, delete enum, add `use roko_core::error::ErrorClass;`

### Code-Level Plan

**Files to modify**:
- `crates/roko-core/src/error/mod.rs` -- add `ErrorClass` enum
- `crates/roko-agent/src/retry.rs:9-26` -- delete enum, add import, keep `From<&ProviderError>` impl
- `crates/roko-learn/src/provider_health.rs:54-68` -- delete enum, add import

**Files to delete**: None

**Test updates**: Minimal -- both crates construct `ErrorClass` values directly.

### Risk Analysis
- **Low risk**: identical variant set, so all match arms continue to work.

---

## 7. CircuitBreaker -- Two Fundamentally Different Implementations

### Locations

| Copy | File:Line | Purpose |
|------|-----------|---------|
| Core | `crates/roko-core/src/error/retry.rs:176` | Simple in-memory breaker: `Closed -> Open -> HalfOpen` state machine |
| Conductor | `crates/roko-conductor/src/circuit_breaker.rs:147` | Per-plan concurrent breaker with `DashMap`, predictive tripping (Holt forecasting), persistence |

### Comparison

| Feature | Core | Conductor |
|---------|------|-----------|
| Thread safety | Not `Send`/`Sync` (uses `&mut self`) | `Send + Sync` via `DashMap` |
| Granularity | Single breaker instance | Per-plan via `DashMap<String, FailureRecord>` |
| Prediction | None | Holt exponential smoothing forecaster (COND-08) |
| Persistence | None | `CircuitBreakerState` snapshot/restore |
| Callers | Zero (co-located with `RetryPolicy` as reference impl) | Used by `roko-conductor/src/conductor.rs:9,215,247` |

### Which Is Better
These serve **different purposes**. The core version is a textbook reference implementation. The conductor version is a production system with real features. They are not duplicates in the "same thing built twice" sense -- they are different abstractions that happen to share a name.

### Consolidation Design

**Recommendation**: Do NOT merge. Instead, rename or scope to avoid confusion:
- Core: rename to `SimpleCircuitBreaker` or delete entirely (zero callers)
- Conductor: keep as `CircuitBreaker` (the production version)

**Target**: `crates/roko-core/src/error/retry.rs` -- delete `BreakerState` and `CircuitBreaker` (lines 150-255) and their tests (lines 350-455), since they have zero callers.

### Code-Level Plan

**Files to modify**:
- `crates/roko-core/src/error/retry.rs` -- delete `BreakerState`, `CircuitBreaker`, and 10 tests (~100 lines)
- `crates/roko-core/src/error/mod.rs` -- remove any re-exports of `BreakerState`/`CircuitBreaker`

**Test updates**: Delete the 10 circuit breaker tests in retry.rs.

### Risk Analysis
- **Zero risk**: core's circuit breaker has zero callers outside its own test module.

---

## 8. CapabilityError -- Two Different Enums, Same Name

### Locations

| Copy | File:Line | Variants |
|------|-----------|----------|
| roko-agent | `crates/roko-agent/src/safety/capabilities.rs:193` | `NotCovered`, `DepthExhausted` |
| roko-orchestrator | `crates/roko-orchestrator/src/safety/capability_tokens.rs:222` | `AlreadyBurned(Uuid)`, `Expired`, `BadSignature`, `Revoked(Uuid)`, `InvalidTarget(String)` |

### Comparison
These are **not duplicates** -- they model different failure modes in different subsystems:
- The agent's `CapabilityError` is about **delegation** (can this warrant be passed to a sub-agent?)
- The orchestrator's `CapabilityError` is about **verification** (is this capability token valid?)

### Consolidation Design

**Recommendation**: Rename to avoid confusion, do NOT merge.
- `roko-agent`: rename to `DelegationError` (matches its domain -- delegation depth, coverage checks)
- `roko-orchestrator`: keep as `CapabilityError` (it's the primary capability system)

### Code-Level Plan

**Files to modify**:
- `crates/roko-agent/src/safety/capabilities.rs:193` -- rename `CapabilityError` to `DelegationError`
- Any callers in roko-agent that reference `CapabilityError` -- update to `DelegationError`

**Risk**: Low -- the name change is internal to roko-agent.

---

## 9. DispatchError -- Three Different Enums, Same Name

### Locations

| Copy | File:Line | Domain |
|------|-----------|--------|
| roko-core | `crates/roko-core/src/dispatch_plan.rs:197` | Plan-level dispatch resolution failures (MissingAuth, UnsupportedProvider, CapabilityMismatch, AmbiguousProvider, AmbiguousModel) |
| roko-cli | `crates/roko-cli/src/dispatch/outcome.rs:77` | Pre-spawn runtime failures (BudgetExceeded, NoModelAvailable, PreValidationFailed) |
| roko-agent-server | `crates/roko-agent-server/src/state.rs:40` | Sidecar dispatch failures (NotConfigured, DispatchFailed) |

### Comparison
These are **not duplicates** -- they describe different layers of the dispatch stack:
- Core: static plan resolution errors
- CLI: runtime budget/model selection errors
- Agent-server: HTTP sidecar errors

### Consolidation Design

**Recommendation**: Rename to clarify scope, do NOT merge.
- `roko-core`: rename to `PlanResolutionError`
- `roko-cli`: keep as `DispatchError` (it's the primary runtime dispatch error)
- `roko-agent-server`: rename to `SidecarDispatchError`

### Code-Level Plan

**Files to modify**:
- `crates/roko-core/src/dispatch_plan.rs:197` -- rename to `PlanResolutionError`
- `crates/roko-core/src/dispatch_plan.rs` -- update all references in the file
- `crates/roko-agent-server/src/state.rs:40` -- rename to `SidecarDispatchError`
- Any importers of these types (check with grep)

**Risk**: Low -- these are different subsystem types.

---

## 10. Signal/Engram Alias

### Locations

| What | File:Line |
|------|-----------|
| Canonical struct | `crates/roko-core/src/engram.rs` (55+ references inside roko-core) |
| Alias module | `crates/roko-core/src/signal.rs:6` (`pub use Engram as Signal`) |
| Downstream usage | 144 files import `Signal`/`Engram` via 179 occurrences |

### Current State
```rust
// crates/roko-core/src/signal.rs
pub use crate::engram::{Engram as Signal, EngramBuilder as SignalBuilder, HdcFingerprint};
```

- roko-core internals: use `Engram` (212 occurrences across 22 files in src/)
- All other crates: use `Signal` (via the alias, 179 occurrences across 144 files)
- `Datum::Engram` variant still says "Engram"
- Doc generation shows `Engram` as canonical
- Error messages mix both names

### Consolidation Design

**Two options**:

**Option A: Complete the rename (Engram -> Signal everywhere)**
- Rename the struct in `engram.rs` to `Signal`
- Rename the file to `signal.rs` (move the struct there)
- Keep `pub use signal::Signal as Engram` as a deprecated backward-compat alias
- Update all 212 internal references
- Rename `Datum::Engram` to `Datum::Signal`
- Rename `EngramBuilder` to `SignalBuilder`
- Effort: M (4 hours, mostly mechanical find-replace)
- Risk: Medium (212 internal references + `Datum` variant rename is a breaking change for any serialized data)

**Option B: Accept the alias and document it**
- Add a doc comment to `engram.rs` explaining the naming history
- Ensure all public API surfaces use `Signal` (not `Engram`)
- Keep `Engram` as the internal name indefinitely
- Effort: S (1 hour)
- Risk: Low (no code changes)

**Recommendation**: **Option B** for now. The alias works, both names compile, and a mass rename risks breaking serialized `Datum::Engram` values in existing `.roko/signals.jsonl` files. The rename can happen later with a migration path for the Datum variant.

---

## Dependency Graph Between Consolidation Tasks

```
 [1] Config loading (P0)
      |
      v
 [2] RetryPolicy (P1) ---- [6] ErrorClass (P2) --- needs roko-core placement
      |
      v
 [3] ContextBidder (P1) --- needs roko-core trait definition
      |
      v
 [4] ErrorPattern + ConductorState (P2) --- check for dep cycles first
 [5] GitOps types (P2) --- mechanical, no dependencies
 [7] CircuitBreaker (P3) --- delete dead code in core
 [8] CapabilityError (P3) --- rename only
 [9] DispatchError (P3) --- rename only
[10] Signal/Engram (P4) --- defer
```

**Execution order**:
1. Start with **#5 GitOps types** and **#7 CircuitBreaker deletion** -- zero risk, immediate wins
2. Then **#4 ErrorPattern** and **#6 ErrorClass** -- low risk, place shared types in roko-core
3. Then **#2 RetryPolicy** -- depends on ErrorClass being in core
4. Then **#1 Config loading** -- highest impact, needs careful env var audit
5. Then **#3 ContextBidder** -- architectural change, biggest design work
6. **#8, #9** anytime -- rename-only changes
7. **#10** deferred indefinitely unless there's a forcing function
