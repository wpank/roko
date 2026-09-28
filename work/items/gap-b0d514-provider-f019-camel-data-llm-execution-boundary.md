+++
id = "gap-b0d514"
kind = "gap"
title = "[provider F019] CaMeL Data-LLM execution boundary not enforced"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F019"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F019"
anchors = ["crates/roko-agent/src/safety/data_llm.rs::DataLlmRouter", "crates/roko-agent/src/safety/mod.rs:86", "crates/roko-core/src/config/agent.rs:77"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The `data_llm.rs` module contains a `DataLlmDecision::SanitizationRequired` routing decision, but no code actually enforces routing through a separate sanitizing LLM. The decision is produced but callers are not required to act on it; there is no production code path that actually routes through...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F019`
- `tmp/backlog/archive/352-data-llm-tainted-content-routing.md#352 — Wire the CaMeL Data-LLM Boundary for Tainted Content`

How to verify: E34 claimed complete; check CaMeL Data-LLM boundary enforcement. Confirm in crates/roko-agent/src/safety/data_llm.rs whether still true: CaMeL Data-LLM execution boundary not enforced / Check: When enabled, External/Untrusted content never enters the main-model request before successful Data-LLM schema validation.; The Data LLM has no tool/MCP/plugin capability and cannot access workspace secrets.; Failure/timeout/malformed output… [evidence: own status: Blocked on shared runtime services]

Merged 2 mined candidates: m2-010, m1-093.

Verified 2026-09-28: still true. `DataLlmRouter`, `DataLlmDecision::SanitizationRequired` and the rest of the Data-LLM API have no callers outside crates/roko-agent/src/safety/data_llm.rs; they are only re-exported (safety/mod.rs:86-87). No dispatch path routes External/Untrusted content through a separate Data-LLM. The cited config file is crates/roko-core/src/config/agent.rs (see :77).
