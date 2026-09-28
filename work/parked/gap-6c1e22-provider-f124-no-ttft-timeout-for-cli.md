+++
id = "gap-6c1e22"
kind = "gap"
title = "[provider F124] No TTFT timeout for CLI providers"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F124"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F124"
anchors = ["crates/roko-agent/src/provider/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The TTFT timeout (time-to-first-token) is only implemented for `OpenAiCompatLlmBackend::stream_turn`. CLI providers have no equivalent; a stalled subprocess blocks until the global `timeout_ms`.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F124`
- `tmp/archive/provider-audit/03-cli-subprocess.md`

How to verify: Confirm in crates/roko-agent/src/provider/ whether still true: No TTFT timeout for CLI providers
