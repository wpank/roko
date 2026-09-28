+++
id = "bug-f7943a"
kind = "bug"
title = "dev.sh fast invokes the removed runner-v2 engine"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["tooling/dev-sh"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
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
evidence = "Verify re-run here at HEAD 725f21e05: dev.sh no longer mentions runner-v2, and no longer passes --engine or --skip-preflight. On the Graph engine, --log-file now writes a JSONL event log (graph_execution/event_log.rs), and FAST's run deadline is ported (graph_execution/fast_lane.rs). The remaining FAST features are tracked as gap-4a6dcb. No live ./dev.sh fast run has been recorded yet."
+++

The FAST self-development wrapper passes `--engine runner-v2` to `roko plan run` (`dev.sh:307`).
Runner-v2 has been removed and its entry point bails with a deprecation error, so `./dev.sh fast plans/<dir>` fails before running any task.
Fix: drop the flag (Graph is the default engine) and add a smoke test for the wrapper.

Fixed in 725f21e05: the flag is gone, `--log-file` writes a JSONL event log on the Graph engine, and the FAST run deadline is ported. The rest of FAST mode is tracked as gap-4a6dcb.
