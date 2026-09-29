+++
id = "gap-b23ebd"
kind = "gap"
title = "docs/v3 carries about 585 citation errors; fix them before publishing the docs"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["docs"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["docs/v3/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The companion audit checked the design docs' citations. An estimated 3.0% are fabricated (CI 1.2–5.1%), 12.0% have major errors and 38.7% have some error. 585 errata locations are in `docs/v3`, the tree the Nous plan would publish on GitHub Pages (NB2-006). The errata list is `tmp/cybernetic-harness/companion-audit/CITATION-ERRATA.md`, in the gitignored workspace.

Do not edit `docs/` until the companion audit's evidence has been re-anchored (Track E, E1). The 2026-09-28 evidence snapshot is in `roko-evidence/audit-2026-09-28/`, next to the repo.
