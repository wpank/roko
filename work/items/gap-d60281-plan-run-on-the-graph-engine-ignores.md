+++
id = "gap-d60281"
kind = "gap"
title = "plan run on the Graph engine ignores --resume, --effort, --skip-preflight, --force and screenshot flags"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/plan-run"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#from-engine-audit-tmpengine-auditsummarymd/engine-graph-drops-flags"
anchors = ["crates/roko-cli/src/commands/plan.rs:2398"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''! grep -n 'not supported with --engine graph and will be ignored' crates/roko-cli/src/commands/plan.rs'''
+++

The 2026-09-01 audit found that `--engine graph` silently dropped 8 flags. They now log a warning but are still ignored (`crates/roko-cli/src/commands/plan.rs:2398-2429`): `--resume`, `--effort`, `--skip-preflight`, `--force`, `--screenshots` and `--screenshot-interval`. Graph is now the only engine, so these flags do nothing on any `plan run`.

Fix: implement each flag on the Graph path (`--resume` maps onto Graph checkpoints), or remove it from the CLI and return a deprecation error.
