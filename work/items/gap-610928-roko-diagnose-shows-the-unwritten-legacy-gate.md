+++
id = "gap-610928"
kind = "gap"
title = "roko diagnose shows the unwritten legacy gate-gaming file as if it were current"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/diagnose"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-9:q-63c19f"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::run_gaming_alerts"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["q-63c19f", "gap-54b2b2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn gaming_alerts_section_names_its_legacy_source' crates/roko-cli/src/commands/diagnose.rs && cargo test -p roko-cli --lib gaming_alerts_section_names_its_legacy_source"
+++

## Problem

`roko diagnose` reports `gate_gaming_alerts` from `.roko/learn/gate-gaming-alerts.jsonl` only
(`crates/roko-cli/src/commands/diagnose.rs:519-540`, `run_gaming_alerts`). Nothing in production writes that file any
more: `GateGamingDetector::observe_and_detect` and `append_alert` (`crates/roko-learn/src/gate_gaming.rs`) have no
caller outside their own module since decision 4108 removed the judge feed, and
`crates/roko-cli/src/graph_task_dispatch/helper_calls.rs:713-714` asserts the file is not written. Live gate-gaming
alerts come from the audit worker's `GamingWatch` (`crates/roko-cli/src/audit/worker.rs:302`) and stay in the audit
vault's incidents directory, outside every workdir (`crates/roko-core/src/audit_home.rs:1-14`). So an empty or
old `gate_gaming_alerts` list reads as "no gaming", which is a false green on the operator's diagnostic surface.

## Why it matters

Goal truth. q-63c19f decided (decision 9, 2026-10-15) to keep audit alerts inside the vault (option C: no
redacted mirror, and `diagnose` does not read the vault, since any agent shell can run `roko diagnose`). The output
must then say where its alerts come from.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::run_gaming_alerts` and the report section that prints
  `gate_gaming_alerts` (text and `--json`).

## Current state

No label; the list looks current.

## Plan

1. In the text report, head the gate-gaming section with one line: these are judge-feed alerts, not written since
   decision 4108; alerts from audits stay in the audit vault and are not shown here.
2. In `--json`, add `"gate_gaming_alerts_source": "legacy-judge-feed"` beside the list.
3. Do not read, count or print anything from the vault: no path, no count.
4. Add the test `gaming_alerts_section_names_its_legacy_source` in `diagnose.rs` covering both outputs.

## Done when

`grep -qw 'fn gaming_alerts_section_names_its_legacy_source' crates/roko-cli/src/commands/diagnose.rs && cargo test -p roko-cli --lib gaming_alerts_section_names_its_legacy_source`
passes.

## Notes

- From q-63c19f (decision 9, T1). Revisit a redacted mirror only when an operator needs gaming alerts in
  `diagnose` and S05's owner agrees the fields leak nothing about audit selection.
- Never weaken `is_vault_path` or add a vault reader to an agent-reachable command.
