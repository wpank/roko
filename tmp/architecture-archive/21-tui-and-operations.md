# TUI Enhancements and Operations

> Part of the [Roko Architecture Specification](00-INDEX.md).
> Folded from `tmp/bardo-integration-plan.md` Phases 8-9. Original bardo source references preserved.

---

## TUI enhancements

These add visualization capabilities to roko's ratatui TUI (`crates/roko-cli/src/tui/`), porting screens from bardo's terminal (`bardo/apps/bardo-terminal/src/screens/`).

---

### 1. DaimonState visualization

**Source**: `bardo/apps/bardo-terminal/src/screens/` — Emotions, Vitality screens
**Target**: `crates/roko-cli/src/tui/views/` (new view)
**Existing**: `roko/crates/roko-daimon/src/` — DaimonState already loaded in orchestrate.rs

New sub-view in Dashboard (F1) or new tab (F11 Affect):

**Layout**:
- PAD vector display: Pleasure [-1,1], Arousal [-1,1], Dominance [-1,1] as horizontal gauges
- Current PadRegion label (Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/Bored)
- Somatic marker histogram: last 10 markers with valence coloring
- Behavioral bias indicators: which biases are active (AvoidTrade, SeekSafety, etc.)

**Data source**: DaimonState is already loaded in orchestrate.rs — pipe it through DashboardEvent to TUI state.

**Visual style**: Use existing braille sparklines and gauge widgets.

**Acceptance criteria**:
- [ ] PAD gauges render with correct values from DaimonState
- [ ] PadRegion label updates in real-time
- [ ] Somatic markers visible with positive (green) / negative (red) coloring
- [ ] View accessible via F-key or tab navigation

**Size**: M (2 days)

---

### 2. Heartbeat status view in Learning tab

**Source**: `bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs`
**Target**: `crates/roko-cli/src/tui/views/learning_view.rs` (extend)

Add a heartbeat status sub-view to the Learning tab (F10):

**Layout**:
1. **Accuracy sparkline**: Rolling sparkline of prediction accuracy (correct/total per window)
2. **Recent predictions**: Table of last 10 prediction/outcome pairs with model, tier, cost
3. **Tier distribution**: Bar chart showing T0/T1/T2 tick percentages
4. **Cost trend**: Sparkline of per-tick cost over last hour

**Data source**: Efficiency events JSONL + episodes JSONL (already tailed by TUI).

**Acceptance criteria**:
- [ ] Accuracy sparkline updates as new episodes arrive
- [ ] Tier distribution shows percentage of T0/T1/T2 ticks
- [ ] Cost trend sparkline renders
- [ ] Accessible as sub-view within F10 (Learning tab)

**Size**: S (1 day)

---

### 3. Knowledge browser in Inspect tab

**Source**: `bardo/apps/bardo-terminal/src/screens/knowledge.rs` — Grimoire stats, top-confidence entries
**Target**: `crates/roko-cli/src/tui/views/` (extend inspect view)

Enhance the Knowledge sub-view in Inspect tab (F7):

**Layout**:
1. **Store stats**: Total entries, per-tier counts (Tier1/2/3/Archive), health percentage
2. **Top entries**: Scrollable list of highest-confidence entries with: title, confidence, tier, last_accessed, type label
3. **Decay visualization**: Time-since-access color gradient (green=fresh, yellow=aging, red=decaying)
4. **Distillation events**: Recent distillation timeline (when knowledge was summarized/compressed)

**Data source**: roko-neuro knowledge store API.

**Acceptance criteria**:
- [ ] Store stats render (entry counts, tier distribution)
- [ ] Top entries scrollable with confidence and tier info
- [ ] Decay visualization uses color gradients

**Size**: S (1 day)

---

## Operational infrastructure

Development tooling and production reliability features.

---

### 4. Justfile (developer convenience)

**Source**: `bardo/justfile` (136 lines)
**Target**: `justfile` at repo root

Common development commands:

```just
build         := cargo build --workspace
test          := cargo test --workspace
lint          := cargo clippy --workspace --no-deps -- -D warnings
fmt           := cargo +nightly fmt --all
fmt-check     := cargo +nightly fmt --all -- --check
check         := cargo check --workspace
ci            := fmt-check && lint && test
coverage      := cargo llvm-cov --workspace --html
watch         := cargo watch -x 'check --workspace'
deny          := cargo deny check
doc           := cargo doc --workspace --no-deps
clean         := cargo clean
serve         := cargo run -p roko-cli -- serve
dashboard     := cargo run -p roko-cli -- dashboard
run           := cargo run -p roko-cli --
```

**Acceptance criteria**:
- [ ] `just ci` runs fmt-check + lint + test
- [ ] `just serve` starts the server
- [ ] `just dashboard` starts the TUI
- [ ] All shortcuts work from repo root

**Size**: S (half day)

---

### 5. E2E test harness

