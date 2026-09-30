+++
id = "bug-17f0e4"
kind = "bug"
title = "Unknown roko subcommands run as agent prompts instead of failing"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/cli"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "94a72dcfc"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/main.rs:411", "crates/roko-cli/src/main.rs:3638"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cli_rejects_unknown_single_word_command' crates/roko-cli/ && cargo test -p roko-cli --bin roko cli_rejects_unknown_single_word_command"
+++

The top-level `Cli` takes a positional one-shot `prompt: Option<String>` (`main.rs:412`) next to its subcommands, so an unknown or mistyped first word (e.g. a stale `roko dream run` from docs) is treated as a prompt and starts an agent run.
Likewise `roko <word> --help` succeeds instead of reporting an unknown command, which defeats doc-command tests.
Fix: reject a single bare word that matches no subcommand (suggest the closest one), keep prompts behind `roko run` / `roko do` or a quoted multi-word form, and add CLI tests.

Re-verified 2026-09-29: unchanged. The prompt field is at main.rs:411 and the one-shot branch is at main.rs:3638.

## Notes

- 2026-09-30 (wk-childenv): Implemented on `work/bug-17f0e4` at `673df99f9` (layout follow-up `a92c7c048`); cargo
  verification deferred to the batch check. A value parser on the one-shot prompt refuses a single word as an
  unrecognized subcommand, so `roko dreem`, `roko dream --help` and `roko fix status` fail with a suggestion
  (`roko knowledge dream`, the nearest top-level command) and `roko run <word>`. `roko dream run` also fails, but with
  `run`'s missing-prompt error: clap parses a trailing subcommand before the pending word.
