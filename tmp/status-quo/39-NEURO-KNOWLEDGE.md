# roko-neuro — Durable Knowledge Store

> Status-quo audit · re-verified 2026-07-08 @ HEAD 5852c93c0 (`refactor: remove deprecated dispatch_direct module`) · sources: 10 roko-neuro src files (~16.5K LOC), roko-cli (orchestrate.rs, knowledge_helpers.rs, commands/knowledge.rs, custody), cascade_router + feedback_service (roko-learn), roko-serve, **roko-runtime/demurrage_consumer.rs (NEW extraction)**, roko-dreams, roko-compose, 6 Cargo.tomls, `.roko/neuro/` live data (2 jsonl files, 156KB knowledge.jsonl), git log, 18 v1 docs + v2 06-MEMORY.md + 16 v2-depth/11-memory docs
>
> **2026-07-08 deep second pass (HEAD `5852c93c05`):** added § "Admission, tiers, writers, decay" with the admission-gate inequality + source-trust table, tier-progression thresholds (2 confirms / 3 distinct-contexts; Persistent only mintable at ingest), the 4-writers table, and full decay math. **Key mechanic clarified:** the serve heartbeat runs **two** demurrage passes per tick — `DemurrageConsumer` (taxes `confidence`, per-domain, ~2.9h cadence, iteration counter resets each restart) **and** `store.apply_demurrage()` (taxes `balance`, but guarded by `balance > 0.0` which is never true → no-op). So only confidence decays; the balance ledger is doubly inert (never credited, decay short-circuits). Neither balance nor freshness enters `score_entry_for_query`.
>
> **Delta since 2026-07-07 audit:** (a) demurrage moved out of `roko-serve/src/lib.rs` inline into `roko-runtime::demurrage_consumer::DemurrageConsumer` (LIFE-10) — now applies **confidence** decay per-domain-multiplier (`validation_interval=250` ≈ 2.9h), driven by serve's `start_demurrage_timer` (lib.rs:347). (b) NEW **undocumented writer** `record_lifecycle_knowledge` (INT-20, knowledge_helpers.rs:199) writes significant `AgentLifecycleTransition` events (Hibernated/Metamorphosing/Degraded/Deleted, budget resumes) into the store as Heuristic/AntiKnowledge entries — direct `add`, does NOT route through admission or reinforce. (c) CLI `commands/knowledge.rs` refactored to thin delegators (→ `cmd_neuro`/`roko_cli::custody`); old inline line numbers below are stale but behavior unchanged. **Core defects UNCHANGED and re-confirmed:** HDC feature off in all binaries; reinforcement never called in prod (balances 0.0); routing IS wired (item 13 done).

## Summary

roko-neuro is one of the most complete subsystems: all six knowledge kinds, four retention tiers, Ebbinghaus decay, confirmation-driven tier promotion, LLM distillation (D1 per-episode, D1–D3 via dreams), evidence-based admission, AntiKnowledge, backup/restore with genomic bottleneck + generational decay, and heuristics with mandatory falsifiers are real code with live data in `.roko/neuro/knowledge.jsonl`. **Knowledge-informed routing (CLAUDE.md "remaining item 13") IS wired** — confirmed in code (orchestrate.rs:15568→15658 → `select_for_frequency_among_with_knowledge`) and git (`feat: knowledge-informed routing…` 2026-04-20; `audit(C3)+items(13,15)` 2026-04-26). CLAUDE.md is stale on this point.

