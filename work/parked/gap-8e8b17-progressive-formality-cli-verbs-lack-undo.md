+++
id = "gap-8e8b17"
kind = "gap"
title = "Progressive-formality CLI verbs lack undo"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/ux"]
created = 2026-08-31
updated = 2026-09-28
source = "gaps-md#partial-10/progressive-formality"
anchors = ["crates/roko-cli/src/main.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

UX parity, progressive formality: 4 of the 5 verbs exist (`do`, `think`, `show`, `tune`). The fifth, `undo` (safely revert the last agent change), is missing.

Fix: specify undo semantics (worktree/branch revert plus state rollback) and implement it, or drop it from the UX spec.
