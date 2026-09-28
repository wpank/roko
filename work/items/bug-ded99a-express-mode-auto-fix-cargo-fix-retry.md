+++
id = "bug-ded99a"
kind = "bug"
title = "Express Mode Auto-Fix (`cargo fix` + Retry Loop)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/118-express-mode-autofix.md#118 — Express Mode Auto-Fix (`cargo fix` + Retry Loop)"
discovered_from = "audit:tmp/backlog/archive/118-express-mode-autofix.md#118 — Express Mode Auto-Fix (`cargo fix` + Retry Loop)"
anchors = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/commands/do_cmd.rs", "runner/event_loop.rs", "event_loop.rs", "types.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/runner/mod.rs", "RetryAction"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Backlog #05 covers the strategist bypass but not the `cargo fix --allow-dirty` auto-fixer agent dispatch and the up-to-3 retry loop that makes express mode useful for routine compile fixes.. Backlog #05 (Express Mode) specifies how roko skips strategist and review phases when `--express` is set…

Imported without verification from:
- `tmp/backlog/archive/118-express-mode-autofix.md#118 — Express Mode Auto-Fix (`cargo fix` + Retry Loop)`
- `tmp/backlog/_archive/_checklist-gaps.md#§1.3`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-10`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `runner/event_loop.rs`.

How to verify: Check: In `--express` mode, a compile gate failure triggers `cargo fix --allow-dirty` before dispatching an LLM.; If `cargo fix` resolves the error, the gate passes and no LLM is invoked for that retry.; If `cargo fix` fails, an auto-fixer agent is… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 5 |]
