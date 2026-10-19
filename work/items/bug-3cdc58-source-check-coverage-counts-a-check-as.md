+++
id = "bug-3cdc58"
kind = "bug"
title = "Source-check coverage counts a check as kept only when a step names its SC id: a plan that runs 12 of a spec's 15 check commands verbatim reports 'kept 0, missing 15' and regenerates for nothing"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#C-p0-plan2"
discovered_from = "agent:hermes-pilot (round v7 phase 0, 2026-10-19, op 437486f8)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::SourceCoverage::of", "crates/roko-cli/src/plan_generate/source_checks.rs::attach_runs", "crates/roko-cli/src/plan_generate/source_checks.rs::normalize_covers"]
links = { depends_on = [], blocks = [], related = ["bug-27f00b", "gap-d21da9", "bug-8fe462"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_step_running_a_checks_command_keeps_it' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_step_running_a_checks_command_keeps_it"
+++

## Problem

Round v7, Undertow phase 0 (hermes-pilot, 2026-10-19; roko 3b34aed92; planner gpt-5-4-mini through the chat-host
rule). A Hermes chat passed the whole of `docs/specs/phase-0-foundation.md` (16,235 characters) to
`roko_plan_generate`. Its "Acceptance checks" table has 15 rows; each row's command is one runnable line, for example
`tools/spec.sh 0 params 'P0.01*'`. Operation `437486f8` lifted all 15 (SC1–SC15). It then reported:

```
source_checks: total 15, kept 0, missing 15, shallow 0, cannot_run 0, dropped 0, regenerated true
```

Yet the plan it wrote (`~/dev/undertow/plans/set-the-plan-title-field-to-exactly-p0-undertow/tasks.toml`) runs 12 of
those 15 commands verbatim, each behind a `grep -q` guard. One example:

```toml
[[task.verify]]
command = "grep -q 'constexpr int kNumParams = 57' core/include/undertow/params.h && tools/spec.sh 0 params 'P0.01*'"
covers = ["AC1"]
```

The planner wrote its own AC ids in `covers` and named no SC id anywhere, in both drafts: the first, and the one roko
asked for again with the 15 checks listed as missing. So:
- the report says 0 of 15 kept;
- the page's coverage line will read "0 of 15 kept · 15 missing" in orange;
- the regeneration was spent asking for checks the plan already ran: $0.096 for the operation, against $0.032 for a
  one-call plan earlier the same session.

Only P0.07, P0.14 and P0.15 are really absent from that plan.

## Why it matters

"You specify checks up front and roko keeps them" is the pitch. A spec-check report that calls kept checks missing
tells the person their spec was ignored when it wasn't. It also trains them to distrust the report, and it spends
model budget on a regeneration that can't help.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs::SourceCoverage::of`: a step covers a check only through
  naming. Either its `covers` names the id, or a criterion its `covers` points to names it (`named_checks`). Nothing
  compares the step's command with the check's.
- `normalize_covers` maps SC ids in `covers` to criteria. It doesn't add SC ids that are missing.
- `attach_runs`: `check.run` comes from `check_command`, which only expands unittest ids. For a table row whose
  command column is already one runnable line (as here), `run` stays `None`. So the planner is never told "use this
  command unchanged", and nothing can match on it.

## Plan

1. When a source check's text carries a single command, as in a table row with a command column (`| P0.01 |
   `tools/spec.sh 0 params 'P0.01*'` | … |`), set `check.run` to it.
2. In `SourceCoverage::of` (or a normalization before it): when one of a step's shell segments (`shell_segments`)
   equals a check's `run`, count the step as covering that check. Add the SC id to the step's criterion text, as
   `normalize_covers` does for named ids.
3. Regenerate only for checks no step covers by name or by command.

## Done when

The phase-0 plan above reports 12 of 15 kept and 3 missing (P0.07, P0.14, P0.15), with no regeneration for the 12.
Covered by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p0-plan2/` (the operation record, `tasks.toml`, and the chat's tool
  call with its 16,235-character prompt).
- Related:
  - bug-8fe462: `run` for unittest wildcards;
  - gap-ecafde: runnable checks;
  - bug-27f00b: the baseline's unpassable checks.
