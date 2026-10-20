# Development record: rollup

Not for workers: no view, skill prompt or `work.py next` reads this, and no metric here is a target.

DEFINITIONS.md sha256 `ee0525db5ff09afe9f0b1509cb0d22162b795f2beae153cf859a98b999f86720` · report date 2026-10-17 · HEAD `d20426419` · prices `prices-2026-09-28`. Every figure is observational; costs are API-equivalent (subscription). Percentiles are by nearest rank; times are UTC.

Inputs: 1461 live events, 705 backfill events (tabled apart), 0 invalid event rows skipped, 0 harvested call rows.

## All merged items

| Metric | Value | Coverage |
|---|---|---|
| Merged items | 138 | |
| Attempts (1 / 2 / 3+; mean) | 104 / 0 / 0; 1 | 104 of 138 merged items (75%) |
| First-try merges | 90/95 (95%) | 95 of 138 merged items (69%) |
| Claim-to-merge hours (median / p90) | 4.97 / 7.82 | 104 of 138 merged items (75%) |
| Conflict rate (abandoned over a conflict: 2) | 0/105 (0%) | 105 of 138 merged items (76%) |
| Post-merge verify failures | 2/96 (2%) | 96 of 138 merged items (70%) |
| Escapes (89 pending) | 0/49 (0%) | 49 of 138 merged items (36%) |
| Interventions per item (items with one) | 0.03 (3/89 (3%)) | 89 of 138 merged items (64%) |
| Unassisted merge share | 86/89 (97%) | 89 of 138 merged items (64%) |
| Cost per merged item, USD (median / p90; total) | – / –; 0 | 0 of 138 merged items (0%) |
| Orchestration overhead, USD (per merged item) | 0 (0.00) | 0 unjoined call rows |

## By day of first merge

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| 2026-09-29 | 7 | 0/0 | – | – | 0 of 7 merged items (0%) |
| 2026-09-30 | 7 | 0/0 | – | – | 0 of 7 merged items (0%) |
| 2026-10-01 | 18 | 0/0 | 3.67 | – | 0 of 18 merged items (0%) |
| 2026-10-02 | 17 | 6/9 (67%) | 3.73 | – | 9 of 17 merged items (53%) |
| 2026-10-04 | 1 | 0/0 | 0.56 | – | 0 of 1 merged items (0%) |
| 2026-10-15 | 1 | 1/1 (100%) | 6.64 | – | 1 of 1 merged items (100%) |
| 2026-10-16 | 29 | 26/27 (96%) | 7.82 | – | 27 of 29 merged items (93%) |
| 2026-10-17 | 58 | 57/58 (98%) | 4.96 | – | 58 of 58 merged items (100%) |


## By executor

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| claude-agent | 103 | 90/95 (95%) | 4.97 | – | 95 of 103 merged items (92%) |
| claude-session | 8 | 0/0 | – | – | 0 of 8 merged items (0%) |
| unknown | 27 | 0/0 | 3.69 | – | 0 of 27 merged items (0%) |


## By kind × size

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bug × ? | 3 | 0/0 | 3.66 | – | 0 of 3 merged items (0%) |
| bug × M | 17 | 12/13 (92%) | 3.79 | – | 13 of 17 merged items (76%) |
| bug × S | 45 | 30/31 (97%) | 4.97 | – | 31 of 45 merged items (69%) |
| decision × S | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| finding × ? | 2 | 0/0 | 3.69 | – | 0 of 2 merged items (0%) |
| gap × ? | 3 | 0/0 | – | – | 0 of 3 merged items (0%) |
| gap × L | 8 | 4/7 (57%) | 3.73 | – | 7 of 8 merged items (88%) |
| gap × M | 22 | 15/15 (100%) | 4.97 | – | 15 of 22 merged items (68%) |
| gap × S | 32 | 27/27 (100%) | 4.97 | – | 27 of 32 merged items (84%) |
| question × ? | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| question × L | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| regression × ? | 1 | 1/1 (100%) | 1.03 | – | 1 of 1 merged items (100%) |
| regression × S | 1 | 1/1 (100%) | 6.64 | – | 1 of 1 merged items (100%) |
| spec × S | 1 | 0/0 | 7.82 | – | 0 of 1 merged items (0%) |


