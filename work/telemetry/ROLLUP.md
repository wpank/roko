# Development record: rollup

Not for workers: no view, skill prompt or `work.py next` reads this, and no metric here is a target.

DEFINITIONS.md sha256 `ee0525db5ff09afe9f0b1509cb0d22162b795f2beae153cf859a98b999f86720` · report date 2026-10-28 · HEAD `027ff7be6` · prices `prices-2026-09-28`. Every figure is observational; costs are API-equivalent (subscription). Percentiles are by nearest rank; times are UTC.

Inputs: 1827 live events, 705 backfill events (tabled apart), 0 invalid event rows skipped, 0 harvested call rows.

## All merged items

| Metric | Value | Coverage |
|---|---|---|
| Merged items | 211 | |
| Attempts (1 / 2 / 3+; mean) | 164 / 3 / 0; 1.02 | 167 of 211 merged items (79%) |
| First-try merges | 141/150 (94%) | 150 of 211 merged items (71%) |
| Claim-to-merge hours (median / p90) | 4.97 / 12.55 | 167 of 211 merged items (79%) |
| Conflict rate (abandoned over a conflict: 2) | 1/171 (1%) | 171 of 211 merged items (81%) |
| Post-merge verify failures | 2/159 (1%) | 159 of 211 merged items (75%) |
| Escapes (161 pending) | 0/50 (0%) | 50 of 211 merged items (24%) |
| Interventions per item (items with one) | 0.04 (5/132 (4%)) | 132 of 211 merged items (63%) |
| Unassisted merge share | 127/132 (96%) | 132 of 211 merged items (63%) |
| Cost per merged item, USD (median / p90; total) | – / –; 0 | 0 of 211 merged items (0%) |
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
| 2026-10-19 | 11 | 8/10 (80%) | 15.91 | – | 10 of 11 merged items (91%) |
| 2026-10-20 | 2 | 2/2 (100%) | 27.53 | – | 2 of 2 merged items (100%) |
| 2026-10-21 | 8 | 7/8 (88%) | 3.96 | – | 8 of 8 merged items (100%) |
| 2026-10-26 | 24 | 14/14 (100%) | 5.66 | – | 14 of 24 merged items (58%) |
| 2026-10-27 | 25 | 19/19 (100%) | 3.02 | – | 19 of 25 merged items (76%) |
| 2026-10-28 | 3 | 1/2 (50%) | 33.47 | – | 2 of 3 merged items (67%) |


## By executor

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| claude-agent | 170 | 141/150 (94%) | 4.97 | – | 150 of 170 merged items (88%) |
| claude-session | 8 | 0/0 | – | – | 0 of 8 merged items (0%) |
| unknown | 33 | 0/0 | 3.69 | – | 0 of 33 merged items (0%) |


## By kind × size

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bug × ? | 3 | 0/0 | 3.66 | – | 0 of 3 merged items (0%) |
| bug × L | 1 | 0/0 | 2.30 | – | 0 of 1 merged items (0%) |
| bug × M | 26 | 17/18 (94%) | 4.96 | – | 18 of 26 merged items (69%) |
| bug × S | 68 | 48/49 (98%) | 4.97 | – | 49 of 68 merged items (72%) |
| decision × S | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| finding × ? | 2 | 0/0 | 3.69 | – | 0 of 2 merged items (0%) |
| finding × L | 1 | 1/1 (100%) | 3.02 | – | 1 of 1 merged items (100%) |
| finding × M | 3 | 2/2 (100%) | 15.91 | – | 2 of 3 merged items (67%) |
| finding × S | 2 | 2/2 (100%) | 16.60 | – | 2 of 2 merged items (100%) |
| gap × ? | 3 | 0/0 | – | – | 0 of 3 merged items (0%) |
| gap × L | 8 | 4/7 (57%) | 3.73 | – | 7 of 8 merged items (88%) |
| gap × M | 32 | 20/21 (95%) | 4.97 | – | 21 of 32 merged items (66%) |
| gap × S | 54 | 43/46 (93%) | 4.97 | – | 46 of 54 merged items (85%) |
| question × ? | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| question × L | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| question × S | 2 | 2/2 (100%) | 15.91 | – | 2 of 2 merged items (100%) |
| regression × ? | 1 | 1/1 (100%) | 1.03 | – | 1 of 1 merged items (100%) |
| regression × S | 1 | 1/1 (100%) | 6.64 | – | 1 of 1 merged items (100%) |
| spec × S | 1 | 0/0 | 7.82 | – | 0 of 1 merged items (0%) |


