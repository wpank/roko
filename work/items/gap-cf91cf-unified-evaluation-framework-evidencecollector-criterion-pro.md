+++
id = "gap-cf91cf"
kind = "gap"
title = "Unified Evaluation Framework (EvidenceCollector / Criterion / Profile)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-eval"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/74-evaluation-framework.md#74 — Unified Evaluation Framework (EvidenceCollector / Criterion / Profile)"
discovered_from = "audit:tmp/backlog/archive/74-evaluation-framework.md#74 — Unified Evaluation Framework (EvidenceCollector / Criterion / Profile)"
anchors = ["crates/roko-eval/", "crates/roko-gate/src/", "crates/roko-gate/src/gate_service.rs", "crates/roko-gate/src/composition.rs", "crates/roko-gate/src/adaptive_threshold.rs", "crates/roko-cli/src/runner/gate_dispatch.rs", "roko-gate/gate_service.rs", "roko-gate/llm_judge_gate.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[todo] Done (2026-09-03) — Phase 1 roko-eval crate with core types and bridge — architecture improvement; current gate pipeline works but has structural. The gate pipeline in roko works end-to-end and is fully wired into the runner-v2 event loop. It is not broken. However, it has three structural…

Imported without verification from:
- `tmp/backlog/archive/74-evaluation-framework.md#74 — Unified Evaluation Framework (EvidenceCollector / Criterion / Profile)`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-EVL-1 (Subsystem: Evaluation)`

Some cited files are gone: `roko-gate/gate_service.rs`, `roko-gate/llm_judge_gate.rs`.

How to verify: Check: `crates/roko-eval/` compiles as a workspace member (`cargo build -p roko-eval`); `EvidenceCollector`, `Criterion`, `Profile` traits defined with correct Rust signatures; `EvidenceBag`, `ArtifactRef`, `CriterionResult`, `Finding`… [evidence: own status: Done (2026-09-03) — Phase 1 roko-eval crate with core types and bridge; CONSOLIDATED P3-EVL-1: deferred; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XL |…]
