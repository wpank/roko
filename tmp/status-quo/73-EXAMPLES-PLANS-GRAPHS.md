# Examples, Plans, And Graph Assets

> **Current-control-plane notice (2026-07-14, CTRL-16):** The body below is the
> preserved deep-pass inventory at `5852c93c05`, not current queue inventory.
> Current generated truth is 30 executable top-level plans/144 tasks plus two
> superseded plans/66 excluded tasks. The tracked executable set includes the
> recovered 24-task `architecture-core-queue`, the three-task
> `architecture-defi-critical-path`, and the two-task `e2e-smoke`.
> `dry-run-flag`, `live-demo-phase1`, and `live-demo-phase2` were deleted in
> `7899494d`; none is a runnable current root, none appears in the index, and
> `e2e-smoke` is not an equivalent replacement for the synthetic live-demo
> tasks. Partial ancestor residue is not completion: only dry-run preview data
> structs and an unexported/untested greeting function survive; runtime dry-run
> wiring, greeting export/test, and all farewell work are absent. The old demo
> script's `--live` mode now fails closed rather than dispatching either absent
> root; its default mode is a deterministic no-network simulation. Use
> [`plans/INDEX.md`](../../plans/INDEX.md) for generated counts,
> [`plans/_meta/IMPLEMENTATION_ORDER.md`](../../plans/_meta/IMPLEMENTATION_ORDER.md)
> for current queue disposition, and the master checklist for dependency order.
>
> **Deep second pass (HEAD `5852c93c05`, 2026-07-08):** on-disk inventory verified. Graph loader (`crates/roko-graph/src/loader.rs:11`) REQUIRES a `[graph]` table (`RawGraphFile.graph` is non-`Option`) plus `[[nodes]]`; the default cell registry (`engine.rs:311`) registers only 14 cell types. Plan loader (`plan_loader.rs`) deserializes via `TasksFile` (`task_parser.rs:698`) which expects `[meta]` + `[[task]]` (serde `rename="task"`, singular). Results: **3 of 7 graph examples are stale-schema and fail to load** (`task-execution`, `conditional-branch`, `parallel-gates` — no `[graph]` table + unregistered cell types); **1 of 33 plan files is stale-schema** (`.roko/prd/plans/self-developing-workflow` uses `[[tasks]]` plural + `[task_groups]`). All 31 `plans/*` dirs and `cursor-composer-backend` use the canonical `[meta]`+`[[task]]` shape and parse cleanly.

This ledger covers runnable-looking assets that can be mistaken for proof.

## Inventory

| Asset class | Count | Current role |
|---|---:|---|
| `examples/graphs/*.toml` | 7 | Graph Engine examples and target-shape demos. |
| `plans/*/tasks.toml` | 31 files | 29 executable plans, 120 ready tasks, and 2 superseded plan files. |
| `demo/demo-resources` md/toml/sh/py files | 53 | Demo automation and scenario scripts. |
| `.roko/prd` files | 13+ | 6 ideas, 1 published PRD, 4 drafts, and 2 PRD plan dirs; evidence of workflow, not canonical roadmap. |
| Provider config examples | 8 | Provider setup references, not proof that each provider path is release-ready. |

## Graph Examples

Schema check run against `loader.rs:11` (`RawGraphFile` needs a `[graph]` table + `[[nodes]]`) and `default_registry()` (`engine.rs:311`, registered cells: `gate.compile`/`gate.test`/`gate.clippy` = real `ShellCell` cargo runs; `noop`/`score`/`compose`/`act` = `NoopCell`; `task-executor` = dry-run stub; cognitive stubs `signal-reader`/`relevance-scorer`/`system-prompt-builder`/`claude-agent`/`gate-pipeline`/`store-writer`/`event-publisher` = `PassthroughCell`). Verdict tags: **live** (parses + real cells), **stub** (parses, no-op/passthrough cells = topology-only), **stale-schema** (fails loader).

