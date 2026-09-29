+++
id = "bug-09690f"
kind = "bug"
title = "README's quick start fails, and the README claims 100% completion"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["docs/readme"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["README.md:108", "README.md:111", "README.md:606", "README.md:9", "README.md:12"]
links = { depends_on = [], blocks = [], related = ["bug-f279ea", "gap-ae2f55"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q -- \"--engine runner-v2\" README.md && ! grep -q \"48 epics\" README.md"
+++
- The public README tells users to run `roko plan run plans/ --engine runner-v2` (`README.md:108`, `:111`, `:606`). That engine was deleted, and the flag exits with an error.
- Its opening (`README.md:9-60`) is an internal roll-up claiming "all 48 epics are accepted" and "124/124 tasks complete (100%)", which the work graph and the companion audit contradict.
- `README.md:12` links to a `tmp/status-quo` file that is not in the repo.
- The assessment also found that it leads with arena, DeFi and x402 content that neither the paper nor the Nous positioning uses.

Fix: rewrite the README around what works today (install, `roko init`, one plan run, serve), move status claims to `work/`, and fix the docs-lint rule that enforces runner-v2 (bug-f279ea).
