# Perf Runner

**Purpose:** drive the 18 performance optimization plans in
`tmp/solutions/perf/implementation/` to completion using the standard
parallel-template machinery (codex, worktrees, cherry-pick, configurable
concurrency).

**Source of truth:** the 18 plan files under
`tmp/solutions/perf/implementation/`. Every prompt under `prompts/` is a
mechanical translation of one plan into a self-contained codex prompt.
Each prompt links back to its source plan and to its row in
`ISSUE-TRACKER.md`.

**Issue tracker:** [`ISSUE-TRACKER.md`](ISSUE-TRACKER.md) — every unfixed
batch + sub-item as a `[ ]` checkbox grouped by phase. Update when an
item lands. Each prompt links back to its tracker row.

**Out of scope for this runner:**

- Plan 15 Feature B (`--batch-async` via OpenAI/Anthropic Batch APIs).
  High-risk, separate state machine. File a follow-up runner if needed.
- Macro-benchmark execution (the runner only writes code; benchmarks
  run from `BENCHMARK-RESULTS.md` §11.1 after merges land).

---

## Layout

```
tmp/runners/perf/
├── README.md             ← this file
├── ISSUE-TRACKER.md      ← master checklist (21 batches + ~120 sub-items)
├── batches.toml          ← DAG: 21 batches with deps, scope, also_read
├── run.sh                ← thin wrapper around parallel-template
├── context-pack/
│   ├── 00-RULES.md           ← global do/don't (no compile-during-batch)
│   ├── 01-FILE-INVENTORY.md  ← crate map + edit-site cheat-sheet
│   ├── 02-ANTI-PATTERNS.md   ← cache, async, perf-specific footguns
│   ├── 03-PERF-CONTRACTS.md  ← measurable invariants per batch
│   └── 04-VERIFY-RECIPES.md  ← copy-paste verification commands
├── prompts/
│   ├── PERF_01.prompt.md
│   ├── PERF_02.prompt.md
│   …
│   └── PERF_21.prompt.md
└── logs/                 ← created at runtime
```

---

## Batches at a glance

The 21 batches are grouped by phase. Phases run roughly sequentially
because later phases assume primitives shipped by earlier ones, but
within a phase batches run in parallel.

| Phase | Group | Batches | Theme | Wave |
|---|---|---|---|---|
| 0 — Low-hanging fruit | `P0` | PERF_01..PERF_05 | config / learning / contract / event-log / substrate | 1 |
| 1 — Prompt assembly cache | `P1` | PERF_06 | per-workdir convention LRU | 1 |
| 2 — Routing + warm pool | `P2` | PERF_07..PERF_11 | routing memo, parallel enrichment, warm dispatch pool (3-step build-out) | 1-3 |
| 3 — Gate pipeline | `P3` | PERF_12..PERF_15 | express gates, source-hash skip, parallel rungs, git diff cache | 1 |
| 4 — Advanced | `P4` | PERF_16..PERF_18 | speculative reviewer, parallel plan dispatch, PGO build | 2-3 |
| External — Eval | `EX` | PERF_19..PERF_21 | HAL wrapper, bench K-trial, bench compare | 1-2 |

**Full ID → plan map:**

| Batch | Plan | Title |
|---|---|---|
| `PERF_01` | 01-shared-config-cache | Shared config cache (B02) |
| `PERF_02` | 02-learning-runtime-single-open | LearningRuntime single-open (B03) |
| `PERF_03` | 03-contract-cache-audit | Contract cache audit (B05) |
| `PERF_04` | 04-buffered-jsonl-logger | Buffered JSONL event logger (B11+B13) |
| `PERF_05` | 05-batch-substrate-writes | Adopt FileSubstrate::put_batch everywhere (B10) |
| `PERF_06` | 06-prompt-assembly-cache | PromptAssemblyService convention cache (B12+B14) |
| `PERF_07` | 07-routing-cache | Routing decision cache (B06) |
| `PERF_08` | 08-parallel-enrichment | Parallel per-dispatch enrichment (B07) |
| `PERF_09` | 09-warm-dispatch-pool §1-2 | WarmDispatchPool module (B15 part 1) |
| `PERF_10` | 09-warm-dispatch-pool §3-4 | Wire pool into EffectDriver + run.rs (B15 part 2; deps PERF_09) |
| `PERF_11` | 09-warm-dispatch-pool §5-7 | Config schema + serve startup + metrics (B15 part 3; deps PERF_10) |
| `PERF_12` | 10-express-gate-mode | Express gate mode + auto-detect (B08-a) |
| `PERF_13` | 11-source-hash-gate-skip | Source-hash gate skip (B08-b) |
| `PERF_14` | 12-parallel-gate-rungs | Parallel gate rungs (B08-c) |
| `PERF_15` | 13-git-diff-cache | Git diff cache for gate phase (B09) |
| `PERF_16` | 14-speculative-execution | Speculative reviewer pre-warm (deps PERF_11) |
| `PERF_17` | 15-batch-inference §A | Plan executor parallel dispatch (Feature A only) |
| `PERF_18` | 16-pgo-build | PGO release build pipeline |
| `PERF_19` | 17-hal-integration | HAL agent wrapper + nightly CI |
| `PERF_20` | 18-bench-suite-extension §1-3 | Bench K-trial + cost wiring |
| `PERF_21` | 18-bench-suite-extension §4-5 | Bench compare subcommand + result layout (deps PERF_20) |

