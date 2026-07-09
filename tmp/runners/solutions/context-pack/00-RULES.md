# Solutions Runner — Global Rules

These rules apply to **every** batch in this runner. Violating any one is
grounds for rejection regardless of how clean the diff looks otherwise.

## CRITICAL: Do NOT compile or run tests inside the batch

**DO NOT run any of these commands during a batch:**
- `cargo check`, `cargo build`, `cargo test`, `cargo clippy`, `cargo run`
- `rustc`, `rustfmt`, `cargo fmt`
- Any compilation or test execution

**Why**: compilation/test runs are handled by the post-merge validation
pipeline. Running cargo inside the batch wastes minutes and burns context.
If you need types or signatures, **read the source file**, do not compile.

## Pre-commit (the merge-back pipeline runs this — you do not)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Anything that breaks this pipeline reverts the batch.

## Tracker discipline

Every batch maps 1:1 to a row in `../ISSUE-TRACKER.md`. The row is
identified by the batch ID (e.g. `STAB_07`). On a successful commit,
include this trailer in the commit message:

```
tracker: <BATCH_ID> done <short-sha-or-blank>
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.
Without the trailer, sync cannot find the batch and the tracker drifts.

## One item per commit

Each batch produces exactly one commit. Do not bundle two batches even if
they touch the same file. Do not "while I'm here" refactor neighbours.
Out-of-scope improvements go into a separate follow-up batch — file a
new tracker row for them.

## Universal anti-patterns

Reject the diff if any of these appear:

1. **No new dispatch paths.** 4+ dispatch paths already exist
   (`ModelCallService`, `ServiceFactory`, `dispatch_direct`, ACP raw
   subprocess, plus serve routes). Fix the existing one. Do not add a 5th.
2. **No new prompt-assembly path for an existing mode.** `SystemPromptBuilder`
   is the canonical builder. Chat that bypasses it is a bug to fix, not a
   parallel implementation to ship.
3. **No new chat/session state owner.** `ChatAgentSession` owns chat state.
4. **No raw provider HTTP in CLI code.** Use the adapter layer.
5. **Demo data must be labelled.** A dashboard rendering seeded fixtures
   must say "seed" or "demo". Mixed seeded/live data without provenance is
   a UX bug.
6. **Unknown ≠ zero.** Missing usage/cost/context must remain `Option::None`.
   Never synthesise `0` to fill a gap. Zero means "definitely zero, observed".
7. **Stub gate is `Skipped`, not `Pass`.** A gate whose machinery isn't
   implemented yet returns `Verdict::Skipped { reason }`, not a fake pass.
8. **Process success ≠ artifact success.** A subprocess that exits 0 having
   produced no useful artifact is a failure. Validate the artifact.
9. **No new top-level crate** for behaviour that already lives in a current
   crate. The 18-crate workspace is the budget. Fix `roko-cli`, not `roko-cli2`.
10. **No broad `orchestrate.rs` refactor mixed with behaviour changes.**
    `orchestrate.rs` is frozen behind `legacy-orchestrate`. Bug fixes only.
    Architectural moves go into the dedicated `ORCH_*` / `XCUT_*` batches.
11. **No silent fallback.** Failed config resolution is a typed error, not
    a synthesised default that hides the misconfiguration.
12. **Missing safety config → restricted, never permissive.** A typo in
    role name must reduce permissions, never expand them.
13. **No string-interpolated payloads.** Use `serde::Serialize` / `toml`.
14. **No regex prompt scraping.** Consume typed `CommandEvent`s.
15. **No `unwrap()` / `panic!()` / `unreachable!()` in changed code.**
    Return typed errors. The point of this runner is to remove those, not
    add more.
16. **No `unsafe { std::env::set_var(...) }` after tokio runtime start.**
    UB in multithreaded Rust ≥ 1.66. Thread overrides through config
    structs.

## Subsystem-specific patterns

These are pulled from the source plan docs. The full per-subsystem rules
live in `01-PHASE-MAP.md`. Most-cited summary:

### Stability (STAB_*, source 01-STABILITY-AND-FIXES)

- AP-1: stub verdicts that return `Pass` instead of `Skipped` (rung_dispatch.rs)
- AP-2: two parallel model selection paths (auth_detect.rs vs ServiceFactory)
- AP-3: config schema split — `[[gate]]` written by init vs `[gates]` read by runtime
- AP-4: `CascadeRouter` has zero live callers
- AP-5: runner v2 writes no learning signals
- AP-6: streaming events silently drained in chat
- AP-7: repo context not wired into plan generation
- AP-8: singleton rate limiter shared across all providers
- AP-9: dual episode writes in `roko run`
- AP-10: `unsafe set_var` for `--provider`

### Orchestration (ORCH_*, source 02-ORCHESTRATION)

- AP-GOD: 22K-line `orchestrate.rs` god file
- AP-2SM: two incompatible state machines (orchestrate vs runner v2)
- AP-SERIAL: serial default despite full DAG infrastructure
- AP-NOCHECK: no checkpoint for `TaskScheduler` state
- AP-RUNG: gate rung mapping duplicated across crates
- AP-AFFECT: affect policy wired but only default used

### Inference Dispatch (DISP_*, source 03-INFERENCE-DISPATCH)

- AP-NOROUTER: `CascadeRouter` has zero live callers
- AP-4PARSE: 4 copies of stream-JSON parsing (independent truncation bugs)
- AP-ENVKEY: direct env var reads bypass the provider system
- AP-NOBUDGET: `BudgetCell` default = unlimited spend
- AP-BARESUBPROC: `Command::new` in ACP bypasses adapters
- AP-NOHEALTH: `ProviderHealthTracker` never gates dispatch
- AP-HARDCODE: 8 hardcoded model strings bypass config

### Gate Pipeline (GATE_*, source 04-GATE-PIPELINE)

- AP-1 (gate): stub gates that silently pass
- AP-5 (gate): hardcoded LLM-judge model
- AP-6 (gate): four separate gate dispatch paths
- AP-7 (gate): feedback only present in `orchestrate.rs`
- AP-9 (gate): ACP runs clippy after test (wrong order)
- AP-10 (gate): no cost tracking for LLM judge

### Gate Evolution (EVAL_*, source 05-GATE-EVOLUTION)

- AP-SUBPROCESS: each gate spawns its own subprocess
- AP-STUBJUDGE: `StubJudgeGate` always skips/fails
- AP-STRINGVERDICTS: gate output is unstructured `String`
- AP-NOEVIDENCE: evidence produced and consumed inside the same gate
- AP-NOFEEDBACK: gate outcomes never feed back to agents
- AP-SINGLEMODEL: `LlmJudgeGate` uses a single oracle, not a panel
- AP-RUNGONLY: adaptive thresholds per-rung, not per-criterion

### Prompt Assembly (PROM_*, source 06-PROMPT-ASSEMBLY)

- ISS-01: small models receive full-tier prompts
- ISS-02: `BudgetPredictor` is never called
- ISS-03: chat path bypasses `SystemPromptBuilder`
- PQ-1: `ContextTier` (Surgical / Focused / Full) drives budgets, not heuristics
- PQ-2: `BudgetPredictor` learns via EMA per role/complexity
- PQ-3: VCG auction only activates under tight budget (<80%)

### Learning & Feedback (LERN_*, source 07-LEARNING-FEEDBACK)

- AP-BLIND: `roko chat` records zero learning signals
- AP-ACPBLIND: ACP records only gate thresholds
- AP-DEADLOOP: full learning loop only in dead code
- AP-DUAL: dual episode writes in `roko run`
- AP-NOANOMALY: anomaly detector not wired
- AP-NOSECTION: section effectiveness collected but unused
- AP-NODREAM: dream cycle has no runtime trigger
- AP-IMPOVERISHED: simplified routing context (9/18 dims zeroed)

### UX & CLI (UX___*, source 08-UX-CLI)

- 4 critical: no aggregation, no funnel, no task validation, no auto-splitting
- Streaming events silently drained in chat (links to STAB_09)
- `--share` without `--serve` produces dead URL
- Model showing "-" in TUI (links to STAB_32)

### ACP & MCP (ACPM_*, source 09-ACP-MCP)

- AP-3TEMPLATES: only 3 of 6 workflow templates implemented
- AP-DUPETOKEN: `estimate_tokens` reimplemented 6 times
- AP-MCPSYNC: MCP transport is synchronous-only
- AP-NOLEARN (MCP): MCP tool outcomes not recorded
- AP-ISOLATED: MCP servers cannot discover each other
- AP-NOCARRY: session does not track touched files
- AP-UNBOUNDED: knowledge query returns unbounded results
- AP-ALLORNONE: context gathered once with no refresh
- AP-SERIAL (ACP): full template claims parallel but runs serial

### Performance (PERF_*, source 10-PERFORMANCE)

- 14 concrete optimisations from `13-PERF-OPTIMIZATION-PLAYBOOK.md`
- Warm pool tiers: HOT / WARM / COLD
- Express gate mode for trivial diffs
- Wave gating for multi-task plans
- Cache alignment for prompt prefixes
- HAL benchmark integration

### Innovations (INNO_*, source 11-INNOVATIONS)

- AP-COLD: agents start cold every run; no memory
- AP-NOLEARN (router): `force_backend` not fed to `CascadeRouter`
- AP-NEURO: `KnowledgeStore` not consulted for routing
- AP-NODREAM (innov): dream consolidation has no trigger
- AP-SAMEFAM: judge uses same model family as task agent
- AP-VERBOSE: tool outputs not truncated in context
- AP-NOEXP: experiment outcomes not fed to router
- AP-UNIFORM: gate pipeline identical for all diffs
- AP-NOCOST: no plan-level budget cap
- AP-SINGULAR: `CFactorSummary` is a single scalar

### Code Debt (DEBT_*, source 12-CODE-DEBT)

- AP-DUP: `GatePipeline` / `ComposedGatePipeline` duplication
- 4 orphan learn modules not in `lib.rs`
- ~14 unused learn modules with zero external callers
- 7 phantom config sections with zero runtime reads
- 6 phantom conductor fields
- Two write-only sinks (`ConductorObservationSink`, `DreamTriggerSink`)

### GTM & Integrations (GTM__*, source 13-GTM-AND-INTEGRATIONS)

- GT-1: adapters are lazy — instantiated only when their trigger fires
- GT-2: `SubAdapter` traits are opt-in
- 90-day shipping sequence: GitHub → Linear → Slack → Sentry → Langfuse
- 5 named recipes per `recipe.toml` schema
- OTel `gen_ai.*` semantic conventions for model calls

### Runner Patterns (RNNR_*, source 14-RUNNER-PATTERNS)

- Worktree isolation per agent
- Wave gating (10x speedup observed)
- Context handoff (4 sub-patterns)
- Failure recovery (4 sub-patterns)
- Conveyor-belt scheduling

### Testing & Verification (TEST_*, source 15-TESTING-VERIFICATION)

- AP-BENCH-STUB: `BenchmarkRegressionGate` always passes
- AP-NO-CONCURRENT: learning artifacts not tested for concurrency
- AP-NO-ROUNDTRIP: learning artifacts not tested for persistence
- AP-NO-PARITY: two gate paths never tested for equivalence
- AP-COST-ZERO: `BenchResult.cost_usd` always 0.0
- AP-UNREACHABLE: `unreachable!()` in config dispatch
- AP-NO-HARNESS: test helpers are CLI-only
- AP-SINGLE-RUN: benchmark runs once, no consistency check

### Config & Wiring (CONF_*, source 16-CONFIG-AND-WIRING)

- AP-1 (cfg): `auth_detect.rs` ignores `roko.toml` providers
- AP-2 (cfg): `roko init` writes `[[gate]]` but runtime reads `[gates]`
- AP-3 (cfg): direct env var reads bypass provider system
- AP-4 (cfg): `unsafe set_var` for `--provider`
- AP-5 (cfg): `ROKO_ACP_LEGACY` env gate
- AP-6 (cfg): `CascadeRouter` loaded but `.observe()` never called
- AP-7 (cfg): `BudgetGuardrail` never instantiated
- AP-8 (cfg): hardcoded `max_tokens` differs per entry point

### Safety & Security (SAFE_*, source 17-SAFETY-SECURITY)

- SA-1: default is safe — `dangerously_skip_permissions = false`
- SA-2: `AgentContract::permissive()` outside `#[cfg(test)]` is a security hole
- AP-2 (safety): missing safety contract YAML → restricted fallback
- Ssrf-validate non-loopback URLs in `RegisterAgentRequest`
- Mask all secret fields including `chain.wallet_key`, webhook secrets

### Observability (OBS__*, source 18-OBSERVABILITY)

- OB-1: events emit actuals (cost from response, tokens from metadata)
- OB-2: no event on success-path noise (no `ModelFallbackEvent` when primary works)
- OB-3: unknown is `None`, not `0`
- AP-2EVENT: two parallel event types for the same occurrence
- AP-EPRINT: 179 `eprintln!` calls — replace with `tracing::*`

### Cross-Cutting (XCUT_*, source 19-CROSS-CUTTING)

- AP-2HUB: `#[path]` include creates two `StateHub` types
- AP-ANYHOW: `anyhow::Result` at public crate boundaries
- AP-NOCANCEL: chat/run/dispatch lack `CancelToken`
- AP-NOSHUT: `GracefulShutdown` built but not wired
- AP-NOESCAL: no `SIGTERM → SIGKILL` escalation
- AP-NOVERSION: zero API versioning on 85 routes
- AP-NOVALID: config accepts unknown keys silently
- AP-DOCKER: single-stage Dockerfile, runs as root
- AP-RPCINLINE: ACP/MCP use inline JSON-RPC error codes
