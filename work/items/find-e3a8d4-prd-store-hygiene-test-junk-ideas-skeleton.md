+++
id = "find-e3a8d4"
kind = "finding"
title = "PRD store hygiene: test-junk ideas, skeleton drafts, stale INDEX"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/prd"]
created = 2026-09-26
updated = 2026-09-28
source = ".roko/prd/INDEX.md"
discovered_from = "audit:.roko/prd/INDEX.md"
anchors = [".roko/prd/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
.roko/prd has 10 ideas/*.md test captures from portal testing (2026-09-23), a test-quick draft, test lines in ideas.md, INDEX 'Recent Ideas' all junk, a frontmatter-less self-developing-workflow draft, and skeleton drafts (doctor-network-v2, justfile-recipes).

Imported without verification from:
- `.roko/prd/INDEX.md`
- `.roko/prd/ideas.md`
- `.roko/prd/ideas/`
- `.roko/prd/drafts/test-quick.md`

How to verify: Confirm with owner, then prune test captures and rebuild INDEX via a roko prd command.
