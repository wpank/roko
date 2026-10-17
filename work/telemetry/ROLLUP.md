# Development record: rollup

Not for workers: no view, skill prompt or `work.py next` reads this, and no metric here is a target.

DEFINITIONS.md sha256 `ee0525db5ff09afe9f0b1509cb0d22162b795f2beae153cf859a98b999f86720` · report date 2026-10-17 · HEAD `31e0c9255` · prices `prices-2026-09-28`. Every figure is observational; costs are API-equivalent (subscription). Percentiles are by nearest rank; times are UTC.

Inputs: 1344 live events, 705 backfill events (tabled apart), 0 invalid event rows skipped, 0 harvested call rows.

## All merged items

| Metric | Value | Coverage |
|---|---|---|
| Merged items | 102 | |
| Attempts (1 / 2 / 3+; mean) | 68 / 0 / 0; 1 | 68 of 102 merged items (67%) |
| First-try merges | 52/57 (91%) | 57 of 102 merged items (56%) |
| Claim-to-merge hours (median / p90) | 4.97 / 7.83 | 68 of 102 merged items (67%) |
| Conflict rate (abandoned over a conflict: 0) | 0/69 (0%) | 69 of 102 merged items (68%) |
| Post-merge verify failures | 2/58 (3%) | 58 of 102 merged items (57%) |
| Escapes (53 pending) | 0/49 (0%) | 49 of 102 merged items (48%) |
| Interventions per item (items with one) | 0.05 (3/56 (5%)) | 56 of 102 merged items (55%) |
| Unassisted merge share | 53/56 (95%) | 56 of 102 merged items (55%) |
| Cost per merged item, USD (median / p90; total) | – / –; 0 | 0 of 102 merged items (0%) |
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
| 2026-10-17 | 22 | 19/20 (95%) | 4.97 | – | 20 of 22 merged items (91%) |


## By executor

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| claude-agent | 67 | 52/57 (91%) | 4.98 | – | 57 of 67 merged items (85%) |
| claude-session | 8 | 0/0 | – | – | 0 of 8 merged items (0%) |
| unknown | 27 | 0/0 | 3.69 | – | 0 of 27 merged items (0%) |


## By kind × size

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bug × ? | 3 | 0/0 | 3.66 | – | 0 of 3 merged items (0%) |
| bug × M | 10 | 5/6 (83%) | 4.97 | – | 6 of 10 merged items (60%) |
| bug × S | 27 | 12/13 (92%) | 7.82 | – | 13 of 27 merged items (48%) |
| decision × S | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| finding × ? | 2 | 0/0 | 3.69 | – | 0 of 2 merged items (0%) |
| gap × ? | 3 | 0/0 | – | – | 0 of 3 merged items (0%) |
| gap × L | 8 | 4/7 (57%) | 3.73 | – | 7 of 8 merged items (88%) |
| gap × M | 18 | 10/10 (100%) | 4.97 | – | 10 of 18 merged items (56%) |
| gap × S | 26 | 20/20 (100%) | 6.80 | – | 20 of 26 merged items (77%) |
| question × ? | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| question × L | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| regression × S | 1 | 1/1 (100%) | 6.64 | – | 1 of 1 merged items (100%) |
| spec × S | 1 | 0/0 | 7.82 | – | 0 of 1 merged items (0%) |


## By lane

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bench | 14 | 9/11 (82%) | 4.97 | – | 11 of 14 merged items (79%) |
| docs | 6 | 5/5 (100%) | 7.82 | – | 5 of 6 merged items (83%) |
| frontend | 2 | 2/2 (100%) | 6.40 | – | 2 of 2 merged items (100%) |
| none | 19 | 4/5 (80%) | 4.96 | – | 5 of 19 merged items (26%) |
| paper | 9 | 4/5 (80%) | 7.82 | – | 5 of 9 merged items (56%) |
| rust-cold | 24 | 14/14 (100%) | 7.82 | – | 14 of 24 merged items (58%) |
| rust-hot | 25 | 11/12 (92%) | 4.97 | – | 12 of 25 merged items (48%) |
| tracker | 3 | 3/3 (100%) | 7.82 | – | 3 of 3 merged items (100%) |


## Coverage

Items closed done without a merge (not counted): 1009 (bug-009c0e, bug-012303, bug-017c2d, bug-019f02, bug-02d5ad, bug-02e264, bug-0320da, bug-045773, bug-0522e8, bug-053644, bug-056b40, bug-05a434, bug-05d1ac, bug-06bd18, bug-06e2d1, bug-07bc75, bug-08d912, bug-09690f, bug-09e21e, bug-09fac4, bug-0a1729, bug-0a934f, bug-0b668a, bug-0b7695, bug-0b7c81, bug-0b997a, bug-0bc2b4, bug-0bc728, bug-0c0747, bug-0c7e11, bug-0c7f25, bug-0d9ac4, bug-0eb8e2, bug-0f8948, bug-0fe64a, bug-103bce, bug-109b5a, bug-11556f, bug-11901c, bug-12153c, …).

Items merged outside the skills (no claim event): gap-8d2c79, gap-d1d92c, gap-96f7ed, gap-60654d, bug-31438d, bug-35379d, gap-8cb382, bug-62e3f4, bug-0ba3d9, bug-633b68, gap-0f3980, bug-779ae7, bug-ba8d42, gap-751ac9, bug-dfb28f, bug-8a78e1, bug-8417d9, bug-9bffe2, bug-2a2f63, bug-ea7723, bug-bb3262, bug-fa1537, gap-3aa9cb, gap-9eebcb, bug-35c901, q-1faa0c, dec-1089ec, gap-231407, q-46a488, bug-a804b1, bug-f090fc, find-2b34c8, gap-959a7c, gap-ac398f.

Items with branch commits that carry no `Executor:` trailer (trail incomplete): gap-8d2c79 (2), gap-d1d92c (2), bug-2a2f63 (1), bug-ea7723 (1), gap-3aa9cb (1), q-1faa0c (2), gap-0d0e81 (3), gap-08120e (1), gap-198c9c (1), gap-46fd19 (1), gap-5ebb4f (1), gap-625195 (1), gap-cb5133 (1), dec-1089ec (1), gap-064c40 (1), gap-ac2611 (1), gap-c06ff3 (1), gap-231407 (38), q-46a488 (38), bug-a804b1 (38), bug-f090fc (38), find-2b34c8 (38), gap-959a7c (38), gap-ac398f (38), gap-dbe8e7 (1), gap-f9fa2c (1).

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
