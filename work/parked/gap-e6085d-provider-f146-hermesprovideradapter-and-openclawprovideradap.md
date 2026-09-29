+++
id = "gap-e6085d"
kind = "gap"
title = "[provider F146] HermesProviderAdapter and OpenClawProviderAdapter missing context overflow detection"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F146"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F146"
anchors = ["crates/roko-agent/src/provider/hermes.rs", "crates/roko-agent/src/provider/openclaw.rs", "HermesProviderAdapter", "OpenClawProviderAdapter"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Neither adapter detects context overflow from subprocess stderr or HTTP responses. Overflow surfaces as `Other/Unknown`.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F146`
- `tmp/archive/provider-audit/16-error-retry.md`

How to verify: Roadmap P3-1 centralized error_classify.rs; check Hermes/OpenClaw context overflow detection. Confirm in crates/roko-agent/src/provider/hermes.rs, crates/roko-agent/src/provider/openclaw.rs whether still true: `HermesProviderAdapter` and `OpenClawProviderAdapter` missing context overflow detection
