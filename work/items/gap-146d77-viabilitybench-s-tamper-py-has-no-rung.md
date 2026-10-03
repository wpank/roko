+++
id = "gap-146d77"
kind = "gap"
title = "ViabilityBench's tamper.py has no rung_file_edited kind, unlike the Rust attempt_diff.rs it ports"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK59, bug-d5d55f)"
discovered_from = "bug-d5d55f"
anchors = ["benchmarks/viabilitybench/audit/tamper.py::KINDS"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_rung_file_edited_in_the_same_diff_is_tamper' benchmarks/viabilitybench/audit/tests/test_tamper.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_tamper.py -k test_rung_file_edited_in_the_same_diff_is_tamper -q"
+++

## Problem

`benchmarks/viabilitybench/audit/tamper.py` is a deliberate Python port of roko-gate's tamper-detection rules —
its own module docstring says so directly ("The screen's rules (`attempt_diff.rs`)", right above `TEST_DIRS`).
Its `KINDS` tuple (`INLINE_KINDS + AUDIT_ONLY_KINDS`, lines 66-69) lists `test_deleted`, `test_renamed`,
`test_emptied`, `tests_removed`, `asserts_removed`, `skip_added`, `verify_script_edited`, `tasks_toml_edited`,
`gate_config_edited`, `test_edited`, `expected_edited`, `timeout_edited`, `test_detection`, `success_string`,
`vacuous_diff` — no `rung_file_edited` (or equivalent). The Rust side gained exactly this kind in bug-d5d55f (done,
`crates/roko-gate/src/attempt_diff.rs::DiffFindingKind::RungFileEdited`, driven by
`GatesConfig::rung_files()`/`AttemptDiffPolicy.rung_files`): a task whose diff touches a configured schema rung's
schema file or a judge rung's rubric file is a tamper finding. The Python port has no analog, so the ViabilityBench
benchmark's own tamper screen cannot catch an agent weakening its own schema/rubric file the same way roko's real
tamper screen now can.

## Why it matters

Goal: proof (ViabilityBench parity with roko's own gates). `tamper.py` exists specifically so the benchmark can
detect the same classes of gaming roko's real pipeline detects — a gap here means a ViabilityBench run can't
measure (or defend against) exactly the attack bug-d5d55f just closed on the Rust side, undermining any claim
that the benchmark's gating matches production.

## Where

- `benchmarks/viabilitybench/audit/tamper.py`: `KINDS`/`INLINE_KINDS` or `AUDIT_ONLY_KINDS` (add the new kind),
  `Policy` (needs a `rung_files` field alongside `files_in_scope`/`verify_scripts`), `check_trees` (needs the new
  check, modelled on `attempt_diff.rs`'s `policy.edited_rung_file(path, old_path)` call).
- Reference implementation: `crates/roko-gate/src/attempt_diff.rs::DiffFindingKind::RungFileEdited`,
  `AttemptDiffPolicy.rung_files`, `crates/roko-core/src/config/gates.rs::GatesConfig::rung_files`.

## Current state

Unimplemented. `Policy.from_manifest` would need to read whichever manifest field names a task's configured rung
files (mirroring how `rung_files()` reads `GatesConfig` on the Rust side) and `check_trees` would need one more
check alongside its existing `test_deleted`/`gate_config_edited`-style ones.

## Plan

1. Add `rung_file_edited` to `INLINE_KINDS` (it can be detected from the diff alone, like `gate_config_edited`,
   without needing a full audit pass).
2. Add a `rung_files: tuple[str, ...]` field to `Policy`, populated from whatever manifest field the task's rung
   config exposes (check how the task manifest already declares schema/rubric paths, if it does today, or add one).
3. In `check_trees`, flag a finding when a path in `policy.rung_files` appears as changed between `base` and
   `final`.
4. Add a test mirroring `attempt_diff.rs`'s `schema_file_edited_in_the_same_diff_is_tamper`.

## Done when

- A ViabilityBench task whose schema/rubric file is edited in the same diff as its artefact gets a
  `rung_file_edited` tamper finding.
- The `[[verify]]` command passes.

## Notes

- Related: bug-d5d55f (done, the Rust-side fix this ports) and its own Notes, which separately flag a "read from
  the base tree via git show" robustness improvement for the Rust side — that improvement is out of scope here;
  this item is just about parity (the kind existing at all in the Python port), not about how either side reads
  the schema's content at validation time.
