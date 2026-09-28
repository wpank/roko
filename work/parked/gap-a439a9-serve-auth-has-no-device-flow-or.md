+++
id = "gap-a439a9"
kind = "gap"
title = "Serve auth has no device flow or Cell/Graph-shaped auth pipeline"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/auth"]
created = 2026-08-15
updated = 2026-09-28
source = "gaps-md#serve-rbac-and-credential-boundaries----resolved-2026-08-15"
anchors = ["crates/roko-serve/src/rbac.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

Serve RBAC is live: four roles, eleven permissions, persisted JWT membership, restart-safe API-key and agent-token registries, and chained relay credentials. Device-flow login and the target Cell/Graph-shaped auth pipeline were left outside E35. By design, local CLI commands act on the workspace directly rather than through serve RBAC.

Fix: add device flow to `roko login`, and model auth as Graph cells when the auth pipeline is revisited.
