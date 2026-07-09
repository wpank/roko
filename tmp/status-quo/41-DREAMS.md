# roko-dreams — Offline Consolidation Cycle

> Status-quo audit · verified 2026-07-08 (HEAD `5852c93c05`) · **2nd pass** re-verified the three-engine split (Graph default / runner-v2 opt-in / orchestrate off) with file:line · sources: 26 crate files (13,741 LOC total: ~9K live + ~4.7K phase2 shells), 8 consumer files across roko-cli/serve/acp/core, 19 v1 docs (`docs/v1/10-dreams/`), 5 v2 docs (`docs/v2-depth/11-memory/06–10`), 6 on-disk artifacts in `.roko/dreams/`

## Summary

The dream subsystem is **built and has run for real**, and its plan-completion consolidation **is live in the runner-v2 run loop** (`event_loop.rs:1952-1953`). The prior draft's framing was imprecise in two directions and is corrected here:

- **Not "dead-by-default because orchestrate is off"** (too pessimistic): plan-completion consolidation was **re-ported** into runner-v2 (`run_dream_consolidation_if_enabled`, `event_loop.rs:5473`), so it survives the `legacy-orchestrate` cutover on the runner-v2 / `roko serve` / `roko do` paths.
- **Not "the live default trigger"** (too optimistic): the **default** `roko plan run` executes the **Graph Engine** (`main.rs:1361` `#[arg(long, default_value = "graph")]`; dispatch `commands/plan.rs:258` matches `PlanEngine::Graph` → `cmd_plan_run_engine`, returns early), and the Graph path has **zero** dream wiring (`rg 'dream' crates/roko-cli/src/commands/plan.rs` → 0; `rg 'dream' crates/roko-graph` → 0). So plan-completion dreaming fires only when the operator passes `--engine runner-v2`, or via `roko serve`/`roko do` (which build runner-v2 `RunConfig` directly).

CLAUDE.md's "Partial (no runtime trigger/cron)" remains *directionally* stale — real triggers exist — but the accurate status is **"wired & live on runner-v2 + serve/do + daemon-idle + ACP-threshold; the default Graph engine has none; orchestrate's copy is dead-code."**

`.roko/dreams/` holds evidence of real runs (May 6-7 2026): 2 dream reports (25 episodes → 12 knowledge entries, 1 playbook, 6 hypnagogia entries), a journal, 235 counterfactuals (`counterfactuals.jsonl`, 143 KB), 5 cross-episode reports, and a populated `staging-buffer.json`.

**Live triggers (engine-independent, 2):** (1) **daemon idle loop** — `daemon.rs:368` → `roko-serve/src/dreams.rs:39` `start_dream_loop`, 60s poll, `dreams.auto_dream` default **true** (`config.rs:228`); (2) **ACP episode-threshold** — `bridge_events.rs:483` (`DREAM_EPISODE_THRESHOLD`, uses `claude`, `effort: medium`). **Live only on runner-v2 / serve / do (1):** **plan-completion** — `event_loop.rs:1953` `run_dream_consolidation_if_enabled` → `:5488` uses `command: "claude"` (real LLM, `effort: low`), gated by `learning.dream_on_completion` (default **true**, `learning.rs:110`); **NOT fired on the default Graph engine.** **Dead by default under `legacy-orchestrate` (3):** plan-completion `maybe_auto_dream` (`orchestrate.rs:8177`), coordination-pattern INT-19 (`:8324`), and the INT-18 daimon-affect + INT-07 staging-promotion passes — none re-ported to runner-v2. Manual: `roko knowledge dream run/report/schedule/journal/archive`, `POST /api/dream/run` (still `command: "cat"` — no real review).

