# 11 — Tier 1: Silent Data Corruption (Reference; All Done)

All eight Tier-1 items shipped. This file documents what shipped and how to
detect a regression.

`git log --oneline | rg '^[a-f0-9]+ T1-'` returns 8 commits.

---

## [x] T1-8: Propagate dispatch metadata into RunnerEvent

**Commit**: `3245beca T1-8 + T1-9: propagate dispatch model/provider on TaskAttemptCompleted; remove legacy emit_feedback path`

**File**: `crates/roko-cli/src/runner/types.rs:596+, 839+, 1093, 1123` and `event_loop.rs:1344-1374`

**What landed**: `RunnerEvent::TaskAttemptCompleted` carries `model: String`
and `provider: String`. `runner_event_to_feedback` (`event_loop.rs:1344+`)
uses these directly to populate `AgentOutcome.model` and `.provider`. The
old `String::new()` placeholders are gone from this path.

**Anti-pattern this fixed**: write-only feedback. The old code wrote
empty-model episodes; the routing sink fanned out a no-op observation; the
knowledge candidate had no attribution.

**Verify (regression check)**:

```bash
# These two lines must not appear inside runner_event_to_feedback
rg 'model: String::new\(\)|provider: String::new\(\)' crates/roko-cli/src/runner/event_loop.rs
# (must be empty in event_loop.rs near line 1340-1380)

cargo test -p roko-cli runner --lib
```

---

## [x] T1-9: Remove dual feedback path

**Commit**: same as T1-8.

**File**: `crates/roko-cli/src/runner/event_loop.rs` (legacy `emit_feedback` removed)

**What landed**: The legacy `emit_feedback()` site is gone. The single feedback
path is `RunnerEvent → FeedbackEvent → FeedbackFacade → sinks`. No more
duplicate episode entries.

**Verify**:

```bash
rg 'fn emit_feedback|emit_feedback\(' crates/roko-cli/src/runner/
# Should be empty
```

---

## [x] T1-10: Replace gate catch-all with explicit match arms

**Commit**: `130477c4 T1-10: Replace gate catch-all with explicit Rung match arms`

**File**: `crates/roko-cli/src/orchestrate.rs::selected_gate_steps` (~line 17240)

**What landed**: The `_ =>` arm was replaced with explicit
`Rung::Symbol`, `Rung::PropertyTest`, `Rung::Integration` arms with
`tracing::debug!` describing why each is skipped. Adding a new `Rung`
variant now causes a compile error here.

**Verify**:

```bash
rg '_ =>' crates/roko-cli/src/orchestrate.rs | grep -i 'selected_gate'
# Should be empty inside selected_gate_steps
cargo check -p roko-cli --lib
```

---

## [x] T1-11: Fix `gate_rung_caps` hardcoded false

**Commit**: `1ac280cb T1-11: Detect symbol/property/integration test scaffolding for gate caps`

**File**: `crates/roko-cli/src/orchestrate.rs:17291-17297`

**What landed**: `gate_rung_caps` now detects scaffolding:

```rust
has_symbol_manifest: exec_dir.join("symbols.json").exists()
    || exec_dir.join(".roko").join("symbols").exists(),
has_property_tests: exec_dir.join("proptest-regressions").exists()
    || exec_dir.join("tests").join("property").exists(),
has_integration_scenario: exec_dir.join("tests").join("integration").exists()
    || exec_dir.join("integration-tests").exists(),
```

These are cheap fs checks that run once per pipeline.

**Verify**:

```bash
rg 'has_symbol_manifest: false|has_property_tests: false|has_integration_scenario: false' crates/roko-cli/src/
# Should be empty
```

Plan 29 (gate-pipeline rungs 3/5/6) actually constructs and runs these gates
when `select_rungs` admits them.

---

## [x] T1-12: Wire `validate_strict_config_toml` into production load

**Commit**: (landed prior to the explicit T-tag work; verified in current
worktree)

**Files**:

- `crates/roko-core/src/config/mod.rs:119` — `validate_strict_config_toml(&text, &strict_source)` is called inside `load_config`.
- `crates/roko-core/src/config/validation.rs` — strict validator with `StrictConfigSource::SharedFile`.
- `LoadConfigError::Validation` variant exists.

**What landed**: Loading a `roko.toml` that contains
`runner.dangerously_skip_permissions = true` returns `LoadConfigError::Validation`,
not a `RokoConfig` with the dangerous bit set.

