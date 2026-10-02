+++
id = "bug-18f79b"
kind = "bug"
title = "PRD/Research Provider Routing Mismatch"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-10-02
source = "tmp/backlog/archive/162-prd-provider-routing.md#162 — PRD/Research Provider Routing Mismatch"
discovered_from = "audit:tmp/backlog/archive/162-prd-provider-routing.md#162 — PRD/Research Provider Routing Mismatch"
anchors = ["crates/roko-cli/src/agent_exec.rs", "agent_exec.rs", "dispatch_v2.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-cli/src/dispatch_v2.rs", "create_agent_for_model()", "TaskCategory"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Codex and other code-execution providers can't be used for PRD/research. Roko has two agent dispatch paths:

Imported without verification from:
- `tmp/backlog/archive/162-prd-provider-routing.md#162 — PRD/Research Provider Routing Mismatch`

How to verify: Check: `prd draft new` with `default_model = "codex"` falls back to a text-generation provider.; `plan run` with `default_model = "codex"` continues to use Codex.; A warning is logged when the default model is overridden for capability reasons. [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 7 |]

2026-10-02 (roko-7d): The PRD commands were removed (merge bfd36512f); only the research half can still apply. Re-check `crates/roko-cli/src/research.rs` before reviving; still parked.
