+++
id = "bug-fafbc0"
kind = "bug"
title = "[provider F071] Binary artifact corruption via lossy UTF-8 in scrub_artifact_body"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F071"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F071"
anchors = ["crates/roko-agent/src/tool_loop/scrub.rs", "scrub_artifact_body"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`scrub_artifact_body(Body::Bytes)` does lossy UTF-8 decode (`String::from_utf8_lossy`) before scrubbing. Binary artifacts (PDFs, images, binary files) are corrupted by this transformation.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F071`
- `tmp/archive/provider-audit/14-tool-loop.md`

Warning: every file this item cites is gone (`crates/roko-agent/src/tool_loop/scrub.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-agent/src/tool_loop/scrub.rs whether still true: Binary artifact corruption via lossy UTF-8 in `scrub_artifact_body`