Two systemic defects undercut the design: **(1) the `hdc` cargo feature is never enabled by any consumer** — roko-cli/serve/dreams all depend on roko-neuro without it (roko-cli/Cargo.toml:42, roko-serve:30, roko-dreams:17 = bare paths; roko-compose:31 declares an `hdc = [..., "roko-neuro/hdc"]` passthrough that no downstream binary turns on). So HDC fingerprinting at ingest, HDC query scoring, the AntiKnowledge repulsion gate, MemoryIndex, and the cross-domain ResonanceDetector are all compiled out of every shipped binary (live entries have `"hdc_vector": null`). **(2) the demurrage economy has taxes but no income**: `record_usage`/`batch_record_usage`/`.reinforce()` (the 5 reinforcement signals) are called only inside roko-neuro's own tests (grep 2026-07-08: zero external callers — the `record_usage` hits in orchestrate.rs:1619 / orchestrator resource_budget.rs are unrelated Usage/token accounting). `RuntimeKnowledgeLifecycle` (the intended facade) is referenced by nothing outside the crate. The `roko-runtime` demurrage consumer applies **confidence** decay on the serve heartbeat — live entries sit at `balance: 0.0`. Balance also does not participate in query scoring, so the v2 economy is currently inert bookkeeping. Knowledge remains a separate `KnowledgeEntry`/JSONL store rather than v2's Knowledge-as-Signal (🕰️); mesh sync is a local outbox/inbox file exchange, not a peer protocol.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Six knowledge kinds enum | v1/06-neuro/01; v2 §2 (as Signal Kinds) | `KnowledgeKind` {Insight, Heuristic, AntiKnowledge, Warning, CausalLink, StrategyFragment} + legacy serde aliases | ✅ (as own enum, not Signal Kind → 🕰️ shape) | crates/roko-neuro/src/lib.rs:117-171 |
| Type half-lives | v1/06-neuro/03 | 30d/90d/1h/60d/14d consts + per-kind default | ✅ | lib.rs:69-115,148-157 (Warning fixed 7d→1h) |
| Four validation tiers | v1/06-neuro/02; v2 06-MEMORY §4 | `KnowledgeTier` {Transient .1×, Working .5×, Consolidated 1×, Persistent 5×} | ✅ multipliers exact | lib.rs:176-199 |
| Ebbinghaus decay + death | v1/06-neuro/07 | `recency_factor` = 0.5^(age/half-life)·(1+0.1·confirmations); `decay()`, `is_dead` @1%, `prune_dead` freeze-first | ✅ | knowledge_store.rs:992-1010,2012-2031,1135-1163 |
| Tier promotion on confirmation | v2 §4 progression | ingest detects confirmations → Transient→Working @2 confirms, Working→Consolidated @3 distinct contexts; gate-driven `evaluate_tier_progression` per task | ✅ (thresholds differ from v2's 3/5) | knowledge_store.rs:561-599; orchestrate.rs:20360-20375 |
| Query API (topic, kind, tier, hits+breakdown) | v1/06-neuro/10 | `query/query_hits/query_kind/by_tier/query_cold/stats`; score = keyword·conf·recency·emotional (+hdc) | ✅ | knowledge_store.rs:634-802,2108-2145 |
| HTTP query API | v1/06-neuro/10 | `POST /api/neuro/query`, `GET /api/knowledge` | ✅ | roko-serve/src/routes/neuro.rs:15-19 |
| CLI (`roko knowledge …`) | CLAUDE.md | query/stats/gc/backup/restore/sync/dream/custody/archive — all real (thin delegators → `cmd_neuro` / `dispatch_knowledge_dream` / `roko_cli::custody` / `cmd_archive`); no stubs | ✅ | commands/knowledge.rs:6-80 (dispatch), :157-173 (custody), :175+ (archive); main.rs:968,993,1033 |
| Custody audit chain (list/show/verify) | v1/06-neuro | `roko_cli::custody::{cmd_custody_list,show,verify}` (custody module in roko-cli, not roko-neuro) — hash-chain over knowledge writes | ✅ | commands/knowledge.rs:159-170; roko-cli custody module |
| HDC encoding + similarity search | v1/06-neuro/04-06; v2 §8 | `KnowledgeHdcEncoder`, role-filler `RoleFillerEncoder`, `MemoryIndex`, `hdc_similarity` in scoring, `ensure_hdc_vector` at ingest | 🔌 **feature `hdc` never enabled by any dependent crate** | roko-neuro/Cargo.toml:15-17 (`default = []`); no `hdc` in roko-cli/Cargo.toml; live data `hdc_vector: null` (.roko/neuro/knowledge.jsonl:1) |
| `query_similar` (1280-byte fingerprint) | v1/06-neuro/06 | not cfg-gated, manual Hamming over stored bytes | 🟡 works only if vectors exist — none do in prod | knowledge_store.rs:651-678,2081-2093 |
| AntiKnowledge (floor, refutation, GC-immunity) | v1/06-neuro/11; v2 §7 | refutation halves target confidence at ingest; 0.3 floor; GC always preserves; `record_anti_pattern_from_failure`; injected as anti-patterns (prompt layer 7) | ✅ | knowledge_store.rs:28,536-559,1030,420-470; knowledge_helpers.rs:127-139; orchestrate.rs:16144 |
| AntiKnowledge HDC repulsion (warn .5 / discount .7 / reject .9, factor .5) | v2 §7 (exact constants) | `check_against_anti_knowledge` in ingest | 🔌 cfg(feature="hdc") → compiled out | knowledge_store.rs:59-71,516-517 |
| Distillation D1 (episode→entries) | v1/06-neuro/12 | `Distiller` + `DistillationBackend`, small-model prompt; **trigger = episode-completion hook** (background tokio task per episode) | ✅ live (`"source":"distiller"` entries on disk) | distiller.rs:33-98; episode_completion.rs:21-66; learning_helpers.rs:417-427 |
| 4-tier distillation D1→D2→D3 (insights→heuristics→PLAYBOOK.md) | v1/06-neuro/12; v2 §9 | `TierProgression::{extract_insights, promote_heuristics(_clustered), compile_playbook, write_playbook}`; **trigger = dream cycle** | ✅ via roko-dreams | tier_progression.rs:1-9,800-1030; roko-dreams/src/cycle.rs:537, runner.rs:782 |
| Dream→neuro staging promotion | v2-depth/11-memory/09 | `StagingBuffer::promote_validated(&knowledge_store)` after auto-dream | ✅ | orchestrate.rs:8253-8284; roko-dreams/src/staging.rs:13 |
| Heuristic when/then + mandatory falsifier + calibration | v2 §5 | `Heuristic` {when, then, falsifier, demotion→AntiKnowledge}; `CalibrationReceipt`/`CalibrationAction`; `HeuristicStore` (observe/demote_expired) | ✅ core; 🟡 no Brier/Wilson/Predicate enum, keyword falsifier match only | tier_progression.rs:94-302,340-540 |
| Admission control (candidates→decisions) | v1/06-neuro (P1) | `KnowledgeAdmissionStore` + `LightAdmissionGate` (conf .5, novelty .3, trust .65); orchestrate routes all writes through it | ✅ | admission.rs:21-70; orchestrate.rs:5185-5260 |
| Source-channel trust discounting | NEURO-07 | `SourceChannel` (user 1.0 … dream 0.5), `ingest_with_source` | ✅ | lib.rs:727-791; knowledge_store.rs:481-489 |
| Demurrage balance + 5 reinforcement signals | v2 06-MEMORY §3 | `balance`, `reinforce()` novelty-weighted, `apply_demurrage`, freeze/thaw, `reinforce_batch`, `score_prediction_utility`, catalytic score | 🟡 **decay wired but reinforcement never called in prod → live balances = 0.0; balance not in query score.** Decay path now `roko-runtime::demurrage_consumer::DemurrageConsumer` (LIFE-10), fed by serve heartbeat `start_demurrage_timer` (lib.rs:347); it applies **confidence** decay w/ per-domain multipliers (gas_patterns 2.0, etc.), `validation_interval=250`≈2.9h — NOT the entry `balance` field | lib.rs:605-714; knowledge_store.rs:1173-1370; roko-runtime/src/demurrage_consumer.rs:1-55; roko-serve/src/lib.rs:347,1986-2037; .roko/neuro/knowledge.jsonl (`"balance":0.0`) |
| Runtime demurrage consumer (heartbeat) | v2 §3 | `DemurrageConsumer` self-contained in roko-runtime; per-tick counter, per-domain decay multipliers, archive_threshold flagging + observability event | ✅ **NEW extraction since prior audit** (STATUS: WIRED header) | roko-runtime/src/demurrage_consumer.rs:1-55; roko-serve/src/lib.rs:347 |
| Lifecycle→knowledge writer (INT-20) | — (undocumented) | `record_lifecycle_knowledge`: significant `AgentLifecycleTransition` (Hibernated/Metamorphosing/Degraded/Deleted, Budget resumes) → Heuristic/AntiKnowledge entry, Transient, conf 1.0, source `lifecycle-monitor`; direct `add`, **bypasses admission** (TODO in code) and does not reinforce | ✅ wired (event handler); 🟡 not routed through admission gate | knowledge_helpers.rs:199-270; orchestrate.rs:5641-5648 |
| `RuntimeKnowledgeLifecycle` (episode→reinforce Retrieved/Gated/Cited/AgentQuoted) | v2 §12 REFLECT | full facade with lifecycle receipts | 🔌 zero references outside roko-neuro (re-confirmed 2026-07-08) | lifecycle.rs:194-380,298-323; grep: no external callers |
| Reinforcement / `record_usage` from runtime | v2 §12 | `record_usage`/`batch_record_usage`/`.reinforce()` on store | 🔌 **no external prod caller** — only roko-neuro unit tests invoke them (grep 2026-07-08) | knowledge_store.rs:1496-1546,1208; callers: tests only |
| Knowledge→routing (CascadeRouter) | CLAUDE.md item 13 | `knowledge_routing_boost` (±0.3 into candidate ranking) + `build_knowledge_routing_advice` → `select_for_frequency_among_with_knowledge`; `KnowledgeRoutingAdvice`/`KnowledgeHint`; routing weight config; knowledge IDs logged for feedback | ✅ **CLAUDE.md stale — landed 2026-04-20** | knowledge_helpers.rs:150-188,527+; orchestrate.rs:15509,15568,15658; cascade_router.rs:404-422,623-679,1610-1646; cascade/types.rs:195; roko-core/src/config/routing.rs:46; git log HEAD ("feat: knowledge-informed routing") |
| Knowledge usage feedback (scores per entry) | v2 §12 | `FeedbackService::record_knowledge_usage` / `knowledge_scores` (`.roko/learn/knowledge-feedback.jsonl`); `record_usage`/`batch_record_usage` on store | ✅ | roko-learn/src/feedback_service.rs:286-345; knowledge_store.rs:1465-1546 |
| Compose enrichment (prompt injection) | v2 §12 RETRIEVE | StrategyFragment context + `ContextAssembler` (canonical in neuro, re-exported by compose) + `ContextSource::KnowledgeEntry` → `AttentionBidder::Neuro` | ✅ | knowledge_helpers.rs:59-121; orchestrate.rs:19533; roko-compose/src/context_assembler.rs:1-4, context_provider.rs:1203 |
| Neuro→gate hints (adaptive thresholds) | — | `apply_neuro_gate_hints` queries failure/stable rungs | ✅ | knowledge_helpers.rs:281-330; orchestrate.rs:8580 |
| ContextAssemblyWeights (40/30/20/10 + cross-domain 15% + 3-tier injection) | v2 §12 (P1-59/60/61) | struct + `composite()` + `injection_tier` | 🟡 built; not the default query path (`score_entry_for_query` uses product formula) | knowledge_store.rs:177-258 |
| Backup (genomic bottleneck) | v1/06-neuro/15 | `export` w/ versioned `BackupHeader`; CLI `--top-n` top-N by confidence + manifest.json | ✅ | knowledge_store.rs:891-926; commands/knowledge.rs:812-890,441-495 |
| Restore (decay + quarantine) | v1/06-neuro/15 | `import` 0.85 discount + Transient reset; CLI `--generation` 0.85^N, `--types`, `--min-confidence` | ✅ | knowledge_store.rs:928-985; commands/knowledge.rs:892-987 |
| Mesh sync | v2-depth/11-memory/14 (WS relay + Iroh gossip, version vectors) | `roko knowledge sync <peer>`: local `.roko/mesh/{outbox,inbox}/delta-<peer>.jsonl`, entry-index as "sequence", 0.7× discount + Transient on receive | 🕰️ file-drop stub, no transport, fake version vector | commands/knowledge.rs:559-678 |
| Worldviews | v2 06-MEMORY §6 (co-citation, Kind::Worldview, rivals) | tag-overlap union-find clusters used only to preserve last survivor during GC | 🟡 | lib.rs:802-979; knowledge_store.rs:1417 |
| Cross-domain transfer (resonance) | v1/06-neuro/08; v2-depth/05 | `ResonanceDetector` (Hamming across domains, min 0.526) | 🔌 cfg(hdc) → compiled out; ResonatorNetwork factorization ❌ (absent from roko-primitives) | hdc.rs:44-130; grep roko-primitives: no "Resonator" |
| Temporal knowledge (Allen algebra) | v2 06-MEMORY §10 | `AllenRelation` (13), `TemporalInterval`, `TemporalIndex`, `KnowledgeEpoch` (25 tests) | 🔌 no callers outside crate; event calculus + 3-tier temporal memory ❌ | temporal.rs:28-429; grep: no external use |
| Freeze/thaw + resurrection | v2 §3 cold threshold | `gc_with_freeze` (freeze-before-delete), `resurrect` (conf 0.6, Transient, fresh balance), `thaw_entry`, `query_cold` | ✅ mechanics; ❌ v2 consensus-freeze/challenge/72h-review | knowledge_store.rs:1052-1133,1377-1415 |
| Knowledge-as-Signal | v2-depth/11-memory/01 | `KnowledgeEntry` + own JSONL (`.roko/neuro/knowledge.jsonl`), separate from signals.jsonl | 🕰️ explicit migration target | lib.rs:337-428; v2-depth/11-memory/01 §1 table |
| Grimoire rename | v1/06-neuro/00 | crate still `roko-neuro`; 2 stray "grimoire" strings repo-wide | ❌ (abandoned) | crates/roko-neuro/Cargo.toml:2 |
| Library of Babel | v1/06-neuro/14 | nothing found | ❌ | — |
| Falsifier records (store-level) | v2 §5 | `FalsifierRecord` + `record_falsification` (halves confidence) | 🟡 helpers exist; no runtime caller found | lib.rs:1229-1268 |
| Runtime feedback sink → offline reinforcement pass | runtime_feedback docs | `KnowledgeCandidate` JSONL + `NeuroKnowledgeIngestor` | 🟡 sink wired; "offline reinforcement pass" it defers to is not in `.roko/GAPS.md` anymore (stale pointer) | roko-cli/src/runtime_feedback/knowledge.rs:1-99; .roko/GAPS.md (no neuro items) |

## Admission, tiers, writers, decay (deep dive)

### Admission gate logic
Two admission mechanisms co-exist. The **fast path** `LightAdmissionGate` (`admission.rs:38-71`) lets a single runtime observation into the durable store immediately iff **all three** hold:

```
confidence ≥ min_confidence(0.5)  AND  (1.0 − similarity) ≥ min_novelty(0.3)  AND  source_trust ≥ min_source_trust(0.65)
```

`novelty = 1.0 − max_similarity_to_existing_entry`. The heavier **evidence-based** `KnowledgeAdmissionStore` (candidate JSONL → decision JSONL) uses stricter floors: `DEFAULT_MIN_ADMISSION_CONFIDENCE=0.72`, `DEFAULT_MIN_ANTI_KNOWLEDGE_CONFIDENCE=0.65` (`admission.rs:22-24`), and weights evidence by source channel:

| Evidence source | Trust weight (`admission.rs:91-101`) |
|---|---|
| `UserInput` | 1.00 |
| `GateOutcome` / `ReviewVerdict` | 0.95 |
| `AgentOutput` | 0.75 |
| `ExternalObservation` | 0.65 |
| `DreamConsolidation` | 0.45 |

### Tier progression thresholds
On ingest, `detect_confirmations` matches new entries to existing ones; each confirmation bumps `confirmation_count` and unions the confirming entry's `source_episodes` into `distinct_contexts`, then auto-promotes (`knowledge_store.rs:561-599`):

| From → To | Trigger | Note |
|---|---|---|
| Transient → Working | `confirmation_count ≥ 2` | — |
| Working → Consolidated | `distinct_contexts.len() ≥ 3` | distinct **contexts**, not raw confirmations |
| → Persistent | **no progression path** | can only be minted at ingest via `inferred_retention_tier` (`knowledge_store.rs:1911`), bypassing progression entirely |

Thresholds (2 / 3-contexts) diverge from v2's design (3 gate-passes / 5 confirmations + consortium). Tier sets the demurrage/decay multiplier: Transient 0.1× / Working 0.5× / Consolidated 1.0× / Persistent 5.0× (`lib.rs:176-199`).

### The four writers (uncontrolled-write census)
Four code paths write into `KnowledgeStore`; only two face the admission gate:

| # | Writer | Path | Gated? | Source tag |
|---|---|---|---|---|
| 1 | Admission-gated success entries | orchestrate → `submit_candidate` / `LightAdmissionGate` (`orchestrate.rs:5185-5260`) | ✅ | gate/agent |
| 2 | Distiller (episode→entries) | `episode_completion.rs:21-66` background tokio task per completed episode | 🟡 own confidence prompt, no admission gate | `"distiller"` |
| 3 | Dream staging promotion | `StagingBuffer::promote_validated` after auto-dream (`orchestrate.rs:8253-8284`) | 🟡 dream-side validation only | `dream` |
| 4 | Lifecycle→knowledge (INT-20) | `record_lifecycle_knowledge` direct `add` (`knowledge_helpers.rs:199-270`) | ❌ **bypasses admission** (code TODO) | `lifecycle-monitor` |

### Decay math — two passes, both largely inert
The serve heartbeat (`start_demurrage_timer`, `roko-serve/src/lib.rs:1986-2078`, spawned at `:347`) runs **two** demurrage passes back-to-back every tick, and both are effectively no-ops in practice:

1. **`DemurrageConsumer` — taxes `confidence`** (`roko-runtime/src/demurrage_consumer.rs:182-265`). Fires only when `validation_interval=250` ticks elapse (interval=40s → **~2.9h continuous uptime**; the iteration counter is in-memory and **resets on every serve restart**, so short-lived servers never trigger it). When it fires: `new_confidence = max(0, confidence − 0.03·domain_multiplier)`. Domain = entry's **first tag** or kind name; multipliers `gas_patterns 2.0 / price_direction 1.5 / volatility_regime 1.0 / yield_trends 0.8 / protocol_behavior 0.5`, default 1.0 (`:49-64`). Entries with `validated_since_last=true` skip decay — but serve maps every entry with `validated_since_last: false` (`lib.rs:2035`) and **nothing sets it true** (reinforce is dead), so *all* entries decay each pass. Below `archive_threshold=0.1` → flagged (counted only). Updated confidences are written back (`lib.rs:2067`).
2. **`store.apply_demurrage()` — taxes `balance`** (`knowledge_store.rs:1173-1190`, called at `lib.rs:2077`). Time-based: `if elapsed_hours > 0 && entry.balance > 0.0 { entry.apply_demurrage(elapsed_hours); taxed += 1 }`. Because `balance` is **never credited** (reinforce/`record_usage` have zero prod callers → all live balances `0.0`), the `balance > 0.0` guard is always false → `taxed=0` → no rewrite. The balance ledger is doubly inert: never credited, and its own decay pass short-circuits.

Net: the **confidence** field decays on a ~2.9h cadence when serve stays up (this is the *only* live demurrage effect); the **balance** field neither moves nor influences `score_entry_for_query` (which is `keyword·confidence·recency·emotional`, `knowledge_store.rs:634-802`). This is distinct from the per-entry Ebbinghaus `decay()` (`0.5^(age/half-life)·(1+0.1·confirmations)`, `knowledge_store.rs:992-1010`) applied at query time — a **third** confidence-decay owner.

## V2-aligned

- Kind/tier taxonomy, tier multipliers (0.1/0.5/1.0/5.0), AntiKnowledge threshold constants (0.5/0.7/0.9, 0.5 discount) match v2 numerically (lib.rs:191-198; knowledge_store.rs:59-71).
- Five reinforcement kinds match v2 names exactly (lib.rs:677-688); novelty-weighted bumps, freeze/thaw, cold query, resurrection, catalytic/autocatalytic metrics (knowledge_store.rs:1324-1375) all present.
- Lifecycle loop (ingest→retrieve→decay→promote→consolidate→prune) exists end-to-end; dream consolidation feeds the store (orchestrate.rs:8253-8284) and distillation runs per completed episode.
- Knowledge participates in RETRIEVE (prompt enrichment via Neuro bidder) and in model routing (Item 13) — the two v2 pipeline touchpoints.
- Heuristics carry mandatory falsifiers and demote to AntiKnowledge (tier_progression.rs:276-302).

## Old paradigm & tech debt

- **Separate store, not Knowledge-as-Signal** (🕰️): duplicate type system/persistence exactly as catalogued in v2-depth/11-memory/01 §1.
- **Three overlapping decay mechanisms**: confidence Ebbinghaus (`decay()`), balance demurrage (`entry.apply_demurrage`), and a free-fn confidence demurrage (`apply_demurrage(entries, rate)` lib.rs:1146-1157) — plus two freeze implementations (`frozen: bool` vs `deprecated`+`__frozen_confidence:` tag, lib.rs:1176-1222). Needs consolidation.
- **Demurrage without reinforcement** in production (RuntimeKnowledgeLifecycle unwired; no external `reinforce`/`record_usage` caller — re-confirmed 2026-07-08) — the economy only taxes; live balances all 0.0 yet retrieval unaffected because scoring ignores balance. Note the runtime `DemurrageConsumer` taxes **confidence**, not the `balance` field, so the balance ledger is doubly inert (never credited, never even debited).
- **Lifecycle writer bypasses admission** (INT-20): `record_lifecycle_knowledge` does a direct `knowledge_store.add` rather than `submit_candidate`, with a code TODO acknowledging it (needs evidence-chain builder). Undocumented in prior audit; a fourth uncontrolled write path alongside distiller, dream staging, and admission-gated success entries.
- **HDC dark matter**: ~1,500 LOC of encoder/index/repulsion/resonance behind a feature no binary enables; `hdc_cluster` (lib.rs:1016) reads vectors that are never written.
- Demurrage rate law is flat-per-hour, not v2's Gesell ODE with per-Kind (r, β); reinforcement magnitudes and tier-promotion thresholds diverge from the v2 tables (2/3-contexts vs 3-gate-passes/5-confirmations).
- Mesh sync "version vector" is entry-count-as-sequence — re-sync after GC/rewrite will mis-window deltas.
- CLAUDE.md item 13 ("remaining") and runtime_feedback's pointer to `.roko/GAPS.md` are both stale.

## Not implemented

- Resonator Networks (factorization), event calculus + 3-tier temporal memory, Kind::Worldview signals with rival worldviews, consensus freeze/challenge/thaw quorum flow, Library of Babel, on-chain InsightStore/PheromoneRegistry bridge (Phase 2+), real peer transport for sync, `roko knowledge heuristic create` CLI, novelty from retrieval-count (`1/(1+ln n)`).

## Migration checklist

- [ ] **[P0]** Enable HDC in shipped binaries: add `roko-neuro = { features = ["hdc"] }` (via roko-cli + roko-serve) or make it default; backfill fingerprints for existing entries — verify: `cargo tree -e features -p roko-cli | grep 'roko-neuro.*hdc'` && `cargo run -p roko-cli -- knowledge stats` then check new entries in `.roko/neuro/knowledge.jsonl` have non-null `hdc_vector`
- [ ] **[P0]** Wire `RuntimeKnowledgeLifecycle` (or equivalent reinforcement calls) into orchestrate's episode-completion path so Retrieved/Gated/Cited/AgentQuoted actually bump balances — verify: run `roko run "…"`, then `grep -o '"balance":[0-9.]*' .roko/neuro/knowledge.jsonl | sort -u` shows values > 0
- [ ] **[P1]** Make balance/freshness a factor in `score_entry_for_query` (or switch default scoring to `ContextAssemblyWeights::composite`) — verify: unit test comparing rank of balance-0 vs reinforced entry on equal keywords
- [ ] **[P1]** Update CLAUDE.md: mark item 13 done; fix runtime_feedback/knowledge.rs GAPS.md pointer or re-add the offline reinforcement pass to `.roko/GAPS.md` — verify: `grep -n "item 13\|knowledge-informed" CLAUDE.md`
- [ ] **[P1]** Consolidate duplicate freeze + demurrage implementations (drop tag-based `freeze_entry`/free-fn `apply_demurrage`) — verify: `grep -rn "__frozen_confidence" crates/ | wc -l` → tests only. Also reconcile the three decay owners: entry `decay()` (confidence Ebbinghaus), `DemurrageConsumer` (confidence per-domain), and the never-credited `balance` demurrage — pick one confidence law and either wire or delete the balance ledger.
- [ ] **[P1]** Route `record_lifecycle_knowledge` (INT-20) through `KnowledgeAdmissionStore::submit_candidate` instead of direct `add`, so lifecycle events face novelty/trust gating like every other writer — verify: build a `KnowledgeCandidateRecord` from the transition; `grep -n "knowledge_store.add(entry)" crates/roko-cli/src/knowledge_helpers.rs` returns nothing in `record_lifecycle_knowledge`
- [ ] **[P2]** Knowledge-as-Signal migration: represent kinds as Signal `Kind` variants, store via Store protocol, keep KnowledgeStore as a view (per v2-depth/11-memory/01) — verify: knowledge entries appear in `.roko/signals.jsonl` with knowledge Kinds
- [ ] **[P2]** Replace flat demurrage with per-Kind Gesell (r, β) table from v2 06-MEMORY §3; align reinforcement magnitudes + novelty formula — verify: `cargo test -p roko-neuro demurrage`
- [ ] **[P2]** Real mesh transport (WS relay first; version vector keyed on entry IDs/seqs, not index) — verify: two workdirs exchange entries without manual file copy
- [ ] **[P3]** Wire `TemporalIndex` to episode/epoch boundaries; add co-citation worldviews + rival retrieval slot; consensus freeze/challenge flow — verify: `roko knowledge query` surfaces epoch/worldview metadata
- [ ] **[P3]** Add Brier/Wilson calibration to heuristics; Predicate enum for `when` — verify: `cargo test -p roko-neuro calibration`

## Open questions

1. Should HDC become non-optional (v2 assumes fingerprints "always present") — or is binary-size/dep on roko-primitives the reason it stayed off?
2. Where should reinforcement fire from: orchestrate post-gate (has context-pack IDs at orchestrate.rs:15575 already) or the serve heartbeat that currently only taxes?
3. Are the intentionally-longer off-chain half-lives (lib.rs:69-97 rationale comments) the desired divergence from the v2 demurrage table, or should v2's per-Kind (r, β) win?
4. Tier thresholds: keep code's (2 confirms / 3 contexts) or adopt v2's (3 gate passes / 5 confirmations + consortium for Persistent)? `inferred_retention_tier` can mint Persistent at ingest (knowledge_store.rs:1911), bypassing progression entirely — intended?
5. Is the mesh outbox/inbox format meant to be the wire format for the future relay, or a throwaway?
