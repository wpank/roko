+++
id = "dec-b75b96"
kind = "decision"
title = "Decide how the work skills reach sessions started inside worktrees"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = [".claude/skills", "tools/work"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "70820a74c"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (§4, R0.2)"
anchors = [".claude/skills/work-batch/SKILL.md", ".claude/skills/work-next/SKILL.md", ".claude/skills/work-sweep/SKILL.md", ".claude/settings.json"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = [], blocks = [], related = ["gap-92033c"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "Will (decided 2026-09-29)"
evidence = "Will decided 2026-09-29: option 1, install the skills at user level. Follow-up item: gap-d6fd3c."
+++

## Problem

The `work-batch`, `work-next` and `work-sweep` skills, and the `work.py hook` PostToolUse hook, live in `.claude/`,
which `.git/info/exclude:7` keeps out of git. A git worktree contains only tracked files, so a Claude session started
inside a worktree (`roko-wt-*`, `roko-work-<id>`) has no `/work-next` and no commit hook. W7's operating model gives
each Rust lane a long-lived session in its own worktree, and those sessions would work without the tracker's
procedure.

## Why it matters

Goal `tooling`, epic spec-1e1b45. The answer also decides where gap-92033c's skill edits go.

## Where

- `.claude/skills/work-{batch,next,sweep}/SKILL.md`: local-only; they hard-code `/Users/will/dev/nunchi/roko/roko`.
- `.claude/settings.json`: the hook, plus a personal allow-list (`Bash(cargo *)`, `gh pr …`).

## Current state

Checked at `41c7ffbd6`. The skills' own workers need nothing: they get a self-contained prompt. Only sessions started
inside a worktree lack the skills.

## Plan

The options:

1. **Install at user level:** symlink the three skills into `~/.claude/skills/`, and find the main checkout with
   `git rev-parse --git-common-dir` instead of the hard-coded path. Every session on the machine gets them; in other
   repos they find no `work.py` and stop. No repo change. The hook can move to user settings too:
   `cmd_hook` already returns silently outside a repo with `work/items`.
2. **Track `.claude/skills/` in git** (`git add -f`), leaving `settings.json` out. Worktrees and outside contributors
   get the skills; the public repo then carries Claude-specific procedures, so scrub absolute paths first.
3. **Keep them local:** start every session in the main checkout and let the skill create the worktree. No change,
   but it depends on discipline, and W7's lane sessions don't fit it.

**Recommended:** option 1 now; revisit option 2 when the repo takes outside contributors.

## Done when

- [ ] Will picks an option, and the choice is recorded in `[closed].evidence`.

## Notes

- `.git/info/exclude` and user-level settings change only with Will's approval (standing rule on ignore files).
