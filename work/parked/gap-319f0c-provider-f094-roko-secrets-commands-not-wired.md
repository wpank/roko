+++
id = "gap-319f0c"
kind = "gap"
title = "[provider F094] roko secrets commands not wired into provider key resolution"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F094"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F094"
anchors = ["crates/roko-cli/src/commands/config.rs", "roko secrets"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`roko config secrets set/get` provides a secrets management interface, but provider key resolution still reads from environment variables via `api_key_env`. The two systems are not connected; secrets set via CLI are not visible to provider dispatch.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F094`

Warning: every file this item cites is gone (`crates/roko-cli/src/commands/config.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-cli/src/commands/config.rs whether still true: `roko secrets` commands not wired into provider key resolution
