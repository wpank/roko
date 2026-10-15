+++
id = "gap-115bae"
kind = "gap"
title = "No command makes a workspace ready for runs a chat host starts, roko doctor does not say whether they can start, and serve calls the fix a restart"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/config"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "b2b899685"
source = "hermes-roko round v5, roko-chathost, 2026-10-15"
anchors = ["crates/roko-cli/src/commands/tune.rs::cmd_config_preset", "crates/roko-cli/src/doctor.rs::run_doctor", "crates/roko-core/src/config/hot_reload.rs::config_diff"]
links = { depends_on = [], blocks = [], related = ["gap-cf427e", "bug-9f4bb6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chat_host_preset_sets_the_data_model_and_keeps_comments' crates/ && cargo test -p roko-cli --lib chat_host_preset_sets_the_data_model_and_keeps_comments"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T19:26:47Z"
commit = "b2b899685"
by = "roko-chathost"
executor = "claude-agent"
forced = false
evidence = "b2b899685: `roko config preset chat-host` (crates/roko-cli/src/chat_host.rs, commands/tune.rs) sets [agent.data_llm] model to the cheapest tool-loop model with its key (ladder rungs first), or --data-llm-model, or with --allow-without-data-llm [serve.mcp] allow_without_data_llm = true; toml_edit keeps comments; --dry-run diff, --json; never the global config. roko doctor gains chat_host_runs; a reload of [agent.data_llm]/[serve.mcp] is live (hot_reload DataLlm/ServeMcp). Tests: chat_host::tests (4, incl. the verify chat_host_preset_sets_the_data_model_and_keeps_comments), doctor::tests::doctor_says_whether_chat_hosts_can_start_runs, hot_reload::tests::chat_host_tables_reload_live, chat_run_tests::a_rung_naming_a_builtin_model_counts, main tests::cli_parses_config_preset_chat_host; lib suites roko-cli 3635, roko bin 458, roko-core 2020, roko-learn 1351, roko-serve 1017 passed (its one failure, the racy jwks::tests::proactive_refresh_task_fetches_without_validation_traffic, fails 3/20 at main too: bug-904a1c); fmt and clippy -D warnings clean. Live (roko serve on :6791, Will's real HOME, a scratch project with his ladder): the MCP probe was refused no_data_boundary; `preset chat-host --yes` wrote [agent.data_llm] model = \"cerebras-gptoss\" with the file's comments kept; 3 s later, no restart, the probe got not_found and serve logged `config hot-reloaded section=DataLlm`; a chat-started architectural task then skipped the claude_cli rung and ran on strong (gpt-5.4-mini), succeeded, $0.0027; roko doctor turned warn -> ok. A CLI-only workspace (scratch HOME): refusal [no_data_boundary, no_tool_loop_model]; --allow-without-data-llm left only no_tool_loop_model."
+++

## Problem

A workspace is not ready for runs a chat host (Hermes, OpenClaw) starts over roko serve's `/mcp` until its
`roko.toml` has a data-model boundary (`[agent.data_llm]`, whose model roko calls over an API with its key set) or,
for local use, `[serve.mcp] allow_without_data_llm = true`. No roko command sets either: `roko init`, `roko setup`
and `roko config preset` (gates, routing, budget, model) have nothing for it. The hand fix the Hermes plugin
suggests ("add an `[agent.data_llm]` table") leaves the model at its default, `claude-haiku-4-5`, which in a
workspace whose Claude models run on `claude_cli` cannot serve as the data model, so every agent roko's tool loop
runs fails to build. `roko config set --project` would rewrite `roko.toml` without its comments (bug-9f4bb6), and
the preset writer (`tui::config_meta::save_pending_edits_to`) goes through `toml::Value` the same way.

`roko doctor` says nothing about whether a chat host can start runs here; only the plugin's own probe finds out.

When `roko.toml` changes, roko serve logs "serve configuration changed (requires restart): false (requires
restart)" for a `[serve.mcp]` change and "agent configuration changed (requires restart)" for an
`[agent.data_llm]` one, although it already applies both: the MCP refusal reads the reloaded config, and every run
loads the workspace config when it starts. The warning sends people to restart for nothing.

Seen 2026-10-15 in Will's workspace (the roko repo): its `roko.toml` has neither table, so every Hermes run is
refused.

## Why it matters

The first thing a person does after installing the Hermes plugin is start a run; today they get a refusal and a
hand edit to make. The plugin's "make ready" key needs one roko command to preview and apply (hermes-roko round
v5, `POST /setup/fix`), and `roko doctor` should say what the refusal says. Related: gap-cf427e (the refusal and
its `no_tool_loop_model` reason), bug-9f4bb6 (`config set` drops comments).

## Where

- `crates/roko-cli/src/commands/tune.rs`: `roko config preset` (`ConfigPresetCmd`, `cmd_config_preset`).
- `crates/roko-core/src/config/serve.rs`: `RokoConfig::chat_run_refusal`, `tool_loop_models` and
  `data_llm_model_problem` (gap-cf427e): what a chat host's run needs, and the models that can serve.
- `crates/roko-cli/src/doctor.rs::run_doctor`: the doctor's checks.
- `crates/roko-core/src/config/hot_reload.rs::config_diff` and `crates/roko-serve/src/routes/config.rs`: what a
  reload reports as needing a restart.

## Current state

Open at `a8e27e4b7` (plus gap-cf427e). No preset, no doctor line; a reload calls both tables restart-only.

## Plan

1. `roko config preset chat-host [--dry-run] [--yes] [--allow-without-data-llm | --data-llm-model MODEL]`
   (`--json` too), logic in a new lib module `crates/roko-cli/src/chat_host.rs`:
   - set `[agent.data_llm] model` to the first of `tool_loop_models()` (the ladder's cheapest such rung, then the
     cheapest by configured price), or the model asked for once roko can call it, or with
     `--allow-without-data-llm` set `[serve.mcp] allow_without_data_llm = true`;
   - edit the project `roko.toml` with `toml_edit`, keeping its comments and layout, and write it through
     `write_checked_config`; never the global config;
   - `--dry-run` prints the unified diff; afterwards say whether chat hosts can start runs, and what still refuses
     them; say that roko serve needs no restart, and to commit `roko.toml` when git tracks it.
2. `roko doctor`: one `chat_host_runs` line, ok (with the data model and what tasks run on) or warn (with serve's
   refusal and the fix).
3. A reload that changes `[agent.data_llm]` or `[serve.mcp]` is applied live, not reported as needing a restart;
   the reload warning stops printing `false`.

## Done when

`chat_host_preset_sets_the_data_model_and_keeps_comments` passes (the `[[verify]]` command), the doctor and
hot-reload tests pass, and against a real `roko serve` the preset turns the MCP refusal into an accepted run with no
restart.

## Notes

Never write `~/.roko/config.toml` (the preset has no `--global`). Keys added to `~/.roko/.env` still need a serve
restart; the preset does not touch keys. The plugin (hermes-plugin-roko, plugin-setup) picks the same model when
the roko on hand has no preset: the first qualifying ladder rung, then the cheapest by price.
