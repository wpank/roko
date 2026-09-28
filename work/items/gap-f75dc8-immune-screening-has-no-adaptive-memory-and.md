+++
id = "gap-f75dc8"
kind = "gap"
title = "Immune screening has no adaptive memory and local ledgers are not externally anchored"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/immune", "roko-agent/safety"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#immune-system-screening-coverage----partial"
anchors = ["crates/roko-graph/src/cells/immune.rs", "crates/roko-agent/src/safety"]
links = { depends_on = [], blocks = [], related = ["gap-2f69e9"], supersedes = [], duplicate_of = "" }
+++

The five-stage immune Graph screens canonical provider primary outputs and every host-visible `ToolDispatcher` result, with durable quarantine, cooldown and isolation. What remains:
- Detectors are deliberately bounded and are not general semantic classifiers.
- No adaptive immune-memory update loop exists.
- Historical receipts prove internal consistency but not that the current authority and vault state is authentic. A wholesale rewrite of all local ledgers goes undetected without an external anchored digest, MAC or key.

This item absorbs backlog #53.

Fix: add an immune-memory feedback loop in which confirmed incidents update detector state. Anchor ledger digests externally, either as a signed checkpoint or as a MAC with a key stored outside the workspace.
