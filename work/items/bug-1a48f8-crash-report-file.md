+++
id = "bug-1a48f8"
kind = "bug"
title = "Crash Report File"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/183-crash-report-file.md#183 — Crash Report File"
discovered_from = "audit:tmp/backlog/archive/183-crash-report-file.md#183 — Crash Report File"
anchors = ["crates/roko-cli/src/tui/app.rs", "crates/roko-cli/src/inline/terminal.rs", "crates/roko-cli/src/main.rs", ".roko/state/", "state-snapshot.json", "crates/roko-cli/src/runner/persist.rs", "crash_report.rs", ".roko/state/crash-report.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
diagnostics; panics produce only a stderr backtrace today, which is lost when the terminal closes or the daemon restarts. When roko panics or encounters a fatal error, the only output is a backtrace printed to stderr. In daemon mode, long-running plan executions, or TUI sessions, this output is…

Imported without verification from:
- `tmp/backlog/archive/183-crash-report-file.md#183 — Crash Report File`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.11 Crash report and recovery`

Some cited files are gone: `.roko/state/crash-report.json`, `crates/roko-cli/src/tui/app.rs`.

How to verify: Check: On panic, `.roko/state/crash-report.json` is written before the process exits; The file contains: error message, location, backtrace, app state, config summary, env info, and SHA-256 signature; The global panic hook delegates to the previous… [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): S | 7 |]
