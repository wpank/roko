+++
id = "gap-ac398f"
kind = "gap"
title = "Plan generation has no crash retry/escalation outside the prd plan path"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/plan-generate"]
created = 2026-08-31
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "gaps-md#partial-10/57"
anchors = ["crates/roko-cli/src/prd.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:16Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done: every generation path (`roko run --plan`, `roko plan generate`, `roko plan regenerate`, serve's `POST /api/plans/generate`) goes through `plan_generate::generate_plan` (gap-2623b2, bb1d7f952; the last PRD-only path went in merge bfd36512f), which retries agent crashes (`classify_agent_crash`, back-off) and escalates the planner model on invalid output (`plan_generate/pipeline.rs::next_tier_model`)."
+++

UX parity item #57: retry and escalation after an initial agent crash exist only on the `roko prd plan` path, not for `roko plan generate`. Backlogs #57 and #85 (TOML reliability) are archived without a status.

Fix: share one plan-generation service, with crash retry and model escalation, across `prd plan`, `plan generate` and serve.
