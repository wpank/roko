+++
id = "gap-082a14"
kind = "gap"
title = "Graph runs never write task status back to tasks.toml"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-10-15
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/02-artifact-inventory.md"
discovered_from = "doc:tmp/work-management/02-artifact-inventory.md"
anchors = ["crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status", "crates/roko-cli/src/index.rs:460", "crates/roko-cli/src/commands/backlog.rs", "plans/portal-programme/01-backend-plan-service/tasks.toml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'inspect_canonical_checkpoint' crates/roko-cli/src/commands/backlog.rs && grep -q 'run status lives in Graph checkpoints' crates/roko-cli/src/index.rs"
+++

Task `status` fields in `tasks.toml` are authored values the Graph engine never updates; completion lives only in `.roko/state/graph/<plan>/checkpoint.json`.
Example: `portal-programme/01-backend-plan-service` has a `succeeded` checkpoint while all its tasks still read `status = "ready"`.
Plan listings, indexes and `backlog audit` that read manifests therefore report finished plans as not started.
Fix: write terminal status back atomically after the checkpoint commits, or make every reader derive status from checkpoints; document which is canonical.

Re-verified 2026-09-29 at d9e79e9d8: still no write-back. Correction: plan listings (`roko plan list`, serve plan listing, TUI dashboard) already overlay checkpoint status via plan.rs overlay_graph_checkpoint_status. The readers that still report finished Graph plans as not started are the plans/INDEX.md generator (index.rs:460 reads tasks.toml status) and `roko backlog audit` (commands/backlog.rs reads no checkpoints).

## Notes

- **From wk-filer2 (2026-09-29):** 13 portal-programme plans ran in side worktrees (roko-portal-wt, roko-portal2-wt, roko-backend-wt, roko-backend2-wt), so their Graph checkpoints live there. MAIN's plan index and backlog readers can't see them; the write-back must go to the plan's own checkout, or the readers must know about side worktrees.
- 2026-10-01 (wk-taskdef): blocked on a decision; no code change here. The two readers left need different answers.
  `roko backlog audit` will derive status from Graph checkpoints with gap-759041 (same branch, work/bug-e3df7d).
  plans/INDEX.md cannot: checkpoints live in untracked `.roko/state/graph/`, and CI checks the tracked INDEX.md
  against a fresh rendering (`roko plan index --check`, `.github/workflows/plan-validate.yml:63`). MAIN holds 27
  succeeded checkpoints, so an overlay would make every INDEX.md rebuilt there fail that check. Next step (Will):
  either write terminal statuses back into tasks.toml after a Graph run (a `toml_edit` write in graph_execution, which
  keeps the INDEX deterministic and fixes every reader at once), or declare checkpoints canonical for run status, have
  the INDEX count authored statuses only, and change this item's verify.
- 2026-10-01 (wk-taskdef): the audit half landed with gap-759041 on the same branch. `roko backlog audit` reads each
  plan's checkpoint through `graph_checkpoint::inspect_canonical_checkpoint`, a name this item's verify grep does not
  accept; widen the grep when the INDEX half is decided.

2026-10-15 (status-quo assessment at 7b6a5af44, finding 41): the audit half landed with gap-759041: `roko backlog audit` reads checkpoints through `graph_checkpoint::inspect_canonical_checkpoint`, a name this item's verify grep does not accept, so the verify fails although that half is done. Widen the grep to accept it; the write-back half (Graph runs writing task status back to tasks.toml) is still open.

2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): Checkpoints are canonical for run status. The engine never writes back to authored tasks.toml. plans/INDEX.md keeps counting authored statuses only, and its header says so in this text: 'Task status here is the authored value; run status lives in Graph checkpoints (roko plan status).' Replace the item's verify with `grep -q 'inspect_canonical_checkpoint' crates/roko-cli/src/commands/backlog.rs && grep -q 'run status lives in Graph checkpoints' crates/roko-cli/src/index.rs`. The item stays open until the header lands. Why: CI renders the tracked INDEX fresh and compares (.github/workflows/plan-validate.yml:63, roko plan index --check), and checkpoints live in untracked .roko/state/graph/, so an overlay would fail that check wherever succeeded checkpoints exist (27 in the main checkout, per the item note). A toml_edit write-back would have the engine edit human-authored tracked files after every run, leaving dirty trees and merge conflicts across worktrees whose checkpoints live elsewhere. The live readers already derive from checkpoints: backlog audit uses inspect_canonical_checkpoint (crates/roko-cli/src/commands/backlog.rs, 2 hits), and per the 09-29 correction the plan listings do too. A header line is the cheapest reversible fix, and it stops the INDEX implying that finished plans never started. If vetoed: With write-back, graph_execution writes each task's terminal status into the plan's tasks.toml after the checkpoint commits, to the plan's own checkout. Every reader is then fixed, but runs dirty tracked plan files.
