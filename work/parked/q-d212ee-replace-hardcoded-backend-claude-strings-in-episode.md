+++
id = "q-d212ee"
kind = "question"
title = "Replace Hardcoded backend=\"claude\" Strings in Episode/Efficiency Logging"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/213-hardcoded-backend-strings.md#213 — Replace Hardcoded backend=\"claude\" Strings in Episode/Efficiency Logging"
discovered_from = "audit:tmp/backlog/archive/213-hardcoded-backend-strings.md#213 — Replace Hardcoded backend=\"claude\" Strings in Episode/Efficiency Logging"
anchors = ["crates/roko-learn/src/task_metric.rs:295", "crates/roko-learn/src/task_metric.rs:427", "crates/roko-learn/src/runtime_feedback.rs:4417", "crates/roko-cli/src/agent_config.rs:149", "crates/roko-cli/src/commands/init.rs:19", "crates/roko-cli/tests/learn_paths.rs:41,82", "crates/roko-cli/tests/agent_config.rs:16,22", "task_metric.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
learning data with wrong provider labels produces misleading routing decisions in the cascade router. Runner episode logging and efficiency event construction contain hardcoded `backend = "claude"` and similar synthetic values in code paths where the actual dispatch context (provider kind, model…

Imported without verification from:
- `tmp/backlog/archive/213-hardcoded-backend-strings.md#213 — Replace Hardcoded backend="claude" Strings in Episode/Efficiency Logging`
- `tmp/backlog/_archive/_mori-diffs-gaps.md#Group F-2`

Some cited files are gone: `crates/roko-learn/src/runtime_feedback.rs`.

How to verify: Check: Zero non-test, non-config instances of hardcoded `backend = "claude"` in episode/efficiency logging; Episode and efficiency records from a non-Claude provider run show the actual provider name; A `tracing::warn!` is emitted when backend is… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): XS | 3 |]
