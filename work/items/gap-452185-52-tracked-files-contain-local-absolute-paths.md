+++
id = "gap-452185"
kind = "gap"
title = "52 tracked files contain local absolute paths under /Users/will, including CLAUDE.md"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
subsystem = ["workspace"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["CLAUDE.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test \"$(git grep -l /Users/will | wc -l)\" -eq 0"
+++
`git grep -l /Users/will` lists 52 tracked files (271 lines), and `uniswap/bardo` appears in 10. The repo is public, so these paths expose the author's machine layout, and the docs and scripts that use them fail for anyone else. CLAUDE.md's "Absolute paths" table is the largest block.

Fix: use repo-relative paths, or environment variables for external reference trees.
