+++
id = "gap-146d77"
kind = "gap"
title = "ViabilityBench's tamper.py has no rung_file_edited kind, unlike the Rust attempt_diff.rs it ports"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "2539f2c77"
source = "wave-7 follow-up reports 2026-10-03 (PK59, bug-d5d55f)"
discovered_from = "bug-d5d55f"
anchors = ["benchmarks/viabilitybench/audit/tamper.py::KINDS"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_rung_file_edited_in_the_same_diff_is_tamper' benchmarks/viabilitybench/audit/tests/test_tamper.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_tamper.py -k test_rung_file_edited_in_the_same_diff_is_tamper -q"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T08:38:30Z"
commit = "2539f2c77"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T06:54:07Z"
forced = false
evidence = "Gate 15b (merged 2539f2c77): verify test_rung_file_edited_in_the_same_diff_is_tamper passes; the audit suite passes. tamper.py has the rung_file_edited kind, ported from attempt_diff.rs, with a planted case in audit/fixtures/tamper.json."
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

## Progress

- Ported `attempt_diff.rs`'s three helpers as `Policy` methods (mirroring its `impl AttemptDiffPolicy`, not free
  functions): `Policy.rung_files: tuple[str, ...] = ()` (new field), `.names_exactly(path)` (an exact
  `files_in_scope` entry, not a directory/glob match, unlike `.declares`), and `.edited_rung_file(path, old_path)`
  (the first `rung_files` entry matching either path, unless `.names_exactly` exempts it). Extracted the shared
  `./`-and-trailing-`/` trim that `covers()` already did inline into `_named_path()`, used by both `covers()`
  (behavior-preserving refactor) and the two new methods, mirroring `attempt_diff.rs`'s own shared `named_path()`.
  `Policy.from_manifest` reads an optional `rung_files` manifest key the same way it already reads `files_in_scope`
  (`vb.task/1` names no such field today; this is parity with the Rust port's data, not a schema change — see
  Notes). Added `"rung_file_edited"` to `INLINE_KINDS` and wired `Policy.edited_rung_file` into `_check_findings`,
  unconditionally, in the same position `attempt_diff.rs` checks it (before the `declares`-gated
  verify-script/tasks-toml/gate-config block).
- Caught one shadowing-shaped bug before it shipped, the kind this wave's gate feedback flagged: my first draft
  appended the new `rung_file_edited` finding straight into `_check_findings`'s shared `found` list, and the
  existing line `check_config = bool(found) and found[0].kind != "expected_edited"` then read `found[0]` — which,
  once a rung finding was prepended, could be the rung finding itself rather than a verify-script/tasks-toml/
  gate-config one, spuriously flipping `check_config` to `True` and firing an unrelated `timeout_edited` on the
  same change. Fixed by isolating the `declares`-gated block's own findings in a local `config_found` list, so
  `check_config` is computed only from that block's own findings, as before. Wrote
  `test_rung_file_edited_does_not_also_trigger_an_unrelated_timeout_finding` as a standing regression test for
  exactly this, and confirmed it fails without the isolation fix and passes with it (temporarily reverted the
  isolation, re-ran, restored).
- `audit/tests/test_tamper.py` did not exist; created it (4 tests): the named
  `test_rung_file_edited_in_the_same_diff_is_tamper` (mirrors `attempt_diff.rs`'s own
  `schema_file_edited_in_the_same_diff_is_tamper`: three `files_in_scope` variants that cover the rung file's
  directory/glob but not the file itself all still flag it; naming the file exactly exempts it), the
  `check_config` regression test above, a defaults/back-compat test, and a direct test of
  `Policy.edited_rung_file`/`.names_exactly` including the old-path-of-a-rename case.
- `audit/tests/test_battery.py::test_a1_flags_every_planted_tamper_kind` asserts the `audit/fixtures/tamper.json`
  fixture has exactly one planted case per `tamper.KINDS` entry — adding `rung_file_edited` to `INLINE_KINDS` broke
  that exhaustiveness check until the fixture gained a matching case. Added `schemas/report.schema` to the
  fixture's `base`, `"rung_files": ["schemas/report.schema"]` to its `manifest`, and a `rung_file_edited` entry to
  `planted` that edits only that file; confirmed it produces exactly `{"rung_file_edited"}` with nothing else
  (`ALSO` needs no new entry) and the whole suite is green again. This was not in the item's own anchors — found
  only by running `audit/tests/test_battery.py`, not just the named `[[verify]]`, per this wave's instruction.
- Verify: named `[[verify]]` command -> 1 passed. Full `benchmarks/viabilitybench/audit` suite (same venv as
  bug-40de03): 61 passed, 0 failed.
