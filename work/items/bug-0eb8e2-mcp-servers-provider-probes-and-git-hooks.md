+++
id = "bug-0eb8e2"
kind = "bug"
title = "MCP servers, provider probes and git hooks still inherit roko's full environment, including provider keys"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-agent/mcp", "roko-agent/harness", "roko-cli"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/hermetic-child-env dc99a9e81"
anchors = ["crates/roko-agent/src/mcp/client.rs::spawn_with_env", "crates/roko-agent/src/mcp/client.rs::spawn_diagnostic", "crates/roko-agent/src/harness/probe_runner.rs::run_probe_command", "crates/roko-cli/src/commands/config_cmd.rs::run_claude_cli_provider_test", "crates/roko-cli/src/worker/cloud.rs::git_commit", "crates/roko-acp/src/runner.rs::run_commit"]
links = { depends_on = [], blocks = [], related = ["bug-7d7200", "spec-ba7bea", "gap-0e2c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn stdio_mcp_server_env_excludes_provider_keys' crates/roko-agent/src && cargo test -p roko-agent --lib stdio_mcp_server_env_excludes_provider_keys && grep -rqw 'fn probe_command_env_excludes_provider_keys' crates/roko-agent/src && cargo test -p roko-agent --lib probe_command_env_excludes_provider_keys"

[[verify]]
command = "grep -qE 'credential_scrub|CredentialScrub|hooksPath' crates/roko-cli/src/worker/cloud.rs && { ! grep -q 'async fn run_commit(' crates/roko-acp/src/runner.rs || grep -qE 'credential_scrub|CredentialScrub|hooksPath' crates/roko-acp/src/runner.rs; }"
+++

## Problem

dc99a9e81 (bug-7d7200) put two child-environment policies in `roko_core::child_env`: gates get an allowlist, and
provider CLIs lose other providers' keys, `.env`-loaded names and `ROKO_*` secrets (`CredentialScrub`). Three
kinds of child process roko starts itself still inherit roko's whole environment, including every key loaded from
`~/.roko/.env` and `.roko/.env`:

1. **MCP servers started by roko's own MCP client.** `StdioTransport::spawn_with_env` (`mcp/client.rs:231`, spawn
   at :237) and `spawn_diagnostic` (:294, spawn at :300) add the configured overlay to the full environment.
2. **Provider probes.** `run_probe_command` (`harness/probe_runner.rs:13`) runs the Hermes and OpenClaw probes,
   including `openclaw doctor --repair` (`openclaw/probe.rs:93`) and `openclaw config show` (:139).
   `run_claude_cli_provider_test` (`config_cmd.rs:2790`, `claude --version` at :2808) and the `--version`
   probes in `auth_detect.rs` (:98, :150, :195), `doctor.rs` (:1885, :1914) and `commands/setup.rs` (:294)
   do the same.
3. **Git hooks.** The repository's hooks run when roko commits, with roko's environment:
   - `worker/cloud.rs::git_commit` (:358, `git commit` at :431) and `git_push` (:455, `git push` at :473), used
     by `roko worker`;
   - `roko-acp/src/runner.rs::run_commit` (:1928, :1977), used by the `ROKO_ACP_LEGACY` pipeline;
   - `serve_runtime.rs::ensure_git_repo_for_runner` (:1074). It commits in a fresh repository, so only
     global hooks run.

   An agent can write `.git/hooks/post-commit` in its workdir. roko's next commit then runs that hook with the
   keys the agent's own environment no longer has.

## Why it matters

bug-7d7200's threat model applies here too: third-party MCP servers (npx packages), hook scripts and probe binaries
can read and spend every provider key the run never chose. Goal `release`, epic spec-ba7bea. The canary test
gap-0e2c40 covers agents, gates and logs, not these processes.

## Where

The anchors are the spawn sites. Production callers:
- `mcp::bridge::discover_mcp_runtime` (`bridge.rs:310`). It is called by `SharedAgentFactory::new`
  (`roko-cli/src/dispatch/factory.rs:123`), whenever an MCP config path is set, and by `openai_compat.rs:409`.
- `mcp::bridge::test_mcp_server` (`bridge.rs:405`), for `roko config mcp test`.
- `worker/cloud.rs::github_create_pr` (:498) and `roko-acp/src/bridge_events/tools.rs:128`, which spawn session
  MCP servers.

The helpers to reuse are `roko_agent::process::apply_credential_scrub` (`process/env.rs:79`, tokio `Command`) and
`CredentialScrub::names_to_strip` for std `Command`s.

## Current state

Checked at `33e107da1`:
- None of the files above mentions `CredentialScrub`, `apply_credential_scrub` or `env_clear`.
- `roko-mcp-code`, `roko-mcp-github` and `roko-mcp-stdio` spawn no processes themselves.
- MCP servers that the Claude CLI starts from `--mcp-config` inherit the CLI's environment, which is already
  scrubbed.
- The Graph worktree manager already disables hooks for its git mutations (`core.hooksPath=/dev/null`,
  `orchestrator/worktree/git_ops.rs:329`).
- `graph_execution/delivery.rs::git_merge` (:116) and `runner/merge.rs` `GitMergeBackend` (:249) would also run
  hooks, but they have no production caller.

## Plan

1. **MCP.** In both `StdioTransport` spawn functions, set the resolved overlay first, then call
   `apply_credential_scrub(&mut cmd, &CredentialScrub::default())`. The overlay's explicit values,
   including `${NAME}` references resolved by `resolve_env`, survive the scrub. Other shell variables, such as a
   `GITHUB_TOKEN` exported in the shell, also stay. Consider keeping `[agent] env_passthrough` too. Test
   `stdio_mcp_server_env_excludes_provider_keys`: spawn `sh -c env` through the transport with an overlay, then assert
   the overlay is present and a provider key is absent. Inject the parent environment, as `ShellGate` tests do with
   `parent_env`, rather than mutating the process environment.
2. **Probes.** Give `run_probe_command` a `CredentialScrub` (`for_kind(Hermes | OpenClaw)`), and add a small
   std-`Command` helper for the `--version` probes in roko-cli. Test `probe_command_env_excludes_provider_keys`.
3. **Git.** For roko-initiated `git commit` / `push` / `merge`, remove the scrubbed names
   (`CredentialScrub::default()`), so that hooks still run but without the keys. This is the recommended option,
   because signing and SSH keep working. The alternative is to skip hooks with `-c core.hooksPath=/dev/null`, as the
   worktree manager does. That is a behaviour change for users who rely on hooks.

## Done when

- MCP servers, probes and roko-initiated git commands see no provider key, `.env`-loaded name or `ROKO_*` secret,
  unless their own config names it: the MCP `env` overlay, `api_key_env` or a passthrough. A probe of provider X
  may keep X's own shell-exported key, as `CredentialScrub::for_kind` allows.
- Both `[[verify]]` commands pass.

## Notes

- The `--version` probes are low risk. Do the MCP spawn and the git commits first.
- Keep the policy in `roko_core::child_env`. Do not add a third list.
