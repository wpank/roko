# TUI Design Sketch

## Tabs (F1–F7)
- F1: Dashboard (plan progress, agent status)
- F2: Agents (list, logs, health)
- F3: Tasks (DAG view, status)
- F4: Gates (pipeline results)
- F5: Episodes (recent agent turns)
- F6: Learning (router stats, experiments)
- F7: Config (current settings)

## Architecture
- ratatui for rendering
- DashboardEvent push model (watch::Sender)
- File watcher for live updates (.roko/ directory)
- TuiBridge convenience methods for each tab

## Status: built, needs wiring
