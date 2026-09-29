+++
id = "gap-f52caf"
kind = "gap"
title = "Schedule the live provider proof matrix (credential-gated #[ignore] probes never run)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider-proof"]
created = 2026-09-25
updated = 2026-09-28
source = "crates/roko-agent/tests/provider_proof_matrix.rs:1196"
discovered_from = "audit:crates/roko-agent/tests/provider_proof_matrix.rs:1196"
anchors = ["crates/roko-agent/tests/provider_proof_matrix.rs::live_anthropic_api_probe", "crates/roko-agent/tests/provider_proof_matrix.rs::live_full_matrix_from_workspace_config"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Six live probes (Anthropic, Cerebras, Gemini, OpenAI-compat, Claude CLI, full workspace matrix) are #[ignore] and need credentials; nothing indicates they run on a schedule, so provider claims rest on hermetic tests only.

Imported without verification from:
- `crates/roko-agent/tests/provider_proof_matrix.rs:1196`
- `crates/roko-agent/tests/provider_proof_matrix.rs:1267`
- `crates/roko-agent/tests/provider_proof_matrix.rs:1316`
- `crates/roko-agent/tests/provider_proof_matrix.rs:1384`
- `crates/roko-agent/tests/provider_proof_matrix.rs:1430`
- `crates/roko-agent/tests/provider_proof_matrix.rs:1488`

How to verify: grep .github/workflows for 'provider_proof_matrix' / '--ignored'; decide on a nightly secret-backed job.
