# TUI Polish And Cross-Surface Verification

## Scope

Use this file for TUI completion work and dashboard/TUI parity checks.

## Implementation checklist

- [ ] Continue TUI work in the existing tab/subview model.
  - `Tab::Marketplace`
  - `Tab::Atelier`
  - config/provider/model pages
  - inspect/knowledge pages
- [ ] Finish subview parity where enums already exist.
  - Provider Health
  - Model Comparison
  - Engram DAG
  - Episode Replay
  - Knowledge Browse
- [ ] Fix data refresh behavior before adding cosmetic widgets.
  - incremental readers;
  - cached logs;
  - websocket vs filesystem source of truth.
- [ ] Add polish only after correctness.
  - command palette;
  - density modes;
  - widget ports;
  - performance audits.
- [ ] Run cross-surface parity checks.
  - same underlying counts;
  - same state labels;
  - same lifecycle stages;
  - clearly documented known mismatches.

## Agent-ready task sequence

1. `TUI-PARITY-01` Tab and subview parity inventory
   - Scope: inventory every current TUI tab and subview, then map each one to its intended dashboard or CLI equivalent.
   - Touches: `crates/roko-cli/src/tui/tabs.rs`, `crates/roko-cli/src/tui/views/mod.rs`, surface docs.
   - Deliverable: one parity map for tabs, subviews, and missing equivalents.
   - Done when: every current tab/subview has a parity status of implemented, partial, intentionally TUI-only, or missing.

2. `TUI-PARITY-02` Refresh and source-of-truth correction
   - Scope: fix refresh semantics so TUI state follows backend truth instead of stale file caches or ad hoc rereads.
   - Touches: `crates/roko-cli/src/tui/dashboard.rs`, incremental readers, websocket/file-source integration points.
   - Deliverable: one refresh model with explicit source-of-truth rules.
   - Depends on: `TUI-PARITY-01`.
   - Done when: the TUI no longer requires manual rereads to converge on current backend state.

3. `TUI-PARITY-03` End-to-end job-flow parity verification
   - Scope: verify that at least one real job or plan flow shows the same counts, labels, and stages across TUI and dashboard.
   - Touches: TUI views, dashboard comparison notes, backend route/event contract references.
   - Deliverable: one verified parity walkthrough and mismatch log.
   - Depends on: `TUI-PARITY-02`.
   - Done when: a real operator flow can be observed and compared across both surfaces without hand-waving.

4. `TUI-PARITY-04` Post-correctness polish
   - Scope: add command palette, density, widget-port, and performance polish only after correctness and parity are established.
   - Touches: TUI interaction layer, widget rendering, performance instrumentation.
   - Deliverable: one polish backlog gated by correctness completion.
   - Depends on: `TUI-PARITY-03`.
   - Done when: polish items are implemented without reintroducing source-of-truth drift.

## Relevant current files

- `crates/roko-cli/src/tui/tabs.rs`
- `crates/roko-cli/src/tui/views/mod.rs`
- `crates/roko-cli/src/tui/dashboard.rs`
- `crates/roko-cli/src/tui/views/marketplace_view.rs`
- `crates/roko-cli/src/tui/views/atelier_view.rs`

## Verification checklist

- [ ] Keybindings, labels, and subview indices are consistent.
- [ ] Data refresh no longer requires full re-read where incremental behavior exists.
- [ ] TUI and dashboard show the same backend truth for at least one end-to-end job flow.

## Acceptance criteria

- TUI features are driven by real backend data.
- Parity issues between TUI and dashboard are explicit and narrow.
- Polish work comes after state correctness and refresh behavior are fixed.
