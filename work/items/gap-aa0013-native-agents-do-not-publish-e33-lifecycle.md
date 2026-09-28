+++
id = "gap-aa0013"
kind = "gap"
title = "Native agents do not publish E33 lifecycle observations directly"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/agent-lifecycle", "roko-agent/telemetry"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e33"
anchors = ["crates/roko-serve/src/agent_lifecycle.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

All 39 E33 telemetry variants have production evidence. Registered agents commit typed lifecycle observations through `POST /api/agents/{id}/observation`, which validates regimes, phases, vitality, ticks and sequence numbers. The native Agent owners (the E23 lifecycle, vitality and SlotManager code) still do not publish that observation payload themselves.

Fix: have native agent lifecycle owners emit observations through the same typed ingress, as an in-process call rather than HTTP, with an integration test.
