+++
id = "gap-d2ad57"
kind = "gap"
title = "A rung skipped by a failed tool-use probe can be brought back before its day is up only by hand-editing .roko/learn/rung-probes.json: add roko learn probes (list, clear <model>, --all) and name it in the skip readout and the all-rungs-skipped refusal"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-ladder, the lead's ask when approving bug-374384 (2026-10-26)"
discovered_from = "agent:roko-ladder (round v7)"
anchors = ["crates/roko-cli/src/dispatch/rung_probe.rs::RungProbes", "crates/roko-cli/src/dispatch/rung_probe.rs::way_out", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::refused_for_no_rung", "crates/roko-cli/src/commands/learn.rs"]
links = { depends_on = [], blocks = [], related = ["bug-374384", "gap-20301d", "gap-b856d3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cleared_probe_is_probed_again' crates/roko-cli/src && cargo test -p roko-cli --lib a_cleared_probe_is_probed_again"
+++

## Problem

A rung whose model failed its tool-use probe is skipped for a day (`.roko/learn/rung-probes.json`). A billing
refusal is probed again when the next run starts (bug-374384), but any other verdict stands. The only way to have
it probed again sooner is the one the readout names: "to probe it again now, remove `<model>` from
.roko/learn/rung-probes.json". The all-rungs-skipped refusal (`graph_task_dispatch/ladder.rs::refused_for_no_rung`)
says the same.

A person fixes a provider (a key, a base URL, a model name) and then has to hand-edit a JSON file to undo the skip.
Hermes can't do it for them, and a wrong edit can break the file for every later run.

## Why it matters

- The skip is the normal case right now: a run where the cheap rung failed its probe starts on mid. After a fix,
  the person wants the cheap rung back in the next run, not tomorrow.
- A command can be named in the readout, the run summary's `refused` reason and the Hermes plugin. A file edit
  can't be done from any of them.
- Goal: golden-path.

## Where

- `crates/roko-cli/src/dispatch/rung_probe.rs`: `RungProbes` (load/save of `rung-probes.json`) and
  `SkippedRung::way_out`.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::refused_for_no_rung`: the refusal's text.
- `crates/roko-cli/src/commands/learn.rs`: `roko learn`'s subcommands.
- `crates/roko-cli/src/commands/config_cmd.rs`: `roko config providers test` and `reset-health`.

## Current state

- bug-374384 (merged in c8f9bff80) re-probes billing failures on the next run, and the readout names the file.
- `roko config providers reset-health [provider]` clears provider health holds. Nothing clears a probe verdict.

## Plan

1. Add `roko learn probes`: list each rung's verdict (model, passed or failed, why, when it expires). Add
   `roko learn probes clear <model>` and `--all`, which remove verdicts from `rung-probes.json` through
   `RungProbes`, so the next run probes those rungs again. Use `--json` for Hermes.
2. Have `SkippedRung::way_out` and `refused_for_no_rung` name the command, not the file:
   "to probe it again now, run `roko learn probes clear <model>`".
3. Optional: `roko config providers test --model <m>` could run the same tool-use probe and record its verdict.
   That's a second way back, but it costs a paid call, so the clear command comes first.
4. Test: `a_cleared_probe_is_probed_again`. A failed verdict that is still fresh skips its rung. After
   `clear <model>`, the next `probe_ladder` probes it again and uses it when the provider answers. `--all` empties
   the file.

## Done when

- The `[[verify]]` passes.
- The readout's skip line and the all-rungs-skipped refusal name the command, and running it brings the rung back
  in the next run.

## Notes

- The lead asked for this follow-up on 2026-10-26 when approving bug-374384: "A person shouldn't hand-edit a JSON
  file to undo a skip."
- Related: bug-374384, gap-20301d, gap-b856d3.
