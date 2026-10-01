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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/02-artifact-inventory.md"
discovered_from = "doc:tmp/work-management/02-artifact-inventory.md"
anchors = ["crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status", "crates/roko-cli/src/index.rs:460", "crates/roko-cli/src/commands/backlog.rs", "plans/portal-programme/01-backend-plan-service/tasks.toml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE 'overlay_graph_checkpoint_status|canonical_checkpoint_status' crates/roko-cli/src/index.rs && grep -qE 'overlay_graph_checkpoint_status|canonical_checkpoint_status' crates/roko-cli/src/commands/backlog.rs"
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
