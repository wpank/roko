+++
id = "find-366bb5"
kind = "finding"
title = "Duplicate type family: AgentConfig/AgentManifestConfig (lifecycle stages)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-19
updated = 2026-09-28
source = "tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#10. AgentConfig/AgentManifestConfig"
discovered_from = "audit:tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#10. AgentConfig/AgentManifestConfig"
anchors = ["crates/roko-core/src/config/agent.rs:30", "crates/roko-agent/src/lifecycle.rs:1547", "crates/roko-demo/src/manifest.rs:153", "AgentSpec", "AgentManifestConfig", "AgentConfig", "ResolvedAgentConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
P2-DT-1 audit (2026-09-19) family #10: AgentConfig/AgentManifestConfig has lifecycle stages independent definitions. Proposed action: Rename to ResolvedAgentConfig. Difference: The first two are in the production path; `AgentSpec` is demo-only and not a real duplicate.

Imported without verification from:
- `tmp/archive/P2-DT-1-type-families-audit-2026-09-23.md#10. AgentConfig/AgentManifestConfig`

How to verify: grep the duplicate definitions of AgentConfig/AgentManifestConfig across crates; open if more than one independent definition remains (action: Rename to ResolvedAgentConfig). CONSOLIDATED P2-DT-1 'done' only covered the audit itself.
