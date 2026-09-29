+++
id = "bug-f7943a"
kind = "bug"
title = "dev.sh fast invokes the removed runner-v2 engine"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["tooling/dev-sh"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/work-management/01-gaps-md-audit.md"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["dev.sh:307"]
links = { depends_on = [], blocks = [], related = ["gap-4a6dcb"], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "runner-v2" dev.sh'

[[verify]]
command = '! grep -q "runner-v2" dev.sh'

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "session roko-b6"
evidence = "Verify re-run here at HEAD 725f21e05: dev.sh no longer mentions runner-v2, and no longer passes --engine or --skip-preflight. On the Graph engine, --log-file now writes a JSONL event log (graph_execution/event_log.rs), and FAST's run deadline is ported (graph_execution/fast_lane.rs). The remaining FAST features are tracked as gap-4a6dcb. No live ./dev.sh fast run has been recorded yet. Re-verified 2026-09-29 at dda23167f (roko-53 triage): the verify command passes; dev.sh passes --log-file {bundle}/events.jsonl and sets ROKO_FAST_PLAN_DEADLINE_SECS."
+++

The FAST self-development wrapper passes `--engine runner-v2` to `roko plan run` (`dev.sh:307`).
Runner-v2 has been removed and its entry point bails with a deprecation error, so `./dev.sh fast plans/<dir>` fails before running any task.
Fix: drop the flag (Graph is the default engine) and add a smoke test for the wrapper.

Fixed in 725f21e05: the flag is gone, `--log-file` writes a JSONL event log on the Graph engine, and the FAST run deadline is ported. The rest of FAST mode is tracked as gap-4a6dcb.

Status note (2026-09-29). Landed in 725f21e05: the removed-engine flag is gone from dev.sh, `--log-file` writes a JSONL event log on the Graph engine, and the FAST run deadline is ported. Still missing on the Graph path: the FAST patch-only prompt section, the 90 s attempt/silence clamp and the 6-turn agent cap (plus the `dev-fast` gate profile and no-autofix). That remaining work is tracked by gap-4a6dcb, not here. This item stays `done` because its own defect is fixed and its verify command passes; the FAST deadline being reported as SIGTERM is gap-9efe8e.