## By lane

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bench | 32 | 27/29 (93%) | 3.73 | – | 29 of 32 merged items (91%) |
| docs | 6 | 5/5 (100%) | 7.82 | – | 5 of 6 merged items (83%) |
| frontend | 4 | 4/4 (100%) | 3.60 | – | 4 of 4 merged items (100%) |
| none | 23 | 9/10 (90%) | 4.01 | – | 10 of 23 merged items (43%) |
| paper | 10 | 5/6 (83%) | 6.40 | – | 6 of 10 merged items (60%) |
| rust-cold | 31 | 21/21 (100%) | 4.97 | – | 21 of 31 merged items (68%) |
| rust-hot | 29 | 16/17 (94%) | 4.97 | – | 17 of 29 merged items (59%) |
| tracker | 3 | 3/3 (100%) | 7.82 | – | 3 of 3 merged items (100%) |


## Coverage

Items closed done without a merge (not counted): 1038 (bug-009c0e, bug-012303, bug-017c2d, bug-019f02, bug-02d5ad, bug-02e264, bug-0320da, bug-045773, bug-0522e8, bug-053644, bug-056b40, bug-05a434, bug-05d1ac, bug-06bd18, bug-06e2d1, bug-07bc75, bug-08d912, bug-09690f, bug-09e21e, bug-09fac4, bug-0a1729, bug-0a7ec1, bug-0a934f, bug-0b668a, bug-0b7695, bug-0b7c81, bug-0b997a, bug-0bc2b4, bug-0bc728, bug-0c0747, bug-0c7e11, bug-0c7f25, bug-0d9ac4, bug-0eb8e2, bug-0f8948, bug-0fe64a, bug-103bce, bug-109b5a, bug-11556f, bug-11901c, …).

Items merged outside the skills (no claim event): gap-8d2c79, gap-d1d92c, gap-96f7ed, gap-60654d, bug-31438d, bug-35379d, gap-8cb382, bug-62e3f4, bug-0ba3d9, bug-633b68, gap-0f3980, bug-779ae7, bug-ba8d42, gap-751ac9, bug-dfb28f, bug-8a78e1, bug-8417d9, bug-9bffe2, bug-2a2f63, bug-ea7723, bug-bb3262, bug-fa1537, gap-3aa9cb, gap-9eebcb, bug-35c901, q-1faa0c, dec-1089ec, gap-231407, q-46a488, bug-a804b1, bug-f090fc, find-2b34c8, gap-959a7c, gap-ac398f.

Items with branch commits that carry no `Executor:` trailer (trail incomplete): gap-8d2c79 (2), gap-d1d92c (2), bug-2a2f63 (1), bug-ea7723 (1), gap-3aa9cb (1), q-1faa0c (2), gap-0d0e81 (3), gap-08120e (1), gap-198c9c (1), gap-46fd19 (1), gap-5ebb4f (1), gap-625195 (1), gap-cb5133 (1), dec-1089ec (1), gap-064c40 (1), gap-ac2611 (1), gap-c06ff3 (1), gap-231407 (38), q-46a488 (38), bug-a804b1 (38), bug-f090fc (38), find-2b34c8 (38), gap-959a7c (38), gap-ac398f (38), gap-dbe8e7 (1), gap-f9fa2c (1), bug-636085 (1), bug-0d0d37 (1), bug-425274 (1).

## Backfill (labelled, not pooled)

Reconstructed once from history (`tools/work_backfill.py`); never mixed into the tables above.

| Executor | Closed items |
|---|---|
| claude-agent | 242 |
| unknown | 172 |
| verification-only | 25 |
| roko-plan | 10 |
| claude-session | 8 |
| human | 6 |

Lane starts from the reflog: 242.

## The switch

The switch point is not fixed: the tracker item "Roko executes work items" is not filed yet.
