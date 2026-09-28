+++
id = "find-288cdf"
kind = "finding"
title = "[provider F105] CursorCliAdapter does not detect ContextOverflow from stderr"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F105"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F105"
anchors = ["crates/roko-agent/src/provider/cursor_cli.rs", "CursorCliAdapter", "ContextOverflow"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
When the Cursor CLI subprocess outputs "context window exceeded" or similar to stderr, `CursorCliAdapter::classify_error` does not detect the keyword. Context overflow falls through to `Other`, causing retries instead of `TryWithSmallerContext`.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F105`
- `tmp/archive/provider-audit/16-error-retry.md`

How to verify: Roadmap P3-1 centralized error_classify.rs; check CursorCli stderr ContextOverflow patterns. Confirm in crates/roko-agent/src/provider/cursor_cli.rs whether still true: `CursorCliAdapter` does not detect `ContextOverflow` from stderr
