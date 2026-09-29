+++
id = "gap-666a64"
kind = "gap"
title = "Deprecate All JSONL File I/O — StateHub as Single Source of Truth"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-runtime"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/110-deprecate-jsonl-statehub-only.md#110 — Deprecate All JSONL File I/O — StateHub as Single Source of Truth"
discovered_from = "audit:tmp/backlog/archive/110-deprecate-jsonl-statehub-only.md#110 — Deprecate All JSONL File I/O — StateHub as Single Source of Truth"
anchors = ["crates/roko-cli/src/tui/cursors.rs", "crates/roko-cli/src/tui/jsonl_tailer.rs", "crates/roko-runtime/src/state_hub.rs"]
links = { depends_on = [], blocks = [], related = ["gap-d81125", "find-275736"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test ! -e crates/roko-cli/src/tui/jsonl_tailer.rs && test ! -e crates/roko-cli/src/tui/cursors.rs"
+++
architectural debt; 26 parallel JSONL write streams and 20+ reader poll sites fragment state and guarantee stale data in the standalone TUI. Every runtime data producer in roko writes to a separate JSONL file. Every consumer reads from a separate JSONL file. This creates 26 parallel write streams…

Imported without verification from:
- `tmp/backlog/archive/110-deprecate-jsonl-statehub-only.md#110 — Deprecate All JSONL File I/O — StateHub as Single Source of Truth`

Some cited files are gone: `.roko/engrams.jsonl`.

How to verify: Check: `grep -rn '\.jsonl' crates/ --include='*.rs' | grep -v test | grep -v doc | grep -v target` — zero results (no production JSONL usage outside tests).; `ls .roko/*.jsonl .roko/learn/*.jsonl .roko/state/*.jsonl 2>/dev/null` — returns nothing… [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): XL | 6 |]

Verified 2026-09-28: JSONL is still everywhere: 181 non-test source files (939 lines) reference .jsonl, and 22 live .roko/*.jsonl and .roko/learn/*.jsonl files exist. tui/cursors.rs and tui/jsonl_tailer.rs also still exist, so the import's 'gone' warning was wrong. The 'zero production JSONL' acceptance check conflicts with JSONL logs documented as canonical (episodes.jsonl, efficiency.jsonl, engrams.jsonl), so the scope needs a decision. Severity p2 (architectural debt).

Re-verified 2026-09-29: still open. 3d0637232 extended tui/jsonl_tailer.rs (+114 lines), moving further from a StateHub-only design. The scope conflict recorded on 2026-09-28 (zero production JSONL versus canonical episodes/efficiency/signals logs) still needs a decision before this can be planned.