Edge gaps unchanged: **cron is dead config** (`DreamRuntimeControls`/`scheduled_cron` constructed **only inside the crate** — 0 external callers, grep-verified), Mattar-Daw **replay planner bypassed** by the shipping cluster-everything cycle, **counterfactuals write-only** (235 records, no reader), dream **routing advice never reaches CascadeRouter** (`dream_advice_to_routing_bias` orphaned), and `phase2/` (~4.7K LOC) is self-described "type shells" (only `DreamJournal`/`DreamArchive`/`sleep_time` live). Architecture is exactly v2 `06`'s critique target: monolithic `run_budgeted` (`cycle.rs:507`) with bespoke scheduling, not a Loop Graph of phase Cells. 🕰️ shape, ⚠️ wiring-regressed.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Dream cycle (3-phase run) | v1 `01-three-phase-cycle.md`, v2 `06` | `cycle.rs:507-796` `run_budgeted` | ✅ wired (🕰️ monolith) | called via `runner.rs:920`; reports on disk `.roko/dreams/dream-*.json` |
| DreamRunner facade + budget | v1 `16` | `runner.rs:720-1012` | ✅ wired | live `consolidate_now` at `event_loop.rs:5508`, `knowledge.rs:688`, `routes/dream.rs:76`, `bridge_events.rs:516`; `orchestrate.rs:8243` now **dead by default** (legacy-orchestrate gate) |
| Hypnagogia engine (4-layer) | v1 `07`, v2 `08` | `hypnagogia.rs` (ThalamicGate/ExecutiveLoosener/DaliInterrupt/HomuncularObserver) | ✅ wired | pre-NREM phase `cycle.rs:629-630` (DREAM-11); 6 entries in latest report |
| NREM = episode clustering + agent review | v1 `02` | `cycle.rs:563-610` `cluster_episodes`/`process_cluster` | ✅ wired | per-cluster review via `AgentDispatcher` (`cycle.rs:56-69`) |
| Mattar-Daw replay planner (gain×need×spacing, 4 modes, affect bias) | v2 `07` | `replay.rs:35-115,241` | 🔌 built-not-wired | only reachable via `DreamRunner::plan_replay` (`runner.rs:789`) — `run_budgeted` never calls it; `select_replay_episodes_with_affect` used in tests only (`replay.rs:788,817`) |
| REM imagination (CausalModel, 3 creativity modes, counterfactual queries) | v1 `03` | `imagination.rs` | ✅ wired | `synthesize_hypotheses` at `cycle.rs:666` |
| Threat simulation → warning knowledge | v1 `09`, v2 `10` | `threat.rs:41` `enumerate_threats` | ✅ wired | `threat_warning_entries_with_floor` at `cycle.rs:668-673`, configurable floor `runner.rs:921-924` |
| Threat rehearsal | v1 `09` | `rehearsal.rs:59` `rehearse_threats` | 🔌 built-not-wired | exported `lib.rs:67`; callers = own tests only |
| Nightmare detection/containment | v2 `10` | `phase2/advanced.rs` (NightmareDetector etc.) | ❌ missing (stubs) | `phase2/mod.rs:1-4` "not yet implemented"; journal hardcodes `nightmares_detected: 0` (`runner.rs:973`) |
| Consolidation staging buffer (Raw→Replayed→Validated→Promoted, 7d GC, HDC redundancy 0.90) | v1 `04`, v2 `09` | `staging.rs` | ✅ wired | `cycle.rs:648-724`; persisted `.roko/dreams/staging-buffer.json` (`cycle.rs:432-436`) |
| HDC counterfactual synthesis | v1 `06` | `cycle.rs:168-193,789-790` `build_counterfactuals` | 🟡 partial (write-only) | writes `.roko/dreams/counterfactuals.jsonl` (235 lines on disk); no reader anywhere (`grep counterfactuals.jsonl` → cycle.rs only) |
| Scheduling policy + triggers enum | v1 `13` | `runner.rs:254-493` (`DreamTrigger`, `DreamSchedulePolicy`, cron via `cron` crate) | 🟡 partial | Idle/Manual/plan-completion/coordination live; **cron + BusPulse never constructed at runtime** (`DreamRuntimeControls` unused outside crate — grep: 0 hits) |
| Daemon background dream loop | v1 `13` | `roko-serve/src/dreams.rs:39-90` `start_dream_loop` | ✅ wired | spawned at `daemon.rs:368`; 60s `DREAM_CHECK_INTERVAL`, idle-threshold gated; `auto_dream` default true (`roko-cli/src/config.rs:228-230`) |
| Plan-completion auto-dream (runner-v2) | — | `event_loop.rs:1952-1953` `run_dream_consolidation_if_enabled` → `:5488` | ✅ wired (runner-v2 / serve / do only) | fires when `--engine runner-v2` (or serve/do); **NOT on the default Graph engine** (`plan.rs:258` returns before the runner loop); `command: "claude"` real LLM, `dream_on_completion` default true (`learning.rs:110`) |
| Plan-completion auto-dream (legacy) | — | `orchestrate.rs:8177` `maybe_auto_dream` | ⚰️ dead-by-default | behind `legacy-orchestrate`; call sites `orchestrate.rs:7066,8145,8358` no longer compiled |
| Coordination-pattern trigger (INT-19) | — | `orchestrate.rs:8324` `maybe_coordination_dream` | ⚰️ dead-by-default | behind `legacy-orchestrate`; called `orchestrate.rs:7072`; trigger metadata discarded (`_trigger`, `:8351`); **no v2 equivalent** |
| Bus-pulse trigger (DREAM-09) | v1 `13` | `runner.rs:342-375` `BusPulseTriggerConfig` | 🔌 built-not-wired | `DreamTrigger::BusPulse` never constructed outside crate |
| Intensive mode (backlog high/low-water) | — | `runner.rs:555-613,887-938` | ✅ wired | DREAM-06; report flag `intensive_mode_active` |
| Sleep-time compute budget (per-phase USD tracking) | v1 `12` | `phase2/sleep_time.rs` used at `cycle.rs:638-679` | 🟡 partial | live but costs are heuristics ($0.001/entry, $0.01/cluster), not real token spend; `SleepTimePrecompute`/`SleepwalkerMode` stubs |
| Dream journal + archive | v1 `14`/`17` | `phase2/advanced.rs` `DreamJournal`/`DreamArchive` | ✅ wired | written `runner.rs:947-988` (DREAM-14); read by `knowledge.rs:93,128` + `GET /api/dream/journal` |
| Routing advice from dreams | v1 `15` | `routing_advice.rs` → `.roko/learn/dream-routing-advice.json` | 🟡 partial | written `cycle.rs:556-558`; consumed for ACP prompt provenance (`bridge_events.rs:43,3079-3096`) but `dream_advice_to_routing_bias` (CascadeRouter bias, `routing_advice.rs:154`) has zero callers |
| Regression engrams (success-rate, c-factor) | — | `cycle.rs:798-908` | ✅ wired | appends `dreams:regression` / `cfactor:regression` to `.roko/engrams.jsonl` |
| Cross-episode consolidation | — | `cycle.rs:543-554` via `roko_learn::pattern_discovery` | ✅ wired | 5 reports in `.roko/dreams/cross-episode/` |
| Daimon affect feedback from dreams | — | `knowledge.rs:692-696`, `roko-serve/src/dreams.rs:225-234` (live); `orchestrate.rs:8290-8298` (dead) | 🟡 partial | `AffectEvent::DreamOutcome`/`DreamFailure` (INT-18); the orchestrate feed is now behind `legacy-orchestrate` — v2 `event_loop.rs` dream path emits **no** affect event |
| CLI `knowledge dream run/report/schedule/journal/archive` | — | `commands/knowledge.rs:82-146,682-775` | ✅ wired | schedule = prints next fire time (`knowledge.rs:768`), not a persistent cron |
| HTTP `POST /api/dream/run`, `GET /api/dream/journal` | — | `roko-serve/src/routes/dream.rs:18-22` | 🟡 partial | registered `routes/mod.rs:169`; run hardcodes review agent `command: "cat"` (`routes/dream.rs:65`); journal reader expects `timestamp`/`phases` fields the writer never emits (`routes/dream.rs:143,148-157` vs `DreamJournalEntry`) |
| Dream cycle as Loop Graph of Cells | v2 `06` | — | ❌ missing | v2 doc §1: monolith, bespoke scheduler, staging outside Store protocol — all still true |
| DeltaConsumer (runtime 3-phase dream loop) | — | `roko-runtime/src/delta_consumer.rs:168-349` | 🕰️ legacy duplicate, 🔌 unwired | phases are stubs "will be connected to roko-dreams::replay when the dream runner is wired" (`delta_consumer.rs:299-333`); never instantiated outside its tests |
| Dream evolution (MAP-Elites), divergence/alpha, hauntology, oneirography, inner-worlds rendering, lucid monitor | v1 `05`,`08`,`10`,`11`,`14`,`17` | `phase2/{evolution,divergence,hauntology,oneirography,rendering,advanced}.rs` | ❌ missing (type shells) | `phase2/mod.rs:1-4`; zero runtime callers (grep outside crate: only chain's unrelated `phase2`) |
| TUI dream view widget | v1 `11` | `tui/widgets/dream_view.rs` | 🔌 built-not-wired | declared `tui/widgets/mod.rs:8`; no dashboard page renders `DreamSnapshot` |

## Triggers table (trigger → live? → file:line)

| Trigger | Live in default `roko plan run` (Graph)? | Where it IS live | file:line | Notes |
|---|---|---|---|---|
| Plan-completion consolidation | ❌ (Graph has no dream hook) | `--engine runner-v2`, `roko serve`, `roko do` | `event_loop.rs:1952-1953` → `run_dream_consolidation` `:5488` | gated by `learning.dream_on_completion` (default true, `learning.rs:110`); `command:"claude"`, `effort:low` |
| Daemon idle loop | ✅ engine-independent (separate process) | `roko daemon` | `daemon.rs:368` → `roko-serve/src/dreams.rs:39` `start_dream_loop` | 60s poll, idle-threshold gated; `dreams.auto_dream` default true (`config.rs:228`) |
| ACP episode-threshold | ✅ engine-independent (ACP bridge) | `roko-acp` bridge | `bridge_events.rs:483` `DREAM_EPISODE_THRESHOLD` | `command:"claude"`, `effort:medium` |
| Manual CLI | ✅ | `roko knowledge dream run/report/schedule` | `commands/knowledge.rs:82-146,682-775` | `schedule` prints next fire time only — no persistent cron |
| Manual HTTP | ✅ (but no real review) | `POST /api/dream/run` | `routes/dream.rs:18-22,65` | review agent hardcoded `command:"cat"` |
| orchestrate plan-completion `maybe_auto_dream` | ⚰️ dead-by-default | `--features legacy-orchestrate` only | `orchestrate.rs:8177` | not re-ported to runner-v2 |
| orchestrate coordination-pattern INT-19 | ⚰️ dead-by-default | `--features legacy-orchestrate` only | `orchestrate.rs:8324` | **no runner-v2 equivalent** |
| orchestrate INT-18 affect-feedback / INT-07 staging-promotion | ⚰️ dead-by-default | `--features legacy-orchestrate` only | `orchestrate.rs:8255-8305` | not re-ported; v2 dream path emits **no** affect event |
| Cron / scheduled | ❌ never constructed | — | `runner.rs:448-456` (`cron` dep) | `DreamRuntimeControls`/`scheduled_cron` has 0 external callers (grep-verified) |
| Bus-pulse (DREAM-09) | ❌ never constructed | — | `runner.rs:342-375` | `DreamTrigger::BusPulse` never built outside crate |

**Persistence reality:** all live triggers write shared `.roko/dreams/` state (`dream-*.json` reports, `staging-buffer.json` via `cycle.rs:436`, `counterfactuals.jsonl` write-only 235 records, `.roko/engrams.jsonl` regression engrams, cross-episode reports). Nothing serializes concurrent consolidations from the three live triggers beyond each run's `processed_through` cutoff. `dream_advice_to_routing_bias` (`routing_advice.rs:154`) never reaches CascadeRouter; counterfactuals have no reader.

## V2-aligned

- **Confidence-ladder staging** (`staging.rs`) matches v2 `09-consolidation-and-staging.md`: Raw 0.20 → Replayed 0.30 → Validated 0.50 → Promoted 0.70, 7-day GC, HDC redundancy check at 0.90, promotion to `KnowledgeTier::Transient` — the "dream hallucinations can't corrupt durable knowledge" invariant is enforced in code.
- **Mattar-Daw replay utility** (`replay.rs:35-115`) is a faithful implementation of v2 `07-replay-and-counterfactual-cells.md` (gain from prediction error, need from novelty+recency, spacing term) — it just isn't on the hot path.
- **Hypnagogia as cheap pre-NREM generator** (`hypnagogia.rs`, four-layer) matches v2 `08-hypnagogia-and-creativity.md`; outputs enter staging at Raw, exactly per doc.
- **Threat-as-knowledge** (`threat.rs` severity = likelihood×impact×(1−detectability) → warning entries) matches v2 `10`'s wake-side half.
- **Trigger diversity** — idle, episode-count, plan-completion, coordination-pattern, manual — approximates v2 13-TRIGGERS intent, albeit hand-rolled.

## Old paradigm & tech debt

- ⚰️ **legacy-orchestrate regression (NEW, top drift)**: `orchestrate.rs` is now `#[cfg(feature = "legacy-orchestrate")]` and **not compiled by default** (`lib.rs:90-95`). Every dream integration that lived only in orchestrate — coordination-pattern INT-19, INT-18 daimon-affect feedback, INT-07 staging promotion, the `maybe_auto_dream`/`maybe_coordination_dream` call graph — is **dead in the shipping binary**. The v2 `event_loop.rs` replacement re-implements *only* plain plan-completion consolidation (`:1953`); coordination-pattern and affect-feedback triggers were **not ported**. This is the single largest correction versus the prior audit, which counted these as live.
- 🕰️ **Monolithic cycle**: `run_budgeted` orchestrates everything inline (`cycle.rs:507`); phases are not composable Cells, scheduling is not Trigger-bound, staging is not a Store partition — v2 `06` §1's three criticisms all stand.
- 🕰️ **DeltaConsumer duplicate** (`roko-runtime/src/delta_consumer.rs`): a second, stubbed dream loop that predates the wiring — dead parallel implementation, classic "built but never connected."
- **Staging path mismatch (now doubly dead)**: cycle persists `.roko/dreams/staging-buffer.json` (`cycle.rs:436`) but orchestrate's INT-07 post-dream promotion loads `.roko/dreams/staging.json` (`orchestrate.rs:8255-8260`) — a file the cycle never writes. Since the cycle already promotes internally *and* orchestrate is now feature-gated off, this is dead code inside dead code. The v2 path does **no** post-dream promotion pass at all (relies solely on the cycle's internal promotion).
- **Journal schema mismatch**: `GET /api/dream/journal` expects `timestamp` and `phases[]` (`routes/dream.rs:143,153-157`); `DreamJournalEntry` writes `cycle_start`/durations — `last_cycle` is always `""` and phases are synthesized.
- **`cat` as review agent**: `POST /api/dream/run` (`routes/dream.rs:65`) and `DreamRunner::default()` (`runner.rs:1000`) use `command: "cat"` — HTTP-triggered dreams do no real LLM review.
- **Journal zeros**: `rem_duration_secs`, `consolidation_duration_secs`, `hypothesis_diversity`, `total_tokens`, `nightmares_detected` hardcoded to 0 (`runner.rs:955-976`).
- **Discarded trigger metadata**: INT-19 builds `DreamTrigger::CoordinationPattern` then drops it (`orchestrate.rs:8351`); journal always records `trigger: Manual` (`runner.rs:954`).
- **Blanket clippy allows** (`lib.rs:7-44`, 37 lints) masking code-quality debt.
- `.roko/GAPS.md` contains **zero** dream entries despite the above — the canonical gap tracker missed this subsystem.

