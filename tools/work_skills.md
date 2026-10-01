# The work skills at user level

roko's tracker procedure (`work/README.md`, "For agents") comes with three Claude Code skills: `/work-next` does one
item, `/work-batch` runs several in parallel, and `/work-sweep` re-checks drifted items. Their sources live in the
main checkout's `.claude/skills/work-{next,batch,sweep}/SKILL.md`. git doesn't track them (`.git/info/exclude`), and a
git worktree holds only tracked files, so a session started inside a worktree has no project skills. Will chose to
install them at user level (dec-b75b96, option 1), so every session on the machine has them.

## Install

Run once, from the main checkout or from any of its worktrees:

```bash
tools/install_work_skills.sh
```

It symlinks the three skill directories into `~/.claude/skills/` (set `CLAUDE_SKILLS_DIR` to use another directory).
It never replaces anything that isn't one of its own links, and it refuses a skill whose `SKILL.md` still names the
checkout's absolute path. Running it again refreshes the links.

## Check

```bash
tools/install_work_skills.sh --check
```

This prints one line per skill, and exits 1 unless all three are linked into this checkout and name no absolute
checkout path. Then start a Claude Code session inside a worktree (for example `../roko-work-<id>`), and confirm that
`/work-next` is in its skill list.

## Uninstall

```bash
tools/install_work_skills.sh --uninstall
```

This removes only the links that point into this checkout.

## How the skills find the checkout

Each skill's first step runs `MAIN=$(cd "$(git rev-parse --git-common-dir)/.." && pwd -P)`. git's common directory is
the main checkout's `.git`, from whichever worktree the session runs in. The skill then uses `$MAIN` for the merge lock,
the shared target directory and the sweep brief. In a repository without `$MAIN/work/items/`, the skill replies "No
roko work tracker here (work/items/ not found)." and stops.

## Not covered

`.claude/settings.json` (the `work.py hook` PostToolUse hook and the permission allow-list) stays project-level. A
session inside a worktree doesn't get the hook. Moving it to user settings is a separate decision.
