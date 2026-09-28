+++
id = "gap-09e478"
kind = "gap"
title = "Dogfood Session Evidence Bundle"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/228-dogfood-session-evidence-bundle.md#228 — Dogfood Session Evidence Bundle"
discovered_from = "audit:tmp/backlog/228-dogfood-session-evidence-bundle.md#228 — Dogfood Session Evidence Bundle"
anchors = ["scripts/run_evidence.py", "dev.sh:307", "crates/roko-cli/src/graph_execution/plan_runner.rs:1159"]
links = { depends_on = [], blocks = [], related = ["bug-f7943a", "bug-230de6"], supersedes = [], duplicate_of = "" }
+++
[partial] SOURCE-IMPLEMENTED; STRICT LOOPBACK/CLI BUNDLE SMOKE VERIFIED (2026-08-31, `bba2f8858` + `25aaca597`)… — self-hosting failures are expensive to reproduce, and today's evidence is assembled manually across terminal logs, runner state, HTTP responses, screenshots, and Git worktrees. The…

Imported without verification from:
- `tmp/backlog/228-dogfood-session-evidence-bundle.md#228 — Dogfood Session Evidence Bundle`
- `tmp/backlog/archive/228-dogfood-session-evidence-bundle.md#(archived copy; status: SOURCE-IMPLEMENTED; STRICT LOOPBACK/CLI BUNDLE SMOKE…)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.7 Proof Case 7: Evidence bundle validation`
- `tmp/archive/backlog-closure-2026-09-01.md#Items receiving verification updates (228)`
- `tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`

How to verify: Check: Run a one-task successful plan and validate the resulting bundle.; Run a mock agent that exits before its first event; verify the bundle records `lost_effect` or; Run a mock gate timeout; verify timing, timeout kind, and ledger event agree. [evidence: own status: Benchmark evidence collection is now scriptable via `scripts/run_benchmark_evidence.sh` (safe by default…; 00-STATUS-SUMMARY 2. Partial /…] / Check recent tmp/dogfood session reports for evidence-bundle usage.

Merged 2 mined candidates: m1-007, m4-012.

Verified 2026-09-28: still partial - scripts/run_evidence.py exists (its validator requires exactly one run start/terminal in the bundle events.jsonl, run_evidence.py:2320-2332), but the FAST bundle path is broken (dev.sh:307 passes the removed --engine runner-v2; bug-f7943a notes the Graph --log-file sink is never written) and bare Graph runs write no .roko/events.jsonl (bug-230de6), so no Graph-engine bundle has been validated.
