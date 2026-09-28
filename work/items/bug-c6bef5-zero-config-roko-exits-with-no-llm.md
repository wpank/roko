+++
id = "bug-c6bef5"
kind = "bug"
title = "Zero-config roko exits with no LLM provider configured despite provider keys in the environment"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/config"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/main.rs::resolve_config_for_workdir", "crates/roko-cli/src/main.rs:4266", "crates/roko-cli/src/main.rs:4279"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`resolve_config_for_workdir` (`roko-cli/src/main.rs:4247`) aborts with "error: no LLM provider configured." when the config is fully default, `agent.command == "cat"` and there is no `[providers]` table (`:4260-4261`).
Environment keys such as `ANTHROPIC_API_KEY` are not considered, so `roko serve` on a fresh machine exits instead of starting.
Fix: detect providers from well-known key variables (as `roko setup` does) or start serve provider-less with a clear health warning.

Verified 2026-09-28 (static check against 3d0ee4d02): resolve_config_for_workdir (crates/roko-cli/src/main.rs:4266) still exits with 'error: no LLM provider configured.' when agent_command/token_budget are Source::Default, agent.command == "cat" and config.providers is empty (:4279-4286). Providers come only from project/global TOML (roko-core config/loader.rs merge_global_config_into:2104; RokoConfig default providers = IndexMap::new() at schema.rs:430); no env-key detection exists, and `roko serve` reaches this check via crates/roko-cli/src/commands/server.rs:31.
