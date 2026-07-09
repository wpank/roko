# G — Forensic Replay & Verdicts as Signals (Docs 12, 15)

Parity analysis of `docs/04-verification/12-forensic-ai-causal-replay.md` and
`docs/04-verification/15-verdicts-as-signals.md` vs the actual codebase.

---

## G.01 — Content-addressed chain foundations

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §3 — every replay-chain element identified by BLAKE3 hash. Doc 12 §4.3 — `ArtifactStore` is content-addressed.
**Reality**: `ArtifactStore` BLAKE3-keyed (C.01, C.06). `Engram` (Signal) uses `ContentHash` via roko-core. `FileSubstrate` persists signals to `.roko/signals.jsonl` with content hashing. Foundation layers exist.

---

## G.02 — Episode log as replay data source

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §4.1 — `.roko/episodes.jsonl` records each agent turn.
**Reality**: `EpisodeLogger` at `crates/roko-learn/src/episode_logger.rs:789`, paths defined at `runtime_feedback.rs:123` (`episodes_jsonl: root.join("episodes.jsonl")`). Append-only JSONL, with `Episode` struct at `episode_logger.rs:169` carrying turn metadata. Fully wired — emission sites across orchestrate.rs (e.g., the single realized loop at `orchestrate.rs:5284-5356`).

---

## G.03 — Signal log as replay data source

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §4.2 — `.roko/signals.jsonl` records every signal including gate verdicts with parent lineage.
**Reality**: `FileSubstrate::open` at `orchestrate.rs:11172` (called during gate verdict emission path); path built at `orchestrate.rs:5156` (`signals_path: self.workdir.join(".roko").join("signals.jsonl")`). Engrams include `lineage` field for DAG chain.

---

## G.04 — Efficiency events as replay data source

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §4.4 — `.roko/learn/efficiency.jsonl` carries per-turn token/cost/timing.
**Reality**: 25 crate references to `efficiency.jsonl`. Emission from orchestrate.rs during enrichment loop. Runtime feedback consumes it for drift detection (`crates/roko-learn/src/drift.rs`).

---

## G.05 — Replay algorithm (doc 12 §5)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 12 §5 — reconstruct from task_id by walking episode log + signal log + artifact store + efficiency events; verify chain integrity via BLAKE3 recomputation.
**Reality**: No replay algorithm exists. `grep -rn 'replay_task\|reconstruct_chain\|replay_algorithm' crates/` returns zero matches. The `roko replay` CLI command (mentioned in CLAUDE.md table) walks signal DAG by hash, but does not reassemble a full multi-source causal chain or verify hash integrity end-to-end.
**Fix sketch**: A `roko replay <task-id>` enhancement that joins episodes + signals + artifacts by task_id/plan_id tag and renders a DAG would satisfy this. Foundations are there; the assembly step is missing.

---

## G.06 — Causal analysis: what-if, root-cause, gap (doc 12 §6)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 12 §6.1–6.3 — what-if (replay with different model), root-cause (trace back through chain), gap (which gate should have caught this?).
**Reality**: None of these three analyses exist. `grep -rn 'what_if\|RootCauseAnalysis\|GapAnalysis' crates/` returns zero matches.

---

## G.07 — Pre-certified agent templates (doc 12 §8)

**Status**: NOT DONE (LOW severity, Phase 2+)
**Doc claim**: Doc 12 §8 — pre-certified templates for regulated industries (versioned prompt sections + gate pipeline config + test templates + audit trail).
**Reality**: No template versioning infrastructure. Role templates exist in `crates/roko-compose/src/templates/` but are not regulatory-pre-certified artifacts with accompanying hash manifests.

---

## G.08 — Hash chain for tamper-evidence (doc 12 §7.4)

**Status**: PARTIAL (LOW severity)
**Doc claim**: Doc 12 §7.4 — each signal's hash incorporates parent hash, making reordering/insertion detectable.
**Reality**: `Engram::builder(...).lineage([parent.id])` pattern is used in orchestrate.rs (e.g., `orchestrate.rs:11175-11185` for verdict derivation). Lineage preserves parent hashes — so a chain exists. **But**: no verification pass that walks `.roko/signals.jsonl` and confirms every entry's lineage resolves. Tamper-evident in structure, but not actively checked.
**Fix sketch**: A `roko verify-chain` subcommand that iterates signals.jsonl and confirms lineage integrity would close this.

---

## G.09 — `Kind::GateVerdict` signal emission (doc 15 §9 "Wired")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 15 §9 table — "Gate verdict emission in orchestrate.rs: Wired (verdicts logged to episodes)."
**Reality**: `Kind::GateVerdict` is emitted from `orchestrate.rs` at two sites:
- `orchestrate.rs:11177` — engram derived from task payload carrying serialized verdict JSON; written to `FileSubstrate`. Tagged with `gate`, `passed`, and implicitly lineage to task payload.
- `orchestrate.rs:11246` — `Kind::GateVerdict` conductor signal emission with plan_id/rung/passed/duration_ms/test_count.

State persistence: `orchestrate.rs:11191-11197` pushes `GateResult::from_verdict(verdict, rung)` onto the plan's `gate_results` vec in the executor state.

All three of the doc-§9 "wired" claims (signal, episodes, executor state) are backed by code.

---

## G.10 — Verdict-to-Signal transformation + 24h HalfLife (doc 15 §2.2)

