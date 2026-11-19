+++
id = "q-f28c6d"
kind = "question"
title = "Should RequireToolBeforeEdit count a run's own write_file of a file as a read of it?"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-78cf31 fix (98fb1ac61, 2026-10-27): an edit of an existing file now needs a read of that same file"
discovered_from = "gap-78cf31"
anchors = ["crates/roko-agent/src/safety/contract.rs::check_read_before_edit"]
links = { depends_on = [], blocks = [], related = ["gap-78cf31", "q-2f780c", "gap-2e455d"], supersedes = [], duplicate_of = "" }
+++

## Problem

Since gap-78cf31, `RequireToolBeforeEdit` admits an edit of an existing file only after a successful read of that
same file in the run (`crates/roko-agent/src/safety/contract.rs::check_read_before_edit`). A file the run created
itself with `write_file` (no read needed: it did not exist) and then wants to change again now needs a `read_file`
of it first. Before gap-78cf31 any earlier read of any file admitted that second write, so an agent that creates a
file, runs its checks and then rewrites or patches the file now spends one extra call per such file, after a
refusal that names the file ("tool `read_file` must run on `x.py` before `edit_file` changes it").

## Why it matters

Goal: truth (the safety contract enforces what it says) against the cost of refusals in real attempts. The rule's
intent is "don't overwrite what you haven't read". A run that wrote a whole file knows every byte it wrote, unless
something changed the file since (a `bash` formatter, `run_tests`, a build step), which a read guards against just
as poorly once time has passed. Counting the run's own successful `write_file` of a file as a read of it would
remove the extra call. It would also make the rule trust a write as a view, which q-2f780c declined for prompt
injection ("a partial or stale view must not stand in for a read").

## Where

- `crates/roko-agent/src/safety/contract.rs::check_read_before_edit` and `has_prior_tool` (which history entries
  count).
- `crates/roko-agent/src/safety/contract.rs::record_tool_result` (records the resolved file each call names).

## Why this needs Will

Two options:

1. **Keep it: only a read tool call counts.** Strict and auditable; one extra call per self-created file the run
   edits again, after a refusal that says which file to read.
2. **Also count the run's own successful `write_file` of the whole file** (not `edit_file` or `apply_patch`, whose
   result the agent has not seen whole). Saves the call; a file changed by another tool after the write would then
   be overwritten unread.

Recommendation: keep option 1 until attempt records show these refusals costing real attempts (search the tool
audit for `RequireToolBeforeEdit` refusals of files the same run wrote first).

## Notes

- Raised while fixing gap-78cf31 (branch `work/gap-78cf31`). Related: q-2f780c, gap-2e455d.
