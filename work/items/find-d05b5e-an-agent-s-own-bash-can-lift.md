+++
id = "find-d05b5e"
kind = "finding"
title = "An agent's own bash can lift an immune control: nothing stops it running roko safety release (now named in an isolated tool's refusal) against the workspace root, and the audit line records the person's $USER as by"
status = "open"
triage = "verified"
severity = "p3"
size = "M"
subsystem = ["roko-agent/safety", "roko-cli/commands"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
source = "gap-a2787f fix (2026-10-21)"
discovered_from = "gap-a2787f"
anchors = ["crates/roko-agent/src/safety/bash.rs::BashPolicy", "crates/roko-cli/src/commands/safety.rs::SafetyCmd", "crates/roko-agent/src/tool_immune.rs::check_tool_control", "crates/roko-cli/src/commands/safety.rs::roko_started_markers", "crates/roko-agent/src/immune_evidence.rs::ReleaseOrigin"]
links = { depends_on = [], blocks = [], related = ["gap-a2787f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn default_policy_refuses_roko_safety_release' crates/roko-agent/src/safety/bash.rs && grep -qw 'fn an_agents_bash_call_that_runs_the_release_is_refused' crates/roko-cli/src/commands/safety.rs && grep -qw 'fn release_refuses_in_an_agent_environment' crates/roko-cli/src/commands/safety.rs && grep -qw 'fn a_persons_release_still_works_and_records_how' crates/roko-cli/src/commands/safety.rs && grep -qw 'fn safety_release_refuses_in_a_claude_agents_shell' crates/roko-cli/tests/safety_cli.rs && cargo test -p roko-agent --lib safety::bash && cargo test -p roko-cli --lib commands::safety && cargo test -p roko-cli --test safety_cli"
+++

## Problem

An immune control is meant to be lifted by a person after review: gap-a2787f made `roko safety release` release
tool controls as well as agent controls, and an isolated tool's refusal now names the command
(`roko safety release --tool <tool> --scope <scope> --reason <why>`). Nothing stops an agent from running that
command itself through its `bash` tool:

- the default `BashPolicy` (`crates/roko-agent/src/safety/bash.rs::BashPolicy::with_defaults`) is a short denylist
  (`rm -rf /`, `sudo`, `curl | sh`, …) with nothing about `roko safety`;
- `run_tests` and `bash` keep `USER` in the child environment, and `roko safety release` records `by` as `$USER`
  when `--by` is absent, so a release an agent ran is recorded as the person's;
- from an attempt worktree, `roko safety release` without `--workdir` resolves to the worktree (it stops at the
  worktree's `.git`), whose ledger is not the canonical one; with `--workdir` pointing at the workspace root
  (a relative `../..` is not checked by the bash path confinement), it releases the control the boundary enforces.

The same holds for agent controls (`roko safety release <agent>`), though an agent never sees an agent-control
refusal, as those deny before dispatch.

## Why it matters

The isolation exists so that a tool source that returned a malformed or prompt-injecting result stays out of the
run until someone checks it. A model that can lift the control itself, under the person's name in the audit file,
makes that review optional and the audit line misleading.

## Where

- `crates/roko-agent/src/safety/bash.rs::BashPolicy::with_defaults`: what an agent's shell may not run.
- `crates/roko-cli/src/commands/safety.rs::SafetyCmd`: the release commands and their default `--by`.
- `crates/roko-agent/src/tool_immune.rs::check_tool_control`: the refusal that names the command.

## Current state

Not reproduced live: this is from reading the code at gap-a2787f's fix. Whether an agent's `PATH` holds a `roko`
binary depends on the install (a person's workspace usually does; roko's own self-hosting runs use
`target/debug/roko`).

## Plan

Options, not exclusive:

1. The bash and run_tests tools mark their child environment (say `ROKO_TOOL_CALL=1`), and `roko safety release`
   refuses to run under it, saying a person releases controls.
2. The default bash policy refuses `roko safety release` (a substring rule is easy to dodge, so only as a hint).
3. `roko safety release` asks for confirmation on a terminal unless `--yes`, and records whether it had one.

Decided 2026-10-26 (see the decision under Notes): option 2 as a rule on the command's shell words rather than a
substring, option 1 with the markers roko already sets instead of a new one, and an audit line that records
whether a terminal was attached (option 3's record, without the question).

## Done when

- [x] An agent's `bash` call that runs `roko safety release` is refused, or its audit line says it came from an
  agent's shell, and a test shows it: `an_agents_bash_call_that_runs_the_release_is_refused` (the dispatcher's
  safety layer refuses it before the command runs) and `default_policy_refuses_roko_safety_release` (the
  spellings).
- [x] `roko safety release` refuses in a process roko started for an agent or a check:
  `release_refuses_in_an_agent_environment` (each marker), `safety_release_refuses_in_a_claude_agents_shell` (the
  binary, in a Claude agent's environment).
- [x] A person's release still works, and its audit line says how it came in:
  `a_persons_release_still_works_and_records_how`, `safety_release_clears_isolation_control` and the serve route's
  `release_route_clears_isolation_control`.
- The `[[verify]]` command runs them.

## Notes

- Found while doing gap-a2787f. An agent with an unrestricted shell can also edit or delete the ledger file
  directly if it can reach it; this item is about not offering a supported path and not misattributing it.

2026-10-26 decision (the coordinator's conservative call under Will's decision 9: a technical question with no
recorded default takes the conservative option; he can veto it): agents must not release immune controls; only a
person may. It is built in three layers.

1. The bash policy: `BashPolicy::with_defaults` refuses an agent's `bash` command that runs `roko safety release`
   in any spelling the command's shell words show (`DenyPattern::RokoSubcommand`, `runs_roko_subcommand` in
   `crates/roko-agent/src/safety/bash.rs`). That covers any path to the binary, `cargo run -p roko-cli --`,
   `$ROKO_BIN` and `$(which roko)`, flags between the words, quotes and backslashes, chained commands, `$(…)` and
   backticks, `sh -c` and `eval` strings, and here-documents fed to a shell. A quoted mention
   (`grep -q 'roko safety release' CLAUDE.md`) is one word and passes. The refusal tells the agent to leave the
   control in force and name it in its hand-off. ACP's `bash` uses the same default policy.
2. The command: `roko safety release` refuses (exit 1, before anything is read or written) in a process roko
   started for an agent or a check. It tells them by variables roko already sets, not a new one
   (`roko_started_markers` in `crates/roko-cli/src/commands/safety.rs`): both of the Claude Code agents'
   `ISOLATION_ENV` pair, a variable naming a provider config in a `roko-cli-provider-*` directory (Gemini),
   `ROKO_TOOL_ARGS` (extension tools), the `ROKO_GATE_*` rung variables (gate commands and the tests they run,
   agent-written ones included), `ROKO_EFFECT_FILE` (effect receipt rungs) and `ROKO_CELL_INPUT` (Graph shell
   cells). The spawn sites now share those names through constants. A person at a terminal still releases, and so
   does `POST /api/safety/controls/{agent_id}/release`, which needs `ConfigEdit`; agent tokens fail closed there.
3. The audit line: both release files record `via` (`ReleaseOrigin`): the channel (`cli` or `serve_api`), where
   `by` came from (`flag`, `user_env`, `fallback`, `authenticated_user` or `local_api`), and for the CLI whether
   its standard input was a terminal. An isolated tool's refusal now tells the agent it cannot release it.

Why: the harm is an agent lifting a control under the person's name. Refusing in the agent's shell and in
roko-started processes removes the supported path, and the audit line no longer passes `$USER` off as a person's
choice. Using only existing markers keeps the change away from every spawn site.

What it leaves open: roko's own `bash` and `run_tests` commands and the Codex, Cursor, Hermes and OpenClaw agents
carry no roko marker. There only the bash policy stops the release, so a script the agent writes and runs gets
through, recorded with `by_source = "user_env"` and `terminal = false`. Filed as gap-31e8a7.

If vetoed (agents may release): drop the bash rule and the refusal, and record each marker found in the release's
`via` instead, so the audit line says the release came from an agent's environment.
