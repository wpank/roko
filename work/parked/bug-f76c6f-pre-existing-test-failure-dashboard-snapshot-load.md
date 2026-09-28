+++
id = "bug-f76c6f"
kind = "bug"
title = "Pre-existing test failure: dashboard_snapshot load_from_workdir_uses_signal_gates_when_executor_has_none"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/FAST-ITERATION-PROMPT-2026-09-23.md#Known issues to watch for"
discovered_from = "audit:tmp/archive/FAST-ITERATION-PROMPT-2026-09-23.md#Known issues to watch for"
anchors = ["dashboard_snapshot::tests::load_from_workdir_uses_signal_gates_when_executor_has_none"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
FAST iteration prompt (2026-09-15) lists a known pre-existing failing test `dashboard_snapshot::tests::load_from_workdir_uses_signal_gates_when_executor_has_none`, described as unrelated to plan execution.

Imported without verification from:
- `tmp/archive/FAST-ITERATION-PROMPT-2026-09-23.md#Known issues to watch for`

How to verify: Run `cargo test -p roko-cli load_from_workdir_uses_signal_gates_when_executor_has_none` (or grep for the test) and see whether it still fails or was removed.
