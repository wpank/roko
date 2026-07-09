# ACP Solutions — Remaining Work (Issue Tracker)

**Created:** 2026-05-01
**Supersedes the live to-do portions of:** 02 / 04 / 05 / 06 / 07 / 08 / 09
**Status:** All R3_F0x / R5_F0x / R7_F0x batches from the original plan have shipped (verified via git log Apr 28–May 1). What remains is captured below.

---

## How this file works

Each open item has:
- A **batch id** (`AS_01`, `AF_01`, …) that maps to a prompt under `tmp/runners/post-parity/prompts/<id>.prompt.md`
- A **TOML entry** in `tmp/runners/post-parity/batches.toml`
- A short summary + the file:line where the gap currently lives

When a batch lands, flip its checkbox here AND in the per-batch `Issue Tracker` section inside its prompt file (each prompt embeds a synchronized copy).

Checkboxes are **never removed**, only flipped — this preserves history of what was done in each runner cycle.

---

## Wave 1 — Independent (no deps; can run in any order)

### AS — Session Manager Concurrency

The wrapper from R7_F03 went around slash-commands but never around `SessionManager` itself. Comment at `crates/roko-acp/src/session.rs:663-668` calls this out.

- [ ] **AS_01** — Wrap `SessionManager` in `Arc<tokio::sync::RwLock<>>` at handler call site → `prompts/AS_01.prompt.md`

---

## Wave 2 — Affect / Mood Ring (deferred R7_F07)

Original spec: `08-NOVEL-WORKFLOWS.md` Workflow 2.
Current blocker: `affect_enabled: false` hardcoded at `crates/roko-acp/src/runner.rs:461`.
Runtime side is complete (`converge-followup(B02)`, `D03`).

- [ ] **AF_01** — Enable `affect_enabled` flag from `roko.toml` + per-session override → `prompts/AF_01.prompt.md`
- [ ] **AF_02** — Add `AffectSnapshot` ACP type with serde + markdown render → `prompts/AF_02.prompt.md`
- [ ] **AF_03** — Emit affect card `AgentMessageChunk` after gate / phase transitions → `prompts/AF_03.prompt.md`
- [ ] **AF_04** — Auto-escalate model on persistent frustration via `CascadeRouter` → `prompts/AF_04.prompt.md`
- [ ] **AF_05** — `ConfigOptionUpdate` reflecting routing change after escalation → `prompts/AF_05.prompt.md`

---

## Wave 2 — Dream Journal at Session Start (deferred R7_F08)

Original spec: `08-NOVEL-WORKFLOWS.md` Workflow 3.
`roko-dreams` is already a dep (`crates/roko-acp/Cargo.toml:27`); `DreamRunner::latest_report` exists at `crates/roko-dreams/src/runner.rs:801`.
Background consolidation runs via `post-parity(PK_04)` (commit `bd02c588`).

- [ ] **AD_01** — Construct `DreamRunner` + cache latest report on `SessionManager` → `prompts/AD_01.prompt.md`
- [ ] **AD_02** — Render `DreamReport` as ToolCall card emitted right after `session/new` → `prompts/AD_02.prompt.md`
- [ ] **AD_03** — Track presented report id in `.roko/sessions/last-dream-shown.txt`; skip duplicates → `prompts/AD_03.prompt.md`
- [ ] **AD_04** — Reflect routing-advice updates as `ConfigOptionUpdate` → `prompts/AD_04.prompt.md`

---

## Wave 3–4 — Tournament Mode (deferred R3_F05)

Original spec: `08-NOVEL-WORKFLOWS.md` Workflow 5.
Building blocks ready: `WorktreeManager` (`crates/roko-orchestrator/src/worktree.rs:163`), N-arm parallel exec via `JoinSet`, `RequestPermissionParams` from R3_F04.

- [ ] **AT_01** — `WorkflowTemplate::Tournament` variant + `TournamentConfig` (arms 2..=4, $-cap) → `prompts/AT_01.prompt.md`
- [ ] **AT_02** — `TournamentArm` / `TournamentRun` data model + helpers → `prompts/AT_02.prompt.md`
- [ ] **AT_03** — Provision N worktrees + arm-tagged event multiplexer → `prompts/AT_03.prompt.md`
- [ ] **AT_04** — Run arms in parallel + collect verdicts + budget-cap watchdog → `prompts/AT_04.prompt.md`
- [ ] **AT_05** — Comparison ToolCall card + `RequestPermission` for winner; ff-only merge on apply → `prompts/AT_05.prompt.md`

---

## Cross-cutting follow-ups (not in any batch)

These are not large enough to warrant their own batch but are tracked so they don't get lost. File a follow-up if any becomes urgent.

### Affect