**Status**: PARTIAL (MEDIUM severity)
**Doc claim**: Doc 15 §2.2 — `verdict_to_signal` function with `Decay::HalfLife { 86_400_000 }`, lineage to task, tags for gate/passed/plan_id/task_id.
**Reality**: The verdict signal emission at `orchestrate.rs:11175-11185` sets tags (`gate`, `passed`) and uses `.derive(...)` for lineage — so half of what doc describes is present. **But**: I could not find an explicit `Decay::HalfLife { 86_400_000 }` call on the verdict builder. The builder uses `Provenance::trusted("orchestrate")` without an explicit decay clause. Need to verify whether default decay matches the 24h half-life doc describes.
**Notes**: Tags `plan_id` / `task_id` are set implicitly via the outer `payload_builder` (orchestrate.rs:11151-11152); the derived verdict inherits lineage but not those tags. Consumers querying by `tag("plan_id", ...)` may need explicit tag propagation.
**Fix sketch**: Add explicit `.decay(Decay::HalfLife { half_life_ms: 86_400_000 })` and `.tag("plan_id", ...)` / `.tag("task_id", ...)` calls when building the verdict engram, matching doc §2.2 precisely.

---

## G.11 — Scorer / Router / Composer / Dreams verdict consumption (doc 15 §4, §9)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 15 §4 — four explicit consumer specs (Scorer appraisal, Router verdict-based escalation, Composer injection, Dreams pattern extraction). Doc §9 honestly labels all four as "Not yet" or "Partially".
**Reality**: Matches doc's own labeling:
- Scorer verdict appraisal: not present (no code queries GateVerdict signals for scoring).
- Router escalation via verdict history: not present. Cascade router uses its own internal arm state, not verdict signal queries.
- Composer verdict injection: Partial. The "Recent Gate Results" prompt section doc describes doesn't exist in `SystemPromptBuilder` as a dedicated layer; gate errors flow through `AutoFix` context injection instead (and that too is raw, not filtered — see D.10).
- Dreams verdict pattern extraction: not present. No `DreamsEngine::consolidate` code that ingests GateVerdict signals.

Doc 15 is honest about its own status. This is the closest to accurate self-reporting in the docs/04-verification/ set.

---

## G.12 — Verdict trend detection: `VerdictTimeSeries` (doc 15 §11)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 15 §11.1 `VerdictTimeSeries`; §11.2 trend classification (Stable/Improving/Degrading/Volatile/RegimeShift).
**Reality**: `grep -rn 'VerdictTimeSeries\|VerdictTrend\|classify_trend' crates/` returns zero matches.

---

## G.13 — Co-failure + signature clustering (doc 15 §12)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 15 §12.1 `CoFailureDetector` with phi-coefficient; §12.2 `SignatureCluster` grouping by error signature.
**Reality**: `grep -rn 'CoFailureDetector\|SignatureCluster\|CoFailurePair' crates/` returns zero matches. No cross-gate or cross-signature analysis.

---

## G.14 — Verdict-driven replanning (doc 15 §13), predictive gate selection (doc 15 §14)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 15 §13 `ReplanEngine`, `ReplanAction` (ModifyTask/ReplaceTask/DecomposeTask/Escalate); §14.1 `PredictiveGateSelector` with per-gate `FailurePredictor`; §14.2 `VerdictPatternMemory` (k-NN over historical patterns).
**Reality**: `grep -rn 'ReplanEngine\|ReplanAction\|PredictiveGateSelector\|VerdictPatternMemory\|FailurePredictor' crates/` returns zero matches across entire `crates/` tree.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 5 (G.01–G.04 foundations, G.09 verdict emission) |
| PARTIAL | 2 (G.08 hash chain structural not verified, G.10 decay/tags incomplete) |
| NOT DONE | 7 (G.05 replay, G.06 causal, G.07 certificates, G.11 4 consumers, G.12 trends, G.13 clusters, G.14 replan + predictive) |

Doc 12 correctly describes itself as "foundations in place, full pipeline designed" and that matches the code. Content addressing (BLAKE3), append-only JSONL logs, and the signal DAG lineage are all real. What's missing is the replay **algorithm** that joins these data sources by task_id into a single causal chain and verifies hash integrity end-to-end. This is a tractable build (a single command that walks the three JSONL files), not a deep research problem.

Doc 15's "verdicts as signals" core path is partially wired: `Kind::GateVerdict` engrams are written to `FileSubstrate` and the executor state, with lineage to the originating task (G.09). The signal path into downstream consumers (Scorer / Router / Composer / Dreams) is honestly marked as "not yet" in doc §9. Doc 15 §11–§14 (trend detection, co-failure clustering, replanning engine, predictive gate selection) are entirely design — zero code presence across the board.

**Recommendation**: Doc 12 §5 (replay algorithm) and Doc 15 §11–§14 should be prominently labeled as "Design — data foundations present, algorithms not started". G.10 should also state explicitly that the current builder path defaults to `Decay::None`, so the 24h verdict half-life is not active unless the signal emission path sets it.

## Agent Execution Notes

### G.08 / G.10 — Signal Contract Hardening

This is the right batch-`04` scope for verdict signals.

Recommended slice:

1. make decay explicit if a 24h half-life is truly intended,
2. propagate the critical tags explicitly,
3. add one chain-integrity verification path.

Acceptance criteria:

- verdict engrams have an explicit, inspectable contract,
- lineage verification exists outside tests,
- the patch does not turn into a full replay engine.

### G.05-G.07 / G.12-G.14 — Usually Defer

Replay assembly, causal analysis, pre-certified templates, trend detection, clustering, replanning, and predictive gate selection are all valid future work, but they are not prerequisites for making the current verdict-signal path honest.
