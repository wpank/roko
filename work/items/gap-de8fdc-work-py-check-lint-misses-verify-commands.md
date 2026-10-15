+++
id = "gap-de8fdc"
kind = "gap"
title = "work.py check --lint misses verify commands that can never pass (syntax errors, grep on a directory, a missing cargo test target) or never fail (a negated grep over a missing file, a report mode that always exits 0)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-41"
anchors = ["tools/work.py::lint_verify", "tools/test_work.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["find-d1a883", "q-61b3f8", "gap-a29711", "spec-6ac537", "gap-082a14", "spec-f8d196", "gap-ba9965", "gap-76fd1c", "gap-0132c2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_lint_flags_unpassable_and_vacuous_verifies' tools/test_work.py && python3 tools/test_work.py -k test_lint_flags_unpassable_and_vacuous_verifies"
+++

## Problem

The 2026-10-15 assessment found verify commands that cannot do their job, none of which `work.py check --lint`
(`lint_verify`, `tools/work.py:318`) flags (finding 41; reports C1, C2, A03, A11):

- **Can never pass:** find-d1a883 (a trailing English parenthetical makes the command a bash syntax error, rc 2);
  q-61b3f8 (`grep -n … crates/roko-core/src/config/`, a directory without `-r`, rc 2); gap-a29711
  (`cargo test --test topology_kill_points`, a target that no longer exists); spec-6ac537 (names
  `gate_pass_increments_selected_playbook`, a test that never existed); gap-082a14 (its grep rejects the name the
  fix landed under, `inspect_canonical_checkpoint`); gap-25065c (closed since).
- **Can never fail:** backlog 3113 (`! grep -q … prd.rs do_cmd.rs cloud.rs`: two files are gone, grep exits 2 and
  `!` turns that into success); spec-f8d196 (`paperlint --report` always exits 0, `tools/paperlint.py:966`); 9342 in
  gap-ba9965 (its greps pass with no R2 view built); gap-76fd1c (names a test that checks 8 of the 48 instances); C4
  in gap-0132c2 (an empirical-Bernstein radius of about 2300/n cannot fail at realistic n).

Each open one gets a dated note; this item makes the linter catch the mechanical kinds before they are filed.

## Why it matters

Goal `tooling`: a verify that can't pass blocks a correct closure, and one that can't fail closes an item whose
Done when doesn't hold. Both erode the work graph's trust.

## Where

`tools/work.py::lint_verify` (used by `check --lint` and by `new`), `tools/test_work.py`.

## Current state

`lint_verify` already flags a pipeline ending in `| head`, an unfiltered `cargo test -p`, and an unguarded test
name (work/README.md "Verify commands").

## Plan

Add checks to `lint_verify`, each a warning naming the rule:

1. `bash -n` fails on the command (syntax error).
2. `grep` without `-r`/`-R` whose last path argument is an existing directory.
3. `! grep …` (or `! test …`) naming a file path that does not exist at HEAD: the negation turns grep's exit 2 into
   success.
4. `cargo test … --test <name>` where `crates/<crate>/tests/<name>.rs` does not exist.
5. A known always-zero command (`paperlint.py --report` without `--strict`/`--check`); keep the list small and named.

Test `test_lint_flags_unpassable_and_vacuous_verifies`: one fixture command per rule is flagged, and a correct
guarded verify is not.

## Done when

The `[[verify]]` command passes and `python3 tools/work.py check --lint` lists the open items above until their
verifies are fixed.

## Notes

Semantic weakness (a test that checks 8 of 48, a radius that can't fail) can't be linted; those stay with the notes
on each item. The tracker lane takes one agent at a time.
