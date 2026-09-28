+++
id = "gap-ac398f"
kind = "gap"
title = "Plan generation has no crash retry/escalation outside the prd plan path"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/plan-generate"]
created = 2026-08-31
updated = 2026-09-28
source = "gaps-md#partial-10/57"
anchors = ["crates/roko-cli/src/prd.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

UX parity item #57: retry and escalation after an initial agent crash exist only on the `roko prd plan` path, not for `roko plan generate`. Backlogs #57 and #85 (TOML reliability) are archived without a status.

Fix: share one plan-generation service, with crash retry and model escalation, across `prd plan`, `plan generate` and serve.
