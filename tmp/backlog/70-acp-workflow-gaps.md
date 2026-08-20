# 70 — ACP Novel Workflow Gaps

**Priority**: P2 — ACP completeness: differentiating features (Affect/Mood Ring, Dream Journal at session start, Tournament Mode) have been fully designed but have zero implementation
**Size**: L (5-7 days total across three independent sections; sections A and B are independent of each other; section C depends on neither)
**Crates**:
- `crates/roko-acp/src/` — primary implementation target
- `crates/roko-dreams/src/` — `DreamRunner` and `DreamReport` (already a dep of roko-acp)
- `crates/roko-daimon/src/` — `DaimonState`, PAD model, `DaimonPolicy`
- `crates/roko-cli/src/orchestrator/worktree.rs` — `WorktreeManager` (Section C)

**Depends on**: None (each section is self-contained; backlog 39 is a related but non-blocking improvement to DaimonState reads)

---

## Background

The ACP (Agent Client Protocol) server in `crates/roko-acp/` exposes a language-server-style protocol that IDE plugins (Cursor, VS Code) use to interact with running agents. The core ACP pipeline (session management, agent dispatch, gate execution, file notifications, cascade routing, safety contracts) is complete and tested at 180 passing tests.

Three categories of "novel workflow" features were fully designed and assigned implementation batch IDs (AF_01–AF_05, AD_01–AD_04, AT_01–AT_05) but were deferred after the core batches shipped. The underlying runtime infrastructure for each already exists: the daimon engine tracks affect state (pleasure/arousal/dominance), the dream runner produces consolidation reports, and worktree management supports multi-path execution. What is missing is purely the surfacing layer: ACP protocol messages that expose this state to the IDE.

Additionally, one structural prerequisite: `affect_enabled` is hardcoded to `false` in `crates/roko-acp/src/runner.rs` at line 585, blocking all affect card emission regardless of configuration.

---

## Current State

1. **`affect_enabled: false` hardcoded** at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` line 585. The `ServiceConfig` struct passed to `ServiceFactory::build()` has this field hardcoded to `false`. This is the only gate blocking affect cards from being emitted; the daimon engine itself works correctly.

2. **`DaimonState` and PAD model** are in `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs`. The `DaimonState` struct (line 358) has a `pad: PadVector` field. `PadVector` carries three f64 dimensions: `pleasure`, `arousal`, `dominance` (accessed in `crates/roko-daimon/src/policy.rs` line 43). The `DaimonPolicy` struct (line 18 of `policy.rs`) wraps `DaimonState` and exposes `pad` as `[f32; 3]` via `AffectPolicy`.

3. **`DreamRunner::latest_report()`** is at `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs` line 877. Returns `Result<Option<DreamReport>>` where `DreamReport` is a type alias for `DreamCycleReport` (line 46). `DreamCycleReport` is in `crates/roko-dreams/src/cycle.rs` line 73. Fields include `routing_recommendations: usize` (line 96), `clusters: Vec<DreamClusterReport>` (line 90), `knowledge_entries_written` (line 98), `playbooks_created` (line 100). The `roko-dreams` crate is already a declared dependency of `roko-acp` at `crates/roko-acp/Cargo.toml` line 29.

4. **`SessionManager`** is in `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` line 1147. It stores `sessions: HashMap<String, AcpSession>`, `workdir: PathBuf`, and `roko_config`. No `pending_dream_report` field or `DreamRunner` reference exists yet.

5. **ACP protocol event types** are in `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`:
   - `SessionUpdate::AgentMessageChunk { ... }` at line 607 — sends streaming content to the IDE
   - `SessionUpdate::ConfigOptionUpdate { ... }` at line 662 — updates the IDE's config UI
   - `RequestPermissionParams` at line 1045 — sends a permission request to the IDE

6. **`WorkflowTemplate`** enum in `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs` line 97 has three variants: `Express`, `Standard`, `Full`. No `Tournament` variant exists.

7. **`WorktreeManager`** is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrator/worktree.rs` line 423. It has `fn new(config: WorktreeConfig) -> Self` (line 449). The `roko-cli` crate is not a dependency of `roko-acp`; for Section C, either add it as a dev/optional dependency or replicate the worktree provisioning logic directly in roko-acp.

8. **`bridge_events.rs`** in `crates/roko-acp/src/bridge_events.rs` contains `request_permission()` at line 1221, which already sends `session/request_permission` to the IDE and awaits a response.

---

## Implementation Plan

### Section A: Affect/Mood Ring (AF_01 through AF_05)

