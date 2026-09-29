+++
id = "gap-233c53"
kind = "gap"
title = "[provider F161] AgentCell has no production host adapter — only test mocks exist"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/cells"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F161"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F161"
anchors = ["crates/roko-graph/src/cells/agent.rs", "AgentCell"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`AgentDispatcher` trait for `AgentCell` has no production implementation. Only `MockAgentDispatcher` and `FailingAgentDispatcher` exist. `AgentCell` cannot be used in production graphs.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F161`
- `tmp/archive/provider-audit/20-graph-integration.md`

How to verify: Graph is sole engine since #260; check AgentCell production host adapter. Confirm in crates/roko-graph/src/cells/agent.rs whether still true: `AgentCell` has no production host adapter — only test mocks exist
