+++
id = "bug-ceab60"
kind = "bug"
title = "The agent command guard lets deletes through find | xargs rm, fd -x rm, and command strings given to ssh or parallel"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-f4e133", "bug-66f5a1", "bug-7de5df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json,subprocess,sys; g='crates/roko-agent/src/claude_cli_guard.py'; cmds=['find . -name x | xargs rm','fd -e rs -x rm','ssh host rm -rf /srv/app','parallel rm -rf ::: a b']; sys.exit(any(subprocess.run(['python3',g],input=json.dumps({'tool_name':'Bash','tool_input':{'command':c}}),text=True,capture_output=True).returncode != 2 for c in cmds))\""
+++

## Problem

The Claude CLI command guard (`crates/roko-agent/src/claude_cli_guard.py`) blocks `rm -rf`, `find -delete` and `find -exec rm`. At 7fa54b873, running the guard on these commands exits 0, meaning allowed:

- `find . -name "*.rs" | xargs rm`: `xargs` is handled as a wrapper, but only a recursive `rm` is refused, and a non-recursive rm over find's output deletes across the tree just like `find -exec rm`;
- `fd -e rs -x rm`: fd's exec flag isn't recognised;
- `ssh host 'rm -rf /srv/app'` and `parallel rm -rf ::: a b`: the guard doesn't look inside the command strings these programs run.

`rm -rf build` and `find . -exec rm {} +` exit 2 (blocked), as expected.

## Why it matters

Secrets and guard (epic spec-ba7bea): each form deletes the same trees the guard exists to protect. bug-f4e133, bug-62e7e6 and bug-0bc728 closed earlier forms.

## Where

The guard's rules for wrappers and `find`, around :28-33, :78, :132, :211 and :238-264.

## Plan

1. Treat `find … | xargs rm` like `find -exec rm`: any `rm` fed by `xargs` from `find` or `fd`.
2. Recognise `fd -x/--exec` and `-X/--exec-batch`.
3. Check the command strings of `ssh`, `parallel` (and `xargs sh -c`) as commands in their own right, or refuse them when they contain a delete.
4. Add these four cases to the guard's tests.

## Done when

- [ ] The guard blocks all four forms.
- [ ] The `[[verify]]` command passes: it runs the guard on each form and expects exit 2.