## By lane

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bench | 41 | 34/38 (89%) | 4.97 | – | 38 of 41 merged items (93%) |
| docs | 10 | 9/9 (100%) | 7.82 | – | 9 of 10 merged items (90%) |
| frontend | 4 | 4/4 (100%) | 3.60 | – | 4 of 4 merged items (100%) |
| none | 69 | 36/38 (95%) | 4.14 | – | 38 of 69 merged items (55%) |
| paper | 10 | 5/6 (83%) | 6.40 | – | 6 of 10 merged items (60%) |
| rust-cold | 39 | 29/29 (100%) | 4.97 | – | 29 of 39 merged items (74%) |
| rust-hot | 35 | 21/23 (91%) | 4.97 | – | 23 of 35 merged items (66%) |
| tracker | 3 | 3/3 (100%) | 7.82 | – | 3 of 3 merged items (100%) |


## Coverage

Items closed done without a merge (not counted): 1178 (bug-009c0e, bug-012303, bug-017c2d, bug-019f02, bug-02d5ad, bug-02e264, bug-0320da, bug-045773, bug-0522e8, bug-053644, bug-056b40, bug-05a434, bug-05d1ac, bug-06ac5f, bug-06bd18, bug-06e2d1, bug-07bc75, bug-082810, bug-08d912, bug-09690f, bug-09e21e, bug-09fac4, bug-0a1729, bug-0a7ec1, bug-0a934f, bug-0b668a, bug-0b7695, bug-0b7c81, bug-0b997a, bug-0bc2b4, bug-0bc728, bug-0c0747, bug-0c7e11, bug-0c7f25, bug-0d9ac4, bug-0e0dd9, bug-0eb8e2, bug-0f30c7, bug-0f8948, bug-0fe64a, …).

Items merged outside the skills (no claim event): gap-8d2c79, gap-d1d92c, gap-96f7ed, gap-60654d, bug-31438d, bug-35379d, gap-8cb382, bug-62e3f4, bug-0ba3d9, bug-633b68, gap-0f3980, bug-779ae7, bug-ba8d42, gap-751ac9, bug-dfb28f, bug-8a78e1, bug-8417d9, bug-9bffe2, bug-2a2f63, bug-ea7723, bug-bb3262, bug-fa1537, gap-3aa9cb, gap-9eebcb, bug-35c901, q-1faa0c, dec-1089ec, gap-231407, q-46a488, bug-a804b1, bug-f090fc, find-2b34c8, gap-959a7c, gap-ac398f, gap-56f761, gap-20301d, bug-374384, bug-e33b7e, bug-bce0ed, gap-b856d3, gap-d91691, bug-693bb3, bug-f6cd5a, gap-326182.

Items with branch commits that carry no `Executor:` trailer (trail incomplete): gap-8d2c79 (2), gap-d1d92c (2), bug-2a2f63 (1), bug-ea7723 (1), gap-3aa9cb (1), q-1faa0c (2), gap-0d0e81 (3), gap-08120e (1), gap-198c9c (1), gap-46fd19 (1), gap-5ebb4f (1), gap-625195 (1), gap-cb5133 (1), dec-1089ec (1), gap-064c40 (1), gap-ac2611 (1), gap-c06ff3 (1), gap-231407 (38), q-46a488 (38), bug-a804b1 (38), bug-f090fc (38), find-2b34c8 (38), gap-959a7c (38), gap-ac398f (38), gap-dbe8e7 (1), gap-f9fa2c (1), bug-636085 (1), bug-0d0d37 (1), bug-425274 (1), gap-56f761 (3), gap-20301d (2), bug-b7f6a8 (1), bug-2a5566 (2), bug-d0906c (1), bug-bb6b73 (1), bug-2a31bc (1), bug-8d8f98 (2), bug-a66fef (2), find-d05b5e (1), bug-374384 (8), bug-e33b7e (8), bug-bce0ed (8), gap-b856d3 (8), bug-2ad4fa (2), bug-cdd885 (3), gap-d91691 (10), bug-7ecea3 (10), bug-c5f406 (10), gap-744bd0 (10), bug-693bb3 (1), bug-f6cd5a (1), gap-326182 (5), gap-d2ad57 (5), gap-dc24dd (2), gap-1220c5 (5), bug-4fadf8 (3), bug-f8655e (2), bug-2f70cd (3), gap-0eb763 (3).

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
