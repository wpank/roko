+++
id = "bug-fa1537"
kind = "bug"
title = "A recursive search reaches .roko key files: grep -r OPENAI . with a .roko/.env present passes both guards"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard", "roko-std/sandbox"]
created = 2026-10-01
updated = 2026-10-01
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
