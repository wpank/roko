+++
id = "bug-d32a50"
kind = "bug"
title = "roko setup prints a typed API key back in full and never stores it"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/setup"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "roko-init round v5, 2026-10-15 (tmp/hermes-roko-v5/findings/roko-init.md)"
anchors = ["crates/roko-cli/src/commands/setup.rs::prompt_for_api_key"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn setup_stores_a_typed_key_without_printing_it' crates/roko-cli/ && cargo test -p roko-cli --bin roko setup_stores_a_typed_key_without_printing_it"
+++

## Problem

The interactive `roko setup` asks for an API key when it finds no provider (`prompt_for_api_key`). It then
prints the key back in full, as `export ANTHROPIC_API_KEY=<the key>` or `export OPENAI_API_KEY=<the key>`, and
stores it nowhere. The key ends up in the terminal's scrollback and in any recording of the session. Setup then
carries on as if a provider were configured. `roko init`, which runs next, reads only the environment, so it
gives that provider no rung on the model ladder.

The function also guesses OpenAI for any key that does not start with `sk-ant-`, so a Cerebras, Z.ai or
OpenRouter key is labelled OpenAI.

## Why it matters

A newcomer who types a key into `roko setup` exposes it on screen and still has no working workspace. Every other
roko surface treats keys as secrets: the log scrubber, `refuse_readable_secrets` for `roko.toml`, and the gate and
CLI env scrubbing.

## Where

- `crates/roko-cli/src/commands/setup.rs::prompt_for_api_key`: reads the key and prints the `export` lines.
- `crates/roko-cli/src/commands/setup.rs::cmd_setup_interactive`: carries on with the returned `AuthMethod`.
- `~/.roko/.env`: since bug-4e336b, `roko setup` creates it (mode 0600, comments only). It is the file roko loads
  keys from before every command.

## Current state

Open at the bug-4e336b branch (2026-10-15). Seen by reading the code; not reproduced with a real key.

## Plan

1. Never print a typed key. Ask which provider it belongs to, or match the known prefixes, instead of guessing
   OpenAI.
2. With consent, write `NAME=<key>` to `~/.roko/.env` (mode 0600; replace an existing `NAME=` line, keep
   everything else), and say which file it went to.
3. Load it into the running process, or tell the person to run `roko init` again, so that init's ladder gets
   the provider.
4. Test: the prompt flow, fed a key on stdin with HOME set to a temp dir, prints no part of the key, and leaves
   one `NAME=` line in the temp `.roko/.env`.

## Done when

`setup_stores_a_typed_key_without_printing_it` passes (the `[[verify]]` command).
