+++
id = "bug-e2cfdf"
kind = "bug"
title = "roko config providers add writes TOML the config schema rejects"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/config"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/commands/config_cmd.rs:600", "crates/roko-cli/src/commands/config_cmd.rs:608", "crates/roko-core/src/config/provider.rs::ProviderConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli config_cmd'
+++

`providers add` renders `api_key = "${ENV}"` for the provider (`commands/config_cmd.rs:600`) and `model = "<slug>"` in each `[models.<slug>]` table (`:608`).
`ProviderConfig` and `ModelProfile` use `deny_unknown_fields` and expect `api_key_env` and `slug`, so the generated stanza fails to load for every provider.
Fix: emit schema-valid keys and add a round-trip test (render, then parse with the real config types) for every catalog provider.
