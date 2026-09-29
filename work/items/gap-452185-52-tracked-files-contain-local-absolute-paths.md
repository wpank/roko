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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["CLAUDE.md", "crates/roko-cli/src/config.rs:2670"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test \"$(git grep -l /Users/will -- . ':!work/' | wc -l)\" -eq 0"
+++
`git grep -l /Users/will` lists 52 tracked files (271 lines), and `uniswap/bardo` appears in 10. The repo is public, so these paths expose the author's machine layout, and the docs and scripts that use them fail for anyone else. CLAUDE.md's "Absolute paths" table is the largest block.

Fix: use repo-relative paths, or environment variables for external reference trees.

Re-checked 2026-09-29: 54 tracked files now match (was 52). Three of them are work-graph files (this item, work/STATUS.md and work/parked/gap-b040f4), so the old verify command could never pass; it now excludes work/. CLAUDE.md still has 30 matching lines. Code hits are crates/roko-cli/src/config.rs:2670, crates/roko-cli/src/inline/primitives/tool_call.rs:197 and crates/roko-cli/examples/inline_demo.rs:352.
