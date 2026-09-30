+++
id = "bug-41bea4"
kind = "bug"
title = "Guard gaps: the roko.toml content check misses grep -r, parallel isn't treated as a bulk delete, and sudo git -C dir rm -r is a false positive"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "0b84bc9fa"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/gap-e9660f at 6820f1c2d)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["gap-e9660f"], blocks = [], related = ["gap-e9660f", "bug-997c6a", "bug-ceab60"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json,subprocess,sys; g='crates/roko-agent/src/claude_cli_guard.py'; run=lambda c: subprocess.run(['python3',g],input=json.dumps({'tool_name':'Bash','tool_input':{'command':c}}),text=True,capture_output=True).returncode; sys.exit(0 if run('parallel rm ::: a b c') == 2 and run('sudo git -C dir rm -r x') == 0 else 1)\""
+++

## Problem

These guard gaps remain on `work/gap-e9660f`:

- **`grep -r`:** the content check that keeps a secret-holding `roko.toml` from agents doesn't catch `grep -r` over the project (reported by wk-guard2).
- **`parallel`:** it isn't treated as a bulk runner. `parallel rm ::: a b c` passes the branch's guard (exit 0).
- **A false positive:** `sudo git -C dir rm -r x` is blocked (exit 2), although `git rm` removes tracked files that git can restore.

## Why it matters

Secrets and guard (epic spec-ba7bea): gaps let reads and deletes through, and false positives train people to disable the guard. p3.

## Where

`crates/roko-agent/src/claude_cli_guard.py` on the branch: the content check, the wrapper and bulk-runner rules, and the recursive-rm rule.

## Plan

1. Apply the content check to recursive greps (`grep -r`, `grep -R`, `rg` without a path filter) over a directory holding a secret-bearing `roko.toml`.
2. Treat `parallel` like `xargs`: check the command it runs.
3. Recognise `git … rm` (after `sudo` and `-C dir`) as a git subcommand, not as `rm`.
4. Add the cases to the guard's tests.

## Done when

- [ ] All three behave as described.
- [ ] The `[[verify]]` command passes: it requires `parallel rm` to be blocked and `sudo git -C dir rm -r x` to be allowed.

## Notes

- Build on gap-e9660f's branch.
- Premise held at 0b84bc9fa: `parallel rm ::: a b c` exited 0, `sudo git -C dir rm -r x` exited 2, and `grep -r` over a directory whose `roko.toml` held a secret ran.
- Decisions: a recursive search (`grep -r`/`-R`/`--recursive`/`-d recurse`, `rgrep`, `rg`) is denied when the tree it searches holds a roko config file with a secret: the `roko.toml` in a searched directory, or the workspace's `roko.toml` (from the call's directory upwards), the `ROKO_CONFIG` file or the legacy global config when the tree covers them, so `grep -r x ..` from a subdirectory is caught. grep's `--include`/`--exclude` and rg's `-g`/`-t`/`-T` filters that leave `roko.toml` out let it run, and `rg --files` passes. A whole word's glob is expanded as the shell would (`cat *`, `grep x *`); an option's value stays literal (`--exclude=*.toml`). The Grep tool uses the same tree rule. `parallel` is a bulk runner like `xargs`. The wrapper scan skips the words git reads itself (its global options' values and subcommand).
- Not covered: `git grep`, `ag`/`ack`, reads through `find`, `xargs` or a script, brace expansion (`{a,b}`), and a search after `cd`, which is judged from the call's directory. A NUL byte in a word still fails closed, as before.
- The Rust hook tests gain the cases; the `[[verify]]` passes, and the guard's scratch suites (earlier ones plus parallel, git subcommand and search cases) pass under python 3.12 and 3.9 through the exact hook command.
- Implemented on `work/bug-41bea4` at `e7a2291df`; cargo verification deferred to the batch check.
