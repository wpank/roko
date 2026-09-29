+++
id = "gap-c95daa"
kind = "gap"
title = "ExecAgent parity matrix: replace permanent #[ignore] with explicit capability N/A (or implement)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider-parity"]
created = 2026-08-17
updated = 2026-09-28
source = "crates/roko-agent/tests/exec_parity.rs:19"
discovered_from = "audit:crates/roko-agent/tests/exec_parity.rs:19"
anchors = ["crates/roko-agent/tests/exec_parity.rs::streaming", "crates/roko-agent/tests/exec_parity.rs::tool_call", "crates/roko-agent/tests/exec_parity.rs::session_continuation"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
ExecAgent streaming, tool_call and session_continuation parity tests are permanently ignored because ExecAgent has no delta protocol, no tool-call wire protocol and is stateless. The matrix should express unsupported capabilities explicitly rather than as ignored tests.

Imported without verification from:
- `crates/roko-agent/tests/exec_parity.rs:19`
- `crates/roko-agent/tests/exec_parity.rs:25`
- `crates/roko-agent/tests/exec_parity.rs:31`

How to verify: Check whether the parity harness supports capability-gated skips; confirm ExecAgent capability flags match.