**Verify**:

```bash
rg 'validate_strict_config_toml' crates/roko-core/src/config/mod.rs
# Must show the call inside load_config, not only re-export

cargo test -p roko-core config --lib
```

If this regresses, the strict validator is bypassed and shared
`dangerously_skip_permissions` becomes silently honored — that is a critical
security regression.

---

## [x] T1-13: Remove ContextualBanditPolicy shadow mode

**Commit**: `0538d1d1 T1-13: Remove ContextualBanditPolicy shadow-mode wiring from runner`

**Files**:

- `crates/roko-cli/src/commands/plan.rs` — Shadow construction removed.
- `crates/roko-cli/src/serve_runtime.rs` — Shadow construction removed.
- `crates/roko-learn/src/contextual_bandit.rs` — implementation kept, no
  longer wired from the runner.

**Verify**:

```bash
rg 'BanditPolicyMode::Shadow' crates/ -g '*.rs'
# Only contextual_bandit.rs should match (the type definition + a debug Display arm)
```

`CascadeRouter` is the active model-selection learner. Resurrecting Shadow
in the runner is a regression.

---

## [x] T1-14: Wire `observe_pipeline` and `drain_spc_alerts`

**Commit**: `a5eb04bd T1-14: Wire observe_pipeline and drain_spc_alerts into gate observation`

**File**: `crates/roko-cli/src/orchestrate.rs:16923-16933` (after the
per-rung `observe()` loop)

**What landed**: After per-rung observation, the orchestrator builds a
`Vec<f64>` of pass rates, calls `observe_pipeline(&pass_rates)` for cross-
rung Hotelling T² detection, and calls `drain_spc_alerts()` to emit
`tracing::warn!` for each alert (CUSUM / EWMA / BOCPD).

The pipeline is **not** blocked by alerts; logs only.

**Verify**:

```bash
rg 'observe_pipeline|drain_spc_alerts' crates/roko-cli/src/orchestrate.rs
cargo test -p roko-gate adaptive_threshold --lib
```

---

## [x] T1-15: Replace permissive safety fallback with restricted

**Commit**: `39782f5c T1-15: Replace permissive safety fallback with restricted defaults`

**File**: `crates/roko-agent/src/safety/mod.rs:246-256, 873-896`

**What landed**:

- `SafetyLayer::with_defaults()` constructs `AgentContract::restricted("default")`.
- `contract_for_role()` returns `AgentContract::restricted(role)` for missing
  YAML, with a `tracing::warn!`.
- A second nested fallback also returns `restricted` for failed loads.
- A third tier returns `restricted` ("deny-all" tone) if even the restricted
  load fails.

`AgentContract::permissive` is preserved in the API but only used in test
helpers (`#[cfg(test)] fn permissive_layer()` at line 1093+).

**Verify**:

```bash
rg 'permissive\(' crates/roko-agent/src/safety/mod.rs
# Should appear only in test code
cargo test -p roko-agent safety --lib
```

Plan 28 expands this with recovery-action invocation and override audit.

---

## Anti-Patterns to Watch (Tier 1 specific)

1. **Every `RunnerEvent::TaskAttemptCompleted` emission must populate
   `model`/`provider`.** Don't let new emission paths regress to empty
   strings.
2. **No `_ =>` in `Rung` matches.** Adding a new rung variant should cause a
   compile error so every match arm is reviewed.
3. **No silent fallback in safety contracts.** Missing → restricted; never
   permissive without an explicit local override.
4. **Strict validator runs on every shared-config load path.** If a new
   load helper appears, it must call the validator or document why
   (e.g. a test fixture loader explicitly bypasses).
5. **No new `BanditPolicyMode::Shadow` callers.** The router learns model
   selection; the bandit is preserved as a library type, not a runner
   participant.

---

## Status

- [x] T1-8 — Propagate dispatch metadata into RunnerEvent
- [x] T1-9 — Remove dual feedback path
- [x] T1-10 — Replace gate catch-all with explicit match arms
- [x] T1-11 — Fix gate_rung_caps hardcoded false
- [x] T1-12 — Wire validate_strict_config_toml into production load
- [x] T1-13 — Remove ContextualBanditPolicy shadow mode
- [x] T1-14 — Wire observe_pipeline and drain_spc_alerts
- [x] T1-15 — Replace permissive safety fallback with restricted

**Tier 1 complete.** Move on to Tier 2 (`12-tier2-delete-dead-code.md`).
