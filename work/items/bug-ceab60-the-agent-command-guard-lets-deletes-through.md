+++
id = "bug-ceab60"
kind = "bug"
title = "The agent command guard lets deletes through find | xargs rm, fd -x rm, and command strings given to ssh or parallel"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
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

## Notes

- Premise confirmed at `942d2a6c3`: the item's verify ran the guard on all four forms and each exited 0.
- The guard now tracks pipes: a command that a pipe feeds with what find or fd lists (`find . | xargs rm`, `find . | while read f; do rm "$f"; done`, through compound commands until the pipeline ends), and any command in a line where a substitution runs find (`rm $(find ...)`), is checked like `find -exec`, so any `rm` in it is denied. fd/fdfind `-x`, `--exec`, `-X` and `--exec-batch` are checked the same way. ssh joins the words after the destination and checks them as the remote command, and checks `-o ProxyCommand`/`LocalCommand`/`RemoteCommand`/`KnownHostsCommand` values; ssh's option values and destination are never read as programs (`ssh git ls`). parallel is a wrapper whose every argument is checked as a command line. A wrapper inside a wrapper is followed (`sudo parallel 'rm -rf {}' ::: a`).
- Over-approximations (fail closed): a substitution that runs find marks the whole line (`x=$(find . | wc -l); rm tmp` is denied), and any rm fed by find is denied, not only a recursive one. `ls | xargs rm` and `xargs rm < list` still pass: nothing ties them to find.
- The item's verify (Python) passes. The four forms and their neighbours are in `settings_hook_denies_destructive_commands_behind_wrappers`.
- Implemented on `work/bug-ceab60` at `2a2b4d4e0`; cargo verification deferred to the batch check.
