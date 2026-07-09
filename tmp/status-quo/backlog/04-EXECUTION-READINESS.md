# 04 — Execution Readiness (M0 Bootstrap)

> **What this doc is:** the gate that sits *before* every other backlog epic. Nothing in
> `backlog/epics/*` can be executed by roko autonomously until the items here are true, in
> dependency order. Each M0 item is verified against code + the runtime traces (docs 96–101).
>
> - Repo: `/Users/will/dev/nunchi/roko/roko`
> - HEAD: `5852c93c05` (branch `main`) · authored 2026-07-09
> - Method: every `file:line` below was opened and read, not inferred. Trace sources:
>   `96-TRACE-RUNNER-V2-EXECUTION.md`, `101-TRACE-GATE-PIPELINE.md`, `98-TRACE-SELF-HOSTING-LOOP.md`.

---

## 0. TL;DR — the one fix that unblocks everything

**`roko plan run plans/E-xx` (bare, no `--engine`) executes the Graph engine, which is a
dry-run stub that does nothing.** An autonomous agent following the self-hosting workflow will
"run" every plan, see green, and change zero files.

- Graph path terminates in `TaskExecutorCell`: `dry_run` defaults `true`
  (`crates/roko-graph/src/cells/task_executor.rs:20,32`); the live branch logs
  *"TaskExecutorCell live dispatch not yet implemented; using dry-run fallback"*
  (`task_executor.rs:81-86`) and returns a synthetic engram.
- The working executor is **Runner v2** (`crates/roko-cli/src/runner/event_loop.rs`), reachable
  **only** via the explicit flag `--engine runner-v2`. The clap arg defaults to `"graph"`
  (`crates/roko-cli/src/commands/plan.rs:1361`) and one call site hardcodes `PlanEngine::Graph`
  (`crates/roko-cli/src/main.rs:2699`) — even though `PlanEngine::default()` is `RunnerV2`
  (`main.rs:1301`). The clap literal wins. (Confirmed: trace 96 §1.2.)

