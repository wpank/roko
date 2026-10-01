+++
id = "bug-fa1537"
kind = "bug"
title = "A recursive search reaches .roko key files: grep -r OPENAI . with a .roko/.env present passes both guards"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard", "roko-std/sandbox"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "49acd1711"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/bug-77413c at 49acd1711)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py", "crates/roko-std/src/tool/builtin/sandbox/reads.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-69a002", "bug-77413c", "bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json,os,subprocess,sys,tempfile; d=tempfile.mkdtemp(); os.makedirs(d+'/.roko'); open(d+'/.roko/.env','w').write('OPENAI_API_KEY=sk-test-123456789\\n'); g=os.path.abspath('crates/roko-agent/src/claude_cli_guard.py'); r=subprocess.run(['python3',g],input=json.dumps({'tool_name':'Bash','cwd':d,'tool_input':{'command':'grep -r OPENAI .'}}),text=True,capture_output=True,cwd=d); sys.exit(0 if r.returncode==2 else 1)\" && grep -rqw 'fn recursive_search_reaching_a_key_file_is_refused' crates/roko-std/src/ && cargo test -p roko-std --lib recursive_search_reaching_a_key_file_is_refused"
+++

## Problem

The search rules in both guards consider only roko config files (a secret-holding `roko.toml`):

- the Python guard (`crates/roko-agent/src/claude_cli_guard.py`, with bug-69a002 merged);
- the Rust port (`crates/roko-std/src/tool/builtin/sandbox/reads.rs`, on bug-77413c's branch).

A recursive search over a directory that contains `.roko/.env` or another key file reads it. Checked at a4e175c9c by running the Python guard in a scratch workspace with a `.roko/.env`:

- `grep -r OPENAI .` exits 0 (allowed);
- `cat .roko/.env` exits 2 (blocked).

## Why it matters

Secrets and guard (epic spec-ba7bea), p1: the most common search an agent runs reads the provider keys that the key-file rules (bug-a66941) exist to protect.

## Where

The recursive-search rules in both files.

## Plan

1. Treat a recursive search whose scope contains a key file (`.roko/.env`, `secrets.toml`, `credentials.json`, `~/.roko/config.toml`) like a direct read of it: refuse it, unless the command excludes the key files explicitly.
2. Add the case to both test suites, including `recursive_search_reaching_a_key_file_is_refused` for roko-std.

## Done when

- [ ] Both guards refuse a recursive search that would read a key file.
- [ ] The `[[verify]]` command passes: it runs the Python guard in a scratch workspace, then the Rust test.

## Notes

- Premise held at 49acd1711: with a `.roko/.env` present, `grep -r OPENAI .` passed the Claude guard (exit 0; this item's verify reproduces it) and roko-std's port, whose search rules looked only for roko config files.
- Rule, in both: a recursive search (`grep -r`/`rgrep`, `rg`, `ag`, `ack`, and `git grep` only with `--untracked` or `--no-index`, since a key file is never tracked) or a read of what find, fd or xargs lists is refused when it reaches a key file of the tree: in the tree's `.roko` (or the tree itself when it is one), in each subdirectory's down two levels (not under a hidden one, at most 4096 a level), in the workspace's above the command's or the call's directory, and in `~/.roko`. Secret-holding configs go through the same per-file check.
- Exclusions are judged per file: grep's `--include`, `--exclude` and `--exclude-dir`; rg's globs (a `!` glob also leaves out a directory above the file), `-t`, `-T`, `--type-add`, `--hidden`, `-.`, `-uu`, `--no-hidden` and `--max-depth`; ag's `-G`, `--ignore`, `-u` and `--depth`; ack's `-t`, `--ignore-dir` and `--ignore-file`; git grep's pathspecs and `--max-depth`; find's whole expression, parsed (`!`, `-o`, `,`, parentheses, name, path and type tests, `-prune`, `-maxdepth`, `-mindepth`; an emacs regex counts only when it matches); fd's pattern (`-g`, `-F`, `-p`), `-e`, `-E`, `-d`, `-H` and `-u`. rg, ag and fd skip hidden files without `--hidden` (rg also enters a hidden path an include glob names), and a list the check cannot see (`ls | xargs cat`) skips hidden directories, as `ls -a` does. Refusals name the fix: leave `.roko` out (`grep --exclude-dir=.roko`, `rg -g '!.roko'`) or name a narrower tree.
- Also fixed in the guard, since the find rule needs it: shlex made a quoted or escaped parenthesis an operator, so `find . \( ... \) -exec ...` lost its `-exec` command, and `find . \( -name x \) -exec rm -f {} +` passed at 49acd1711. A find with `-o` or `-prune -o` no longer counts as leaving roko.toml out.
- Tests: the shared table, `secret_read_cases.txt`, grows from 105 to 165 rows; its fixture holds `.roko/.env`, `.roko/secrets.toml` and `vendor/pkg/.roko/credentials.json`, removed with the secret for the all-allowed pass. `recursive_search_reaching_a_key_file_is_refused` (this item's verify) adds the workspace-above-the-command case, and roko-agent's destructive-command list gains two parenthesized finds.
- Known gaps: a `.roko` more than two levels below the searched root and not above the command's directory is not found; git grep trusts that key files are untracked; Claude Code's Grep tool runs `rg --hidden` and still has only the config rule (bug-6af02b).
- Checked without cargo: the guard passes the table, its other suites and this item's Python check under python 3.12 and 3.9. The real `sandbox.rs` and `reads.rs`, built standalone against the workspace's regex build, pass the table, the earlier key-file cases and the three tests, and lint clean with the workspace's clippy settings. A differential fuzz of 23,000 random tree reads, each run with and without the key files, found no disagreement between the guard and the port.
- Implemented on `work/guard-l9` at `4a75e9965` (`69394ee8b` plus an ASCII fix to the guard); cargo verification deferred to the batch check.
