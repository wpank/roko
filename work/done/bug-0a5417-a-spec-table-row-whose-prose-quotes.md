+++
id = "bug-0a5417"
kind = "bug"
title = "A spec table row whose prose quotes a second command gets no command of its own, so the step that runs its command keeps it no more than its acceptance text does: phase 1's P1.16 (tools/spec.sh 1 plugin, which T2 runs, and tools/install.sh --dest in its text) read as missing and paid for a regeneration"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "d2c5e1f21"
source = "round-v8 hermes-pilot-v8, phase 1 re-ask op 541860eb (Undertow 62155c4, roko ba5866664), tmp/hermes-roko-v8/pilot/evidence/p1c-*"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::quoted_command", "crates/roko-cli/src/plan_generate/source_checks.rs::attach_runs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-3cdc58", "bug-6ebc03"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_rows_command_cell_is_its_command' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_rows_command_cell_is_its_command"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T17:29:46Z"
commit = "d2c5e1f21"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "S"
forced = false
evidence = "quoted_command gives a table row the command of its command cell, a cell that is one code span quoting a command, when no other cell is one, whatever its prose quotes; otherwise a check still needs exactly one command span. Red first (ea134deac on main 0777b00a3): a_rows_command_cell_is_its_command found P1.16's run None. Green at d2c5e1f21 on the trial 08b5b30f0 (main e2dfc4e6c + this branch): P1.16's run is `tools/spec.sh 1 plugin`, the re-ask's plan (op 541860eb, Undertow 62155c4) keeps every check, and a row with two command cells or two commands in its prose still has none; all 24 source_checks tests pass, roko-cli lib 4140 passed 0 failed, nightly fmt --check clean, cargo clippy --workspace --no-deps -D warnings clean."
+++

## Problem

A spec table row whose prose quotes a second command gets no command of its own. So a step that runs the row's
command doesn't keep it, and the row reads as missing.

Round v8's phase 1 re-ask (op 541860eb, Undertow 62155c4, roko ba5866664) reported 21 checks, 19 kept, 2 missing:
- SC20, P1.20: the double count bug-6ebc03 fixed.
- SC16, P1.16: `| P1.16 | `tools/spec.sh 1 plugin` | The build makes VST3, AU and Standalone bundles named Metakosmia
  Undertow 2, …, and `tools/install.sh --dest` installs those three |`.
  - T2 runs `tools/spec.sh 1 plugin` exactly, the step that keeps P1.10 to P1.15 and P1.17.
  - `quoted_command` gives a check its run only when exactly one of its code spans is a command. This row has two, so
    its `run` is `None`.
  - Coverage then credits it only through an acceptance criterion that names it, and the re-ask's T2 had none (the
    first write's had "AC4: SC16: P1.16 …").
- The false miss asked the planner again (`regenerated: true`), a second call of the write's $0.26.

## Why it matters

- A spec's table names each check's command in its own column. A sentence that mentions another command, such as the
  tool a check runs, is common, and every such row costs a regeneration and shows as missing on the card.
- Goal: core (plan writing).

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs`: `quoted_command`, which `attach_runs` calls.

## Plan

1. A table row's command cell, a cell that is exactly one code span quoting a command, is its command when it is the
   row's only such cell, whatever its other cells quote. Otherwise the rule stays one command span in the whole check.
2. Test `a_rows_command_cell_is_its_command`, on the 62155c4 spec and the re-ask's plan as written:
   - P1.16's run is `tools/spec.sh 1 plugin`;
   - the plan keeps every table check, with P1.20 the plan's own (bug-6ebc03).

## Done when

- The `[[verify]]` command passes.

## Notes

- Found by hermes-pilot-v8 (evidence: `tmp/hermes-roko-v8/pilot/evidence/p1c-*`).
- Related: bug-3cdc58 (a check kept by its command), bug-6ebc03.