**Source**: `bardo/tests/harness/src/lib.rs` — BardoTestHarness, HealthReport, TerminalProbe
**Target**: `tests/harness/`

Multi-component integration test framework:

1. **RokoTestHarness** struct: Manages spawning roko-serve + mirage-rs as child processes
2. **spawn_serve(config) -> ServerHandle**: Start roko-serve on random port, wait for health check
3. **spawn_mirage(config) -> MirageHandle**: Start mirage-rs on random port
4. **health_check(url) -> HealthReport**: Poll `/api/health` until ready or timeout (30s)
5. **cleanup()**: Kill all child processes on Drop (no leaked processes)
6. Add to workspace as `[dev-dependencies]` for integration tests

**Acceptance criteria**:
- [ ] `RokoTestHarness::new()` spawns serve + mirage
- [ ] Health check waits up to 30s for services to be ready
- [ ] Drop impl kills all child processes
- [ ] Integration test using harness passes: spawn → health check → stop

**Size**: M (2 days)

---

### 6. Self-healing supervisor script

**Source**: `bardo/bardo-supervisor.sh` (381 LOC)
**Target**: `scripts/roko-supervisor.sh`

Production crash recovery:

1. **Crash detection**: Monitor roko process exit code. On non-zero exit, extract panic signature from stderr.
2. **Error deduplication**: Track error signatures in `/tmp/roko-supervisor-errors.json`. Skip auto-fix for already-seen errors.
3. **Auto-fix** (optional, requires Claude CLI): Feed crash report + recent logs to Claude for diagnosis. Apply suggested fix. Restart.
4. **Circuit breaker**: After 3 consecutive restarts within 5 minutes, stop trying. Alert via stderr.
5. **Signal handling**: Forward SIGTERM/SIGINT to child process. Clean shutdown.
6. **Configurable**: `ROKO_SUPERVISOR_MAX_RESTARTS=3`, `ROKO_SUPERVISOR_WINDOW_SECS=300`, `ROKO_SUPERVISOR_AUTOFIX=false`

**Acceptance criteria**:
- [ ] Script restarts roko on crash
- [ ] Error signatures deduplicated
- [ ] Circuit breaker stops after N restarts in window
- [ ] SIGTERM forwarded to child process
- [ ] Works without Claude CLI (autofix disabled by default)

**Size**: M (1-2 days)

---

## Conductor watcher configuration (added 2026-04-25)

> Backported from `tmp/architecture-plans/06-architecture-implementation.md` Phase OG and `20-orchestrator-gaps.md` spec clarifications.

All 10 conductor watchers are implemented in `crates/roko-conductor/src/watchers/`. Their thresholds are configurable via `[conductor]` in `roko.toml`:

```toml
[conductor]
# Watcher thresholds (all have sensible defaults)
ghost_turn_max_secs = 5           # GhostTurn: no output + fast turn
review_loop_max_consecutive = 3   # ReviewLoop: consecutive REVISE verdicts
iteration_loop_max = 6            # IterationLoop: cycling strategist/implementer
test_failure_budget_pass_rate = 0.70  # TestFailureBudget: force advance threshold
silence_timeout_secs = 180        # SilenceTimeout: no output
compile_fail_max_consecutive = 3  # CompileFailThreshold: consecutive failures
task_stall_secs = 300             # TaskStall: single task blocking
context_pressure_percent = 80     # ContextPressure: prompt >80% of window
phase_timeout_secs = 1800         # PhaseTimeout: 30min wall-clock
cooldown_filter_secs = 120        # CooldownFilter: debounce interval
```

Missing keys use the hardcoded defaults from the watchers table in `20-orchestrator-gaps.md`.

---

## Implementation state (updated 2026-04-25)

### What already exists

| Item | Location | Status |
|------|----------|--------|
| TUI with F1-F7 tabs | `roko-cli/src/tui/` | **EXISTS** — ratatui, file watcher, live data |
| DaimonState loaded per-task | `roko-cli/src/orchestrate.rs` | **EXISTS** — PAD vector computed, used in dispatch |
| Efficiency events JSONL | `.roko/learn/efficiency.jsonl` | **EXISTS** — per-turn cost/latency/outcome |
| Episode log JSONL | `.roko/episodes.jsonl` | **EXISTS** — full agent turn records |
| Knowledge store | `roko-neuro/src/knowledge_store.rs` | **EXISTS** — query API for knowledge browser |
| 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — all rules implemented |

### What needs building

| Task | Status | Notes |
|------|--------|-------|
| DaimonState TUI view | **Missing** | PAD gauges, somatic markers, behavioral bias |
| Heartbeat status in Learning tab | **Missing** | Accuracy sparkline, tier distribution, cost trend |
| Knowledge browser in Inspect tab | **Missing** | Store stats, top entries, decay visualization |
| Justfile | **Missing** | Developer convenience commands |
| E2E test harness | **Missing** | Multi-component integration test framework |
| Self-healing supervisor | **Missing** | Crash recovery, circuit breaker, signal forwarding |