> **THE unblocker → M0.1.** Flip the default so bare `roko plan run` reaches Runner v2 (or make
> Graph delegate to Runner v2's live dispatch). Cargo already ships `default =
> ["legacy-runner-v2"]` (`crates/roko-cli/Cargo.toml:16` — P11 T1 landed), so the runner code is
> compiled; only the CLI default selection is wrong.

Everything else on this page assumes M0.1 is done (or that the agent always passes
`--engine runner-v2`). With the flag forced, the *next* real blocker is honest pass/fail (M0.3):
gate rungs 3–6 stub-**pass** unconditionally, so "green" is not yet trustworthy.

---

## 1. Readiness gates (dependency order)

Each item: **problem → evidence → exact fix → verify command → plan mapping.**
Legend: **[BLOCKER]** must ship for MVP self-hosting · **[TRUST]** required for honest pass/fail ·
**[DEFERRED]** reliability/scale, not correctness — safe to run without it (serially / single-plan).

### M0.1 — Flip engine default to Runner v2  **[BLOCKER]**
- **Problem:** bare `roko plan run` → Graph → dry-run stub → no file changes (§0).
- **Evidence:** `task_executor.rs:20,81-86`; clap `default_value="graph"` `commands/plan.rs:1361`;
  hardcode `main.rs:2699`; `PlanEngine::default()=RunnerV2` `main.rs:1301`.
- **Fix:** (a) change `#[arg(long, default_value = "graph", …)]` → `default_value = "runner-v2"`
  at `commands/plan.rs:1361`; (b) change the `PlanCmd::Run{…engine: PlanEngine::Graph…}` construction
  at `main.rs:2699` to `PlanEngine::RunnerV2` (or `Default::default()`). Keep `graph` selectable for
  debugging. Alternatively, make `cmd_plan_run_engine` (Graph arm, `plan.rs:258`) call
  `event_loop::run`.
- **Verify:** `cargo run -p roko-cli -- plan run plans/e2e-smoke 2>&1 | grep -q 'event_loop\|runner-v2'`
  and confirm files actually change (`git status --porcelain` non-empty after a real task).
- **Plan:** **P11-runner-v2-default** (T1 landed; T2 "make RunnerV2 the default variant" + clap/hardcode
  fixes are the remaining work).

### M0.2 — Confirm per-task VerifyStep runner  **[TRUST] — CONFIRMED WORKING**
- **Status:** ✅ already works on the live path. Task `[[task.verify]]` steps from `tasks.toml`
  are run as `ShellGate("bash -o pipefail -c …")` and appended to the rung verdicts:
  `run_verify_steps(...)` at `runner/gate_dispatch.rs:121-123` (shell wrap `:359-371`);
  `passed = verdicts.iter().all(|v| v.passed)` `:140`. A failing verify command *does* fail the task.
- **Keep-honest note:** verify runs only when the gate path is reached; read-only roles
  (`researcher|strategist|quick-reviewer`) auto-pass and **skip gates entirely** (`event_loop.rs:4747-4787`),
  so don't assign verify-bearing tasks a read-only role.
- **Verify:** author a task with `verify.command = "false"`; run `--engine runner-v2`; assert the task
  is marked failed in `.roko/state/run-ledger.jsonl`.
- **Plan:** none needed (regression guard only).

### M0.3 — Make gate verdicts honest (no stub-pass)  **[TRUST] — BLOCKER for trustworthy acceptance**
- **Problem:** the live gate path (a) caps at `max_gate_rung = 2` by default
  (`runner/types.rs:1427-1431,1482`); (b) for tiers ≥ integrative, selects Symbol / GeneratedTest /
  Integration / FactCheck / LlmJudge rungs that **`stub_verdict → Verdict::pass` unconditionally**
  because their inputs are `RungExecutionInputs::default()` — no oracle/manifest is attached
  (`gate_dispatch.rs:104,110-119`; `rung_dispatch.rs:290`); (c) **never calls `enrich_rung_config`**
  (that lives only in the DEAD legacy `orchestrate.rs:18492`); (d) threshold EMAs only ever update at
  key `rung=2` and `GateThresholds::save` has **zero callers** — `.roko/learn/gate-thresholds.json`
  is read-only in the live path (trace 101 §1.6-1.7). Net: a "pass" on a complex task can be a fiction.
- **Fix (minimum, fail-closed):** in `gate_dispatch.rs` build the default pipeline with only the rungs
  that have real verifiers (Compile / Lint / Test / PropertyTest); either (i) drop stub rungs from
  `select_rungs`, or (ii) make `run_canonical_rung` return `Verdict::skip` (not `pass`) when inputs are
  `default()`, and treat skip≠pass in acceptance reporting. **Fuller fix:** port `enrich_rung_config`'s
  input wiring (source_roots, SymbolManifest, oracles) into the Runner v2 pipeline builder so 3–6
  actually run.
- **Verify:** run an `architectural`-tier task that fails a symbol check; assert the gate reports
  **fail/skip**, not pass. Then `grep -c '"passed":true' .roko/signals.jsonl` reflects only real rungs.
- **Plan:** **P14-gate-rung-fix** — ⚠️ **RE-SCOPE REQUIRED.** P14 T1 edits
  `orchestrate.rs:17965` (the *dead* legacy engine). It must be redirected to
  `runner/gate_dispatch.rs` + `roko-gate/src/rung_dispatch.rs` to affect the live path. **Partly NEW.**

### M0.4 — Safety for unattended agents  **[BLOCKER for tool restriction; DEFERRED for worktree]**
- **Problem:** the CLI provider runs with `dangerously_skip_permissions: true`
  (`commands/plan.rs:503`) and there is **no `ToolDispatcher` / pre-post safety funnel** in the runner
  event loop — the Claude-CLI subprocess writes files with no roko-side authorization. Separately,
  there is **no `git worktree` isolation**: all plans mutate one shared working tree and "merge" is a
  `cargo check` on that tree (`runner/merge.rs:146-205`; trace 96 §10) → parallel runs corrupt each other.
- **Fix:** (a) pass a per-role `--disallowed-tools` deny-list into the CLI invocation so unattended
  agents can't run destructive tools even with skip-permissions; (b) [deferred] create one
  `git worktree` per plan in `MergeBranch` and make merge a real branch merge.
- **Verify (a):** `grep -q 'disallowed_tools' crates/roko-cli/src/dispatch_v2.rs`; run a plan and confirm
  a denied tool is rejected in `.roko/events.jsonl`.
- **Plan:** **P16-safety-contracts** (disallowed_tools wiring — the T1–T5 tasks target exactly
  `dispatch_v2.rs`). Worktree isolation = **NEW** (runner-v2 holdout, no plan yet).

### M0.5 — Provider/tool correctness  **[BLOCKER for non-Claude; TRUST otherwise]**
- **Problem:** (a) tool-alias bug — Claude PascalCase names (`Read,Write,Edit`) are not resolved to
  canonical snake_case on non-Claude providers, so `parse_allowed_tools_csv` **silently strips them**
  and the agent loses file-editing tools; (b) only ~16 of 37 builtin tools have live handlers, so the
  in-process **Bridge** provider path is thinner than the CLI path.
- **Evidence / fix:** call `roko_core::tool::aliases::canonical_of_claude` inside
  `parse_allowed_tools_csv` (P09 plan, `openai_compat.rs:252-260`; alias table `aliases.rs:38-118`).
- **Verify:** `grep -q 'canonical_of_claude' crates/roko-agent/src/provider/openai_compat.rs`; unit test
  `parse_allowed_tools_csv(Some("Read,Write,Edit"))` → `[read_file, write_file, edit_file]`.
- **Plan:** **P09-tool-alias-fix** (T1–T3). Builtin-handler coverage = **NEW**.
- **MVP note:** the default provider is Claude-CLI, which uses native names, so this is *not* a hard
  blocker for a Claude-only MVP — but it is the moment you route any task to a non-Claude model.

### M0.6 — Resume reliability  **[DEFERRED]**
- **Problem:** the CLI `--resume` selection path is entangled with the Graph engine; once M0.1 lands,
  resume must default to the Runner v2 snapshot. The runner's *internal* resume already works
  (`resume::prepare_resume_with_force`, fingerprint drift re-queue, `runner/resume.rs:136-233`;
  trace 96 §14) — the gap is the engine-selection glue, not the mechanism.
- **Fix:** ensure `plan run … --resume` routes through `event_loop::run` (falls out of M0.1); keep the
  auto-resume seeding at `plan.rs:361-376`.
- **Verify:** interrupt a multi-task run, `plan run plans/<x> --resume .roko/state/executor.json`,
  confirm completed tasks are not re-executed.
- **Plan:** rides on **P11**; residual glue = small NEW follow-up.

### M0.7 — Plan validation & schema  **[TRUST] — CONFIRMED WORKING**
- **Status:** ✅ `roko plan validate` exists (`commands/plan.rs:1274 cmd_plan_validate`) and
  `plan run` calls `validate_before_run` **mandatorily** before executing (`plan.rs:248,1229`), so a
  malformed `tasks.toml` aborts the run rather than silently no-op'ing. Keep as a pre-flight guard.
- **Verify:** `cargo run -p roko-cli -- plan validate plans/e2e-smoke` exits non-zero on a broken task.
- **Plan:** none needed.

### M0.8 — Real DAG / intra-plan parallelism  **[DEFERRED]**
- **Problem:** Runner v2 runs **one agent per plan**, parallelism only across ≤4 plans
  (`max_concurrent_plans = 4` hardcoded, `roko-core/src/defaults.rs:313`); `max_concurrent_tasks` only
  sizes the gate semaphore, never throttles agents; `runner/task_dag.rs::TaskDag` is imported for a
  single helper and otherwise **dead** (trace 96 §4-5). A 20-task plan executes essentially serially.
- **Why deferred:** serial execution is *correct*, just slow. Not required for MVP.
- **Fix:** honor `meta.max_parallel` and wire `TaskDag` into `tick`/dispatch.
- **Plan:** **P12-runner-parallelism** (T1 reads `meta.max_parallel`). DAG wiring = larger NEW.

### M0.9 — Gate-failure replan + observability  **[DEFERRED / TRUST]**
- **Problem A (replan):** gate-failure "replan" is **prompt-only** — `set_replan_context` appends text
  to the next prompt; there is **no `tasks.toml` rewrite** (`event_loop.rs:1549-1590`; trace 96 §9).
  Retry still works, so runs make progress; true re-planning is deferred.
- **Problem B (observability):** gate verdicts are written to **`.roko/signals.jsonl`** as a hand-rolled
  `{"kind":"GateVerdict",…}` blob (`event_loop.rs:1147-1168`), but the canonical / dashboard-facing log
  is **`.roko/engrams.jsonl`** (`roko-fs/src/layout.rs:202-205` vs legacy `signals_path` `:219`). An
  operator watching the canonical log or TUI sees an incomplete picture. Episodes land correctly
  (`.roko/episodes.jsonl` via `FeedbackFacade`).
- **Fix:** emit gate verdicts as real `Engram`s to `engrams_path()` (or point the dashboard reader at
  `signals.jsonl`). Port `build_gate_failure_plan_revision` for true replan.
- **Plan:** replan crash-classification = **P15-error-recovery-wiring** (partial: wires
  `classify_agent_crash`, not tasks.toml rewrite); tasks.toml rewrite + engram unification = **NEW**.

---

## 2. Dependency diagram

```
                       ┌────────────────────────────────────────────┐
                       │  M0.1  Engine default → Runner v2  (P11)    │  ◀── THE unblocker
                       │  (until this, nothing below even executes)  │
                       └───────────────┬────────────────────────────┘
                                       │ enables real dispatch
         ┌─────────────────────────────┼──────────────────────────────┐
         ▼                             ▼                              ▼
 ┌───────────────┐          ┌────────────────────┐          ┌──────────────────┐
 │ M0.2 verify   │          │ M0.3 honest gates  │          │ M0.4a disallowed │
 │ runner ✅     │          │ no stub-pass (P14* │          │ tools    (P16)   │
 │ (confirmed)   │          │ RE-SCOPE to live)  │          │ safety funnel    │
 └───────────────┘          └─────────┬──────────┘          └──────────────────┘
         │                            │  honest pass/fail            │
         └──────────────┬─────────────┘                              │
                        ▼                                            ▼
             ╔═══════════════════════════════╗            ┌────────────────────┐
             ║  MINIMUM VIABLE SELF-HOSTING  ║            │ M0.5 tool aliases  │
             ║  = M0.1 + M0.2 + M0.3         ║            │ (P09, non-Claude)  │
             ╚═══════════════┬═══════════════╝            └────────────────────┘
                             │ reliable + honest single-/few-plan runs
        ─────────────────────┼──────────────────── (post-M0, not blocking) ───────
         ▼            ▼              ▼                 ▼                  ▼
   M0.6 resume   M0.7 validate  M0.8 real DAG /   M0.9a replan     M0.9b engram
   glue (P11)     ✅ works      parallelism(P12)  rewrite(P15+)    unification(NEW)
```

---

## 3. Minimum Viable Self-Hosting — definition

> The smallest set of fixes after which
> `roko plan run plans/<x> --engine runner-v2` **reliably executes a real plan and reports
> honest pass/fail**:

| # | Item | Why it's in the minimum set |
|---|------|-----------------------------|
| **M0.1** | Engine default → Runner v2 | Without it, bare `roko plan run` does nothing. (If the agent *always* passes `--engine runner-v2`, this is the only item it can skip — but no unattended agent should depend on remembering a flag.) |
| **M0.2** | Per-task verify runner | Already works — the mechanism that makes acceptance real. Ship a regression guard. |
| **M0.3** | Honest gates (no stub-pass) | Without it, "pass" on integrative+ tasks is fiction; acceptance can't be trusted. |

**Everything else is post-M0:** M0.4 (safety/worktree) and M0.8 (parallelism) matter the moment you
run **parallel** plans or route to **non-Claude** models (M0.5); M0.6/M0.7 are glue/confirmed; M0.9 is
richer recovery + dashboards. A single Claude-CLI plan run, executed serially, is *correct and honest*
after M0.1 + M0.3.

---

## 4. M0 → existing-plan mapping

| M0 | Existing plan | Status of that plan | Gap / action |
|----|---------------|---------------------|--------------|
| M0.1 | **P11-runner-v2-default** | T1 landed (`Cargo.toml:16`); T2+ pending | finish clap default + `main.rs:2699` hardcode |
| M0.2 | — | live path works (`gate_dispatch.rs:121`) | add regression test only |
| M0.3 | **P14-gate-rung-fix** | ⚠️ targets DEAD `orchestrate.rs` | **RE-SCOPE** to `runner/gate_dispatch.rs` + `rung_dispatch.rs`; partly NEW |
| M0.4a | **P16-safety-contracts** | ready (`dispatch_v2.rs` deny-list) | execute as written |
| M0.4b | — | no plan | **NEW** — per-plan `git worktree` isolation in `runner/merge.rs` |
| M0.5 | **P09-tool-alias-fix** | ready | execute; builtin-handler coverage = **NEW** |
| M0.6 | rides on **P11** | mechanism works (`resume.rs`) | small NEW glue after M0.1 |
| M0.7 | — | `cmd_plan_validate` works + mandatory pre-flight | none |
| M0.8 | **P12-runner-parallelism** | T1 ready; DAG wiring larger | execute T1; `TaskDag` wiring = **NEW** |
| M0.9a | **P15-error-recovery-wiring** | partial (crash classify only) | tasks.toml rewrite = **NEW** |
| M0.9b | — | no plan | **NEW** — write gate verdicts as Engrams to `engrams.jsonl` |

---

## 5. One-command readiness smoke (run after M0.1 + M0.3)

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-cli
cargo run -p roko-cli -- plan validate plans/e2e-smoke              # M0.7
cargo run -p roko-cli -- plan run plans/e2e-smoke --engine runner-v2 --fresh
git status --porcelain                                             # M0.1: real edits present?
grep -c '"kind":"GateVerdict"' .roko/signals.jsonl                 # M0.3: verdicts recorded
tail -5 .roko/state/run-ledger.jsonl                               # honest task pass/fail
```
If `git status` is empty after a successful run, M0.1 is not actually done (still Graph dry-run).
