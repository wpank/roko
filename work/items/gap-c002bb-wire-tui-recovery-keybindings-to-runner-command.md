+++
id = "gap-c002bb"
kind = "gap"
title = "Wire TUI Recovery Keybindings to Runner Command Channel"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md#386 — Wire TUI Recovery Keybindings to Runner Command Channel"
discovered_from = "audit:tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md#386 — Wire TUI Recovery Keybindings to Runner Command Channel"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1522", "crates/roko-cli/src/tui/app/actions.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
pause/retry/skip are visual-only with no runner effect. TUI parity audit found that recovery keybindings (pause, retry, skip) write to `.roko/engrams.jsonl` and show a toast, but the runner has no command channel from TUI. These are visual-only with no actual effect on execution.

Imported without verification from:
- `tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md#386 — Wire TUI Recovery Keybindings to Runner Command Channel`

How to verify: Check whether the gap described in tmp/backlog/archive/386-tui-recovery-keybindings-runner-channel.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: The P2-TUI-3 command channel exists and Cancel/Pause/Resume take effect (plan_runner.rs exec_cmd_rx drain). SoftRetry/Repair/ReverifyGates/Skip/Approve/RejectApproval/Reset are only acked `Accepted` ('TUI command queued (post-execution; plan still running)') with no consumer, and once the flow ends they are acked 'plan finished — re-run to apply'. Retry and skip therefore still have no runner effect.
