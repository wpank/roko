+++
id = "gap-3b5361"
kind = "gap"
title = "Successful plan attempts are never accepted: accept_attempt has no production caller"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs::accept_attempt"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "\.accept_attempt(" crates --include="*.rs" | grep -v "orchestrator/worktree/tests.rs" | grep -q .'
+++

`accept_attempt` (`orchestrator/worktree/mod.rs:885`) is the only function that records an accepted commit for an attempt; its only caller is `orchestrator/worktree/tests.rs:1799`.
Successful attempts are released without an accepted-commit record, so nothing durably links a passed task to the commit that passed its gates.
Fix: on gate pass call `accept_attempt`, release the worktree retained for review, and record the accepted commit in the checkpoint / attempt log.
