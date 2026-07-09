# SOURCE-INDEX — Code Anchors for 10-Dreams Parity

Verified code references for batch `10`, organized around the current
dream runtime and the docs most likely to drift from it.

Generated: 2026-04-16

---

## Important Corrections First

- `roko-dreams` is the active runtime owner for dreams, hypnagogia, imagination, replay, and threat simulation.
- `roko-dreams/src/lib.rs` no longer re-exports from `roko-golem`.
- `dream run`, `dream report`, and `dream schedule` already ship in `roko-cli`.
- scheduled cron triggers already ship.
- manual triggers already ship at both crate and CLI levels.
- hypnagogia and threat simulation already ship in `roko-dreams`.
- oneirography, Sleepwalker mode, nightmare detection, dream sharing, and world-model integrations do not ship.

---

## crates/roko-dreams/src/

### Public runtime surface

| File | What | Section |
|------|------|---------|
| `lib.rs:1-83` | public exports plus local `DreamsEngine` / `DreamsSubsystemId` compatibility types | E.01 |
| `runner.rs:156-216` | `DreamBudget` | A.08 |
| `runner.rs:221-342` | `DreamTrigger`, `DreamSchedulePolicy`, trigger delay logic, cron/manual support | A.01, A.05-A.07 |
| `runner.rs:349-395` | `DreamHeartbeatPolicy`, `DreamHeartbeatReport` | A.09 |
| `runner.rs:513-581` | `DreamRunner` construction and scheduling helpers | A.03, A.11 |
| `runner.rs:628+` | `DreamCycle` invocation from runner | A.03 |
| `runner.rs:796+` | load latest dream report | A.11 |

### Core cycle and outputs

| File | What | Section |
|------|------|---------|
| `cycle.rs:67-117` | `DreamCycleReport` and report metadata | A.03, E.06 |
| `cycle.rs:259-324` | `DreamClusterKey`, `DreamOutcome`, `DreamClusterReport` | C.03 |
| `cycle.rs:333+` | `DreamCycle` | A.03 |
| `cycle.rs:499-502` | hypnagogia + threat generation wired into cycle | D.01, D.08 |
| `cycle.rs:511-535` | report assembly | A.03, B.11 |
| `cycle.rs:591-612` | dreams regression signal writing | E.05 |
| `cycle.rs:674+` | report persistence | A.11 |

### Replay, imagination, liminal, threat

| File | What | Section |
|------|------|---------|
| `replay.rs:17-114` | `DreamReplayMode`, `DreamReplayBatch`, `utility_score`, replay selection | B.01-B.03 |
| `imagination.rs:19-174` | `CounterfactualQuery`, `ImaginationMode`, `ImaginationOutcome`, `imagine()` | B.04-B.06 |
| `imagination.rs:178-289` | `synthesize_hypotheses()` and Boden’s three modes | B.05, B.11 |
| `imagination.rs:293-317` | `counterfactual_episode()` | B.04 |
| `hypnagogia.rs:17-98` | four-layer hypnagogia structs + engine | D.01-D.03 |
| `hypnagogia.rs:111+` | hypnagogia engine implementation | D.01-D.03 |
| `threat.rs:15-85` | `ThreatScenario`, severity, `enumerate_threats()`, `threat_warning_entries()` | D.08, D.10 |

---

## crates/roko-cli/src/

### User-facing and long-running dreams surfaces

| File | What | Section |
|------|------|---------|
| `main.rs:5585-5678` | `dream run`, `dream report`, `dream schedule` CLI | A.07, E.02 |
| `main.rs:5665+` | build `DreamRunner` from config | E.02 |
| `orchestrate.rs:4985-5036` | orchestrator dream config / report checks | E.02, E.05 |
| `daemon.rs:239-268` | daemon starts long-running dream loop | E.02 |

---

## crates/roko-learn/src/

### Supporting infrastructure

| File | What | Section |
|------|------|---------|
| `pattern_discovery.rs:99+` | `PatternMiner` | B.10 |
| `pattern_discovery.rs:291+` | `CrossEpisodeConsolidator` | B.09, C.02 |
| `pattern_discovery.rs:360+` | consolidator uses `k_medoids()` | C.02 |
| `hdc_clustering.rs:81+` | `k_medoids()` and clustering primitives | C.02 |
| `runtime_feedback.rs:337, 574` | live PatternMiner / consolidator usage elsewhere in runtime | B.09, B.10 |
| `playbook.rs` | `PlaybookStore` for dream-generated revisions | C.07, E.05 |

