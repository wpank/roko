# Solutions Runner

**Purpose**: drive the 735-task backlog enumerated in
`tmp/solutions/roko/tasks/01-..19-..` (created 2026-04-29) to completion.

**Source of truth**: the 19 task files under `tmp/solutions/roko/tasks/`.
Every prompt under `prompts/` is generated mechanically from those files
by `bin/generate-prompts.py`. **Do not hand-edit prompts** — re-run the
generator if the source changes.

**Runner format**: same `parallel-template` machinery used by `mega-parity`
and `post-parity` — codex, worktrees, cherry-pick, configurable concurrency.

**Issue tracker**: [`ISSUE-TRACKER.md`](ISSUE-TRACKER.md) — every unfixed
task as a `[ ]` checkbox grouped by source file. Update this when an item
lands. Each prompt links back to its tracker row.

---

## Layout

```
solutions/
  README.md              <- this file
  ISSUE-TRACKER.md       <- master checklist (735 items)
  STATUS.md              <- per-phase progress snapshot, regenerated
  run.sh                 <- delegates to ../parallel-template/run-parallel.sh
  batches.toml           <- generated; one [[batch]] per task with deps
  context-pack/
    00-RULES.md          <- global anti-patterns and pre-commit rules
    01-PHASE-MAP.md      <- which prefix maps to which phase/track
    02-FILE-INVENTORY.md <- crate map, runner v2 vs orchestrate.rs, dispatch paths
    03-VERIFICATION.md   <- standard verify recipes (cargo, ripgrep)
  bin/
    generate-prompts.py  <- source -> prompts/ + batches.toml
    sync-tracker.py      <- regenerates STATUS.md from ISSUE-TRACKER.md state
    verify-prompts.sh    <- structural lint over generated prompts
  prompts/               <- generated; ~735 *.prompt.md files
  logs/                  <- run-YYYYMMDD-HHMMSS/ subdirs
```

---

## Naming convention

Each batch ID is `<PREFIX>_<NN>` where the prefix is derived from the
source phase/file:

| Prefix | Source file | Phase | Count |
|---|---|---|---|
| `STAB` | 01-STABILITY-AND-FIXES | Phase 0 | 78 |
| `ORCH` | 02-ORCHESTRATION | Phase 1 | 28 |
| `DISP` | 03-INFERENCE-DISPATCH | Phase 1 | 38 |
| `GATE` | 04-GATE-PIPELINE | Phase 1 | 27 |
| `EVAL` | 05-GATE-EVOLUTION | Phase 2/3 | 48 |
| `PROM` | 06-PROMPT-ASSEMBLY | Phase 2 | 33 |
| `LERN` | 07-LEARNING-FEEDBACK | Phase 0/3 | 27 |
| `UX__` | 08-UX-CLI | Phase 0/2 | 47 |
| `ACPM` | 09-ACP-MCP | Phase 4 | 40 |
| `PERF` | 10-PERFORMANCE | Phase 3 | 45 |
| `INNO` | 11-INNOVATIONS | Phase 3 | 65 |
| `DEBT` | 12-CODE-DEBT | Phase 1 | 37 |
| `GTM_` | 13-GTM-AND-INTEGRATIONS | Phase 4 | 43 |
| `RNNR` | 14-RUNNER-PATTERNS | Phase 2/3 | 30 |
| `TEST` | 15-TESTING-VERIFICATION | Phase 4 | 38 |
| `CONF` | 16-CONFIG-AND-WIRING | Phase 0/1 | 26 |
| `SAFE` | 17-SAFETY-SECURITY | Phase 4 | 21 |
| `OBS_` | 18-OBSERVABILITY | Phase 3 | 33 |
| `XCUT` | 19-CROSS-CUTTING | Phase 1/2 | 31 |
| **Total** | | | **735** |

The numeric suffix matches the source `Task N.NN` ordering. So source task
`1.07` becomes `STAB_07`, source `4.21` becomes `GATE_21`, etc.

---

## Each prompt's structure

Every generated prompt is a single self-contained Markdown file with this
exact skeleton:

