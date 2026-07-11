# Runner-v2 diagnosis feed is unwired

- Severity: medium
- Status: code-confirmed
- Area: conductor/diagnosis telemetry

## Observation

The TUI only renders `DashboardSnapshot.diagnoses` (`crates/roko-cli/src/tui/state.rs:2264`). Diagnosis publishing exists in legacy orchestration paths, but runner-v2's event loop has no corresponding diagnosis bridge. The panel therefore says `no conductor diagnoses yet` even after repeated gate failures and classified error digests.

## Expected

Runner-v2 should publish gate classifications, retry decisions, blockers, and conductor findings into the diagnosis stream, or the panel should be hidden/renamed when that producer is unavailable.