**AF_01: Read `affect_enabled` from config instead of hardcoding `false`**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` line 585, replace:
```rust
affect_enabled: false,
```
with:
```rust
affect_enabled: roko_config.acp.affect_enabled,
```

Add `affect_enabled: bool` to the `[acp]` section of `RokoConfig` in `crates/roko-core/src/config/schema.rs` with `#[serde(default)]` (default: `false`). This preserves existing behavior by default and lets users opt in via `roko.toml`:
```toml
[acp]
affect_enabled = true
```

**AF_02: `AffectSnapshot` struct in `types.rs`**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`, add:
```rust
/// Snapshot of the daimon engine's PAD affect state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectSnapshot {
    /// Pleasure dimension (-1.0..1.0). Positive = approach; negative = avoidance.
    pub pleasure: f32,
    /// Arousal dimension (-1.0..1.0). Positive = activated; negative = calm.
    pub arousal: f32,
    /// Dominance dimension (-1.0..1.0). Positive = in-control; negative = constrained.
    pub dominance: f32,
    /// Human-readable label (e.g., "frustrated", "focused", "exploring").
    pub label: String,
    /// Optional somatic note (e.g., "high arousal sustained 3 phases").
    pub somatic_note: Option<String>,
}

impl AffectSnapshot {
    /// Derive a label from PAD values.
    pub fn label_from_pad(pleasure: f32, arousal: f32) -> &'static str {
        match (pleasure > 0.0, arousal > 0.3) {
            (true, true) => "engaged",
            (true, false) => "focused",
            (false, true) => "frustrated",
            (false, false) => "depleted",
        }
    }

    /// Render as a markdown card for `AgentMessageChunk`.
    pub fn to_markdown(&self) -> String {
        format!(
            "**Affect**: {} | P:{:.2} A:{:.2} D:{:.2}{}",
            self.label,
            self.pleasure,
            self.arousal,
            self.dominance,
            self.somatic_note
                .as_deref()
                .map(|n| format!(" — {n}"))
                .unwrap_or_default(),
        )
    }
}
```

**AF_03: Emit affect card after phase transitions**

In `bridge_events.rs`, after gate results are processed and a phase transition is emitted, check the current `DaimonState`. Load the state from `.roko/daimon-state.json` (or the path configured in `roko.toml`) using `DaimonPolicy::load_or_new()`. If the PAD vector has changed by more than a threshold (e.g., delta > 0.15 on any dimension) since the last emitted card, emit an `AgentMessageChunk` with the affect markdown:

```rust
if affect_enabled {
    let daimon = DaimonPolicy::load_or_new(&roko_dir.join("daimon-state.json"));
    let affect = daimon.current_affect(); // returns AffectState with pad field
    let snapshot = AffectSnapshot {
        pleasure: affect.pad[0],
        arousal: affect.pad[1],
        dominance: affect.pad[2],
        label: AffectSnapshot::label_from_pad(affect.pad[0], affect.pad[1]).to_string(),
        somatic_note: None,
    };
    // Emit only if PAD changed significantly.
    if pad_changed_significantly(&last_pad, &[affect.pad[0], affect.pad[1], affect.pad[2]]) {
        send_update(SessionUpdate::AgentMessageChunk {
            content: snapshot.to_markdown(),
            // ... other required fields ...
        }).await;
        last_pad = [affect.pad[0], affect.pad[1], affect.pad[2]];
    }
}
```

Use a helper: `fn pad_changed_significantly(prev: &[f32; 3], next: &[f32; 3]) -> bool { prev.iter().zip(next).any(|(a, b)| (a - b).abs() > 0.15) }`.

**AF_04: Auto-escalate model on persistent frustration**

Track consecutive frustrated phases in the ACP session state (add `frustrated_phase_count: u32` to `AcpSession`). Define "frustrated" as `pleasure < 0.0 && arousal > 0.3`. Read the threshold from `roko.toml` `[acp].affect_escalate_after_iters` (default: 2).

When `frustrated_phase_count >= threshold`, call the cascade router to select a more capable model. Add `affect_escalate_after_iters: u32` to the `[acp]` config section.

**AF_05: Emit `ConfigOptionUpdate` on escalation**

When AF_04 escalates, emit:
```rust
SessionUpdate::ConfigOptionUpdate {
    key: "model".to_string(),
    value: new_model.to_string(),
    reason: Some(format!("Auto-escalated due to {} frustrated phases", count)),
}
```

### Section B: Dream Journal at Session Start (AD_01 through AD_04)

**AD_01: Cache latest `DreamReport` in `SessionManager`**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`, add a field to `SessionManager`:
```rust
/// Latest dream consolidation report, cached at initialization.
pub pending_dream_report: Option<roko_dreams::DreamReport>,
/// ID of the last dream report shown to the user (to avoid repeating).
pub last_shown_dream_id: Option<String>,
```

