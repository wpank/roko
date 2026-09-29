+++
id = "gap-92033c"
kind = "gap"
title = "The work skills add Work-Item, Executor and Conflicts trailers to merge commits"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = [".claude/skills"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (A3)"
anchors = [".claude/skills/work-batch/SKILL.md", ".claude/skills/work-next/SKILL.md"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = ["gap-0b9056"], blocks = [], related = ["dec-b75b96"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'Executor:' .claude/skills/work-batch/SKILL.md && grep -q 'Executor:' .claude/skills/work-next/SKILL.md && grep -q -- '--executor' .claude/skills/work-batch/SKILL.md && grep -q -- '--reason' .claude/skills/work-batch/SKILL.md && grep -q 'event merged' .claude/skills/work-next/SKILL.md"
+++

## Problem

The skills' merge commits say only `merge: <id> <title>`, with no machine-readable record of who executed the item or
whether the merge conflicted. Neither skill passes gap-0b9056's claim flags, gives a release reason, or logs gap-d0643c's
`merged` and `post-verify` events. Conflicts and hand fixups appear in prose, if at all (`188c43c8d` names three
conflicted files and two semantic fixes only in its message).

## Why it matters

Goal `proof`, epic spec-f2463d. The conflict rate, post-merge verify failures and executor attribution per commit
come from these trailers and events; the rollup (gap-ccb87e) reads them with `git log --merges`.

## Where

- `.claude/skills/work-batch/SKILL.md`: step 2 (claim), 4 (release), 5 (merge) and 6 (bookkeeping).
- `.claude/skills/work-next/SKILL.md`: steps 2, 7 and 8.
- Both are local-only: `.git/info/exclude:7` excludes `.claude/`, so worktrees lack them and git has nothing to commit.

## Current state

Checked at `41c7ffbd6`: no trailers, flags or events. The worker prompt in work-batch step 3 stays as it is (W12:
workers write no events).

## Plan

1. Claim with `--executor claude-agent --via work-batch --size <size>` (work-next:
   `--executor claude-session --via work-next`).
2. Release with `--reason verify-fail|blocked|decision-needed|conflict|timeout|session-limit`.
3. Merge with `git merge --no-ff --no-commit <branch>`, run the item's `[[verify]]` on the merged tree, then commit
   with the trailers `Work-Item: <id>`, `Executor: <executor>`, `Conflicts: <files git reported; 0 if clean>` and
   `Post-Merge-Verify: pass`. If the verify fails, `git merge --abort` (the skill's own merge) and release with
   `--reason verify-fail`.
4. Then `work.py event merged <id> --merge-sha <sha> --conflicts N` and `event post-verify <id> --rc 0`.
5. Bookkeeping commits the session's events file, and the day's manifest once gap-ccb87e lands, by explicit path.
6. New rule: one executor per commit; operator fixups get their own commit.

## Done when

- [ ] Both skills pass the claim flags and a release reason.
- [ ] Both skills write the trailers and log both events.
- [ ] The `[[verify]]` command passes in the main checkout.

## Notes

- Edit in the main checkout; close with `--evidence` summarising the skill diff. If dec-b75b96 moves the skills first,
  edit them where they now live.
- Step 3 reorders the README's merge rule ("Parallel work"): ask Will first. Without approval, drop the
  `Post-Merge-Verify` trailer and keep only the event.
- **Still open (not accepted on 2026-09-29):** reordering the README's merge rule so the check runs before the merge commit. Until Will approves it, drop the `Post-Merge-Verify` trailer and keep only the event.
