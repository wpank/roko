+++
id = "bug-76dc76"
kind = "bug"
title = "roko chat ignores [agent] env_passthrough"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/chat"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/hermetic-child-env dc99a9e81"
anchors = ["crates/roko-cli/src/chat_session.rs::ChatAgentSession::credential_scrub", "crates/roko-cli/src/config.rs::from_roko_config", "crates/roko-cli/src/config.rs::ExecAgentConfig"]
links = { depends_on = [], blocks = [], related = ["bug-7d7200"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'env_passthrough' crates/roko-cli/src/chat_session.rs && grep -rqw 'fn chat_credential_scrub_keeps_agent_env_passthrough' crates/roko-cli/src && cargo test -p roko-cli --lib chat_credential_scrub_keeps_agent_env_passthrough"
+++

## Problem

Since dc99a9e81, the Claude CLI that `roko chat` starts loses every variable roko loaded from `~/.roko/.env` or
`.roko/.env`, other providers' keys and roko's `ROKO_*` secrets. `[agent] env_passthrough` is the documented
opt-back-in: "The provider's `api_key_env`, `[agent] env_passthrough`, and variables an MCP config refers to as
`${NAME}` are always kept", for provider CLIs "including `roko chat`" (`docs/v2/19-CONFIG.md:497`). Agents built by
`create_agent_for_model` honour it: plan runs (`dispatch_v2.rs:1551`, :1793), `agent_spawn.rs` and the direct
provider chat (`chat.rs::run_direct_provider_chat`). The default `roko chat` path does not.
`ChatAgentSession::credential_scrub` (`chat_session.rs:466`) keeps only the provider's `api_key_env` and the MCP
config's `${NAME}` references.

Example: a Bedrock user keeps `AWS_*` in `~/.roko/.env` and sets `[agent] env_passthrough = ["AWS_*"]`. Plan runs
authenticate; `roko chat` loses the variables and fails.

## Why it matters

The same `roko.toml` behaves differently in chat and in plan runs, and the documented fix does nothing in chat.
It is a regression from today's merge for users whose chat CLI needs a `.env`-loaded variable. The only
workaround is the provider's `api_key_env`, which admits one name. Follow-up of bug-7d7200.

## Where

- `crates/roko-cli/src/chat_session.rs::ChatAgentSession::credential_scrub` (:466). It is applied on both turn
  paths: `build_agent` (:997, `.with_credential_scrub`) and `build_streaming_command` (:1252,
  `apply_credential_scrub`).
- `ChatAgentSession::new` (:411) takes the CLI `crate::config::Config`, not `RokoConfig`.
  `crates/roko-cli/src/config.rs::ExecAgentConfig` (:207) has no `env_passthrough` field, and
  `Config::from_roko_config` (:146) does not copy `core.agent.env_passthrough`. So chat cannot see the setting today.
- Entry points: `chat_inline/session.rs:382` and `unified.rs:309`, which call `ChatAgentSession::new`.
- Reference: `roko-agent/src/provider/mod.rs:374-375` fills `AgentOptions::env_passthrough` from
  `config.agent.env_passthrough`, and `provider_credential_scrub` (:715) keeps it.

## Current state

Checked at `33e107da1`: `chat_session.rs` does not mention `env_passthrough`. dc99a9e81 added
`credential_scrub` with the two keep lists above and nothing else.

## Plan

1. Add `env_passthrough: Vec<String>` to `ExecAgentConfig` (serde default, empty), and fill it from
   `core.agent.env_passthrough` in `Config::from_roko_config`.
2. Store it on `ChatAgentSession`: the field, `new`, `clone_for_test` (:1189) and the test literals (:2027,
   :2047). In `credential_scrub`, add `.keep_all(self.env_passthrough.iter().cloned())`.
3. Test `chat_credential_scrub_keeps_agent_env_passthrough` in `chat_session.rs`. Build a session whose passthrough is
   `["AWS_*"]`, give it a `DotenvNames` that lists `AWS_PROFILE`, and assert the scrub keeps `AWS_PROFILE` and
   still strips `OPENAI_API_KEY`. Use `CredentialScrub::strips` directly; the process-wide startup dotenv record is
   write-once.

## Done when

- `roko chat` keeps the variables that `[agent] env_passthrough` names, exactly as plan runs do.
- The `[[verify]]` passes.

## Notes

- Keep the chat scrub `CredentialScrub::for_kind(ProviderKind::ClaudeCli)`; only add the keep list.