## Not implemented

- Cron-scheduled dreaming: machinery exists (`runner.rs:448-456`, `cron` dep) but `DreamRuntimeControls`/`scheduled_cron` has no config surface and no runtime constructor — every call site uses `DreamRunner::new` with defaults.
- Bus-pulse (DREAM-09) trigger wiring; engram-bus reactivity.
- Dream evolution (MAP-Elites archive), divergence/alpha convergence, hauntology/spectral provenance, oneirography/dream-art, inner-worlds rendering, lucid monitoring, constitutional self-critique, fleet dream sharing / circadian coordination (all `phase2/` shells, v1 docs 05/08/10/11/14/17).
- Counterfactual **consumption**: 235 records logged, nothing replays or validates them (v2 `07` counterfactual-cell loop absent).
- Dream→CascadeRouter routing bias (`dream_advice_to_routing_bias` orphaned) — overlaps CLAUDE.md remaining-work item 13.
- v2 Loop-Graph refactor: phase Cells, budget-gate node, `DreamQualityDashboard` lens, feedback-driven phase budget reallocation.

## Migration checklist

- [ ] **[P0]** **Re-port the lost orchestrate triggers to v2** (`event_loop.rs`): coordination-pattern consolidation (INT-19) and daimon-affect feedback (INT-18) were dropped when orchestrate went behind `legacy-orchestrate`. Decide keep-or-kill; if keep, add call sites near `run_dream_consolidation_if_enabled` (`event_loop.rs:1953`). Verify: `cargo build -p roko-cli` (default features) then `grep -rn 'CoordinationPattern\|DreamOutcome' crates/roko-cli/src/runner/` returns hits.
- [ ] **[P0]** Fix staging path mismatch (`orchestrate.rs:8255-8260` → `staging-buffer.json`, or delete the redundant INT-07 pass since `cycle.rs:687` already promotes) — verify: `cargo run -p roko-cli -- plan run plans/ && cat .roko/dreams/staging-buffer.json | python3 -m json.tool | head`
- [ ] **[P0]** Align journal writer/reader schemas (either emit `phases[]`+`timestamp` in `runner.rs:persist_journal_entry` or read `cycle_start` in `routes/dream.rs:143`) — verify: `curl -s localhost:6677/api/dream/journal | jq '.last_cycle'` (non-empty)
- [ ] **[P1]** Route the shipping cycle through the Mattar-Daw planner (`select_replay_episodes` before `cluster_episodes` in `cycle.rs:563`) so replay prioritization actually gates what gets consolidated — verify: `cargo test -p roko-dreams replay && cargo run -p roko-cli -- knowledge dream run` then inspect report `processed_episodes < total_episodes` under backlog
- [ ] **[P1]** Give `POST /api/dream/run` a real review agent (reuse `build_dream_runner` config resolution from `knowledge.rs:777`) — verify: `curl -X POST localhost:6677/api/dream/run` and check report cluster `agent_review` is non-empty
- [ ] **[P1]** Wire `dream_advice_to_routing_bias` into CascadeRouter model selection (CLAUDE.md item 13) — verify: `roko learn router` shows dream-sourced bias after a dream run
- [ ] **[P1]** Expose `DreamRuntimeControls` (cron, budget, imagination mode, threat floor, intensive) in `roko.toml` → `DreamRunner::with_controls` at all call sites — verify: set `dreams.scheduled_cron` and confirm `roko knowledge dream schedule` reflects it
- [ ] **[P2]** Delete or wire `roko-runtime/src/delta_consumer.rs` (duplicate loop) — verify: `grep -rn DeltaConsumer crates/ --include='*.rs' | grep -v test` returns lib.rs export only or nothing
- [ ] **[P2]** Wire `rehearse_threats` into the REM phase or delete `rehearsal.rs` — verify: report gains a rehearsal section after `knowledge dream run`
- [ ] **[P2]** Close the counterfactual loop: read `counterfactuals.jsonl` in the next cycle and stage confirmed hypotheses (v2 `07`) — verify: staging buffer contains entries with `source: counterfactual`
- [ ] **[P2]** Record real trigger kind + phase durations + token spend in journal entries (`runner.rs:947-988`) — verify: `roko knowledge dream journal` shows non-zero REM duration and non-Manual triggers from daemon runs
- [ ] **[P2]** Render `tui/widgets/dream_view.rs` in a dashboard tab fed from `.roko/dreams/` — verify: `roko dashboard` shows dream phase panel
- [ ] **[P3]** v2 refactor: decompose cycle into phase Cells in a Loop Graph with Trigger-based scheduling and quality-lens budget feedback (v2 `06` §2) — verify: `plans/` contains dream-consolidation graph TOML executable by the v2 engine
- [ ] **[P3]** Promote or prune `phase2/` shells (4,730 LOC); implement nightmare detection first since threat warnings already flow (v2 `10`) — verify: `nightmares_detected > 0` possible in journal
- [ ] **[P3]** Update CLAUDE.md roko-dreams row ("Partial … no runtime trigger/cron" → wired via v2 `event_loop.rs` plan-completion + daemon idle triggers; **coordination-pattern & affect triggers regressed with the legacy-orchestrate cutover**; cron unwired) and log residual gaps to `.roko/GAPS.md` (currently **zero** dream entries) — verify: `grep -i dream .roko/GAPS.md`

