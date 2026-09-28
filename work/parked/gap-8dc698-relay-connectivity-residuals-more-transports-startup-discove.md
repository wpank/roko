+++
id = "gap-8dc698"
kind = "gap"
title = "Relay/connectivity residuals: more transports, startup discovery, MCP/A2A/x402 execution, finality, dashboard"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/connectivity"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#7.4 Additional Relay Transports"
discovered_from = "audit:docs/v3/39-ROADMAP.md#7.4 Additional Relay Transports"
anchors = ["agent-relay", "roko-serve relay routes"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
E29 has one supervised HTTP JSON adapter. Remaining: other transport backends, startup discovery protocol, MCP/A2A/x402 execution integration, finality/reorg processing, dashboard relay status; cross-deployment commons (mesh sync) partial.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#7.4 Additional Relay Transports`
- `docs/v3/depth/16-coordination/07-agent-mesh-sync.md:262`
- `docs/v3/depth/16-coordination/11-exponential-flywheel.md:296`
- `docs/v3/18-CONNECTIVITY.md:7`

How to verify: List Connect trait implementations; check relay status surface in TUI/serve.
