# Routes panel renders zero data rows

- Severity: medium
- Status: code-confirmed
- Area: TUI layout

## Observation

The state layer creates a fallback route metric for each visible agent (`crates/roko-cli/src/tui/state.rs:2222-2245`). For one route, the dashboard allocates four total lines (`dashboard_view.rs:229-238`). After borders/header, the renderer applies another `saturating_sub(2)` and takes zero rows (`dashboard_view.rs:438-455`).

This matches the screenshot: Routes displays its column headers but no agent/model/tier row.

## Expected

Sizing and rendering should share one definition of border/header overhead. A panel with one route must allocate and render one data row.

