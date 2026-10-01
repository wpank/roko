+++
id = "bug-383ba7"
kind = "bug"
title = "`config show` output hard to use (truncation, no section filter)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/config"]
created = 2026-09-19
updated = 2026-09-29
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-11: `config show` truncates after providers section"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-11: `config show` truncates after providers section"
anchors = ["crates/roko-cli/src/config_cmd.rs::cmd_show", "crates/roko-cli/src/config_cmd.rs::print_resolved", "crates/roko-cli/src/main.rs::ConfigCmd::Show", "crates/roko-cli/src/commands/config_cmd.rs:61"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'fn config_show_section_[A-Za-z0-9_]*' crates/roko-cli/ && cargo test -p roko-cli config_show_section_"
+++
ISSUE-11: output stopped after the first provider; ISSUE-26: full dump is 272 lines and needs `config show <section>` filtering (nice-to-have).

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-11: `config show` truncates after providers section`
- `tmp/dogfood/2026-09-19-session.md#ISSUE-26: Config show outputs 272 lines — hard to find specific settings`

How to verify: Run roko config show [section].

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Truncation (ISSUE-11) is not present in the current code: cmd_show (crates/roko-cli/src/config_cmd.rs:185-188) calls print_resolved (:732), which prints the redacted providers TOML at :811-815 and then continues with models, dreams.* and later fields, with no early return or break. A providers serialization error falls back to a placeholder rather than stopping. The fixing commit was not identified. Still missing (ISSUE-26): `ConfigCmd::Show` accepts only --workdir and --effective (crates/roko-cli/src/main.rs:2836-2843), so `config show <section>` filtering does not exist and the full dump is still printed.

Re-verified 2026-09-29: still partial. One thing remains: a `roko config show <section>` filter (ISSUE-26). ConfigCmd::Show takes only --workdir and --effective (dispatch at commands/config_cmd.rs:61), and cmd_show/print_resolved always print the whole resolved config.

## Notes

- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `roko config show <section>` takes an optional positional section: a top-level table (`agent`, `dreams`) or a
  dotted path (`providers.anthropic`, `dreams.auto_dream`). It prints that part of the fully-resolved config as TOML
  under its own table header, redacted like `--effective` (`config_cmd::cmd_show_section` and
  `render_config_section`); an unknown section names the keys at that level. Without a section the output is
  unchanged. Tests: `config_show_section_prints_only_that_section`,
  `config_show_section_names_the_keys_of_an_unknown_section`. `docs/v3/28-CLI.md` documents the argument.
