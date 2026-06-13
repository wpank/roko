# Binary Issues Runner

**Purpose**: Close out the still-open items from `tmp/binary-issues/MASTER-INDEX.md` after the 2026-05-01 verification pass.
**Source of truth**: `ISSUE-TRACKER.md` (one row per batch).
**Format**: Same as `parallel-template` — codex, worktrees, cherry-pick, 16 concurrent.

## What this runner is (and isn't)

This is **not** a sweeping refactor. It's a focused mop-up of the ~56 issues from the original binary audit that:

1. Are still **OPEN or PARTIAL** in the default build (`cargo build -p roko-cli`, `default = []`).
2. Are **not** redundant with `post-parity` (which targets a wider rewrite of dispatch, chat, and gates).
3. Have a **mechanical, single-file or small-multi-file fix** with verifiable acceptance criteria.

Items inside `#[cfg(feature = "legacy-orchestrate")]` modules are excluded — they no longer affect the shipping binary, even if they remain in-tree.

## Layout

```
binary-issues/
├── BATCHES.md            # Human-readable batch table + DAG (mirrors batches.toml)
├── ISSUE-TRACKER.md      # Master checklist (every batch maps to one row)
├── README.md             # This file
├── run.sh                # Thin wrapper for parallel-template runner
├── batches.toml          # 56 batch definitions with deps + scope
├── context-pack/         # Injected into every prompt
│   ├── 00-RULES.md       # Anti-patterns and "do NOT" rules
│   ├── 01-ARCHITECTURE.md  # Default-build wiring map (chat, run, serve)
│   └── 02-VERIFICATION-NOTES.md  # The 2026-05-01 audit verification report
├── prompts/              # 56 prompt files, one per batch
│   ├── BI_01.prompt.md   # ... → BI_56.prompt.md
│   └── ...
└── logs/                 # Created at runtime
```

## Group summary

| Group | Prefix range | Focus | Count |
|-------|--------------|-------|-------|
| BI_SEC | BI_01..BI_07 | Security defaults that still leak | 7 |
| BI_PHN | BI_08..BI_16 | Phantom features (built but unwired) | 9 |
| BI_CMD | BI_17..BI_23 | Slash commands that confirm but don't act | 7 |
| BI_STR | BI_24..BI_25 | Streaming for `roko run` and `roko plan run` | 2 |
| BI_SUB | BI_26..BI_31 | Subprocess discipline (timeout, stderr, cancel) | 6 |
| BI_ERR | BI_32..BI_37 | Silent error swallowing | 6 |
| BI_HRD | BI_38..BI_45 | Hardcoded values → config | 8 |
| BI_COD | BI_46..BI_49 | Code dedup and structural cleanup | 4 |
| BI_MTX | BI_50..BI_53 | Mutex / unwrap / lint-suppression risk | 4 |
| BI_PRT | BI_54..BI_56 | Complete the partial fixes | 3 |
| **Total** | | | **56** |

## Wave schedule

```
Wave 1 (16 parallel — independent):
  BI_01..BI_07         (Security)
  BI_32..BI_37         (Silent errors)
  BI_50..BI_53         (Mutex/unwrap)
  BI_38..BI_45 except BI_43  (Hardcoded — most)
  BI_26, BI_27, BI_29, BI_31 (Subprocess — leaf fixes)

Wave 2 (16 parallel — needs wave 1 in some cases):
  BI_17, BI_18, BI_22  (Slash commands — config-write paths)
  BI_28, BI_30         (Cancellation, non-blocking serve)
  BI_43                (CostTable from config — depends on BI_42)
  BI_46..BI_49         (Code dedup)
  BI_54, BI_55, BI_56  (Partial completion)
  BI_08, BI_10, BI_12, BI_13  (Phantom features — single-file)
  BI_15                (Share endpoint mismatch)

Wave 3 (8 — needs streaming + WorkflowEngine surface):
  BI_24, BI_25         (Streaming)
  BI_19, BI_20         (Slash /run, /plan run inline executors)
  BI_21, BI_23         (/prd idea, tune gates)
  BI_09, BI_11, BI_14, BI_16  (Phantom: dreams, VCG, share, knowledge)
```

## Running

```bash
# Full run (16 concurrent)
./run.sh

# Resume interrupted run
./run.sh --continue

# One group at a time
./run.sh --group BI_SEC
./run.sh --only BI_01,BI_04,BI_06

# DAG dry-run
./run.sh --dry-run

# Cherry-pick to your working branch as batches finish
./lib/auto-pick.sh --interval 90 --target-branch wp-arch2

# Status
./run.sh --status
./run.sh --watch
```

## After every successful batch

1. Flip the corresponding `[ ]` → `[x]` in `ISSUE-TRACKER.md`.
2. The batch's commit message must reference its `BI_NN` ID and the source `MASTER-INDEX` `S` ID.
3. If a batch reveals that the underlying issue was already fixed, mark the row `[x]` with a one-line note and close the batch with `success_noop`.

## Cross-references

- Audit source: `tmp/binary-issues/MASTER-INDEX.md`
- Verification report: `tmp/runners/binary-issues/context-pack/02-VERIFICATION-NOTES.md`
- Related runners (do not re-litigate work in flight):
  - `post-parity` — broader chat/dispatch rebuild (PA, PB, PC, PD overlap with S1, S2, S3, S6)
  - `mega-parity` — already-landed parity work
  - `converge-followup` — runtime/contract repair
