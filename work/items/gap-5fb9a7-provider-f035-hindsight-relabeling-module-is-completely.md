+++
id = "gap-5fb9a7"
kind = "gap"
title = "[provider F035] Hindsight relabeling module is completely unwired"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-learn/hindsight"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F035"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F035"
anchors = ["crates/roko-learn/src/hindsight.rs::HindsightRelabeler", "crates/roko-learn/src/lib.rs:94"]
links = { depends_on = [], blocks = [], related = ["bug-121c35", "find-34a4b5"], supersedes = [], duplicate_of = "" }
+++
`HindsightRelabeler` is defined in `crates/roko-learn/src/hindsight.rs` (210 LOC, 1 test) but is never instantiated or called from any runtime path. Episode labels are never corrected for retrospective information (regressions, contradictions). The entire module is dead code.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F035`
- `tmp/archive/provider-audit/17-learning-loop.md`

How to verify: E25 claims hindsight adjustments wired; check hindsight relabeling callers. Confirm in crates/roko-learn/src/hindsight.rs whether still true: Hindsight relabeling module is completely unwired

Verified 2026-09-28: HindsightRelabeler (crates/roko-learn/src/hindsight.rs:43, scan at :64) has no reference outside its own file. roko-learn/src/lib.rs:94 only declares the module, and no crate uses roko_learn::hindsight. This contradicts CLAUDE.md's E25 claim that hindsight adjustments are wired.
