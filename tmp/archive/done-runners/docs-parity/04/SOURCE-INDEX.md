# SOURCE-INDEX — Current Code Anchors For 04-Verification

Refreshed source anchors for the post-audit verification parity pack.

Generated: 2026-04-18

---

## Important Corrections First

- The live runtime path is **not** best described by old `orchestrate.rs:111xx` anchors. Those are stale.
- `run_gate_pipeline(...)` now lives at `crates/roko-cli/src/orchestrate.rs:12604-12732`.
- The actual runtime rung mapping now lives in `crates/roko-gate/src/rung_dispatch.rs:76-120`.
- Adaptive-threshold load/save anchors moved to `3828/3962/4100` and `4563-4568`.
- `GatePipeline` remains a reusable abstraction, but it is not the clearest production-dispatch anchor.

---

## roko-core

| Anchor | Why it matters |
|---|---|
| `crates/roko-core/src/traits.rs:102-108` | canonical `Gate` trait |
| `crates/roko-core/src/engram.rs:131-136` | `Engram::derive(...)` lineage helper |
| `crates/roko-core/src/engram.rs:161-173` | builder defaults (`Decay::None`, neutral score, empty lineage/tags) |
| `crates/roko-core/src/engram.rs:185-187` | explicit `.decay(...)` setter |
| `crates/roko-core/src/decay.rs:21-30` | `Decay::HalfLife` contract |
| `crates/roko-core/src/decay.rs:57-86` | decay application behavior |
| `crates/roko-core/src/decay.rs:104-107` | `Decay::WISDOM` = 24h half-life |

---

## roko-gate

### Inventory

| Anchor | Why it matters |
|---|---|
| `crates/roko-gate/src/lib.rs:17-40` | module inventory |
| `crates/roko-gate/src/lib.rs:42-56` | public re-exports used by downstream crates |

### Live runtime dispatch

| Anchor | Why it matters |
|---|---|
| `crates/roko-gate/src/rung_dispatch.rs:29-60` | runtime inputs/config for rung execution |
| `crates/roko-gate/src/rung_dispatch.rs:76-120` | actual 7-rung runtime mapping |

### Canonical library abstractions

| Anchor | Why it matters |
|---|---|
| `crates/roko-gate/src/rung_selector.rs:25-55` | `PlanComplexity` helpers |
| `crates/roko-gate/src/rung_selector.rs:65-107` | `Rung` + `CANONICAL_ORDER` |
| `crates/roko-gate/src/rung_selector.rs:168-214` | `base_rungs(...)` + `select_rungs(...)` |
| `crates/roko-gate/src/gate_pipeline.rs:36-96` | `GatePipeline` struct/builders |
| `crates/roko-gate/src/gate_pipeline.rs:145-224` | `GatePipeline` `Gate` impl |

### Thresholds / feedback / storage

| Anchor | Why it matters |
|---|---|
| `crates/roko-gate/src/adaptive_threshold.rs:71-79` | load / `load_or_new(...)` |
| `crates/roko-gate/src/adaptive_threshold.rs:88-100` | atomic save |
| `crates/roko-gate/src/adaptive_threshold.rs:131-153` | EMA observe/update path |
| `crates/roko-gate/src/adaptive_threshold.rs:159-188` | retry / skip advisories |
| `crates/roko-gate/src/feedback.rs:53-95` | `GateFeedback` schema |
| `crates/roko-gate/src/feedback.rs:100-192` | classifier helpers |
| `crates/roko-gate/src/feedback.rs:202-237` | `feedback_for_agent(...)` |
| `crates/roko-gate/src/artifact_store.rs:21-66` | in-memory content-addressed store |
| `crates/roko-gate/src/ratchet.rs:16-74` | `GateRatchet` primitive |

---

## roko-cli / orchestrate runtime

| Anchor | Why it matters |
|---|---|
| `crates/roko-cli/src/orchestrate.rs:6181-6245` | `RunGate` entry, gate episode creation, adaptive-threshold update |
| `crates/roko-cli/src/orchestrate.rs:3828-3832` | threshold load path |
| `crates/roko-cli/src/orchestrate.rs:3962-3966` | threshold load path |
| `crates/roko-cli/src/orchestrate.rs:4100-4104` | threshold load path |
| `crates/roko-cli/src/orchestrate.rs:4563-4568` | threshold save path |
| `crates/roko-cli/src/orchestrate.rs:12604-12732` | `run_gate_pipeline(...)` |
| `crates/roko-cli/src/orchestrate.rs:12635-12646` | persisted `GateVerdict` engrams |
| `crates/roko-cli/src/orchestrate.rs:12706-12715` | conductor `Kind::GateVerdict` signal |
| `crates/roko-cli/src/orchestrate.rs:12800-12805` | post-merge rung-3 follow-up |
| `crates/roko-cli/src/orchestrate.rs:12913-12930` | `run_gate_rung(...)` wrapper |

---

## roko-learn / downstream recording

| Anchor | Why it matters |
|---|---|
| `crates/roko-learn/src/episode_logger.rs:90-119` | gate verdict schema in episodes |
| `crates/roko-learn/src/episode_logger.rs:860-885` | append-only episode persistence |
| `crates/roko-learn/src/runtime_feedback.rs:782-845` | completed-run recording / learning updates |
| `crates/roko-learn/src/skill_library.rs:1261-1279` | skill extraction entrypoints |

---

## CLI / TUI / HTTP threshold views

| Anchor | Why it matters |
|---|---|
| `crates/roko-cli/src/main.rs:5468-5484` | CLI adaptive-threshold summary |
| `crates/roko-cli/src/tui/dashboard.rs:2655-2664` | TUI retry-threshold table |
| `crates/roko-cli/src/tui/dashboard.rs:3533-3542` | TUI skip-advisory view |
| `crates/roko-cli/src/tui/dashboard.rs:3838-3868` | TUI threshold summary page |
| `crates/roko-serve/src/routes/learning.rs:101-113` | HTTP threshold endpoints |
| `crates/roko-serve/src/routes/learning.rs:256-265` | HTTP retry/skip summary fields |

---

## Stale Anchors To Remove

- `orchestrate.rs:11144-11272` for `run_gate_pipeline`
- `orchestrate.rs:11175-11185` for persisted verdict engrams
- `orchestrate.rs:11246` for conductor `GateVerdict`
- `orchestrate.rs:11339` for post-merge follow-up
- `orchestrate.rs:11423-11461` as if it still contained the real rung mapping
- `orchestrate.rs:3292/3411/3534` and `3740-3741` for adaptive thresholds

Those references describe an older code layout and should not remain in this parity pack.
