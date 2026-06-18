# productionizing runner

**Purpose**: Land the unfinished items from `tmp/productionizing/` (plans 10, 11, 12) on `wp-arch2`.
**Runner format**: parallel-template (codex, worktrees, cherry-pick, 12 concurrent).
**Issue tracker**: [`ISSUES.md`](./ISSUES.md). Single source of truth for what's open vs done.

## Summary

| Phase | Group | Batches | Source plan |
|---|---|---|---|
| Hardening (production blockers) | `H` | 13 (P04, P05, P07–P13, P15–P18) | `tmp/productionizing/10-IMPLEMENTATION-PLAN.md` |
| Frontier wiring | `F` | 6 (F01, F02, F03, F05, F06, F07) | `tmp/productionizing/11-FRONTIER-CAPABILITIES-PLAN.md` |
| Production economics | `D` | 9 (D01–D09) | `tmp/productionizing/12-PRODUCTION-DEPLOYMENT-PLAN.md` |
| **Total** | | **28** | |

Already-done items (P01, P02, P03, P06, P14, F04) are explicitly excluded with line-number citations in `ISSUES.md`.

## Execution DAG

```
Wave 1 (independent — 22 batches in parallel up to PARALLEL=12):
  H: P04 P05 P07 P08 P10 P11 P12 P13 P15 P16 P18
  F: F01 F02 F03 F06 F07
  D: D01 D02 D03 D04 D05 D07 D08

Wave 2 (deps satisfied — 6 batches):
  H: P09 (←P04)        P17 (←P16)
  F: F05 (←F01)
  D: D06 (←D01)        D09 (←D04)
```

## Running

```bash
# List + dry-run
bash tmp/runners/productionizing/run.sh --list
bash tmp/runners/productionizing/run.sh --dry-run

# Full run, 12 concurrent
bash tmp/runners/productionizing/run.sh

# Just the hardening group (CRITICAL items first)
bash tmp/runners/productionizing/run.sh --group H

# Specific batches
bash tmp/runners/productionizing/run.sh --only P05,P10,P08

# Resume after interrupt
bash tmp/runners/productionizing/run.sh --continue

# Cherry-pick to wp-arch2 as batches succeed
bash tmp/runners/productionizing/../parallel-template/lib/auto-pick.sh \
  --interval 90 --target-branch wp-arch2

# Status / disk / cleanup
bash tmp/runners/productionizing/run.sh --status
bash tmp/runners/productionizing/run.sh --disk
bash tmp/runners/productionizing/run.sh --cleanup
```

## After a green run

1. Open [`ISSUES.md`](./ISSUES.md). Re-grep the codebase for each unticked box (line-number citations are provided). Tick only what survives the grep.
2. Run the integration sniff in `tmp/productionizing/09-OPERATIONS-RUNBOOK.md` (health URLs, Railway shape).
3. Move stale items to the "Already done in tree" section with a fresh line-number citation.

## Conventions

- Group letter = wave gate scope. `H`, `F`, `D` each get their own `cargo check + clippy` after their batches complete.
- Batch IDs match the source plan IDs (P04, F01, D06…) — easy to bisect against the docs.
- `deps = [...]` enforces the DAG. Cross-group deps are fine (e.g. F05 ← F01).
- Every prompt:
  - Names exact files + line numbers
  - Shows BEFORE / AFTER code
  - Lists explicit "Do NOT" constraints
  - Ends with a `Verification` block (no compile, just grep / tree state)
- Context-pack injects `00-RULES.md`, `01-CRATE-MAP.md`, `02-ANTI-PATTERNS.md`, `03-EXISTING-CODE.md` into every prompt.
