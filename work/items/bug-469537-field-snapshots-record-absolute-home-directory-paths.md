+++
id = "bug-469537"
kind = "bug"
title = "Field snapshots record absolute home-directory paths, so they cannot be published as they are"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/field-tools"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:50, wk-rp-appx's report on gap-2abf34, finding 4)"
anchors = ["tmp/cybernetic-harness/tools/field_capture.py::provenance", "tmp/cybernetic-harness/tools/field_capture.py::excerpts", "tmp/cybernetic-harness/tools/field_note.py::scrub"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = ["bug-652bb7", "gap-29a64e", "gap-184da5", "gap-3986d0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''test -d tmp/cybernetic-harness/evidence/field/snapshots && ! grep -rqF "$HOME/" tmp/cybernetic-harness/evidence/field/snapshots'''

[[verify]]
command = "grep -q 'def test_snapshot_has_no_home_paths' tmp/cybernetic-harness/tools/test_field_capture.py && python3 tmp/cybernetic-harness/tools/test_field_capture.py -k test_snapshot_has_no_home_paths"
+++

## Problem

`field_capture.provenance` (`field_capture.py:183-217`) writes the workspace as its absolute path (`"workspace": root`)
and each entry of `binary_candidates` with an absolute `path`. All 42 `manifest.json` files under
`tmp/cybernetic-harness/evidence/field/snapshots/` therefore contain `/Users/<name>/…` paths, and so does one
`excerpts.json`, where an agent's output quotes a path. The excerpts are scrubbed of credentials (`field_note.scrub`)
but not of home paths.

## Why it matters

The field evidence is meant to be published: the whitepaper's §7 cites it, and the research paper's App. E, the
companion report and the S10 showcase bundles will too. Absolute paths reveal the operator's user name and directory
layout. The whitepaper's frozen copies were scrubbed by hand (gap-29a64e, merged in `f5b070d86`), but the tool still
writes the paths, so every later export has to be scrubbed again. Part of epic spec-f2463d.

## Where

- `tmp/cybernetic-harness/tools/field_capture.py`: `provenance` (workspace and binary paths) and `excerpts` (agent
  output cut to 280 characters).
- `tmp/cybernetic-harness/tools/field_note.py::scrub`: the credential scrubber the excerpts already use.
- `tmp/cybernetic-harness/evidence/field/snapshots/*/*/manifest.json` and `excerpts.json`: the existing snapshots.

## Current state

Checked in MAIN on 2026-09-29: 42 of 42 manifests and 1 excerpts file contain a home-directory path.
`workspace_label` (the directory's base name) is already recorded next to the absolute path.

## Plan

1. `provenance` records `workspace_label` and repo-relative paths only (binaries as `target/debug/roko`), with no
   absolute path.
2. `scrub` (or a new helper next to it) replaces the home directory with `~` in excerpts and derived rows.
3. Rewrite the existing snapshots once with the same rules. Keep the snapshot directory names, which already use the
   label.
4. Add `tmp/cybernetic-harness/tools/test_field_capture.py` with `test_snapshot_has_no_home_paths`: capture a fixture
   run from a temporary workspace under a fake home and assert that no written file contains the home path.

## Done when

- [ ] New snapshots contain no absolute home-directory path.
- [ ] The existing snapshots are rewritten, and the rollup still builds from them with the same totals.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Untracked files: edit them in place in the main checkout, and restart the capture watcher afterwards
  (`evidence/field/.capture.pid`).
- Don't edit the frozen copies in `docs/whitepaper/evidence/`: they are already clean, and their sha256 sums are cited.
- bug-652bb7 edits the same two tools. Run the two items one after the other.
