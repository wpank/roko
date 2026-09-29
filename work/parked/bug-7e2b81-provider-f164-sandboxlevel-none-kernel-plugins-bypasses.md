+++
id = "bug-7e2b81"
kind = "bug"
title = "[provider F164] SandboxLevel::None (kernel plugins) bypasses all sandbox enforcement"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F164"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F164"
anchors = ["crates/roko-agent/src/safety/sandbox.rs", "SandboxLevel::None"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`PluginTier::Kernel` maps to `SandboxLevel::None`, bypassing all sandbox policy. There is no runtime admission gate that prevents a plugin from declaring `PluginTier::Kernel`.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F164`
- `tmp/archive/provider-audit/20-safety-screening.md`

How to verify: Confirm in crates/roko-agent/src/safety/sandbox.rs whether still true: `SandboxLevel::None` (kernel plugins) bypasses all sandbox enforcement
