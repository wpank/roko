+++
id = "bug-997c6a"
kind = "bug"
title = "ls | xargs rm and xargs rm < list still pass the agent command guard"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-ceab60, branch work/bug-ceab60 at f80d4eb50)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["bug-ceab60"], blocks = [], related = ["bug-ceab60", "bug-f4e133"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json,subprocess,sys; g='crates/roko-agent/src/claude_cli_guard.py'; cmds=['ls | xargs rm', 'xargs rm < list.txt']; sys.exit(any(subprocess.run(['python3',g],input=json.dumps({'tool_name':'Bash','tool_input':{'command':c}}),text=True,capture_output=True).returncode != 2 for c in cmds))\""
+++

## Problem

bug-ceab60's fix (branch `work/bug-ceab60`, 2a2b4d4e0) makes the guard deny an `rm` fed by `find` or `fd`. Other feeds still pass. Running the branch's guard gives:

- `ls | xargs rm`: exit 0 (allowed);
- `xargs rm < list.txt`: exit 0;
- `find . -name x | xargs rm`: exit 2 (blocked).

Its notes say so: "`ls | xargs rm` and `xargs rm < list` still pass: nothing ties them to find."

## Why it matters

Secrets and guard (epic spec-ba7bea): these delete a whole listing just as the blocked forms do. p3, because they need a deliberate list.

## Where

The pipe-tracking rules in `claude_cli_guard.py` on the branch.

## Plan

1. Deny `xargs rm` (and `xargs` with any delete command) whatever feeds it: a pipe, a redirect or a file argument (`-a`). Or allow it only with an explicit, short, literal list.
2. Add both forms to the guard's tests.

## Done when

- [ ] The guard blocks both forms.
- [ ] The `[[verify]]` command passes: it runs the guard on both and expects exit 2.

## Notes

- Build on bug-ceab60's branch.
