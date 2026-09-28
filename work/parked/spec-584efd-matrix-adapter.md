+++
id = "spec-584efd"
kind = "spec"
title = "Matrix Adapter"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-channel-matrix"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/411-matrix-adapter.md#411 — Matrix Adapter"
discovered_from = "audit:tmp/backlog/archive/411-matrix-adapter.md#411 — Matrix Adapter"
anchors = ["roko-core/src/connector.rs", "Cargo.toml", "client.rs", "sync.rs", ".roko/state/matrix-sync-token.json", "access.rs", "commands.rs", "format.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Matrix is the leading open, federated chat protocol; important for self-hosted and privacy-focused deployments. Matrix is an open standard for decentralized, federated communication. Anyone can run a homeserver (Synapse, Dendrite, Conduit) and communicate across federation. The Client-Server API…

Imported without verification from:
- `tmp/backlog/archive/411-matrix-adapter.md#411 — Matrix Adapter`

Some cited files are gone: `.roko/state/matrix-sync-token.json`.

How to verify: Check: `roko-channel-matrix` crate builds and implements `PlatformTransport` from #224; `/sync` long-poll loop receives and processes room messages; Bot commands (`!roko status`, `!roko plan`, `!roko doctor`) return formatted responses [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