| File | `[graph]` table? | cell_types | Registered? | Verdict |
|---|---|---|---|---|
| `linear-gates.toml` | ✅ | `gate.compile/test/clippy` | ✅ all (ShellCell) | **live** — best proof; real `cargo check/test/clippy` |
| `single-gate.toml` | ✅ | `gate.compile` | ✅ (ShellCell) | **live** — minimal smoke proof |
| `cognitive-loop.toml` | ✅ | `signal-reader`, `relevance-scorer`, `system-prompt-builder`, `claude-agent`, `gate-pipeline`, `store-writer`, `event-publisher` | ✅ all (PassthroughCell stubs) | **stub** — parses/validates but every cell passes through; topology-only |
| `score-compose.toml` | ✅ | `score`, `compose`, `act` | ✅ all (NoopCell) | **stub** — file self-admits no-op cells |
| `task-execution.toml` | ❌ top-level `name=` | `compose`, `agent`, `gate` | ❌ `agent`/`gate` unregistered | **stale-schema** — fails loader (no `graph` table) + unknown cells |
| `conditional-branch.toml` | ❌ top-level `name=` | `compose`, `agent` | ❌ `agent` unregistered | **stale-schema** — fails loader + unknown cell |
| `parallel-gates.toml` | ❌ top-level `name=` | `compose`, `gate` | ❌ `gate` unregistered | **stale-schema** — fails loader + unknown cell |

**Tally: 2 live · 2 stub · 3 stale-schema.** The 3 stale files predate the `[graph]`-table loader schema and use bare `agent`/`gate` cell names that were never registered; `roko graph validate` on them errors on the missing `graph` field before it even reaches cell resolution.

## Proof-Worthy vs Demonstrative

| Asset | Status | Use |
|---|---|---|
| `demo/demo-resources/bin/roko-demo verify-local` | Proof-worthy local smoke target. | Best candidate for serve + dashboard API proof in a temp workspace. |
| `demo/demo-resources/benchmark-flow` | Proof-worthy if assertions stay deterministic. | Local benchmark/C-factor path without Docker or live LLM dependency. |
| `examples/graphs/single-gate.toml`, `linear-gates.toml` | Loader/schema and gate-cell proof candidates. | Promote only after real gate execution is asserted. |
| `examples/graphs/score-compose.toml`, `cognitive-loop.toml` | Topology-only proof. | Keep target-labeled until no-op/passthrough cells are replaced. |
| `examples/graphs/parallel-gates.toml`, `conditional-branch.toml`, `task-execution.toml` | Stale schema examples. | Fix or quarantine before including in automated proof. |
| `demo/demo-web`, `demo/demo-old` | Archive/static surfaces. | Do not use as live product proof. |

## Plan Queue Reality

`plans/_meta/IMPLEMENTATION_ORDER.md` is the current human-readable queue for the P08-P34 repair series. It says to run `P08-search-command-fix` through `P34-verification-sweep` in order, with `architecture-defi-critical-path`, `e2e-smoke`, and demo queues separate.

| Plan group | Status |
|---|---|
| `P08`-`P34` | Active self-development repair queue; should be reconciled with `24` and `67`. |
| `self-dev-ux`, `self-dev-extras` | Marked superseded by the queue; keep for archaeology until every unique task is promoted. |
| `architecture-defi-critical-path` | Separate architecture/chain queue; do not run before core chain prerequisites. |
| `.roko/prd/plans/*` | Generated PRD-derived plans; useful evidence of product workflows, not canonical roadmap by itself. |
| `.roko/prd/plans/self-developing-workflow/tasks.toml` | Historical PRD plan shape; uses `[[tasks]]` and lacks canonical task statuses. |
| `e2e-smoke` | Should become a proof gate after execution honesty is fixed. |

## Plan / PRD-plan file inventory (schema-validity, deep pass)

On-disk `find . -name tasks.toml` (excluding `.claude/worktrees/*` and `.roko/worktrees/*` agent copies) = **33 canonical files: 31 in `plans/`, 2 in `.roko/prd/plans/`**. Canonical schema = `[meta]` + `[[task]]` (singular). Schema check per file below; all `plans/*` parse. `task` counts are `[[task]]` block counts.

