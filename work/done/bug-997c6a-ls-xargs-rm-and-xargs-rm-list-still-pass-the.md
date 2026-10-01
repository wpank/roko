+++
id = "bug-997c6a"
kind = "bug"
title = "ls | xargs rm and xargs rm < list still pass the agent command guard"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-ceab60, branch work/bug-ceab60 at f80d4eb50)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["bug-ceab60"], blocks = [], related = ["bug-ceab60", "bug-f4e133"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json,subprocess,sys; g='crates/roko-agent/src/claude_cli_guard.py'; cmds=['ls | xargs rm', 'xargs rm < list.txt']; sys.exit(any(subprocess.run(['python3',g],input=json.dumps({'tool_name':'Bash','tool_input':{'command':c}}),text=True,capture_output=True).returncode != 2 for c in cmds))\""

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in e1e6159f7. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: the guard denies 'ls | xargs rm' and 'xargs rm < list.txt' (python verify exit 0 on the merged tree)."
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
- Premise held at 8a88c6267: `ls | xargs rm` and `xargs rm < list.txt` exited 0.
- Decision: deny any delete (`rm`, `unlink`, `shred`) that `xargs` runs, whatever feeds it (a pipe, a redirect, `-a list`), rather than allow a short literal list. `xargs` running anything else (`xargs cat < list.txt`) stays allowed, and so does `git ls-files -z | xargs -0 git rm --cached`: a word after `git` is a subcommand, not a program, unless a value option precedes it (`sudo -u git rm -rf x` is still checked).
- The guard test in `claude_cli_agent.rs` gains the four denied forms and the two allowed ones; the `[[verify]]` (python) passes, and the guard's scratch suites pass under python 3.12 and 3.9.
- Not covered: `parallel rm` is not treated as bulk, and `sudo git -C dir rm -r x` is still a false positive.
- Implemented on `work/gap-e9660f` at `802e91bd5`; cargo verification deferred to the batch check.
