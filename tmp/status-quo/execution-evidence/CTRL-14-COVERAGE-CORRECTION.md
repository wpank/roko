# CTRL-14 source-coverage and plan-index correction

## Assignment

- Base: `a4278ced0`
- Branch: `agent/CTRL-14-coverage-correction`
- Scope: canonical source-coverage ledger/plan, source-coverage roll-up, executable-plan index, and this evidence file
- Trigger: the CTRL-14 supersession proof found the newly canonical `MASTER-EXECUTION-CHECKLIST.md` absent from both coverage layers and found `plans/00-INDEX.md` frozen at the pre-expansion 149-task E01-E18 census.

## Correction

- Added `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md` to DOC-SQ-01's context, structural coverage command, and status-quo source ledger.
- Changed only the affected corpus totals: status-quo 108→109 and global 744→745; the six DOC plans and 71 DOC tasks are unchanged.
- Reconciled `plans/00-INDEX.md` against the canonical per-epic task manifests: E01-E18 now total 169, with the corrected E01/E08/E09/E14/E15/E17/E18 rows. This matches `06-EXECUTABLE-TASK-FILE-COVERAGE.md` and direct TOML counts.
- Preserved the July audit's historical 744-source statement as baseline history; current canonical roll-ups now carry the post-master 745-source truth.

## Verification contract

- Enumerate all direct `tmp/status-quo/*.md` plus recursive `docs/v1`, `docs/v2`, and `docs/v2-depth` Markdown sources and assert every path appears in a source ledger and DOC task manifest: expected `sources=745`, zero missing in both layers.
- Parse every E01-E18 manifest with `tomllib`, assert each meta total matches its task array, and assert the subtotal is 169 and the seven corrected index rows match the manifests.
- Parse `DOC-status-quo-corpus/tasks.toml`, assert DOC-SQ-01 names the master in `read_files`, and execute its structural coverage command.
- Strict-validate `DOC-status-quo-corpus` with the integrated prerequisite-aware validator from a disposable root so generated indexes do not touch source files.
- Run `git diff --check` and prove only the four canonical control files plus this evidence record changed.

## Status

Implementation verification passed:

- `sources=745`, `missing_from_ledgers=0`, `missing_from_doc_tasks=0`.
- Direct TOML census reports 169 E01-E18 tasks, with every meta total matching its task array and every index row matching its manifest.
- DOC-SQ-01's structural coverage command passes with the master included.
- Integrated `roko plan validate --strict .../DOC-status-quo-corpus` reports `0 diagnostics in 1 plan`.
- The validator's generated top-level `plans/INDEX.md` side effect was restored byte-for-byte to sealed SHA-256 `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`; no generated file is in scope.
- `git diff --check` passes; the candidate changes only the four assigned canonical control files and this evidence record.

Independent review and an integrated CTRL-14 rerun remain required before this correction or CTRL-14 is DONE.
