# PAJ_01: Add projection staleness TTL to DashboardSnapshot

## Task
Add `ProjectionMeta` with timestamp to `DashboardSnapshot` so TUI and serve routes can detect and display stale data.

## Runner Context
Runner PAJ (Projection & Background Tasks), batch 1 of 3. No dependencies.

## Problem
PB-1 anti-pattern: "Projections without freshness." `DashboardSnapshot` (dashboard_snapshot.rs:688-737) has ~20 data fields but no timestamp. TUI and serve routes read snapshots without knowing data age. If the runner crashes, consumers silently display stale data.

## Current Code

**DashboardSnapshot** — `crates/roko-core/src/dashboard_snapshot.rs:688-737`:
```rust
pub struct DashboardSnapshot {
    pub plans: Vec<PlanSummary>,           // L690
    pub tasks: Vec<TaskSummary>,           // L692
    pub agents: Vec<AgentSummary>,         // L694
    pub gates: Vec<GateSummary>,           // L696
    pub diagnoses: Vec<DiagnosisSummary>,  // L699
    pub experiment_winners: Vec<...>,      // L702
    pub agent_topology: Vec<...>,          // L705
    pub efficiency_trend: Vec<...>,        // L708
    pub cfactor_trend: Vec<...>,           // L711
    pub gate_trends: Vec<...>,             // L714
    pub gate_recent_failures: Vec<...>,    // L717
    pub episodes: Vec<...>,                // L720
    pub errors: Vec<...>,                  // L722
    pub event_log: Vec<...>,               // L725
    pub task_outputs: Vec<...>,            // L728
    pub cascade_router_json: Option<...>,  // L731
    pub gate_thresholds_json: Option<...>, // L734
    pub marketplace_jobs: Vec<...>,        // L737
    // NO timestamp field
}
```

**DashboardEvent** — `dashboard_snapshot.rs:25-160`:
Event types that update the snapshot. Many but no "snapshot age" tracking.

**TUI status bar** — `crates/roko-cli/src/tui/widgets/status_bar.rs:23-100`:
`render_status_bar` function renders the bottom bar. No staleness indicator.

**Serve dashboard route** — `crates/roko-serve/src/routes/status/dashboard.rs:14-17`:
```rust
pub async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    // Returns dashboard_scaffold — no age headers
}
```

## Exact Changes

### Step 1: Add ProjectionMeta to roko-core

In `crates/roko-core/src/dashboard_snapshot.rs`, before the DashboardSnapshot struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionMeta {
    pub last_updated_ms: u64,
    pub source: String,
}

impl ProjectionMeta {
    pub fn now(source: &str) -> Self {
        Self {
            last_updated_ms: chrono::Utc::now().timestamp_millis() as u64,
            source: source.to_string(),
        }
    }

    pub fn is_stale(&self, max_age: std::time::Duration) -> bool {
        let age_ms = chrono::Utc::now().timestamp_millis() as u64
            .saturating_sub(self.last_updated_ms);
        std::time::Duration::from_millis(age_ms) > max_age
    }

    pub fn age(&self) -> std::time::Duration {
        let age_ms = chrono::Utc::now().timestamp_millis() as u64
            .saturating_sub(self.last_updated_ms);
        std::time::Duration::from_millis(age_ms)
    }
}
```

### Step 2: Add meta field to DashboardSnapshot

At `dashboard_snapshot.rs:688`, add the field:

```rust
pub struct DashboardSnapshot {
    pub meta: ProjectionMeta,  // NEW — add as first field
    pub plans: Vec<PlanSummary>,
    // ... existing fields unchanged ...
}
```

### Step 3: Set meta at all snapshot construction sites

Search for `DashboardSnapshot {` to find all construction sites. At each one, add the meta field:

```rust
DashboardSnapshot {
    meta: ProjectionMeta::now("runner"),  // or "serve", "tui" depending on source
    plans: ...,
    // ...
}
```

### Step 4: Add staleness indicator to TUI status bar

In `crates/roko-cli/src/tui/widgets/status_bar.rs`, inside `render_status_bar` (around line 23-100):

```rust
// After existing status spans:
if snapshot.meta.is_stale(std::time::Duration::from_secs(30)) {
    let age = snapshot.meta.age();
    spans.push(Span::styled(
        format!(" STALE ({:.0}s) ", age.as_secs_f64()),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));
}
```

### Step 5: Add staleness headers to serve dashboard route

In `crates/roko-serve/src/routes/status/dashboard.rs:14-17`:

```rust
pub async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.dashboard_scaffold();
    let age_secs = snapshot.meta.age().as_secs();

    let mut headers = HeaderMap::new();
    headers.insert("X-Data-Age-Seconds",
        HeaderValue::from_str(&age_secs.to_string()).unwrap());
    if snapshot.meta.is_stale(std::time::Duration::from_secs(60)) {
        headers.insert("X-Data-Stale", HeaderValue::from_static("true"));
    }

    (headers, Json(snapshot))
}
```

## Write Scope
- `crates/roko-core/src/dashboard_snapshot.rs` (ProjectionMeta struct + add to DashboardSnapshot)
- `crates/roko-cli/src/tui/widgets/status_bar.rs` (staleness indicator)
- `crates/roko-serve/src/routes/status/dashboard.rs` (staleness headers)
- All DashboardSnapshot construction sites (add meta field)

## Read-Only Context
- `crates/roko-core/src/dashboard_snapshot.rs:25-160` (DashboardEvent types)
- `crates/roko-cli/src/tui/app.rs:2288-2290` (status footer in main TUI)

## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test -p roko-core 2>&1 | tail -20
cargo test -p roko-serve 2>&1 | tail -20
```

## Acceptance Criteria
- `ProjectionMeta` with timestamp on every `DashboardSnapshot`
- TUI shows yellow "STALE (Ns)" when data age > 30s
- Serve routes include `X-Data-Age-Seconds` and `X-Data-Stale` headers
- Fresh data renders normally (no visual change)
- `cargo build --workspace` passes

## Do NOT
- Auto-refresh stale projections (that's the watcher's job)
- Block on stale data (display with warning)
- Change the DashboardEvent types
- Add ProjectionMeta to DashboardEvent (only on DashboardSnapshot)
