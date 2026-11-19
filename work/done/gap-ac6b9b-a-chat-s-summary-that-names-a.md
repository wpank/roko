+++
id = "gap-ac6b9b"
kind = "gap"
title = "A chat's summary that names a spec file is planned without the spec's checks: roko lifts source checks from the request text only (Undertow phase 0's first plan write)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "a1d12fc1c"
source = "round-v7-hermes-pilot#C-p0-plan1"
discovered_from = "agent:hermes-pilot (round v7 phase 0, op 50608b6f)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::source_checks", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["gap-fe4225", "bug-3cdc58"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_summary_naming_its_spec_is_planned_with_the_specs_checks' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_summary_naming_its_spec_is_planned_with_the_specs_checks"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T16:18:12Z"
commit = "b155840c0"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 1ae6cab88 (work/bug-16b798: b155840c0). A request that states no checks itself and names markdown documents of the workspace by path (relative, or absolute inside it; at most 256 KiB; no `..`) is planned with the checks of those that state some: they become the source checks (numbered across the documents), their text goes into the planner prompt under \"Documents the prompt names\", the planner block says which document states them, and GenerationSourceChecks.documents lists them in the outcome. Verify at 1ae6cab88: a_summary_naming_its_spec_is_planned_with_the_specs_checks passes (a prompt naming docs/specs/widget.md: 1 call, 2 of 2 kept, documents = [docs/specs/widget.md], the spec's table in the prompt, the owner task runs both commands), with a_summary_names_the_spec_whose_checks_it_keeps (the chat's real phase-0 summary, fixture summary.md, names the phase-0 spec: 15 checks, the same as planning from the spec; an outside path, `../`, a missing file and a non-markdown file are not read). Branch: as bug-16b798."
+++

## Problem

Round v7, Undertow phase 0 (roko 3b34aed92, op 50608b6f). Asked naturally ("Use roko to write the plan for phase 0 from
docs/specs/phase-0-foundation.md"), the Hermes chat read the spec and sent roko a 10-line summary that names the file:
"Create a roko plan for phase 0 from docs/specs/phase-0-foundation.md. Requirements: … Ensure each task has concrete
checks aligned to the acceptance checks in the spec."

roko lifts source checks only from the request text, and the summary states none. So the planner prompt had no
"Source checks" block, the plan had no spec-check report, and nothing kept the planner to the spec's 15 commands. It
used 13 of them; P0.07 was missing.

## Why it matters

The pitch is that the person writes a spec with checks and roko plans from it. A chat in between should not decide
which of the spec's checks reach the planner.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs`: `source_checks` lifts from the request text only.
- `crates/roko-cli/src/plan_generate/pipeline.rs`: `generate_plan` builds the planner prompt.

## Plan

When the request states no checks itself and names markdown documents of the workspace by path, roko reads them. The
checks of the ones that state checks become the plan's source checks, and their text goes into the planner prompt.
gap-fe4225's `spec_path` is the explicit form of the same thing.

## Done when

A request that names a spec and states no checks is planned with the spec's checks, and its outcome names the spec.
Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p0-plan1/` and the summary in
  `crates/roko-cli/tests/fixtures/p0_source_checks/summary.md`.
