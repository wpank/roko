#!/usr/bin/env bash
# Install roko's work skills (/work-next, /work-batch, /work-sweep) at user level, so that a Claude Code session
# started in any worktree of this repository gets them (dec-b75b96, option 1; gap-d6fd3c). See tools/work_skills.md.
#
# The skill sources stay in the main checkout's .claude/skills/, which git does not track (.git/info/exclude). This
# script symlinks each skill directory into ~/.claude/skills/ (or $CLAUDE_SKILLS_DIR). The skills find the main
# checkout at run time with `git rev-parse --git-common-dir`, so the links work from every worktree, and in other
# repositories the skills stop with a one-line message.
#
# Usage: tools/install_work_skills.sh [--check | --uninstall]
#   (no flag)    create or refresh the links; never replaces anything that is not a link into this checkout
#   --check      report each skill; exit 1 unless all three are linked here and name no absolute checkout path
#   --uninstall  remove the links that point into this checkout
set -euo pipefail

skills=(work-next work-batch work-sweep)
here=$(cd "$(dirname "$0")" && pwd -P)
common=$(git -C "$here" rev-parse --git-common-dir)
case $common in /*) ;; *) common="$here/$common" ;; esac
main=$(cd "$common/.." && pwd -P)
src="$main/.claude/skills"
dest="${CLAUDE_SKILLS_DIR:-$HOME/.claude/skills}"
mode=${1:-install}

case $mode in install | --check | --uninstall) ;; *) echo "usage: $0 [--check | --uninstall]" >&2; exit 2 ;; esac

problems=0
for s in "${skills[@]}"; do
    link="$dest/$s"
    target="$src/$s"
    case $mode in
    install)
        if [[ ! -f "$target/SKILL.md" ]]; then
            echo "$s: no $target/SKILL.md; the skills live in the main checkout's .claude/skills/" >&2
            problems=1
        elif grep -q -F "$main" "$target/SKILL.md"; then
            echo "$s: SKILL.md still names $main; it must find the checkout with git rev-parse" >&2
            problems=1
        elif [[ -e "$link" && ! -L "$link" ]]; then
            echo "$s: $link exists and is not a link; leaving it alone" >&2
            problems=1
        else
            mkdir -p "$dest"
            ln -sfn "$target" "$link"
            echo "$s: $link -> $target"
        fi
        ;;
    --check)
        if [[ -L "$link" && "$(readlink "$link")" == "$target" ]] && ! grep -q -F "$main" "$target/SKILL.md"; then
            echo "$s: ok ($link -> $target)"
        else
            echo "$s: not installed (want $link -> $target, with no absolute checkout path in SKILL.md)"
            problems=1
        fi
        ;;
    --uninstall)
        if [[ -L "$link" && "$(readlink "$link")" == "$target" ]]; then
            rm "$link"
            echo "$s: removed $link"
        else
            echo "$s: no link of ours at $link"
        fi
        ;;
    esac
done
exit $problems
