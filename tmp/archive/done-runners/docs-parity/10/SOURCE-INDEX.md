# SOURCE-INDEX - Code Anchors for 10-Dreams Parity

Verified code references for batch `10`, refreshed to current line ranges.

Generated: 2026-04-18

---

## First Corrections

- `roko-dreams` is the active runtime owner for dreams, replay, imagination, hypnagogia, and threat simulation.
- `roko-dreams/src/lib.rs` no longer re-exports from `roko-golem`.
- `dream run`, `dream report`, and `dream schedule` already ship in `roko-cli`.
- scheduled cron triggers already ship.
- manual triggers already ship.
- oneirography, Sleepwalker mode, dream sharing, and lucid monitoring do not ship.

---

## `crates/roko-dreams/src/`

### Public runtime surface

| File | What |
|------|------|
| `lib.rs:36-60` | module exports for cycle, replay, imagination, hypnagogia, runner, and threat |
| `runner.rs:156-216` | `DreamBudget` |
| `runner.rs:221-343` | `DreamTrigger`, `DreamSchedulePolicy`, cron/manual support |
| `runner.rs:347-500` | `DreamHeartbeatPolicy`, `DreamHeartbeatReport`, heartbeat readiness |
| `runner.rs:405-442` | `DreamRuntimeControls` |
| `runner.rs:504-566` | `DreamRunner`, `plan_replay()`, latest-report facade |
| `runner.rs:646-648` | `PlaybookStore` plus `DreamCycle::new(...)` wiring |
| `runner.rs:829-848` | `load_latest_dream_report()` |
| `runner.rs:891-950` | `build_dream_review_dispatcher()` |

### Core cycle and outputs

| File | What |
|------|------|
| `cycle.rs:67-96` | `DreamCycleReport` |
| `cycle.rs:259-324` | `DreamClusterKey`, `DreamOutcome`, `DreamClusterReport` |
| `cycle.rs:333-540` | `DreamCycle` run path |
| `cycle.rs:502-509` | liminal phase wiring: hypnagogia, REM hypotheses, threat warnings |
| `cycle.rs:518-539` | report assembly |
| `cycle.rs:681-689` | report persistence |
| `cycle.rs:1100-1146` | HDC similarity / vector construction helpers |
| `cycle.rs:1959-1978` | cluster-vector HDC construction |

### Replay, imagination, liminal, threat

| File | What |
|------|------|
| `replay.rs:14-117` | `DreamReplayMode`, `DreamReplayPolicy`, `DreamReplayBatch`, `select_replay_episodes()` |
| `replay.rs:126-156` | utility calculation (`reward * novelty * recency`) |
| `imagination.rs:17-174` | `CounterfactualQuery`, `ImaginationMode`, `CausalModel`, `imagine()` |
| `imagination.rs:176-289` | `synthesize_hypotheses()` and the three creativity modes |
| `imagination.rs:291-317` | `counterfactual_episode()` |
| `hypnagogia.rs:15-160` | four-layer hypnagogia engine plus `run()` |
| `threat.rs:14-120` | `ThreatScenario`, `enumerate_threats()`, `threat_warning_entries()` |

---

## `crates/roko-cli/src/`

### User-facing and long-running dream surfaces

| File | What |
|------|------|
| `main.rs:5609-5704` | `dream run`, `dream report`, `dream schedule` |
| `main.rs:5704-5723` | `build_dream_runner()` |
| `daemon.rs:239-268` | daemon starts the dream loop |
| `orchestrate.rs:5890-5969` | auto-dream trigger after plan completion |

---

## Supporting Infrastructure

| File | What |
|------|------|
| `pattern_discovery.rs:99-245` | `PatternMiner` |
| `pattern_discovery.rs:291-390` | `CrossEpisodeConsolidator` |
| `hdc_clustering.rs:54-120` | `k_medoids()` core |
| `playbook.rs:192-237` | `PlaybookStore` |
| `runtime_feedback.rs:337-376` | learning runtime owns `PatternMiner` |
| `runtime_feedback.rs:570-574` | learning runtime exposes cross-episode consolidation |

Important narrowing:

- `PlaybookStore` is directly used by dreams.
- `PatternMiner`, `CrossEpisodeConsolidator`, and `k_medoids()` ship in `roko-learn`, but the current dreams cycle does not directly call them.

---

## Missing / Absent

These features do not have matching production code in the active tree:

| Absent feature | Search |
|----------------|--------|
| intensive consolidation / backlog watermarks | `rg -n "intensive|high_watermark|low_watermark|backlog" crates/roko-dreams --glob '*.rs'` |
| advanced counterfactual diversity stacks | `rg -n "DiCE|FACE|LOF|GLOBE|DPP" crates/roko-dreams --glob '*.rs'` |
| HER / PER / ERE replay families | `rg -n "hindsight|prioritized_replay|generative_replay|ERE" crates/roko-dreams --glob '*.rs'` |
| MAP-Elites / evolution runtime | `rg -n "MAP.?Elites|quality_diversity|memetic|dream_evolution" crates/roko-dreams --glob '*.rs'` |
| sleep-time compute / `rethink_memory` / Sleepwalker | `rg -n "rethink_memory|Sleepwalker|sleep_time" crates --glob '*.rs'` |
| world models | `rg -n "Dreamer|IRIS|Genie|world_model" crates --glob '*.rs'` |
| targeted dream incubation | `rg -n "TDI|Targeted Dream|incubation" crates/roko-dreams --glob '*.rs'` |
| divergence / alpha runtime | `rg -n "alpha_convergence|divergence|experiential_wisdom" crates/roko-dreams --glob '*.rs'` |
| rendering / oneirography runtime | `rg -n "oneirography|dream render|image generation" crates --glob '*.rs'` |
| dream sharing / lucid monitoring | `rg -n "dream share|lucid|meta_awareness" crates --glob '*.rs'` |

---

## Working Rule

If a topic-10 task requires runtime implementation rather than status
correction, capture the seam and defer it.