- [ ] Wire affect to `run_with_workflow_engine` ServiceFactory path too — currently only the legacy pipeline emits cards (AF_03 follow-up)
- [ ] Daimon state persistence after each ACP prompt (currently only at WE shutdown)
- [ ] Cap card emission frequency to STATE-TRANSITIONS only (not every phase) if PADs flap
- [ ] Add `affect.escalate_after_iters` config option (AF_04 currently hardcodes `2`)
- [ ] Demote model when struggling resolves mid-pipeline (AF_04 only stops escalating new dispatches; doesn't roll back)
- [ ] Cost ceiling: integrate AF_04 escalation with `WorkflowRun.total_cost_usd` budget (cross-ref `D1E_*`)

### Dreams

- [ ] File watcher to invalidate `pending_dream_report` mid-process when a new report appears
- [ ] Render `PhaseTwoDreamCycleReport` (REM/NREM split) when present (AD_02 currently DreamCycleReport-only)
- [ ] `roko dreams journal --replay` CLI command to re-show the latest journal in a session
- [ ] Persist user override of dream routing advice (needs new RPC)

### Tournament

- [ ] CLI: `roko tournament list / show / discard <id>` for post-hoc inspection
- [ ] Custom RPC `tournament/diff` returning per-arm patch (for editor "Compare diffs" button)
- [ ] Single-arm cancellation (cancel one arm without aborting tournament)
- [ ] Stagger arm starts by 500ms to ease upstream rate-limit pressure
- [ ] Cap concurrent dispatches across the whole tournament (4 arms × 3 phases × retries can blow rate limits)
- [ ] Knowledge ingestion: feed losing arms' diffs to `roko-neuro` as failure-of-strategy training data
- [ ] Episode logging: emit one combined episode for the tournament, not N separate
- [ ] Fall back to merge `--no-ff` (with chunk warning) when ff-only fails

### Concurrency / lifecycle

- [ ] After AS_01: switch `HashMap<String, AcpSession>` to `DashMap` if write-lock contention shows up under daemon mode
- [ ] Document the `await`-while-holding-write-lock invariant in `crates/roko-acp/src/lib.rs` doc comment
- [ ] Cross-process: persisted session JSON in `.roko/sessions/` is not file-locked; multi-process ACP daemons could race

---

## What is no longer in this file

The following were deferred per `08-NOVEL-WORKFLOWS.md` priorities and are now tracked here. The historical analysis docs (00–09) remain in `tmp/solutions/acp/` for reference but should be considered **archive** — do not edit them as a live to-do list:

| Doc | Status | Action |
|---|---|---|
| `00-INDEX.md` | Stale (TL;DR no longer accurate) | Archive (do not delete; refer for context) |
| `01-CURRENT-STATE.md` | LOC count outdated; "what doesn't work" largely fixed | Archive |
| `02-GAP-ANALYSIS.md` | Gaps 1–12 implemented; gaps 13+ tracked here | Archive |
| `03-ARCHITECTURE-PLAN.md` | Implemented via `ServiceFactory` (`converge-followup(C01)`) | Archive |
| `04-MEGA-PARITY-OVERLAP.md` | Strategy planning, no longer actionable | Archive |
| `05-TASK-BATCHES.md` | All 10 ACP-W batches landed | Archive |
| `06-MEGA-PARITY-INTEGRATION.md` | All R3/R5/R7 F-batches merged | Archive |
| `07-UX-GAP-ANALYSIS.md` | All 7 screenshot-parity items addressed | Archive |
| `08-NOVEL-WORKFLOWS.md` | P0 shipped; P1 (Mood Ring, Dream Journal) → AF/AD here; P2 (Tournament) → AT here | Archive |
| `09-NOVEL-BATCHES.md` | R5_F06 + R7_F10 shipped; R7_F07/R7_F08/R3_F05 → batches in this tracker | Archive |
| **`REMAINING.md`** (this file) | **Live tracker** | **Edit this file going forward** |

---

## Runbook

```bash
# All open ACP batches: 15 total across 4 group families
ls tmp/runners/post-parity/prompts/AS_*.prompt.md tmp/runners/post-parity/prompts/AF_*.prompt.md tmp/runners/post-parity/prompts/AD_*.prompt.md tmp/runners/post-parity/prompts/AT_*.prompt.md

# Run only ACP groups (parallel template skips groups not requested)
./tmp/runners/post-parity/run.sh --group AS_A,AF_A,AF_B,AD_A,AT_A,AT_B,AT_C

# Or run single batches:
./tmp/runners/post-parity/run.sh --only AS_01,AF_01,AF_02,AF_03,AF_04,AF_05

# Wave-friendly ordering (use this if running sequentially):
# Wave 1 (independent):              AS_01, AF_01, AD_01, AT_01
# Wave 2 (per-group seconds):        AF_02, AD_02, AT_02
# Wave 3 (per-group thirds):         AF_03, AD_03, AD_04, AT_03
# Wave 4 (per-group fourths):        AF_04, AT_04
# Wave 5 (finalizers):               AF_05, AT_05
```

---

## Convention for follow-up audits

When a runner finishes:
1. Flip the checkboxes here for landed batches.
2. Move any newly-discovered work into the matching "Cross-cutting follow-ups" sub-section (don't create new groups unless you have ≥3 batches' worth).
3. If a follow-up grows to ≥3 batches, promote it to a new group with its own prompts and add a new section in this file.
4. Don't archive this file — it's the persistent ACP issue tracker.
