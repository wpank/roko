+++
id = "bug-c6bef5"
kind = "bug"
title = "Zero-config roko exits with no LLM provider configured despite provider keys in the environment"
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
anchors = ["crates/roko-cli/src/main.rs::resolve_config_for_workdir", "crates/roko-cli/src/main.rs:4287", "crates/roko-core/src/config/schema.rs::effective_providers"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn resolve_config_for_workdir/,/^}/p' crates/roko-cli/src/main.rs | grep -q 'effective_providers'"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 2cb2fc282. Zero-config roko counts exported ANTHROPIC/OPENAI/GEMINI/PERPLEXITY keys through RokoConfig::effective_providers. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: static check passes on MAIN."
+++

`resolve_config_for_workdir` (`roko-cli/src/main.rs:4247`) aborts with "error: no LLM provider configured." when the config is fully default, `agent.command == "cat"` and there is no `[providers]` table (`:4260-4261`).
Environment keys such as `ANTHROPIC_API_KEY` are not considered, so `roko serve` on a fresh machine exits instead of starting.
Fix: detect providers from well-known key variables (as `roko setup` does) or start serve provider-less with a clear health warning.

Verified 2026-09-28 (static check against 3d0ee4d02): resolve_config_for_workdir (crates/roko-cli/src/main.rs:4266) still exits with 'error: no LLM provider configured.' when agent_command/token_budget are Source::Default, agent.command == "cat" and config.providers is empty (:4279-4286). Providers come only from project/global TOML (roko-core config/loader.rs merge_global_config_into:2104; RokoConfig default providers = IndexMap::new() at schema.rs:430); no env-key detection exists, and `roko serve` reaches this check via crates/roko-cli/src/commands/server.rs:31.

Re-verified 2026-09-29: unchanged; the function is now at main.rs:4274 and the check at :4287-4295. The fix is wiring. RokoConfig::effective_providers (roko-core/src/config/schema.rs:618) already builds providers from well-known key variables, so the zero-config check should test effective_providers().is_empty() instead of config.providers.is_empty(). The printed hint 'Set ANTHROPIC_API_KEY, OPENAI_API_KEY, or ZAI_API_KEY' is currently false.

## Notes

- 2026-09-30 (wk-childenv): Implemented on `work/bug-17f0e4` at `4e9112b9c`; cargo verification deferred to the batch
  check. The zero-config check asks `RokoConfig::effective_providers`, which detects `ANTHROPIC_API_KEY`,
  `OPENAI_API_KEY`, `GEMINI_API_KEY` and `PERPLEXITY_API_KEY` (not `ZAI_API_KEY`); the hint now lists those four.