In `SessionManager::new()`, after the existing initialization code, load the report:
```rust
use roko_dreams::{DreamLoopConfig, DreamRunner};

let dream_runner = DreamRunner::new(&workdir, DreamLoopConfig::default());
let pending_dream_report = dream_runner.latest_report().ok().flatten();
let last_shown_dream_id = Self::load_last_shown_dream_id(&workdir);
```

Add a private helper:
```rust
fn load_last_shown_dream_id(workdir: &Path) -> Option<String> {
    let path = workdir.join(".roko").join("sessions").join("last-dream-shown.txt");
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}
```

**AD_02: Emit dream report as ToolCall card on `session/new`**

In the `session/new` handler (in `bridge_events.rs` or `handler.rs` where `handle_session_prompt` or the new-session path lives), after the session is created, check for a pending dream report:

```rust
if let Some(ref report) = session_manager.pending_dream_report {
    // Skip if same report was already shown.
    let report_id = format!("{}", report.started_at.timestamp());
    if session_manager.last_shown_dream_id.as_deref() != Some(&report_id) {
        let card_content = render_dream_report_card(report);
        send_update(SessionUpdate::AgentMessageChunk {
            content: card_content,
            // kind: "tool_call", title: "Dream report — consolidated"
        }).await;
    }
}
```

The `render_dream_report_card()` function formats the key fields:
```rust
fn render_dream_report_card(report: &DreamReport) -> String {
    format!(
        "## Dream consolidation report\n\
         - Episodes processed: {}\n\
         - Knowledge entries written: {}\n\
         - Playbooks created: {}\n\
         - Routing recommendations: {}\n\
         - Clusters found: {}",
        report.processed_episodes,
        report.knowledge_entries_written,
        report.playbooks_created,
        report.routing_recommendations,
        report.clusters.len(),
    )
}
```

**AD_03: Persist the shown report ID**

After emitting the card, write the report ID to disk and update the in-memory field:
```rust
let report_id = format!("{}", report.started_at.timestamp());
let id_path = workdir.join(".roko").join("sessions").join("last-dream-shown.txt");
let _ = std::fs::create_dir_all(id_path.parent().unwrap());
let _ = std::fs::write(&id_path, &report_id);
session_manager.last_shown_dream_id = Some(report_id);
```

**AD_04: Emit `ConfigOptionUpdate` for routing advice**

If `report.routing_recommendations > 0`, emit a `ConfigOptionUpdate` after the dream card:
```rust
if report.routing_recommendations > 0 {
    send_update(SessionUpdate::ConfigOptionUpdate {
        key: "routing_updated".to_string(),
        value: "true".to_string(),
        reason: Some(format!(
            "Dream cycle produced {} routing recommendations",
            report.routing_recommendations
        )),
    }).await;
}
```

### Section C: Tournament Mode (AT_01 through AT_05)

This is the most complex section. It is entirely independent of Sections A and B and can be deferred.

**AT_01: Add `WorkflowTemplate::Tournament` variant**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`, add to `WorkflowTemplate`:
```rust
/// Parallel multi-arm competition: N agents solve the same problem independently.
Tournament {
    arms: u8,          // 2..=4
    strategies: Vec<String>,
    budget_cap_usd: Option<f64>,
},
```

Add a `TournamentConfig` struct with these fields and a `WorkflowTemplate::from_tournament_config(config: TournamentConfig) -> Self` constructor.

**AT_02: `TournamentArm` and `TournamentRun` data model**

Add to `types.rs`:
```rust
#[derive(Debug, Clone)]
pub struct TournamentArm {
    pub strategy: String,
    pub worktree_path: PathBuf,
    pub gate_passed: bool,
    pub cost_usd: f64,
    pub total_tokens: u64,
    pub output_summary: String,
}

