# V2 Coverage

> Status-quo overview · **verified against code 2026-07-08 at git HEAD `5852c93c05`**. This is the navigable summary; the exhaustive per-concept matrix, deep notes, and full checklist live in [`85-V2-COVERAGE-KERNEL.md`](85-V2-COVERAGE-KERNEL.md).

Status vocab: ✅ wired end-to-end | 🔌 built-not-wired | 🟡 partial | ❌ missing | 🕰️ legacy-v1-shape

## V2 Target

The v2 docs describe a system organized around:

- `Signal`/`Engram` as the durable noun; `Pulse`/`Bus` as the ephemeral event kernel.
- `Cell` and `Graph` as the execution shape (v2-depth renames Cell → **"block"**, docs-only).
- `Store`, `Score`, `Verify`, `Route`, `Compose`, `React`, plus newer `Observe`, `Connect`, `Trigger` protocols.
- Lenses/projections for surfaces and telemetry.
- Groups/arenas/connectivity for multi-agent coordination.
- Extension/plugin/security layers around graph/cell execution.

## The one-sentence verdict

**V2 exists as types and surfaces, but the v2 *execution contract* does not run by default.** The kernel is split across two parallel realities: a hollow v2-shaped skeleton (roko-graph's Cell/Graph/Engine — the *default* `roko plan run` path, which **dry-runs every task**) and a working v1-shaped production engine (Runner v2, reachable via `--engine runner-v2`, used by `roko do`/`serve`/PRD auto-exec) that implements most of the orchestrator spec but almost none of the kernel vocabulary. Roughly one-third of the kernel model is built; the built third is the wrong third to be the default.

## Per-spec status (verified HEAD 5852c93c)

| Spec | Verdict | One-line state |
|---|---|---|
| 01-SIGNAL | 🟡🕰️ | Noun is still `Engram`; `Signal` is an alias. No SignalId/SignalRef, no projection/`to_pulse`, no calibration stack, demurrage taxes-without-income, log filenames inverted vs spec. |
| 02-CELL | 🟡 | 9 protocol verbs exist as traits; two incompatible `Cell` traits; Verdict is untouched v1 (no `reward`/criteria/evidence — reward lives on a separate `Outcome` struct); VCG built but off default path. |
| 03-GRAPH | 🟡 | TOML→petgraph→topo→sequential interpret works and is CLI-surfaced; no NodeKinds, no conditional-edge eval (evaluator built, never called), no policy/snapshot/Graph-as-Cell. |
| 04-EXECUTION | ❌/🔌 | Nothing implements the `Engine` API. 7-cell cognitive loop = PassthroughCell stubs ticking at 1 Hz doing nothing. Real gamma/theta/delta heartbeats exist but only feature-gated-off orchestrate.rs drives them. |
| 05-AGENT | 🟡 | Vitality, Regime clocks, PAD affect, somatic bias all exist as pieces; no composite `Agent<S>` type, no Space/slots/T0-probes/reflexes/cognitive-energy/emergent-goals/contrarian-retrieval. |
| 06-MEMORY | 🟡 | Strongest area (roko-neuro): kinds, tiers, falsifiers, AntiKnowledge, D1-D3, dreams with a real post-plan trigger. But HDC compiled out of every binary, demurrage income unwired, hindsight = zero code. |
| 07-LEARNING | 🟡 | L2 (routing) + most of L3 (consolidation) real and wired; L1 fragmentary (adaptive gate thresholds not consulted by runner rung path); L4 (structural adaptation) essentially absent. |
| 27-ORCHESTRATOR | ✅core/🟡gaps | Runner v2 faithfully matches §2-§7/§14; of 12 mori-parity gaps: 4/8 wired, 9/10/11/12/3 partial, 1/2 types-only, 6 container-only, 5+express+apply_rustc_fixes missing. |

## Coverage summary matrix

| V2 concept | Current code state | Gap | Status |
|---|---|---|---|
| Signal noun | `Engram` primary; `Signal` alias (`signal.rs:6`). | Rename direction unresolved; log files inverted (`engrams.jsonl` primary, spec wants `signals.jsonl`). | 🟡🕰️ |
| SignalId/SignalRef, `to_pulse`, projection | Absent (grep → 0). | Blocks lineage-join predict-publish-correct. | ❌ |
| Calibration stack (temperature/ECE/Beta-Binomial/isotonic) | Absent (grep → 0). | 5-axis Score + floors unbuilt. | ❌ |
| Store/Score/Verify/Route/Compose/React (+Observe/Connect/Trigger) | Traits exist in `roko-core`; Observe/Connect/Trigger impls test-only. | Not all behavior mediated through traits. | 🟡 |
| Cell | `roko-core::Cell` (:91) and `roko-graph::Cell` (:74) both exist, incompatible contexts; third de-facto `NodeOutput` family. | Pick canonical API. | 🟡 |
| Verdict redesign | v1 `passed/score/reason`; scalar reward on separate `Outcome` struct. | No reward-on-Verdict, no criteria/evidence, no verify_pre. | 🕰️ |
| Graph engine | loader/registry/topo/sequential-exec/hot-loop real, CLI-surfaced. | Plan-task cells dry-run; no NodeKinds/conditions/policy/snapshot/Bus. | 🟡 |
| Engine API (start/resume/pause/register_hot/estimate) | One-shot `GraphEngine::execute` only. | No lifecycle/budget/resume/pulses on engine path. | ❌ |
| Default `plan run` real work | `--engine graph` + `TaskExecutorCell{dry_run:true}`. | **Prints SUCCESS doing nothing.** | ❌ P0 |
| `roko resume` | Hardcodes `PlanEngine::Graph` (main.rs:2699), drops snapshot. | Broken; should route to Runner v2. | ❌ P0 |
| Pulse/Bus | `Pulse`, `PulseBus`, runtime `EventBus`, server bus, learn bus all exist. | Multiple buses, not one kernel; no replay_since; no source/lineage_hint/trace_id. | 🟡 |
| Predict-publish-correct | `CalibrationPolicy` (LEARN-09) + prediction trackers. | Not a universal per-Cell/topic mechanism. | 🟡 |
| Demurrage | Flat decay wired via serve heartbeat; 5 ReinforceKinds impl'd. | `RuntimeKnowledgeLifecycle` zero external callers → balances stuck at 0.0. | 🔌 |
| HDC (fingerprint/repulsion/resonance) | bind/bundle/permute real in roko-primitives; `hdc` cargo feature. | Enabled by **no consumer** → all 89 knowledge entries `hdc_vector:null`; dark in prod. | 🔌 |
| EFE routing | CascadeRouter = 3-stage LinUCB + persistence; active-inference tier-selector side path. | Full EFE/POMDP framing remains design-level. | 🟡🕰️ |
| Dream cycle | roko-dreams real; post-plan trigger wired in Runner v2 (event_loop.rs:1314-1480). | Hindsight relabeling = zero code. | 🟡 |
| Lenses/telemetry | StateHub + projection contracts exist; ad-hoc surfaces. | **No `Lens` trait** (grep → 0); observability-as-Lens-pipeline unbuilt. | ❌ |
| Groups/arenas | Relay, agent server, jobs, c-factor pieces exist. | No first-class Group/Arena execution model. | 🟡 |
| L4 structural adaptation | — (only MAP-Elites `skill_library.rs` is adjacent). | RecursiveSafetyMonitor/StructuralChange/AutonomyLevel = zero code. | ❌ |
| Extensions | Hook chain + plugin SDK exist. | Manifest/registration/permission model partial. | 🟡 |
| Security | Dispatcher safety real. | Cell/graph capability declarations + taint lattice ops not universal (grep taint join/flows_to → 0). | 🟡 |

## Newest-naming drift (v2-depth vs code)

The v2-depth layer (most recent authoring pass) uses vocabulary the code never adopted. A reader arriving from v2-depth will grep for symbols that do not exist:

- **block → Cell**: `docs/v2-depth/02-block/` renames the concept, but its own INDEX links back to `02-CELL.md` and **no `Block` type exists in code**. Three-way drift (code=`Cell`, v2=`Cell`, v2-depth="block"), zero code motion. *(Confirms the pack's "Cell-vs-Block is docs-only" finding.)*
- **Verdicts-as-Signals**: depth doc gives Verdicts demurrage/Kind/lineage/content-hash; code Verdict is v1 pass/score. Zero code.
- **Lens / observability-as-Lens-pipeline**: no `Lens` trait anywhere. Telemetry is ad-hoc. *(Confirms "telemetry-as-Lens-pipeline unbuilt.")*
- **VCG attention auction**: built (`vcg_allocate` + `AttentionBidder`) but greedy dominates; unreachable on default path. *(Confirms "VCG unreachable.")*
- **Layer-count self-drift**: depth prose says "9-layer SystemPromptBuilder" while its absorbed source is `02-system-prompt-builder-7-layer.md`.

**Navigation fix**: route v2-depth readers through a single "terms that don't exist in code yet" glossary before any depth doc, else each depth doc reads as if its subject ships.

## Migration Verdict

V2 is not absent, but it is not the runtime contract yet. The implementation is a hybrid:

- v2 **types and surfaces** exist broadly.
- v2 **execution semantics** are only partial.
- v2 **default CLI routing** was flipped too early for plan execution (`--engine graph` default while `PlanEngine::default()` = RunnerV2).
- v2 **docs** — especially v2-depth — describe the target as if it were the operational path, using nouns (`block`, Verdict-as-Signal, Lens) with no code behind them.

## Roadmap (priority-ordered; full verify steps in 85-KERNEL)

**P0 — make the default path do real work:**
- [ ] Flip clap `plan run --engine` default to `runner-v2` (main.rs:1361) to match `PlanEngine::default()`, OR hard-fail the graph path when task-executor is a dry-run stub.
- [ ] Fix `roko resume` (main.rs:2699): route to `PlanEngine::RunnerV2` until graph snapshots exist.
- [ ] Implement `TaskExecutorCell` live dispatch (inject a real dispatcher; register a live factory from roko-cli).
- [ ] Enable the `hdc` feature on roko-neuro from roko-cli + roko-serve (or make it default) and backfill fingerprints.
- [ ] Close the demurrage income side: call `RuntimeKnowledgeLifecycle` reinforce from Runner v2's episode-completion path.

**P1 — align contract with spec:**
- [ ] Verdict v2: add `reward`/hard-soft criteria/typed Evidence to `roko-core::Verdict` (serde-compat), feed reward into CascadeRouter.
- [ ] Unify the two `Cell`/`CellContext` families; port `AgentCell`/`ComposeCell`/`condition::evaluate` off the `NodeOutput` side-family.
- [ ] Wire conditional edges in the graph engine (unify `EdgeCondition` + `Condition`, evaluate at successor activation).
- [ ] Wire the runner prompt path into the canonical Compose stack (replace `dispatch/prompt_builder.rs`'s local mirror).
- [ ] Consult adaptive gate thresholds from Runner v2's rung path.
- [ ] Pulse v2 fields (`source`/`lineage_hint`/`trace_id`) + `Signal::to_pulse` + Bus `replay_since`.
- [ ] Pick the canonical log filename and write Signal-shaped records through FileSubstrate.

**P2/P3 — depth features & governance:** hindsight relabeling, Hot Graph state retention + `[graph.policy.hot]` parsing, parallel frontier execution, heartbeat/CorticalState in a non-legacy home, taint lattice ops, temporal knowledge wiring, L4 skeleton (StructuralProposal + RecursiveSafetyMonitor + approval CLI), c-factor-as-observable-pipeline, `roko plan enrich` as a separate command, and reconciling the `legacy-runner-v2` feature / engine-default / stale GAPS pointer drifts.

## Checklist (doc-hygiene)

- [ ] Declare `Engram` vs `Signal` public naming direction.
- [ ] Pick one `Cell` API or make `roko-core::Cell` and `roko-graph::Cell` roles explicit.
- [ ] Make Graph plan execution live before keeping it as default.
- [ ] Turn Pulse/Bus/StateHub/EventBus into a documented layered model.
- [ ] Migrate gates, compose, agent dispatch, store write, and event publish into real graph cells.
- [ ] Move v2 + v2-depth docs from target-state language to status-tagged language.
- [ ] Publish a "v2-depth terms not yet in code" glossary (block, Verdict-as-Signal, Lens, Block) at the pack's navigation root.
