# M052 — Wire StateHub into TUI, HTTP, and WebSocket

## Objective
Wire StateHub projections into all three rendering surfaces: TUI tabs consume projections for display, HTTP routes under `/api/statehub/` expose projections as JSON, and SSE/WebSocket streams push projection updates in real-time. All three surfaces reflect the same data from the same source, ensuring consistency.

## Scope
- Crates: `roko-cli`, `roko-serve`
- Files: `crates/roko-cli/src/tui/` (multiple tab files), `crates/roko-serve/src/routes/statehub.rs` (new), `crates/roko-serve/src/routes/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.7
- Spec ref: `tmp/unified/09-TELEMETRY.md` SS4

## Steps
1. Read the current TUI data sources:
   ```bash
   grep -rn 'AppState\|state\.' crates/roko-cli/src/tui/ --include='*.rs' | head -20
   ls crates/roko-cli/src/tui/
   ```

2. Read the current HTTP route structure:
   ```bash
   grep -rn 'Router\|route\|get\|post' crates/roko-serve/src/routes/mod.rs | head -20
   ls crates/roko-serve/src/routes/
   ```

3. **TUI integration**: Add StateHub as a data source for TUI tabs.
   - Pass `Arc<StateHub>` to the TUI app state
   - Each tab reads its relevant projection: Agents tab reads `AgentStatus`, Plans tab reads `FlowSummary`, etc.
   - Refresh on interval or on projection version change

4. **HTTP integration**: Create `crates/roko-serve/src/routes/statehub.rs`:
   ```rust
   // GET /api/statehub/flows       -> Vec<FlowSummary>
   // GET /api/statehub/agents      -> Vec<AgentStatus>
   // GET /api/statehub/space       -> SpaceStatus
   // GET /api/statehub/memory      -> MemoryStats
   // GET /api/statehub/routing     -> RouteStats
   // GET /api/statehub/verify      -> VerifyStats
   // GET /api/statehub/cost        -> CostSummary
   // GET /api/statehub/all         -> all projections
   ```

5. **SSE/WebSocket integration**: Add a projection update stream.
   - SSE: `GET /api/statehub/stream` sends projection updates as SSE events
   - WebSocket: `WS /api/statehub/ws` pushes projection updates as JSON messages
   - Both use StateHub's subscribe mechanism from M051

6. Register new routes in `crates/roko-serve/src/routes/mod.rs`.

7. Write tests:
   - HTTP endpoint returns valid JSON for each projection type
   - TUI tab renders with StateHub data (unit test with mock StateHub)
   - SSE stream delivers update when projection changes

## Verification
```bash
cargo check -p roko-cli
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- statehub
# Manual: cargo run -p roko-cli -- serve & curl http://localhost:6677/api/statehub/all
```

## What NOT to do
- Do NOT duplicate projection computation in each surface -- all surfaces read from the same StateHub
- Do NOT break existing TUI tabs -- add StateHub as an additional data source alongside existing ones
- Do NOT break existing HTTP routes -- add new routes under `/api/statehub/`
- Do NOT add authentication to statehub routes -- that is a cross-cutting concern handled elsewhere