#[derive(Debug, Clone)]
pub struct TournamentRun {
    pub arms: Vec<TournamentArm>,
    pub started_at: std::time::Instant,
    pub winner_index: Option<usize>,
}
```

**AT_03: Provision worktrees and arm-tagged event multiplexer**

For each arm in the tournament, create a temporary worktree using `git worktree add`. This avoids the `roko-cli` dependency — call `std::process::Command::new("git").args(["worktree", "add", ...])` directly in `roko-acp`. Tag each `CognitiveEvent` emitted from an arm with `arm_id: usize` so the IDE can display per-arm progress.

**AT_04: Run arms in parallel with budget watchdog**

Use `tokio::task::JoinSet` to run all arms concurrently. Each arm runs the standard `run_acp_workflow()` function with an arm-specific worktree as the working directory. A watchdog task polls cumulative per-arm cost every 5 seconds and calls `JoinSet::abort_all()` if any arm exceeds `budget_cap_usd`.

**AT_05: Comparison card and winner selection**

When all arms complete (or are cancelled by the watchdog), emit an `AgentMessageChunk` with a side-by-side comparison table:
```
| Arm | Strategy    | Gates | Cost   | Tokens  |
|-----|-------------|-------|--------|---------|
| 1   | conservative| ✓     | $0.23  | 45,200  |
| 2   | aggressive  | ✗     | $0.41  | 89,100  |
```

Then use `request_permission()` from `bridge_events.rs` line 1221 to ask the IDE to select a winner. On selection, run `git merge --ff-only` to fast-forward the working tree to the winner's worktree HEAD. Clean up losing worktrees with `git worktree remove --force`.

---

## Acceptance Criteria

### Section A (Affect/Mood Ring)
1. `[acp].affect_enabled = true` in `roko.toml` enables affect cards; the hardcoded `false` is removed from `runner.rs` line 585.
2. `AffectSnapshot` struct in `types.rs` round-trips through serde correctly.
3. When affect is enabled and a gate result causes a phase transition with PAD delta > 0.15, an `AgentMessageChunk` is emitted containing the affect markdown.
4. After 2+ consecutive frustrated phases (`pleasure < 0.0 && arousal > 0.3`), `CascadeRouter` is called to select a more capable model and a `ConfigOptionUpdate` is emitted.
5. When PAD values have not changed significantly between phases (delta <= 0.15), no affect card is emitted.

### Section B (Dream Journal)
1. `SessionManager::new()` calls `DreamRunner::latest_report()` and caches the result.
2. The first `session/new` call after a new dream report is available emits an `AgentMessageChunk` ToolCall card with the report summary.
3. A second `session/new` call with the same report does not re-emit the card (`.roko/sessions/last-dream-shown.txt` is checked).
4. If `report.routing_recommendations > 0`, a `ConfigOptionUpdate` is emitted after the dream card.

### Section C (Tournament Mode)
1. `WorkflowTemplate::Tournament { arms: 2, .. }` compiles and is selectable via session config.
2. Two worktrees are provisioned (one per arm) via `git worktree add`.
3. Both arms run in parallel; progress events are tagged with `arm_id`.
4. A budget watchdog cancels arms exceeding `budget_cap_usd`.
5. Completion emits a comparison card; `session/request_permission` allows winner selection.
6. Winner's worktree is fast-forward merged; losing worktrees are removed.

### Cross-cutting
- `cargo test -p roko-acp` passes with zero failures after all changes.
- `cargo clippy -p roko-acp --no-deps -- -D warnings` is clean.

---

## Verification Checklist

- [ ] Set `[acp] affect_enabled = true` in `roko.toml`; run an ACP session; confirm affect cards appear in the SSE stream after gate transitions
- [ ] Set `[acp] affect_enabled = false` (default); confirm no affect cards are emitted
- [ ] Run `roko knowledge dream run` to produce a fresh dream report; start a new ACP session; confirm the dream card is emitted
- [ ] Start a second ACP session immediately; confirm the dream card is NOT emitted again
- [ ] `cargo test -p roko-acp` passes
- [ ] `cargo clippy -p roko-acp --no-deps -- -D warnings` passes

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` | Replace `affect_enabled: false` at line 585 with `roko_config.acp.affect_enabled` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Add `affect_enabled: bool` and `affect_escalate_after_iters: u32` to `AcpConfig` section |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` | Add `AffectSnapshot`, `TournamentArm`, `TournamentRun` structs |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` | Add `pending_dream_report` and `last_shown_dream_id` fields to `SessionManager`; populate in `new()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | Add affect card emission after phase transitions; add dream report card on `session/new` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs` | Add `WorkflowTemplate::Tournament` variant (Section C) |

---

## Not in Scope

- **ACP stability hardening** (panics, races, silent failures) — covered by backlog 17
- **ACP spec v0.13.6 upgrade** — covered by backlog 18
- **Full DaimonState read and CalibrationTracker feedback** — covered by backlog 39 (improves the daimon state read but is not required for this spec)
- **Plugin tier check and per-command tool ceiling** — covered by backlog 45
- **Default-open read capability for code/chat modes** — covered by backlog 56
- **Custom workflow templates** (`.roko/workflows/*.toml`) — future product scope
- **Session replay / Time Warp** (Workflow 9 from design docs) — deferred to Phase 2+
- **`PhaseTwoDreamCycleReport` rendering** — deferred follow-up
- **`roko dreams journal --replay`** — deferred follow-up
- **Knowledge ingestion from losing tournament arms** — deferred follow-up
