+++
id = "bug-e2cfdf"
kind = "bug"
title = "roko config providers add writes TOML the config schema rejects"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/config"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "90307ad5e"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/commands/config_cmd.rs:597", "crates/roko-cli/src/commands/config_cmd.rs:608", "crates/roko-core/src/config/provider.rs::ProviderConfig", "crates/roko-core/src/config/provider.rs::ModelProfile"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn providers_add_snippet_parses_for_every_catalog_provider' crates/roko-cli/ && cargo test -p roko-cli providers_add_snippet_parses_for_every_catalog_provider"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 2cb2fc282. providers add writes api_key_env and slug and quotes model table keys; a test parses the stanza for every catalog provider. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

`providers add` renders `api_key = "${ENV}"` for the provider (`commands/config_cmd.rs:600`) and `model = "<slug>"` in each `[models.<slug>]` table (`:608`).
`ProviderConfig` and `ModelProfile` use `deny_unknown_fields` and expect `api_key_env` and `slug`, so the generated stanza fails to load for every provider.
Fix: emit schema-valid keys and add a round-trip test (render, then parse with the real config types) for every catalog provider.

Re-verified 2026-09-29: unchanged. The existing verify `cargo test -p roko-cli config_cmd` passes while the bug is present because no current test renders the snippet and parses it back. Replace it with a round-trip test.

## Notes

- 2026-09-30 (wk-childenv): Implemented on `work/bug-17f0e4` at `857272182`; cargo verification deferred to the batch
  check. Besides `api_key_env` and `slug`, model table keys are now quoted (`gemini-2.5-pro` made nested tables) and
  the real slug is kept for slugs with `/`. Still open, separately: after `roko init`, `providers add anthropic`
  appends `[models."claude-sonnet-4-6"]`, which the init template already defines, so the file gets a duplicate table.
