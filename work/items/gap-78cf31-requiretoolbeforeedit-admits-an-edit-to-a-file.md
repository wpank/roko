+++
id = "gap-78cf31"
kind = "gap"
title = "RequireToolBeforeEdit admits an edit to a file the agent never read, once it has read any other file: T8 patched py/textutil.py having read only py/test_textutil.py"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gp2 golden-path rerun, live run 1 review (find-4003b9, gap-f30b8e; 2026-10-18)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-agent/src/safety/contract.rs::has_prior_tool", "crates/roko-agent/src/safety/contract.rs::GovernanceRule"]
links = { depends_on = [], blocks = [], related = ["q-2f780c", "bug-c8153a", "gap-2e455d", "gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_read_of_another_file_does_not_admit_an_edit' crates/roko-agent/src && cargo test -p roko-agent --lib a_read_of_another_file_does_not_admit_an_edit"
+++

## Problem

`RequireToolBeforeEdit` lets an agent edit an existing file it has not read, if it has read any other file. Seen live
in run 1 of the golden-path rerun (`15837120f`, 2026-10-18; Graph run `graph-09f0bd03-b7ec-463f-b311-5d6409516cac`),
in the run's `.roko/tool_audit.jsonl`. The independent review spotted it, and the operator checked the audit rows:

- T1's and T7's first `apply_patch` came before any successful `read_file`. Both were refused: "contract violation
  for role `implementer` (RequireToolBeforeEdit): tool `read_file` must run before `apply_patch`".
- T8 read only `py/test_textutil.py` (its read of `py/test_accept_word_list.py` failed: the file did not exist yet).
  Its `apply_patch` of `py/textutil.py` then went through: "patched …/py/textutil.py (2 hunk(s))".

## Why it matters

Will's decision on q-2f780c (2026-10-15) keeps "a read tool call before any write to an existing file", because a
partial or stale view must not stand in for a read: "the blind overwrite the rule exists to stop". The code's own
comment says the same ("don't overwrite what you haven't read"). As built, the rule stops only an agent that has read
nothing. An agent that reads a test file, then rewrites the module from the prompt's first 100 lines, passes it.
Safety contracts in this codebase fail closed. Here the rule enforces less than it says.

## Where

- `crates/roko-agent/src/safety/contract.rs::GovernanceRule` (the `RequireToolBeforeEdit` arm of its check): an
  edit tool on an existing file is refused only when `has_prior_tool(ctx, "read_file")` is false.
- `crates/roko-agent/src/safety/contract.rs::has_prior_tool`: true for any successful call of the tool in the
  session's history, whatever path it read.

## Current state

Checked at `15837120f`: as above. The new-file exception (a `write_file` to a path that does not exist) is right and
stays.

## Plan

1. Record each successful read's path, canonicalized against the worktree (the read history already holds the call's
   arguments). For an edit tool on an existing file, require a successful read of that same path in the session. For
   `apply_patch` and `multi_edit`, require it for every file the call changes.
2. Keep the refusal message, and name the unread file in it (gap-2e455d's feedback names the refused edit).
3. Tests: a read of another file does not admit an edit; a read of the same file does; a new file needs no read.

## Done when

- [ ] The `[[verify]]` test passes, and fails on `15837120f`.

## Notes

- Related: q-2f780c (decision, done), bug-c8153a, gap-2e455d. Found by the gp2 golden-path rerun (find-4003b9,
  gap-f30b8e); evidence in `tmp/status-quo/2026-10-16-live/evidence/gp2/golden-path/run1/` (`review.md`, and the
  `tool_audit.jsonl` rows at ts 1792346905089, 1792346914203, 1792346915263 and 1792346938668 in the run's scratch
  repo).
