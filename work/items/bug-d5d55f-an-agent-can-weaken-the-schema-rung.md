+++
id = "bug-d5d55f"
kind = "bug"
title = "An agent can weaken the schema rung's own schema file in the same diff as the artefact it checks"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-gate/attempt-diff", "roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK75 gap-3c3729)"
discovered_from = "gap-3c3729"
anchors = ["crates/roko-gate/src/attempt_diff.rs::AttemptDiffPolicy", "crates/roko-gate/src/attempt_diff.rs::DiffFindingKind", "crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn schema_file_edited_in_the_same_diff_is_tamper' crates/roko-gate/ && cargo test -p roko-gate schema_file_edited_in_the_same_diff_is_tamper"
+++

## Problem

The schema rung (`roko_gate::schema_gate::check_schema`, backlog task 9124) validates a task's artefact against a
schema file it reads from inside the attempt's own writable workspace:
`crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs:258`:

```rust
let schema = rung.schema.as_deref().unwrap_or_default();
match std::fs::read_to_string(self.workdir.join(schema)) {
```

`self.workdir` is the attempt's own worktree — the same tree the agent's diff changes. Nothing in
`AttemptDiffPolicy` (`crates/roko-gate/src/attempt_diff.rs`), the inline tamper screen run before verify steps,
recognizes a schema file as a category to protect. Its `DiffFindingKind` enum covers test files, `tasks.toml`,
verify scripts, pinned accept destinations, and `GateConfigEdited` ("roko.toml's `[gates]`, CI config, a test
snapshot, or test-runner config") — a schema document (e.g. `report.schema`, `table.schema`) matches none of
these. So an agent whose artefact fails `check_schema` can simply loosen or empty the schema file in the very same
diff that produces the artefact, and the rung will then validate the artefact against the agent's own weakened
schema and pass — with zero tamper finding, regardless of whether the schema path sits inside or outside the
task's declared `files` scope (and even an out-of-scope edit is only a soft, opt-in `OutsideScope` finding, not
tampering, under `[gates] diff_scope = "enforce"`).

## Why it matters

This defeats the schema rung's entire purpose: a check that can be edited by the thing it checks is not a check.
`attempt_diff.rs`'s own module doc frames exactly this class of gaming ("what an agent's attempt changed that it
must not have") as its reason to exist, and already covers the structurally identical case of a task editing its
own verify script or `tasks.toml` — a schema file is the same shape of problem (gate configuration co-located
with the artefact) left out of the enumeration. Discovered while reviewing gap-3c3729 (PK75, the citations/judge/
schema rungs package), which shipped `schema_rung_fails_invalid_json_artefact` (a correctness test) but no
anti-gaming test for this rung.

## Where

- `crates/roko-gate/src/attempt_diff.rs::AttemptDiffPolicy` (struct at line 105) and `DiffFindingKind` (line 141) —
  needs a new field (e.g. `schema_files: Vec<String>`) and a new `DiffFindingKind::SchemaEdited` variant (or fold
  the check into `GateConfigEdited`'s existing category, extending its doc comment), checked in
  `check_attempt_diff` (line 252 on) alongside the existing `TasksTomlEdited`/`VerifyScriptEdited` checks.
  `is_tamper()` (line 178) must return `true` for the new kind so it fails the attempt by default, matching
  `GateConfigEdited`'s treatment.
- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs:214-263` — the only production reader of a rung's
  `schema` field; wherever `AttemptDiffPolicy` is constructed for a task that has a schema rung must populate the
  new field from `rung.schema`.

## Current state

Unmitigated. `pack_rungs.rs` reads the schema from the attempt's own workdir for every task that declares a
`schema` rung (`RungKind` schema variant, `roko_core::config::schema::DomainProfile`/`resolve_profile`); nothing
constrains where that file may live or protects it from the attempt's own diff.

## Plan

1. Thread the schema rung's path (`rung.schema`) into `AttemptDiffPolicy` wherever it's built for a task (find the
   policy-construction call site(s) — likely in `crates/roko-cli/src/graph_task_dispatch/` near where
   `pack_rungs` and the inline tamper screen are both wired into one attempt's verification).
2. Add `DiffFindingKind::SchemaEdited` (or extend `GateConfigEdited`'s coverage and doc comment to explicitly name
   schema files) and a check in `check_attempt_diff` that flags any change to a declared schema path, mirroring
   the existing `TasksTomlEdited`/`VerifyScriptEdited` checks structurally.
3. Decide whether a *legitimate* task that is itself meant to produce or edit a schema (e.g. a task whose job is
   schema authoring) needs an opt-out — if `rung.schema` and the task's own artefact path can coincide in a real
   domain profile, this needs a rule; if they're always disjoint in practice, a flat prohibition is simplest.
4. New test modeled on the existing `attempt_diff.rs` test style (e.g. `schema_file_edited_in_the_same_diff_is_tamper`)
   feeding a change to the declared schema path and asserting a tamper finding.

## Done when

- A diff that both weakens an artefact's schema file and edits the artefact it's checked against is flagged as
  tampering by `check_attempt_diff`, not silently passed.
- The `[[verify]]` command passes.

## Notes

- Severity p1: the vulnerable code path (`pack_rungs.rs`'s `workdir.join(schema)`) is unconditional for every task
  using the schema rung — this is not a theoretical risk contingent on an unusual workspace layout, it is how the
  rung is wired today for any real use.
- `diff_gate.rs` (`DiffGate`/`analyze_diff`) is a *different* mechanism (vacuous-impl/forbidden-token detection on
  diff content) and does not protect file paths at all; do not confuse the two when fixing this.

## Progress

- bug-d5d55f: implemented on work/bug-d5d55f at 39ef31ea5; cargo verification deferred to the batch gate. `RungFileEdited` (tamper) covers every configured schema rung's schema and judge rung's file rubric (`GatesConfig::rung_files`), wired into Graph dispatch's pre-verify screen and the audit worker's A1; a task whose `files` name the rung file itself (not its directory) may write it. Tests: `schema_file_edited_in_the_same_diff_is_tamper` (roko-gate) and `loosening_a_schema_rung_s_schema_is_tampering` (dispatch).
