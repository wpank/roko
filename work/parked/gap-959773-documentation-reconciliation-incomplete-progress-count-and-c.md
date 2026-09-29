+++
id = "gap-959773"
kind = "gap"
title = "Documentation reconciliation incomplete; progress count and counts in docs disagree"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["docs"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#remaining-work-estimate/tranche-2"
anchors = ["docs/v3", "CLAUDE.md", "README.md"]
links = { depends_on = [], blocks = [], related = ["gap-c50b85", "bug-f279ea"], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

DOC reconciliation progress is recorded as 18/71 in GAPS.md and 3/71 in CLAUDE.md. Known drift from the audits:
- HTTP route counts: ~376 canonical / ~421 total are claimed. Regenerate the counts with `python3 tools/http_route_inventory.py`.
- About 15 CLI subcommands are undocumented: `job match`, `do`, knowledge export/import/backfill-hdc, and plan pause/resume/cancel/retry/status/queue.
- `orchestrate.rs` is still mentioned in 5 doc files. In code, only a historical comment remains (`crates/roko-cli/src/lib.rs:177`).
- CLAUDE.md and README.md restate the same epic status tables by hand.

Fix: finish the DOC reconciliation tasks, and generate counts with tools instead of typing them. Replace the hand-maintained status tables in CLAUDE.md and README.md with links to `work/STATUS.md`.