| File | Schema shape | Tasks | Valid vs `TasksFile`? | Role |
|---|---|---:|---|---|
| `plans/P08…P34` (27 dirs) | `[meta]` + `[[task]]` | 1–6 each | ✅ canonical | Active P08-P34 repair queue |
| `plans/self-dev-ux/tasks.toml` | `[meta]` + `[[task]]` | 55 | ✅ canonical | Superseded by P-queue; keep for archaeology |
| `plans/self-dev-extras/tasks.toml` | `[meta]` + `[[task]]` | 11 | ✅ canonical | Superseded; keep for archaeology |
| `plans/architecture-defi-critical-path/tasks.toml` | `[meta]` + `[[task]]` | 3 | ✅ canonical | Separate chain/arch queue |
| `plans/e2e-smoke/tasks.toml` | `[meta]` + `[[task]]` | 2 | ✅ canonical | Candidate proof gate |
| `.roko/prd/plans/cursor-composer-backend/tasks.toml` | `[meta]` + `[[task]]` | 6 | ✅ canonical | Generated PRD plan; live shape |
| `.roko/prd/plans/self-developing-workflow/tasks.toml` | `[meta]`(slug/title) + `[task_groups]` + `[[tasks]]` | 13 (plural) | ❌ **stale-schema** | Historical PRD plan; `[[tasks]]` plural deserializes to 0 tasks under `TasksFile` (expects `[[task]]`) |

**Tally: 32 of 33 schema-valid; 1 stale-schema** (`self-developing-workflow`, plural `[[tasks]]` + `[task_groups]` header not in `TaskMeta`/`TasksFile`). Note `cursor-composer-backend` uses `[meta].plan` while `self-developing-workflow` uses `[meta].slug`/`title` — a second divergence: `TaskMeta` keys drifted between the two generated PRD plans.

> Caveat: `roko plan validate` uses lenient loading (`load_plan_lenient`, plan_loader.rs:34) and `#[serde(default)]` on most `TaskMeta` fields, so a plural-`[[tasks]]` file loads as a **zero-task plan** rather than hard-erroring — it silently validates as empty. The stale schema is a *silent* failure, not a crash.

## Demo Resource Reality

`demo/demo-resources` contains scenario automation for agent workflows, chain coordination, benchmark flow, dashboard quickstart, research, PRD workflow, provider routing, full self-hosting, and smoke tests. Treat these as demo scripts, not CI proof, unless a workflow invokes them and asserts outcomes.

## Deep-pass roadmap (schema fixes)

1. **[P1]** Migrate the 3 stale graph examples (`task-execution`, `conditional-branch`, `parallel-gates`) to the `[graph]`-table + `[[nodes]]` schema OR quarantine them under `examples/graphs/_target/`. Their `agent`/`gate` cell names must map to registered types (`task-executor`, `gate.compile/test/clippy`) or be registered as stubs.
2. **[P1]** Regenerate or migrate `.roko/prd/plans/self-developing-workflow/tasks.toml` from `[[tasks]]`+`[task_groups]` to `[meta]`+`[[task]]`; it currently loads as a zero-task plan silently.
3. **[P2]** Make `roko plan validate` reject unknown top-level tables (`[[tasks]]`, `[task_groups]`) instead of lenient-defaulting to empty, so stale schema fails loud.
4. **[P2]** Reconcile `[meta].plan` vs `[meta].slug`/`title` divergence between the two PRD-plan generators.
5. **[P2]** Add a loader test asserting each `examples/graphs/*.toml` parses and every cell_type is in `default_registry()`.

## Checklist

- [ ] Add `roko graph validate examples/graphs/*.toml` to a proof tier.
- [ ] For every graph example, label cells as `live`, `stub`, `target`, or `unsupported` (done in this doc: 2 live / 2 stub / 3 stale-schema).
- [ ] Fix or quarantine `parallel-gates.toml`, `conditional-branch.toml`, and `task-execution.toml` (all fail the `[graph]`-table loader) before they count as examples.
- [ ] Add a Graph-vs-Runner artifact parity test before any graph example is advertised as execution proof.
- [ ] Reconcile the 29 executable plans / 120 ready tasks in `plans/_meta/IMPLEMENTATION_ORDER.md` with `24-OPEN-ISSUE-LEDGER.md` and `67-TMP-FEEDBACK-2-CROSSWALK.md`.
- [ ] Add generated plan index checks: every `tasks.toml` has status, dependencies, owners, and proof commands.
- [ ] Promote `demo/demo-resources/smoke-test.sh` and relevant scenario scripts into CI only after they are deterministic.
- [ ] Exclude generated `.DS_Store`, `dist`, `node_modules`, contract `out`, and stale demo artifacts from proof counts.
