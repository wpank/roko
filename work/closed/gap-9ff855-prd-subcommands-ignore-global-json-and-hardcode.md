+++
id = "gap-9ff855"
kind = "gap"
title = "PRD subcommands ignore global --json and hardcode --role"
status = "superseded"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/prd"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "gaps-md#from-cli-audit-tmpcli-auditsummarymd/json-ignored"
anchors = ["crates/roko-cli/src/prd.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:21Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded: the PRD subcommands were removed (merge bfd36512f); the `roko prd` stub honours `--json` (it prints a JSON error object naming the replacement)."
+++

The 2026-09-01 CLI audit found about 15 subcommands that ignored `--json`: all 9 PRD commands, 6 knowledge commands, `learn tune` and `agent status`. It also found that the research and PRD commands hardcode their roles instead of honouring `--role`. Knowledge JSON (#309), learn (#311), research (#306) and agent (#305) were later marked implemented. Backlog #303 (PRD CLI consistency) is archived as blocked on #262, #280 and #283, so the PRD part is probably still open.

Fix: honour `--json` and `--role` in every PRD subcommand. Add a CLI test that runs each subcommand with `--json` and parses the output.
