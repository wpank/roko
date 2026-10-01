# Development record: rollup

Not for workers: no view, skill prompt or `work.py next` reads this, and no metric here is a target.

DEFINITIONS.md sha256 `ee0525db5ff09afe9f0b1509cb0d22162b795f2beae153cf859a98b999f86720` · report date 2026-10-01 · HEAD `ad14d38d7` · prices `prices-2026-09-28`. Every figure is observational; costs are API-equivalent (subscription). Percentiles are by nearest rank; times are UTC.

Inputs: 6 live events, 705 backfill events (tabled apart), 0 invalid event rows skipped, 0 harvested call rows.

## All merged items

| Metric | Value | Coverage |
|---|---|---|
| Merged items | 14 | |
| Attempts (1 / 2 / 3+; mean) | 0 / 0 / 0; – | 0 of 14 merged items (0%) |
| First-try merges | 0/0 | 0 of 14 merged items (0%) |
| Claim-to-merge hours (median / p90) | – / – | 0 of 14 merged items (0%) |
| Conflict rate (abandoned over a conflict: 0) | 0/0 | 0 of 14 merged items (0%) |
| Post-merge verify failures | 0/0 | 0 of 14 merged items (0%) |
| Escapes (14 pending) | 0/0 | 0 of 14 merged items (0%) |
| Interventions per item (items with one) | – (0/0) | 0 of 14 merged items (0%) |
| Unassisted merge share | 0/0 | 0 of 14 merged items (0%) |
| Cost per merged item, USD (median / p90; total) | – / –; 0 | 0 of 14 merged items (0%) |
| Orchestration overhead, USD (per merged item) | 0 (0.00) | 0 unjoined call rows |

## By day of first merge

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| 2026-09-29 | 7 | 0/0 | – | – | 0 of 7 merged items (0%) |
| 2026-09-30 | 7 | 0/0 | – | – | 0 of 7 merged items (0%) |


## By executor

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| unknown | 14 | 0/0 | – | – | 0 of 14 merged items (0%) |


## By kind × size

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bug × ? | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| bug × M | 2 | 0/0 | – | – | 0 of 2 merged items (0%) |
| bug × S | 4 | 0/0 | – | – | 0 of 4 merged items (0%) |
| gap × L | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| gap × M | 2 | 0/0 | – | – | 0 of 2 merged items (0%) |
| gap × S | 4 | 0/0 | – | – | 0 of 4 merged items (0%) |


## By lane

| Group | Merged | First-try | Claim→merge h (median) | Cost USD (median) | Coverage (first-try) |
|---|---|---|---|---|---|
| bench | 1 | 0/0 | – | – | 0 of 1 merged items (0%) |
| none | 2 | 0/0 | – | – | 0 of 2 merged items (0%) |
| paper | 2 | 0/0 | – | – | 0 of 2 merged items (0%) |
| rust-cold | 4 | 0/0 | – | – | 0 of 4 merged items (0%) |
| rust-hot | 5 | 0/0 | – | – | 0 of 5 merged items (0%) |


## Coverage

Items closed done without a merge (not counted): 418 (bug-012303, bug-019f02, bug-02e264, bug-0320da, bug-0522e8, bug-056b40, bug-05d1ac, bug-06e2d1, bug-07bc75, bug-08d912, bug-09690f, bug-09fac4, bug-0b668a, bug-0bc728, bug-0c7e11, bug-0d9ac4, bug-0eb8e2, bug-12153c, bug-12be66, bug-12d48c, bug-131421, bug-1440cd, bug-165b22, bug-16a6b6, bug-17f0e4, bug-1c93b4, bug-1f098f, bug-207f35, bug-2116ec, bug-21687a, bug-220385, bug-2379dc, bug-25d24e, bug-28becc, bug-28f2b9, bug-2930a8, bug-32d57f, bug-32eb77, bug-34c16c, bug-367f33, …).

Items merged outside the skills (no claim event): gap-8d2c79, gap-d1d92c, gap-96f7ed, gap-60654d, bug-31438d, bug-35379d, gap-8cb382, bug-62e3f4, bug-0ba3d9, bug-633b68, gap-0f3980, bug-779ae7, bug-ba8d42, gap-751ac9.

Items with branch commits that carry no `Executor:` trailer (trail incomplete): gap-8d2c79 (2), gap-d1d92c (2).

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
