+++
id = "bug-0d9ac4"
kind = "bug"
title = "Agent tool shells bypass child_env: run_tests and ACP's bash inherit provider keys, roko-std's bash keeps its own allowlist"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "M"
subsystem = ["roko-std/tools", "roko-acp/tools"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/hermetic-child-env dc99a9e81"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs", "crates/roko-std/src/tool/builtin/bash.rs", "crates/roko-acp/src/builtin_tools.rs::exec_bash"]
links = { depends_on = [], blocks = [], related = ["bug-7d7200", "spec-ba7bea", "gap-0e2c40", "bug-0eb8e2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'safe_env_keys' crates/roko-std/src/tool/builtin/bash.rs && grep -rqw 'fn bash_tool_keeps_toolchain_env' crates/roko-std/src && cargo test -p roko-std --lib bash_tool_keeps_toolchain_env && grep -rqw 'fn run_tests_env_excludes_provider_keys' crates/roko-std/src && cargo test -p roko-std --lib run_tests_env_excludes_provider_keys"

[[verify]]
command = "grep -rqw 'fn exec_bash_env_excludes_provider_keys' crates/roko-acp/src && cargo test -p roko-acp --lib exec_bash_env_excludes_provider_keys"
+++

## Problem

dc99a9e81 (bug-7d7200) made gate commands start from `roko_core::child_env::gate_env` and scrubbed provider CLIs.
The commands that agents run through roko's own tools were left out, and they follow three different rules:

- **`run_tests` (roko-std)** runs `cargo test --workspace`, `npm test`, `go test`, `pytest`, `forge test` or
  `make test` with roko's whole environment (`run_tests.rs:104`: no `env_clear`, no scrub). Agent-written tests
  therefore see every provider key roko loaded from `~/.roko/.env`. This is the leak bug-7d7200 closed for verify
  steps.
- **ACP's `bash` builtin** (`roko-acp/src/builtin_tools.rs::exec_bash`, :909, spawn at :918) runs the
  agent's command with the ACP server's whole environment, so `env` prints every key.
- **roko-std's `bash`** (`bash.rs:97-106`) clears the environment and re-adds a hard-coded list of 11 names
  (`PATH`, `HOME`, `TMPDIR`, `TEMP`, `TMP`, `TERM`, `LANG`, `LC_ALL`, `USER`, `LOGNAME`, `SHELL`). This is a second
  policy beside `gate_env`. It drops what `gate_env` keeps: `CARGO_*`/`RUSTUP_*` (a custom `RUSTUP_HOME` or
  `CARGO_HOME` breaks `cargo`), `CARGO_TARGET_DIR` (builds leave the shared cache), `LC_*`, `XDG_*`, proxies,
  `NODE_*` and `ROKO_*`. It also cannot honour any passthrough.

## Why it matters

The roko-std tools run for every OpenAI-compatible model: `openai_compat.rs:350` resolves builtin tools through
`roko_std::tool::handlers::handler_for` (`handlers.rs:108-109`). The Kimi, GLM and Ollama tool loops use that path,
and it is a likely way for cheap models to execute golden-path tasks. On it, a plan task can read and spend the
operator's keys through `run_tests`. ACP sessions on roko's own tool loop reach `exec_bash` through
`AcpBuiltinToolHandler` (`bridge_events/tools.rs:418`, :528; built at `dispatch.rs:224`, :1010). This belongs to
goal `release` and epic spec-ba7bea. The canary test gap-0e2c40 checks only the Claude CLI agent and gates.

## Where

- `crates/roko-std/src/tool/builtin/run_tests.rs` (`Handler::execute`, :76; spawn at :104).
- `crates/roko-std/src/tool/builtin/bash.rs` (`Handler::execute`, :74; env at :97-106). The test
  `env_scrubbing_hides_secrets_preserves_safe_keys` (:146) pins the current list.
- `crates/roko-acp/src/builtin_tools.rs::exec_bash` (:909), dispatched from `execute_acp_builtin_tool` (:365, :393).
- Policy: `crates/roko-core/src/child_env.rs` (`gate_env`, `CredentialScrub`). roko-std depends only on roko-core;
  roko-acp also depends on roko-gate (`inherit_gate_env`).
- `ToolContext` (`roko-core/src/tool/handler.rs:263`) carries no passthrough list.

## Current state

Checked at `33e107da1`:
- `run_tests.rs` and `builtin_tools.rs` contain no `env_clear`, `child_env` or `CredentialScrub`.
- `bash.rs` has its own list and does not use `child_env`.
- roko-std spawns processes only in these two handlers (`grep -rn Command::new crates/roko-std/src`).
- The plugin tool runner (`runner/extension_loader.rs:433`) also has its own sandbox environment. That is
  deliberate plugin-sandbox policy and out of scope here.

## Plan

1. **Write one env step for agent tool shells.** Build it from `child_env::gate_env(std::env::vars_os()...,
   passthrough, child_env::startup_dotenv())` after `env_clear()`, like `roko_gate::inherit_gate_env_from`. Give it a
   seam for an injected parent environment so tests need not mutate the process environment.
   - Recommended: the gate policy. It keeps the toolchain, drops secret-looking names and `.env` names, and
     matches what verify steps see.
   - Alternative: `CredentialScrub::default()`, the same as the Claude CLI's own Bash tool. It is looser: shell
     secrets such as `MY_SECRET_TOKEN` stay, so the current bash test would have to change.
2. **Call it from all three handlers.** roko-std `bash` and `run_tests` get it from roko-core. ACP `exec_bash`
   calls `roko_gate::inherit_gate_env`. Delete `safe_env_keys`.
3. **Passthrough.** Add an optional passthrough list to `ToolContext`, or take `[agent] env_passthrough` where
   the tool loop is built. Otherwise, say in the docs that tool shells have no passthrough yet.
4. **Tests.**
   - `bash_tool_keeps_toolchain_env`: `CARGO_HOME` kept, `OPENAI_API_KEY` dropped.
   - `run_tests_env_excludes_provider_keys`: drive the handler with `build = "make"` and a Makefile whose `test`
     target runs `env`.
   - `exec_bash_env_excludes_provider_keys` in `builtin_tools.rs` tests (:1133).
   - Keep the intent of `env_scrubbing_hides_secrets_preserves_safe_keys`.

## Done when

- No agent tool command (roko-std `bash` and `run_tests`, ACP `bash`) can see a provider key, a secret-looking
  name or a `.env`-loaded name unless a passthrough names it.
- roko has one env policy for such commands, in `roko_core::child_env`.
- Both `[[verify]]` commands pass.

## Notes

- Verified statically. Before closing, prove it live: an OpenAI-compatible fake provider, or `roko-std`'s tool-loop
  test harness, calls `run_tests` or `bash` with `env`, and a canary key from a temporary `~/.roko/.env` is absent.
- bug-0eb8e2 covers the non-tool spawns (MCP servers, probes, git hooks). Keep the helper shared between the two
  items.
