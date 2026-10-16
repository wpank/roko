+++
id = "bug-ca1de5"
kind = "bug"
title = "LOG1 block E cannot materialize any of its 10 F8 honeypots: F8's task.json is not vb.task/1 (no spec, no recoverability, six extra keys)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["viabilitybench/families", "viabilitybench/driver"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-f90bd0 worker, 2026-10-16 (materialize F8-l1-0022: invalid manifest)"
discovered_from = "bug-f90bd0"
anchors = ["benchmarks/viabilitybench/families/f8_honeypot/gen.py::generate", "benchmarks/viabilitybench/families/f8_honeypot/reference/solutions.py::apply", "benchmarks/viabilitybench/families/f8_honeypot/gaming.py::detect", "benchmarks/viabilitybench/families/f8_honeypot/hidden.py::run", "benchmarks/viabilitybench/streams/log1_f8_honeypots.toml"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-f90bd0", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_every_log1_f8_honeypot_materializes' benchmarks/viabilitybench/driver/test_materialize.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_materialize.py -k test_every_log1_f8_honeypot_materializes -q"
+++

## Problem

`materialize()` refuses every F8 instance. For `F8-l1-0022`, the first instance of `streams/log1_f8_honeypots.toml`
(LOG1 block E, S09 §4.3), it raises `invalid manifest: $: missing required field 'spec'`. F8's `gen.py` writes a
manifest that `schema/task.schema.json` (`vb.task/1`, `additionalProperties: false`) rejects in three ways: it has no
`spec` (F8 renders no `spec.precise.md`), no `recoverability`, and six top-level keys the schema does not allow:
`base_family`, `base_instance_id`, `base_task_path`, `base_task`, `conflict` and `conflict_hashes`. The verifier CI
stays green because it reads `task.json` directly and never validates it.

## Why it matters

Goal `proof`. Block E (80 billed runs, H5's detector recall) is all F8, so it cannot start. bug-f90bd0 fixed the same
defect for F2, F3, F5 and F7 (the H3 and P1-core families); F8 was outside that item.

## Where

- `benchmarks/viabilitybench/families/f8_honeypot/gen.py::generate`: the manifest.
- Its readers: `reference/solutions.py::apply` (it gets only the manifest dict: `base_task`, `base_family`,
  `conflict`), `gaming.py::detect` (`base_family`, `base_task`, `conflict_hashes`) and `hidden.py::run`
  (`base_family`, `conflict.flag_prefix`; `gen.load_base_instance` reads `base_task_path`).
- `driver/materialize.py::materialize`, which validates the manifest before anything runs.

## Current state

Since bug-f90bd0, F2, F3 and F5 (F8's base families) render `spec.precise.md` and `recoverability`, and their
solutions, gaming detectors and truth suites derive their per-instance values from the manifest's `ladder` and `seed`
and from the pristine tree, not from extra manifest keys. So most of F8's embedded `base_task` is redundant now: the
base family is `BASE_ORDER[seed % len(BASE_ORDER)]`, the base instance has F8's own level and seed, and the conflict
text is `CONFLICTS[base_family]`.

## Plan

1. Derive what the manifest carries today from (level, seed): the base family and the conflict as above, and the base
   instance's private directory at its fixed path `DIR/base/` for the readers that have the instance directory.
   `solutions.apply` can hand the base family's own `apply` a manifest-shaped dict with the base's `ladder` and
   `seed`.
2. Compare the conflict files against the pristine tree in `gaming.detect`, as F5's detector does for its protected
   files (bug-f90bd0), instead of reading `conflict_hashes`.
3. Give F8 a `spec.precise.md` in F1's section layout (the base family's precise spec, plus the repository's
   `SPEC_CONFLICT:` convention as an acceptance criterion) and a `recoverability` list (the base's entries, plus the
   flag requirement with `README.md`, `NOTES.md` and `CONTRIBUTING.md` as evidence).
4. Bump `GENERATOR_VERSION` and keep the task repo byte-identical (compare pristine trees before and after).

The alternative is to extend `vb.task/1` with one optional object for wrapper metadata. That changes the shared
schema, which is cheaper before the pre-registration lock (gap-394f28) than after.

## Done when

Every instance of `streams/log1_f8_honeypots.toml` materializes (precise) with a valid manifest, the verifier CI for
f8 stays green, and the `[[verify]]` command passes.

## Notes

Don't edit the stream's instance list: `experiments/test_log1.py` checks it. Agents are never told an instance is a
honeypot (task 3325; F8's README states the conflict convention generically), so the spec must not tell them either.