---

## crates/roko-neuro/src/ and related supporting surfaces

| File | What | Section |
|------|------|---------|
| `KnowledgeStore` paths used by dreams | knowledge persistence and additions | A.02, B.11 |
| `TierProgression` | knowledge tier progression used by dreams docs | B.12 |
| `roko-primitives/src/hdc.rs` | canonical HDC primitive and `text_fingerprint` owner | C.01, C.04 |
| `roko-daimon/src/lib.rs` | depotentiation cross-system surface | B.08, E.05 |

---

## Missing / Absent (code-search negatives)

These doc features have no matching production code in the active tree:

| Absent Feature | Search | Section |
|----------------|--------|---------|
| intensive consolidation / backlog high-low watermarks | `rg -n "intensive|high_watermark|low_watermark|backlog" crates/roko-dreams --include=*.rs` | A.10 |
| DiCE / FACE / LOF / advanced counterfactual diversity | `rg -n "DiCE|FACE|LOF|GLOBE|DPP" crates/roko-dreams --include=*.rs` | B.07 |
| HER / PER / ERE / generative replay implementations | `rg -n "hindsight|prioritized_replay|generative_replay|ERE" crates/roko-dreams --include=*.rs` | B.13 |
| MAP-Elites / dream evolution runtime | `rg -n "MAP.?Elites|quality_diversity|dream_evolution|memetic" crates/roko-dreams --include=*.rs` | C.05, C.06 |
| Lin sleep-time compute / `rethink_memory` / Sleepwalker | `rg -n "rethink_memory|Sleepwalker|SIGPAUSE|sleep_time" crates --include=*.rs` | C.08-C.10 |
| world models / Dreamer / IRIS / Genie runtime | `rg -n "Dreamer|IRIS|Genie|world_model" crates --include=*.rs` | C.11 |
| targeted dream incubation | `rg -n "TDI|Targeted Dream|incubation_cue" crates/roko-dreams --include=*.rs` | D.04 |
| divergence / alpha taxonomy runtime | `rg -n "alpha_convergence|experiential_wisdom|divergence" crates/roko-dreams --include=*.rs` | D.06, D.07 |
| rendering / oneirography runtime | `rg -n "oneirography|dream render|portal mode|image generation" crates --include=*.rs` | E.04 |
| dream sharing / nightmare detection / lucid monitoring | `rg -n "nightmare|dream share|lucid" crates/roko-dreams crates/roko-cli --include=*.rs` | E.06 |

---

## Runtime Negatives That Matter For Batch 10

These matter because docs could mislead agents about what really ships:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| Doc 16 still assumes older `roko-golem` ownership | `roko-dreams/src/lib.rs` owns current compatibility types locally | E.01 |
| manual trigger is still easy to undercount | crate support plus `roko-cli` `dream run` / `report` / `schedule` all ship | A.07, E.02 |
| Doc 04 overstates staging-buffer complexity | current code writes/tags knowledge more directly | B.11 |
| mesh/nightmare/journal futures are mixed into integration docs | Docs 15 and 17 blend real and aspirational surfaces | E.05, E.06 |

---

## Practical Search Priorities

Before editing, search these first:

```bash
rg -n "DreamTrigger|scheduled_cron|manual_enabled|DreamHeartbeatPolicy|DreamBudget" crates/roko-dreams crates/roko-cli
rg -n "DreamReplayMode|utility_score|CounterfactualQuery|ImaginationMode|HypnagogiaEngine|ThreatScenario" crates/roko-dreams
rg -n "PatternMiner|CrossEpisodeConsolidator|k_medoids" crates/roko-learn docs/10-dreams
rg -n "roko-golem|dream run|Sleepwalker|Oneirography|nightmare|lucid" docs/10-dreams
rg -n "^> \\*\\*Implementation\\*\\*:" docs/10-dreams/*.md
```

## Working Rule

If a dreams task requires:

- new dream runtime implementation,
- new mesh/nightmare/safety implementation,
- new oneirography/image pipeline work,
- or new sleep-time compute/runtime architecture,

then batch `10` should normally implement the smallest honest
documentation/status contract and defer the runtime work.