---

## Dependency DAG

```text
Wave 1 (15 parallel — most of Phase 0/1/2/3 + scaffolding):
  PERF_01, PERF_02, PERF_03, PERF_04, PERF_05,
  PERF_06,
  PERF_07, PERF_08, PERF_09,
  PERF_12, PERF_13, PERF_14, PERF_15,
  PERF_17, PERF_18, PERF_19, PERF_20

Wave 2 (3, gated on Wave 1):
  PERF_10  ← deps PERF_09
  PERF_21  ← deps PERF_20

Wave 3 (2):
  PERF_11  ← deps PERF_10
  (PERF_16 still blocked)

Wave 4 (1):
  PERF_16  ← deps PERF_11
```

---

## Naming convention

`PERF_NN` where NN is the zero-padded batch index. The numeric ordering
matches the recommended landing order in
`tmp/solutions/perf/implementation/00-INDEX.md`.

---

## Each prompt's structure

Every generated prompt is a single self-contained Markdown file with
this exact skeleton (matches `parallel-template`'s expected format):

```
# PERF_NN: <title>

## Task
<one-sentence summary>

## Tracker & sources
- Issue tracker row: [ISSUE-TRACKER.md#perf_nn](../ISSUE-TRACKER.md#perf_nn)
- Plan: tmp/solutions/perf/implementation/NN-<name>.md
- Bottleneck: B0X (from BOTTLENECK-ANALYSIS.md)
- Priority / effort / depends-on

## Problem
<3-6 sentences, no fluff>

## Exact Changes
<file paths, line anchors, code snippets, step-by-step>

## Write Scope
<bullet list — must match batches.toml `scope`>

## Read-Only Context
<bullet list — must match batches.toml `also_read`>

## Acceptance Criteria
<checklist; mirrors the plan's "Status check">

## Verify
<commands; SAME RECIPE every prompt; pulled from context-pack/04>

## Do NOT
<negatives, including the runner-global "do not run cargo" rule>

## Tracker update
On success, include this trailer in the commit message:
  tracker: PERF_NN done <commit-sha>
```

---

## Workflow

```bash
# 1. List all batches.
bash tmp/runners/perf/run.sh --list

# 2. Show the wave schedule without executing.
bash tmp/runners/perf/run.sh --dry-run

# 3. Execute Wave 1 (15-way parallel).
bash tmp/runners/perf/run.sh --parallel 15

# 4. Execute a single phase.
bash tmp/runners/perf/run.sh --group P0
bash tmp/runners/perf/run.sh --group P3 --parallel 4

# 5. Run a specific subset (handy for re-runs).
bash tmp/runners/perf/run.sh --only PERF_09,PERF_10,PERF_11

# 6. Watch live status.
bash tmp/runners/perf/run.sh --watch

# 7. Resume after interrupt.
bash tmp/runners/perf/run.sh --continue
```

---

## Updating the tracker when a batch lands

**Manual:** edit `ISSUE-TRACKER.md`, change `[ ]` to `[x]` on the row
matching the batch ID, append the commit hash in the trailing comment.

**Automatic:** agents finishing a batch include this trailer in their
commit message:

```text
tracker: PERF_03 done 9f1c8a2
```

A `bin/sync-tracker.sh` script can later be added to grep commit
trailers and rewrite the tracker; for now the manual flow is enough.

---

## Relationship to other artefacts

| Artefact | Purpose | Relationship |
|---|---|---|
| `tmp/solutions/perf/implementation/*.md` | Detailed plans (200-440 LOC each) | **Source.** Each prompt is a mechanical extract. |
| `tmp/solutions/perf/BENCHMARK-RESULTS.md` | Baseline measurements | Numbers fed into perf-contract assertions. |
| `tmp/solutions/perf/BOTTLENECK-ANALYSIS.md` | B01-B15 catalogue | Cross-referenced from prompts. |
| `tmp/solutions/perf/WARM-POOL-DESIGN.md` | Warm pool architecture | Required reading for PERF_09..11. |
| `tmp/runners/parallel-template/` | The DAG runner | Consumed via `run.sh`. |
| `tmp/runners/post-parity/` | Sister runner with ~330 batches | Pattern reference; some batches overlap (PA = HTTP client = already shipped). |

---

## Post-merge measurement

After each phase lands on `main`, run the macro-benchmark from
`BENCHMARK-RESULTS.md` §11.1 and record results in
`.roko/bench/perf/<date>/`. Compare to baseline; phase rolls back if
regression > 20 % on any tracked metric.
