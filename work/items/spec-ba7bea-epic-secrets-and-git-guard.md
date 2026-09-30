+++
id = "spec-ba7bea"
kind = "spec"
title = "Epic: secrets and git guard"
status = "open"
triage = "unverified"
severity = "p0"
goal = "release"
size = "L"
subsystem = ["roko-agent/claude_cli_agent", "roko-agent/safety", "roko-gate/shell", "roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e3"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #3); workstreams/assessment/W8-roko-as-executor.md (gate G2)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::build_settings_json", "crates/roko-gate/src/shell.rs::ShellGate", "crates/roko-agent/src/safety/path.rs::canonicalize_with_policy", "crates/roko-cli/tests/secrets_and_git_guard_canary.rs"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-cold"
links = { depends_on = ["bug-7d7200", "gap-8be530", "gap-5f4852", "bug-7de5df", "bug-a66941", "gap-0e2c40", "gap-8f8544", "bug-f4e133", "bug-66f5a1", "bug-63327d", "find-570af2", "gap-b7a2d5", "bug-6930e6", "gap-a3fc5b", "gap-585bd2", "bug-3f3990", "bug-62e7e6", "bug-0bc728", "bug-ceab60", "bug-34c16c", "bug-a9a251", "gap-e9660f", "bug-c6ad88", "bug-997c6a", "bug-41bea4", "bug-5a6636", "bug-69a002", "bug-cef888"], blocks = [], related = ["spec-ae5f94"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn secrets_and_git_guard_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test secrets_and_git_guard_canary"
+++

## Problem

Agents run in the operator's checkout, within reach of the operator's keys and uncommitted work:

- child processes (agents and verify commands) inherit roko's whole environment, including the keys loaded from
  `~/.roko/.env` (bug-7d7200);
- agents can read the key files themselves (bug-a66941);
- Claude CLI runs load the user's own `~/.claude` settings, hooks and plugins (gap-8be530);
- nothing proves that a key never reaches a persistent file (gap-5f4852);
- the only command guard is a narrow regex hook that fails open and misses `git reset`, `stash` and `clean`
  (bug-7de5df).

## Why it matters

- **Release:** this is a security defect in a public repository, and tldr/05 P0 #3.
- **Executor safety:** W8 bars Roko from the main checkout until gate G2 holds.
- **Benchmark:** W10 found the benchmark's `VB_SECRET` in `~/.roko/.env`, so the bench's check SC4 fails by
  construction until E3 lands.
- **Operator's work:** research note B4 shows an agent can `git stash` or `git reset --hard` the operator's work
  today.

## Where

- `crates/roko-gate/src/shell.rs::ShellGate`, and the Claude CLI child environment in `claude_cli_agent.rs`
  (bug-7d7200).
- `crates/roko-agent/src/claude_cli_agent.rs::build_settings_json`: the hook, and the key-file deny rules.
- `crates/roko-agent/src/safety/path.rs`: roko's own file tools.
- `crates/roko-fs/src/observability.rs::RunScrubber` and `crates/roko-cli/tests/`.

## Current state

Checked at `41c7ffbd6`:
- **bug-7d7200 is in flight.** Branch `fix/hermetic-child-env` has no commits yet. Its worktree `../roko-wt-env`
  holds about 45 uncommitted files (+636/−86):
  - it adds `roko_core::child_env`;
  - it touches `claude_cli_agent.rs` (not `build_settings_json`), `shell.rs`, `gate_dispatch.rs`,
    `graph_task_dispatch.rs` and `main.rs`.
- **The other existing children are still open.** The static parts of their verify commands still fail: no
  `--setting-sources`, and nothing outside `roko-fs` uses `RunScrubber`.
- **The C2 test file does not exist yet.** It is anchored at the path it will have.

## Plan

This is the implementation plan.

1. **Now (cold, one agent):** bug-7de5df, the hook.
2. **Wait for bug-7d7200** to merge from the portal session.
3. **Then, one at a time in `claude_cli_agent.rs`:** bug-a66941 (key files), then gap-8be530 (`--setting-sources`).
   gap-5f4852 (scrubbers and the canary plan run) can run in parallel with them.
4. **Last: integration test C2** (gap-0e2c40). It can be written sooner, but it merges after the fixes.

## Done when

- [x] bug-7d7200: Agents and verify commands inherit roko's whole environment, including provider API keys (existing item)
- [x] gap-8be530: Claude Code runs load the user's own ~/.claude settings, hooks and plugins (existing item)
- [x] gap-5f4852: Secret-canary persistence test never run for scrubbers/persistent sinks (existing item)
- [x] bug-7de5df: The agent git guard misses reset, stash and clean, and commands after the first in a chain
- [x] bug-a66941: Agents can read the provider key files, such as ~/.roko/.env
- [x] gap-0e2c40: Integration test C2: no provider key reaches an agent, a gate or a log, and the git guard denies destructive commands
- [ ] gap-8f8544: Sandbox levels are enforced only in-process: no OS sandbox confines agent processes
- [x] bug-f4e133: The agent command guard lets recursive rm through under sudo, -R, subshells and sh -c
- [x] bug-66f5a1: A git alias such as co = checkout bypasses the agent git guard
- [x] bug-63327d: roko-std's file tools don't check key files, so a dispatch without SafetyLayer can read .roko/.env
- [x] find-570af2: When HOME is the workdir, the key-file policy refuses agents the whole .roko directory
- [x] gap-b7a2d5: roko chat and dispatch_v2 spawn claude without the Claude Code isolation flags
- [x] bug-6930e6: find_mcp_config hands Claude runs an ancestor directory's or $HOME's .mcp.json
- [x] gap-a3fc5b: Claude Code isolation doesn't cover shell snapshots, and a managed-mcp.json makes Claude refuse --strict-mcp-config
- [x] gap-585bd2: Every implementer is offered the 17 chain tools, transfer and swap included, whatever the task domain
- [ ] bug-3f3990: The Linux firejail plugin sandbox ignores sandbox.allowed_paths and filesystem_write, which macOS Seatbelt enforces
- [x] bug-62e7e6: roko-std's bash tool and SafetyLayer's bash policy never check commands for key files
- [x] bug-0bc728: The command guard misses command strings passed to wrappers, find -exec and -delete, and busybox rm
- [x] bug-ceab60: The agent command guard lets deletes through find | xargs rm, fd -x rm, and command strings given to ssh or parallel
- [x] bug-34c16c: The project roko.toml can hold serve.auth.api_key, and agents can read it
- [x] bug-a9a251: roko chat's resolve_mcp_config still falls back to ~/.claude/mcp-config.json
- [x] gap-e9660f: Whole-project reads such as grep -r, rg or cat * can still show agents a secret stored in roko.toml
- [x] bug-c6ad88: ACP's builtin tools don't check for key files
- [x] bug-997c6a: ls | xargs rm and xargs rm < list still pass the agent command guard
- [x] bug-41bea4: Guard gaps: the roko.toml content check misses grep -r, parallel isn't treated as a bulk delete, and sudo git -C dir rm -r is a false positive
- [ ] bug-5a6636: Config secrets other than provider api_key_env (extra_headers, file secrets, serve.auth.api_key) aren't added to the log scrubber
- [ ] bug-69a002: The guard's secret-read check misses git grep, ag/ack, reads through find or xargs and brace globs, and judges a search after cd from the wrong directory
- [ ] bug-cef888: Every .env value of 8 or more characters counts as a secret, so non-secret settings kept in .env are redacted from records
- [ ] The epic's `[[verify]]` command (test C2) passes on the merged branch.

## Notes

- **One writer at a time in `claude_cli_agent.rs`.** bug-7d7200, gap-8be530, bug-7de5df and bug-a66941 all edit
  it, and so do bug-690dc6 and gap-ad0d39 in E4. Their anchors keep `work.py next` from running them together.
- **Existing children keep their goal and severity:** gap-8be530 stays `core` (p2).
- **Proposed split of gap-5f4852 (L):**
  - (a) wire `RunScrubber` into the persistent writers (episodes, efficiency, costs, activities, transcripts), M;
  - (b) the plan-run canary `canary_absent_from_every_file_after_plan_run`, S, which shares helpers with C2.
- **Not in this epic:**
  - Codex, Cursor and Gemini CLI agents get no command guard at all;
  - W8 hazard: the Claude CLI agent skips permission prompts by default;
  - an OS sandbox (B4).

  The author may want these filed.
- **E16:** bug-7d7200 is also a release blocker there.
- **Decided 2026-09-29 (Will):** no OS sandbox in v1. The git guard is best effort, and the whitepaper and README document this as a limitation; revisit it before any hosted demo. bug-7d7200 closed in 70820a74c.
- **Still open (not accepted on 2026-09-29):** whether the Codex, Cursor and Gemini CLI agents get a command guard (they have none).
