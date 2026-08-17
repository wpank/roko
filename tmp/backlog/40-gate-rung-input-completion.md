# Gate Rung Input Completion

**Priority**: P2
**Size**: M (1-2 days)

---

## Problem

The gate pipeline has 7 rungs. Rungs 1-2 (compile, lint) run unconditionally and
produce real verdicts today. Rungs 3-6 (SymbolManifest, DependencyAudit, FactCheck,
LlmJudge) have real implementations in `roko-gate`, but the runner only supplies
complete inputs for two of them. The third and fourth are therefore silently degraded
or skipped.

There are three concrete gaps.

### Gap 1: `llm_judge_signal.diff` is always empty

`build_rung_execution_inputs()` in `runner/gate_dispatch.rs` (around line 960)
constructs a `JudgePayload` with an empty `diff` field. The comment says:

> *"The diff is left empty here because we cannot run `git diff` synchronously in this
> context."*

The runner already manages a background git-diff watcher (used by the TUI) and emits
`DashboardEvent::TaskPhaseChanged` events that include commit context. A `git diff
HEAD~1` or `git diff --cached` subprocess call after task completion is not a new
capability — it is done in `tui/git_watch.rs` already. The LlmJudgeGate receives the
description without the actual diff and therefore evaluates whether the description
sounds correct rather than whether the implementation matches the description.

### Gap 2: `fact_check_signal` oracle fields are `None`

`build_rung_execution_config()` at line 989 documents this explicitly:

> *"Oracle fields (fact-check, llm-judge) remain `None` — the rung dispatch fails
> closed with explicit skipped/not-wired verdicts when required oracles are absent."*

The FactCheck gate uses `SearchOracle` (backed by Perplexity) to verify acceptance
criteria against external sources. The runner has a Perplexity provider wired in
`roko-std` (available as an HTTP client). The oracle field on `RungExecutionConfig`
accepts an `Arc<dyn SearchOracle>`; no caller populates it. FactCheck therefore always
returns a `Skipped` verdict, silently.

### Gap 3: `effective_rungs()` has no callers in the runner

`GatesConfig::effective_rungs()` in `crates/roko-core/src/config/gates.rs:94` returns
custom rung sequences when operators configure `[gates.custom_rungs]` in `roko.toml`.
`GatePipelineBuilder` in `roko-gate/src/rung_dispatch.rs` checks
`config.has_custom_rungs()` and calls `effective_rungs()` to build a custom pipeline.
But the runner never passes the gates config into `GatePipelineBuilder::from_config`
— it always constructs the pipeline with the default complexity-based rung set. An
operator who configures a custom rung sequence in `roko.toml` gets the default
behavior silently.

### What already exists

| Component | Location | Status |
|---|---|---|
| `build_rung_execution_inputs` | `crates/roko-cli/src/runner/gate_dispatch.rs:897` | EXISTS (diff always empty) |
| `build_rung_execution_config` | `crates/roko-cli/src/runner/gate_dispatch.rs:989` | EXISTS (oracle always None) |
| `LlmJudgeGate` | `crates/roko-gate/src/llm_judge_gate.rs` | EXISTS and runs |
| `FactCheckGate` | `crates/roko-gate/src/` | EXISTS, always returns Skipped |
| `SearchOracle` trait | `crates/roko-gate/src/` | EXISTS (no ACP/Perplexity impl wired) |
| `effective_rungs()` | `crates/roko-core/src/config/gates.rs:94` | EXISTS (no runner caller) |
| `GatePipelineBuilder::from_config` | `crates/roko-gate/src/rung_dispatch.rs:110` | EXISTS (not called from runner) |
| `git diff` subprocess | `crates/roko-cli/src/tui/git_watch.rs` | EXISTS (not reused by gate dispatch) |
| Perplexity HTTP provider | `crates/roko-std/` | EXISTS (not adapted to SearchOracle) |

### What is missing

1. **Async git diff in gate dispatch** — After a task attempt completes and before the
   gate worker is spawned, run a bounded `git diff HEAD` subprocess (or read from the
   background watcher's latest snapshot) and inject the result into the
   `llm_judge_signal.diff` field. The gate worker already runs in a background tokio
   task, so this does not block the event loop.

2. **`SearchOracle` adapter for Perplexity** — Implement `SearchOracle` backed by the
   existing Perplexity HTTP client in `roko-std`, and wire it into
   `build_rung_execution_config` when the workspace has a Perplexity key configured.
   The oracle should be optional: if the key is absent, FactCheck continues to skip
   rather than error.

3. **`effective_rungs()` caller in the runner** — In the gate-dispatch path that
   constructs `GatePipelineBuilder`, pass the workspace `GatesConfig` and call
   `GatePipelineBuilder::from_config` instead of the default complexity-based
   constructor. This allows operators to override rung selection and order via
   `roko.toml` without changing code.

---

## Proposed changes

### Change A: async diff in gate dispatch
In `gate_dispatch.rs`, before spawning the gate background task, spawn a short-lived
`tokio::process::Command` for `git diff HEAD -- <workdir>` (bounded timeout, e.g. 5s).
Pass the output into `build_rung_execution_inputs` as a new `diff_text: Option<String>`
parameter. Populate `JudgePayload.diff` from it.

Estimated: ~60 lines. Risk: low (subprocess, bounded timeout, optional).

### Change B: `PerplexitySearchOracle` adapter
In `crates/roko-gate/src/` (or `crates/roko-std/src/`), add a struct
`PerplexitySearchOracle` that implements `SearchOracle` using the existing Perplexity
HTTP client. Wire it into `build_rung_execution_config` behind a config check.

Estimated: ~80 lines. Risk: medium (new trait impl, requires HTTP client context at
gate-config build time).

### Change C: pass `GatesConfig` into pipeline builder
In the gate dispatch call site that constructs `GatePipelineBuilder`, load
`workspace_config.gates` and call `GatePipelineBuilder::from_config(&gates_config, ...)`
instead of the complexity-based default. No new public API is needed — the method
already exists.

Estimated: ~20 lines. Risk: low.

---

## Acceptance criteria

1. Run a plan whose task has a non-empty `description` and a TOML `context.symbols`
   list. Rung 6 (LlmJudge) produces a verdict with a non-empty `diff` field in its
   payload (verified by `cargo test -p roko-gate` and by inspecting the gate log).
2. `effective_rungs()` is called from at least one site in
   `crates/roko-cli/src/runner/` (verified by grep).
3. `GatePipelineBuilder::from_config` is called from the runner gate-dispatch path.
4. FactCheck returns a non-`Skipped` verdict on at least one real acceptance criterion
   when a Perplexity key is present in the workspace config (integration test or manual
   run).
5. `cargo test --workspace` passes with zero failures.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## References

- `crates/roko-cli/src/runner/gate_dispatch.rs` — `build_rung_execution_inputs` (~line 897), `build_rung_execution_config` (~line 989)
- `crates/roko-core/src/config/gates.rs` — `GatesConfig`, `effective_rungs()`, `GateRungConfig`
- `crates/roko-gate/src/rung_dispatch.rs` — `GatePipelineBuilder::from_config`, `selected_rung_labels`
- `crates/roko-gate/src/llm_judge_gate.rs` — `JudgePayload`, `LlmJudgeGate`
- `crates/roko-cli/src/tui/git_watch.rs` — existing git subprocess pattern
- `crates/roko-std/src/` — Perplexity HTTP client
