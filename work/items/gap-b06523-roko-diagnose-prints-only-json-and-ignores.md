+++
id = "gap-b06523"
kind = "gap"
title = "roko diagnose prints only JSON and ignores the global --json flag"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/diagnose-graph-runs 05f8854ce"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::cmd_diagnose", "crates/roko-cli/src/commands/diagnose.rs::DiagnoseReport", "crates/roko-cli/src/main.rs:3807"]
links = { depends_on = [], blocks = [], related = ["bug-165b22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_graph_report_renders_as_readable_text' crates/roko-cli/src && cargo test -p roko-cli --lib a_graph_report_renders_as_readable_text && grep -A8 'Command::Diagnose {' crates/roko-cli/src/main.rs | grep -q 'cli.json'"
+++

## Problem

`roko diagnose <plan-id>` always prints the report as pretty JSON (`cmd_diagnose`: `serde_json::to_string_pretty`,
then `println!`). Since 05f8854ce the report covers every task, its attempts, verify failures, episodes, the resume
preview and the recovery steps. For a failed plan that is hundreds of lines of JSON, where the few lines a person
needs are hard to find: which task failed, why, and what to run next. The global `--json` flag ("Emit JSON output
instead of human-readable text", `main.rs:373-375`) is not passed to the command, so there is no text mode to
turn off.

## Why it matters

`roko diagnose` is the command a person runs after a failed `roko plan run`. Text output makes it usable without
`jq`. JSON stays available for scripts and agents.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::cmd_diagnose` (:51-56): builds the `DiagnoseReport` (:63) and prints JSON.
- `crates/roko-cli/src/main.rs`: the `Diagnose` subcommand (:680-695, help says "Outputs structured JSON") and its
  dispatch at :3807-3814, which passes `verbose` but not `cli.json`.
- Docs: `docs/v3/28-CLI.md` (§ `roko diagnose`) says it outputs JSON.

## Current state

Checked at 33e107da1. JSON only. Other commands read `cli.json` (for example `commands/agent.rs:46`,
`main.rs:3860`).

## Plan

1. Add a text renderer for `DiagnoseReport` in the library module. Show the status line, then each failed or
   blocked task with its last error, the failing verify step and its command, the attempts with cost, and the
   resume preview and suggested recovery. `--verbose` adds completed tasks.
2. Pass `cli.json` from `main.rs` to `cmd_diagnose`. Print text by default and JSON with `--json`, as the global
   flag describes.
3. This changes the default output. Say so in the commit and update `docs/v3/28-CLI.md` and the subcommand's help.
   The CLI table in CLAUDE.md also says "structured JSON output"; ask before editing CLAUDE.md.
4. Add `a_graph_report_renders_as_readable_text` (roko-cli lib). Render a report with a failed and a blocked task,
   and assert that the text names both, the failure reason and the resume command, and is not JSON.

## Done when

- `roko diagnose <plan>` prints a readable report and `roko --json diagnose <plan>` prints the JSON report.
- The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `diagnose::render_text` prints the status, the run, spend and task counts, then each task that did not complete
  (every task with `--verbose`) with its reason (which carries the last error), the verify step it failed and that
  step's command, and its attempts with cost, then the next steps (they include the resume command) and notes.
  `cmd_diagnose` takes `json`, and main.rs passes `cli.json`: text is the default, `--json` prints the report.
  Help text, `docs/v3/28-CLI.md` and the newcomer overview are updated. `scripts/run_evidence.py` now runs
  `roko diagnose <plan> --json`, since it stores the JSON answer (`docs/v2/30-EVIDENCE-BUNDLES.md` updated); its
  end-to-end tests (`scripts/test_run_evidence_graph.py`) need a roko built from this branch, which was not run.
  Test: `a_graph_report_renders_as_readable_text`. CLAUDE.md's CLI table still says "structured JSON output";
  left for Will to approve.
