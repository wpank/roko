+++
id = "bug-e2cfdf"
kind = "bug"
title = "roko config providers add writes TOML the config schema rejects"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/config"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/commands/config_cmd.rs:597", "crates/roko-cli/src/commands/config_cmd.rs:608", "crates/roko-core/src/config/provider.rs::ProviderConfig", "crates/roko-core/src/config/provider.rs::ModelProfile"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn providers_add_snippet_parses_for_every_catalog_provider' crates/roko-cli/ && cargo test -p roko-cli providers_add_snippet_parses_for_every_catalog_provider"
+++

`providers add` renders `api_key = "${ENV}"` for the provider (`commands/config_cmd.rs:600`) and `model = "<slug>"` in each `[models.<slug>]` table (`:608`).
`ProviderConfig` and `ModelProfile` use `deny_unknown_fields` and expect `api_key_env` and `slug`, so the generated stanza fails to load for every provider.
Fix: emit schema-valid keys and add a round-trip test (render, then parse with the real config types) for every catalog provider.

Re-verified 2026-09-29: unchanged. The existing verify `cargo test -p roko-cli config_cmd` passes while the bug is present because no current test renders the snippet and parses it back. Replace it with a round-trip test.
