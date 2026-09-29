+++
id = "gap-d6fd3c"
kind = "gap"
title = "Install the work skills at user level so sessions in worktrees get them"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = [".claude/skills"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "work/items dec-b75b96 (decided 2026-09-29: option 1)"
anchors = [".claude/skills/work-next/SKILL.md", ".claude/skills/work-batch/SKILL.md", ".claude/skills/work-sweep/SKILL.md"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -L ~/.claude/skills/work-next && test -L ~/.claude/skills/work-batch && test -L ~/.claude/skills/work-sweep && ! grep -rq '/Users/will/dev/nunchi/roko/roko' .claude/skills/work-next .claude/skills/work-batch .claude/skills/work-sweep"
+++

## Problem

The `work-batch`, `work-next` and `work-sweep` skills live in the main checkout's `.claude/skills/`, which git
excludes (`.git/info/exclude:7`). A git worktree holds only tracked files, so a Claude session started inside a
worktree has none of these skills. The skill files also hard-code the path `/Users/will/dev/nunchi/roko/roko`.

## Why it matters

Will chose on 2026-09-29 to install the skills at user level (dec-b75b96, option 1). Lane sessions that live in their
own worktrees need the tracker's procedure: claim, branch, verify, close. This item is part of epic spec-1e1b45.

## Where

- `.claude/skills/work-{batch,next,sweep}/SKILL.md` in the main checkout.
- The target is `~/.claude/skills/`.

## Current state

At HEAD the skills exist only in the main checkout, and nothing is installed at user level.

## Plan

1. **Find the main checkout at run time.** Replace the hard-coded path with a lookup: `git rev-parse
   --git-common-dir` gives the main checkout's `.git`, and its parent is the checkout.
2. **Symlink** the three skill directories into `~/.claude/skills/`.
3. **Stop cleanly elsewhere.** In a repo without `work/items/`, each skill stops with a one-line message.
4. **Leave settings alone.** `.claude/settings.json` (the hook and the permission allow-list) stays project-level;
   it is out of scope here.

## Done when

- [ ] The three skills resolve from `~/.claude/skills/`.
- [ ] No skill file contains `/Users/will/dev/nunchi/roko/roko`.
- [ ] A session started inside a worktree lists `/work-next`. Check this once by hand and note it in the evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- This user-level change was approved by Will (dec-b75b96, 2026-09-29).
- Do not track `.claude/` in git.