```
# <BATCH_ID>: <Title>

## Tracker
- Issue tracker row: [ISSUE-TRACKER.md#<batch-id>](../ISSUE-TRACKER.md#<batch-id>)
- Source: tmp/solutions/roko/tasks/<NN-FILE>.md, Task <N.NN>
- Priority: <P0|P1|P2|P3>
- Effort: <N hours>
- Depends on: <list or "none">

## Problem
<Verbatim "Context" section from source>

## Exact Changes
<Verbatim "Implementation Steps" section from source, with file paths and code snippets>

## Write Scope
<bullet list from "Files to Modify">

## Read-Only Context
<bullet list of "also_read" files inferred from steps>

## Verify
<verbatim "Verification Criteria" checklist + standard cargo recipe>

## Acceptance Criteria
- All verification checkboxes pass
- No items added to write scope
- No new files outside scope
- Pre-commit (fmt + clippy + test) green
- Tracker row marked [x] in commit message: `tracker: <BATCH_ID> done`

## Do NOT
<negative constraints, pulled from "Design Guidance" + global rules>
```

Every prompt links back to its tracker row by anchor, so when an agent
finishes a task it knows exactly which checkbox to tick.

---

## Workflow

```bash
# 1. (Re)generate prompts and batches.toml from source.
#    Preserves `[x]` / `[~]` / `<!-- ... -->` row state from the existing
#    ISSUE-TRACKER.md while refreshing titles, priorities, and deps from source.
python3 bin/generate-prompts.py

# 2. Sanity-check prompts.
bash bin/verify-prompts.sh

# 3. Sync tracker status (counts, percentages by phase).
python3 bin/sync-tracker.py

# 4. List batches without running.
./run.sh --list

# 5. Dry-run a phase (Phase 0 = STAB + LERN + CONF + UX__ tracker entries).
./run.sh --group STAB --dry-run

# 6. Execute Phase 0 with 20-way concurrency.
./run.sh --group STAB --parallel 20

# 7. Tail live progress.
./run.sh --watch
```

---

## Pre-seeding "already done" items

This runner was created on 2026-05-01, after several earlier runners
(`mega-parity`, `post-parity`) had already shipped some of the items the
`tmp/solutions/roko/` plan covers. Those commits use different ID
schemes (`R3_F04`, `PG_02`, etc.) so `sync-tracker.py` cannot find them.

To pre-seed those known-done rows once:

1. Edit `preseed.txt` — one `BATCH_ID  note` line per known-done item.
2. Dry-run: `python3 bin/preseed-tracker.py preseed.txt`
3. Apply:   `python3 bin/preseed-tracker.py preseed.txt --apply`

Going forward, every newly-landing batch must include the trailer
`tracker: <BATCH_ID> done <sha>` and `sync-tracker.py --apply` will
flip it automatically.

## Updating the tracker when a task lands

Two ways:

**A. Manual** — edit `ISSUE-TRACKER.md`, change `[ ]` to `[x]` on the
row matching the batch ID, add the commit hash in the trailing comment.

**B. Automatic** — agents finishing a batch include a trailer line in the
commit message:

```
tracker: STAB_07 done <commit-sha>
```

Then `python3 bin/sync-tracker.py --apply` will rewrite `ISSUE-TRACKER.md`
to mark every batch with such a trailer as `[x]`.

---

## Relationship to other runners

| Runner | Source | Notes |
|---|---|---|
| `mega-parity` | older audit | superseded; many items are now closed |
| `post-parity` | post-mega-parity wiring | partly running; only ~20 of 330 attempted |
| `solutions` | tmp/solutions/roko/ | this one — the 735-task plan |
| `tmp/subsystem-audits/05-01/41-consolidated-backlog.md` | newer T0–T5 | smaller, sharper, currently driving recent commits |

`solutions` is the **comprehensive** backlog. `41-consolidated-backlog.md`
is a curated 42-item subset focused on Tier 0/1 stop-bleeding work. The
two should be kept in sync via `ISSUE-TRACKER.md` — when a T0/T1 item
lands, mark the corresponding `STAB_*` / `CONF_*` row.
