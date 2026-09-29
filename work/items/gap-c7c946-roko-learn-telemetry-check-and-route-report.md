+++
id = "gap-c7c946"
kind = "gap"
title = "roko learn telemetry: check and route-report over the attempt records (S01.P0-13)"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/telemetry", "roko-cli/commands"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e4"
discovered_from = "tmp/cybernetic-harness/specs/S01-instrumentation.md (P0-13, P0-13b, §5.9, §7 criteria 2 and 4)"
anchors = ["crates/roko-learn/src/telemetry/report.rs", "crates/roko-cli/src/commands/learn.rs::dispatch_learn", "crates/roko-cli/src/main.rs::LearnCmd"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = ["gap-96f7ed"], blocks = [], related = ["gap-0d0e81"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn route_report_counts_labels_by_source' crates/roko-learn/src/ && cargo test -p roko-learn --lib route_report_counts_labels_by_source"

[[verify]]
command = "grep -rqw 'fn learn_telemetry_route_report_parses' crates/roko-cli/src/ && cargo test -p roko-cli --bin roko learn_telemetry_route_report_parses"
+++

## Problem

Once attempt records exist (gap-528762, gap-96f7ed), nothing reads them. No command checks a run's records for join
coverage, duplicate settlements, ordering or the mix of cost sources, and nothing reports how routing did per
source. Every question means ad-hoc `jq`.

## Why it matters

This is S01's first-slice demo: "`roko learn telemetry route-report` then prints, from files alone…". The command's
output becomes closing evidence for the E4 items and input to E13, the record of the work itself. It is part of epic
spec-b7303f.

## Where

- **New:** `crates/roko-learn/src/telemetry/report.rs`, holding pure `check` and `route_report` functions over the
  record files, testable in the library.
- `crates/roko-cli/src/commands/learn.rs::dispatch_learn`: the two handlers. `commands` is compiled into the `roko`
  binary, not the library.
- `crates/roko-cli/src/main.rs::LearnCmd`: a new `Telemetry` variant. It has 17 variants today, none for telemetry.

## Current state

Nothing exists at `41c7ffbd6`.

## Plan

1. **`roko learn telemetry check [--run <id>]`** checks:
   - schema validity;
   - join coverage of the efficiency, cost and episode rows;
   - duplicate settlement ids;
   - `seq` ordering;
   - the cost sources present.

   It exits non-zero on a failure.
2. **`roko learn telemetry route-report [--run <id> | --since <date>] [--explain]`** prints, per routing source:
   - n;
   - the pass rate (label 1);
   - the count of null labels;
   - how often the planned model differs from the executed one.

   `masked` and ι print `n/a` until route decision records exist; that is S01.P0-8, which is outside this epic.
   `--explain` lists the attempt keys behind each number.
3. **Read-only:** neither command writes anything.

## Done when

- [ ] On fixture records, `route_report` counts labels per source, and `check` flags a duplicate settlement.
- [ ] `roko learn telemetry route-report --help` parses.
- [ ] Tests `route_report_counts_labels_by_source` and `learn_telemetry_route_report_parses` pass: both `[[verify]]`
      commands.

## Notes

- Registering the variant in `main.rs` (S01.P0-13b) touches a hot file, so batch it with other `main.rs` edits.
- If gap-0d0e81 has already moved `LearnCmd` into `commands/learn.rs`, this item touches no hot file.
