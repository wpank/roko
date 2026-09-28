+++
id = "bug-b2a9de"
kind = "bug"
title = "[provider F005] ACP path has 7 P0 panics and 12 race conditions"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F005"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F005"
anchors = ["crates/roko-acp/src/bridge_events/", "crates/roko-acp/src/builtin_tools.rs:449", "crates/roko-acp/src/session.rs:676"]
links = { depends_on = [], blocks = [], related = ["bug-c59522", "bug-f0f108"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8: bridge_events.rs is now split into crates/roko-acp/src/bridge_events/, and no .unwrap(), .expect(, panic!( or unreachable!( remains before the #[cfg(test)] module of any crates/roko-acp/src file (builtin_tools.rs:449 replaced the unreachable! with return None). Poisoned locks are recovered (bridge_events/protocol.rs:169-191; transport.rs:32, :69, :229). The 12 race conditions were never itemized (tmp/archive/provider-audit/04-acp-integration.md:471); the itemized backlog #17 races are fixed (see bug-c59522, e.g. session.rs:676 atomic idle-to-busy transition). Remaining sustained-load risks are tracked in bug-f0f108."
+++
The ACP server code path contains panics at points that can be triggered by normal client behavior (invalid session IDs, concurrent access patterns). Additionally, 12 race conditions exist from non-atomic read-modify-write patterns on shared state. These represent gaps tracked as Gap #17.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F005`
- `tmp/archive/provider-audit/04-acp-integration.md`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-04: ACP Stability (7 Panics, 12 Race Conditions)`
- `tmp/dogfood/2026-09-20-final-session.md#P0 (Critical)`
- `tmp/dogfood/2026-09-20-final-session.md#Known Open Items (11 deferred, 12 open)`

A source claims this was fixed; confirm against current code before closing.

Some cited files are gone: `crates/roko-acp/src/bridge_events.rs`.

How to verify: Related: refactoring-audit P0-01-ACP-CRASH-ANALYSIS. Check roko-acp unwrap/expect and RMW races. Confirm in crates/roko-acp/src/ whether still true: ACP path has 7 P0 panics and 12 race conditions / grep unwrap/expect/panic in bridge_events.rs; review shared-state locking.

Merged 2 mined candidates: m2-001, m4-052.

Verified 2026-09-28: fixed; see `[closed].evidence`.
