+++
id = "gap-cf920e"
kind = "gap"
title = "[cli-audit agent] `agent serve` relay/chain/identity/passport/wallet flags accepted but inert"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/agent_serve"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/04-agent.md"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/04-agent.md"
anchors = ["04-agent.md:215-219", "RelayConfig", "AlloyChainClient", "AgentRegistration"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
--relay-url, --chain-rpc-url, --identity-registry, --passport-id, --wallet-key are stored/logged/passed to AgentRegistration but no relay connection, chain ops, identity registration or signing occurs.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/04-agent.md`

How to verify: Check agent_serve.rs for use of these values beyond logging/registration.