## Open questions

1. **Which trigger owns dreaming long-term?** Post-cutover the live set is run-loop plan-completion (`event_loop.rs`), daemon idle, and ACP threshold — all firing independently against shared `.roko/dreams/` state with nothing serializing concurrent consolidations beyond the `processed_through` cutoff. The orchestrate-era coordination/affect triggers are gone by default. Is a single Trigger-bound scheduler (v2 `06`) meant to subsume these, and should the dropped coordination-pattern/affect triggers be re-added or intentionally retired?
2. **Is clustering-as-NREM the intended design**, or a placeholder until Mattar-Daw replay drives selection? The v1 doc (`02-nrem-replay.md`) and v2 `07` both describe utility-driven replay; the shipping code clusters everything since the last cutoff.
3. **Should hypnagogia/liminal entries bypass staging?** `cycle.rs:726-739` writes them directly to the knowledge store *and* stages them — the staging ladder's protection is partially undermined in the same cycle that implements it.
4. **Phase-budget realism**: per-phase USD figures are entry-count heuristics, not measured spend. Does the sleep-time compute doc (v1 `12`) intend real token accounting via `DreamBudget::consume_episode` (which exists, `runner.rs:222-233`, but isn't fed by phase costs)?
5. **`roko serve` vs `roko daemon`**: should the control plane also run `start_dream_loop`, or is daemon the sole background host by design?
