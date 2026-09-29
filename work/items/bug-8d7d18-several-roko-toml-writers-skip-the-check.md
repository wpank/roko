+++
id = "bug-8d7d18"
kind = "bug"
title = "Several roko.toml writers skip the check-before-write: config preset, tune and the TUI config and effects saves"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (17:09, wk-onboard's report on bug-e1327f, branch work/bug-e1327f)"
anchors = ["crates/roko-cli/src/commands/tune.rs::cmd_config_preset", "crates/roko-cli/src/commands/tune.rs::ensure_project_config", "crates/roko-cli/src/tui/config_meta.rs::save_pending_edits", "crates/roko-cli/src/tui/effects_config.rs::save_preset_to_root", "crates/roko-cli/src/tui/app/actions.rs::dispatch_action"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = ["bug-e1327f"], blocks = [], related = ["bug-e2cfdf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE 'write_checked_config|check_config_text' crates/roko-cli/src/tui/config_meta.rs && grep -qE 'write_checked_config|check_config_text' crates/roko-cli/src/tui/effects_config.rs && grep -qE 'write_checked_config|check_config_text' crates/roko-cli/src/commands/tune.rs && grep -rqw 'fn every_roko_toml_writer_checks_before_writing' crates/roko-cli/src/ && cargo test -p roko-cli --lib every_roko_toml_writer_checks_before_writing"
+++

## Problem

bug-e1327f (on `work/bug-e1327f`, `fab9168a9`, not merged at BASE) adds a check-before-write for `roko.toml`: `config_cmd::check_config_text` parses and validates the new text, and `write_checked_config` writes it atomically only if it passes. Only `roko init`, `config set` and one more `config_cmd.rs` path call it. Other writers still write unchecked text straight to `roko.toml`:

- `roko config preset` (and the deprecated `roko tune`): `commands/tune.rs::cmd_config_preset` saves through `tui::config_meta::save_pending_edits`, which calls `std::fs::write` (`config_meta.rs:751` on the branch); `ensure_project_config` (`tune.rs:383`) writes a template the same way;
- the TUI config editor, through the same `save_pending_edits`;
- the TUI effects preset: `tui/effects_config.rs::save_preset_to_root` (:225);
- the TUI action that writes a default `roko.toml` (`tui/app/actions.rs:362`, in `dispatch_action`).

So these paths can leave a `roko.toml` that the next command refuses to load.

## Why it matters

Release blockers (epic spec-ae5f94): onboarding breaks when a supported command writes a config that roko itself rejects. bug-e2cfdf already covers `roko config providers add` (`cmd_provider_add`).

## Where

The anchors; the checked writer is `crates/roko-cli/src/config_cmd.rs::write_checked_config` on the branch.

## Current state

Found by searching the branch for non-test writes near `roko.toml`. Two more unchecked writes turned up: `config_cmd.rs::cmd_migrate` (:666) and `cmd_edit`'s placeholder (:804); decide whether each should be checked. wk-onboard's report also named "agent config" and "prd". On the branch, the prd writes found are PRD drafts (`commands/prd.rs:405`) and the agent writes are agent entries and manifests (`agent_serve.rs:1201`, `:1997`), not `roko.toml`. Confirm with wk-onboard's report before counting them.

## Plan

1. After bug-e1327f merges, route every production `roko.toml` write through `write_checked_config`, or `check_config_text` plus an atomic write, starting with `save_pending_edits`, `save_preset_to_root`, `ensure_project_config` and the TUI default write.
2. On a failed check, keep the old file and report the diagnostics (the TUI shows them in its status line).
3. Add `every_roko_toml_writer_checks_before_writing`: each lib writer, given edits that make the config invalid, returns an error and leaves the file unchanged.

## Done when

- [ ] No production path writes `roko.toml` without the check.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on bug-e1327f. Skip `cmd_provider_add` here: it is bug-e2cfdf.
