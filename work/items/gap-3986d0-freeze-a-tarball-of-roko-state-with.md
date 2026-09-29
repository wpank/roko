+++
id = "gap-3986d0"
kind = "gap"
title = "Freeze a tarball of .roko state with each audit tag"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["cybernetic-harness/evidence"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:52 and 14:58, wk-companion-e1's report on gap-cdd5f4); tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md:357"
anchors = ["tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md", "tmp/cybernetic-harness/companion-audit/04-EVALUATION-PLAN.md", "tmp/cybernetic-harness/tools/field_note.py::scrub"]
lane = "tracker"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-cdd5f4", "bug-469537", "gap-ccb87e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/tools/freeze_state.py && grep -q 'def test_archive_excludes_secret_files' tmp/cybernetic-harness/tools/test_freeze_state.py && python3 tmp/cybernetic-harness/tools/test_freeze_state.py"
+++

## Problem

An audit tag such as `audit/baseline-2026-09-28` (`91b4745f8`) freezes only tracked files. `.roko/` is gitignored, and
the tag holds only `.roko/GAPS.md`. To re-derive the companion's numbers at the tag, gap-cdd5f4 had to copy MAIN's live
`.roko/` and cut its JSONL files at a timestamp (`E1-REDERIVATION.md:60`). That cannot recover state files that are
rewritten in place: seven JSON state files had been rewritten on 2026-09-29 around 08:50Z with no older copy (`:80`),
so the numbers read from them can only be cited "as read on 2026-09-28". E1's recommendation for §0 of the evaluation
plan (`:357`) is to freeze a tarball of `.roko/` state with each tag.

## Why it matters

Every number the companion report and the research paper derive from `.roko/` state must be reproducible at its tag.
Without a frozen copy, each new tag repeats E1's windowing and loses the rewritten files. Epic spec-f8d196.

## Where

- New: `tmp/cybernetic-harness/tools/freeze_state.py` and `tmp/cybernetic-harness/tools/test_freeze_state.py`.
- `tmp/cybernetic-harness/companion-audit/04-EVALUATION-PLAN.md` §0: the tagging procedure.
- `tmp/cybernetic-harness/tools/field_note.py::scrub`: the credential scrubber to reuse.

## Current state

Nothing freezes `.roko/` today. `audit/baseline-2026-09-28` has no state archive, and it cannot get one after the fact
for the rewritten files.

## Plan

1. `freeze_state.py <tag>` writes a compressed archive of `.roko/` state and logs (JSONL and JSON files) to a
   location outside git, for example `~/.roko-evidence/<tag>/`. It also writes a manifest with each file's path, size,
   row count and sha256, plus the archive's own sha256.
2. Exclusions: `.roko/.env`, `secrets.toml`, credentials, worktrees and build caches. Transcripts are left out unless
   asked for. Scrub home-directory paths as bug-469537 does.
3. Record the archive's sha256 in the tag's annotation, or in the evidence README next to the tag.
4. Add the step to §0 of the evaluation plan, and run it for the next audit tag.
5. Tests: `test_archive_excludes_secret_files`, and a manifest round-trip that checks the hashes.

## Done when

- [ ] One command freezes `.roko/` state for a tag, with a manifest and hashes, and without secrets.
- [ ] The evaluation plan's tagging procedure includes it.
- [ ] The `[[verify]]` command passes.

## Notes

- Untracked tooling: create it in the main checkout's `tmp/cybernetic-harness/tools/`.
- Never commit the archive, which can hold private run data. Commit only its hash.
