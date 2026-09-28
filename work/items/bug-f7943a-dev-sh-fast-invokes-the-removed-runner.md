+++
id = "bug-f7943a"
kind = "bug"
title = "dev.sh fast invokes the removed runner-v2 engine"
status = "in_progress"
triage = "verified"
severity = "p1"
subsystem = ["tooling/dev-sh"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/01-gaps-md-audit.md"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["dev.sh:307"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "runner-v2" dev.sh'

[[verify]]
command = '! grep -q "runner-v2" dev.sh'
+++

The FAST self-development wrapper passes `--engine runner-v2` to `roko plan run` (`dev.sh:307`).
Runner-v2 has been removed and its entry point bails with a deprecation error, so `./dev.sh fast plans/<dir>` fails before running any task.
Fix: drop the flag (Graph is the default engine) and add a smoke test for the wrapper.

In progress (2026-09-28): a parallel session is fixing this together with bug-96aff4. Scope is larger than the flag: FAST mode also passes `--log-file {bundle}/events.jsonl`, which the Graph engine never writes (its event sink is stored but never read), and FAST semantics (`ROKO_FAST_MODE`) have no handling on the Graph dispatch path, so they need porting.
