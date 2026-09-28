+++
id = "bug-7c20b8"
kind = "bug"
title = "Fix GLM tool-loop result rendering / e2e assertion drift"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/tool-loop"]
created = 2026-09-05
updated = 2026-09-28
source = "crates/roko-agent/tests/glm_tool_loop.rs:108"
discovered_from = "audit:crates/roko-agent/tests/glm_tool_loop.rs:108"
anchors = ["crates/roko-agent/tests/glm_tool_loop.rs::glm_full_tool_loop", "crates/roko-agent/tests/tool_loop_integration.rs::tool_loop_glm_e2e"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Two GLM tool-loop tests are ignored as pre-existing drift (result rendering + e2e assertions); the GLM tool loop path currently has no passing end-to-end coverage.

Imported without verification from:
- `crates/roko-agent/tests/glm_tool_loop.rs:108`
- `crates/roko-agent/tests/tool_loop_integration.rs:100`

How to verify: cargo test -p roko-agent --test glm_tool_loop --test tool_loop_integration -- --ignored and diff actual vs expected rendering.
