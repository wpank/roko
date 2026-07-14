# CTRL-02 TUI bounds precursor implementation evidence

Assignment:
- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0 `CTRL-02`; bounded precursor adjacent to `SH04-T06` in `tmp/status-quo/self-heal/plans/SH04-runtime-telemetry-tui/tasks.toml`
- Base SHA: `bafcebb686d12bd83b0c9a76f0d937c8b53083dd`
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3` relative to parent `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Branch/worktree: `agent/CTRL-02-tui-bounds` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-TUI-BOUNDS`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `crates/roko-cli/src/tui/state.rs`, `crates/roko-cli/src/tui/views/dashboard_view.rs`, and this evidence record
- Dependencies and their integration commits: the clean integration base already contains the exact `3041d095d` TUI precursor; this bounded output/layout subset does not claim the unresolved structured-identity prerequisite `SH04-T01`

Requirement:
- Original defect: the July 14 precursor fixed repeated re-append of an authoritative connected-snapshot output ring, bounded displayed agent output to 50 lines, rebuilt task-output tails rather than leaking stale task entries, and corrected route table capacity so a single route can occupy the first row after the header. Those changes were embedded in the broad 31-path commit `3041d095d` and needed task-sized attribution and present-tree proof.
- Residual defect reproduced here: an explicitly empty connected-snapshot ring was filtered out, so the reducer retained the previous task's `output_lines` and `last_output_line` across a task transition even though the connected snapshot is authoritative.
- Expected behavior: every connected snapshot replaces the selected task's output cache, including with empty; non-empty rings retain only their latest 50 lines; replacing snapshots never duplicate output; stale task-tail keys disappear; and a route panel with border, header, and one data row renders that row at its minimum height of four cells.
- Acceptance for this bounded candidate: focused connected-snapshot regressions, direct route-buffer proof, full `cargo test -p roko-cli tui`, formatting, affected-package all-target check, diff hygiene, and independent review of the exact candidate.
- Explicit non-goals: phase inference/reconciliation, `phase_compact.rs`, `agents_view.rs`, structured agent identity, approval-TUI event connection, Git refresh, manifests, task status, the master checklist, or full `SH04-T06` acceptance. In particular, issue `11-active-agent-can-show-all-phases-complete.md`, phase-invariant acceptance, and the remaining findings in `14-TUI-DASHBOARD.md` stay open.

Reproduction:
- Pre-fix command (working directory above): `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli --lib connected_snapshot_empty_output_ring_clears_previous_task_output -- --nocapture`
- Expected: the new connected snapshot for `task-2` clears output cached for `task-1` and exposes the authoritative empty `task-2` ring.
- Actual before correction: exit 101; 0 passed, 1 failed. `state.agents[0].output_lines.is_empty()` failed because `task-1` output was retained.
- Historical route reproduction: `git diff 1649c18b2c3d2b3602bfe17398b0e1454a19c5ef..3041d095d4daebed2c9e05c63eacb18e668e37e3 -- crates/roko-cli/src/tui/views/dashboard_view.rs` shows the old renderer taking `inner.height.saturating_sub(2)` rows after the block had already removed borders; an inner height of two therefore rendered the header and zero data rows. The inherited `route_data_row_capacity` correction subtracts only the one table-header row.

Implementation:
- `TuiState::update_from_dashboard_snapshot` now replaces output from the authoritative `task_outputs` map on every connected snapshot. An absent or explicitly empty task ring produces empty `output_lines` and `last_output_line` instead of falling back to a previous row.
- Existing `bounded_output_lines` remains the single 50-line projection for both agent rows and the rebuilt `task_output_tails` map. Existing tests prove repeated snapshots do not duplicate entries, stale task keys are removed, and a 100-line source keeps lines 50 through 99.
- Added an adversarial task-transition regression covering the empty-ring boundary and both rendered caches.
- Added a direct ratatui buffer regression at the route panel's exact minimum height (`48x4`): two border rows, one header row, and one route data row. It asserts both the agent and model survive rendering.
- Compatibility and resource behavior: no public API or serialized format changed. Connected snapshot memory remains bounded to 50 lines per published task/agent projection; the change removes stale retained data. Pull/disk state paths were deliberately not changed.

Verification:
- `cargo fmt --all -- --check`
  - Exit 0.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli --lib connected_snapshot -- --nocapture`
  - Exit 0; 3 passed, 0 failed: authoritative replacement, empty-ring clearing, and 50-line bounding.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli --lib one_route_is_visible -- --nocapture`
  - Exit 0; 2 passed, 0 failed: full Agents-buffer route and exact minimum-panel route.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo test -p roko-cli tui`
  - Exit 0; the library TUI selection ran 245 tests, all passed; all other binary/integration targets had zero matching tests. The pre-existing `tests/plan_validation.rs` missing-crate-doc warning remains outside this scope.
- `CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target cargo check -p roko-cli --all-targets`
  - Exit 0; the same pre-existing `tests/plan_validation.rs` warning was emitted.
- `git diff --check`
  - Exit 0.
- Artifact disposition: all builds used the integration-owned shared Cargo target; this worker created no local `target`, symlink, generated index, log, runtime process, or other artifact.

Review readiness:
- Implementation components: inherited bounded-output and route-capacity implementation in `3041d095d`; present-base empty-ring correction and direct regressions in this candidate.
- Exact candidate SHA: supplied by the independent reviewer after this evidence and the two reserved source files are committed together.
- Cumulative candidate scope: exactly the two reserved Rust files and this evidence record.
- Required reviewer focus: authoritative-full-snapshot semantics for absent/empty rings, no cross-task stale output, preservation of the 50-line tail, minimum-height route table arithmetic, and proof that this bounded candidate does not imply phase-invariant or full `SH04-T06` closure.
- Known limitations: full `SH04-T06` remains `ready`; issue 11 phase contradictions require structured identity and phase-state reconciliation in separately reviewed work.

Integration:
- Independent review `14dc40953d83859a8c8293413e6dcdbfb977d380`
  accepted the exact candidate with no findings; integrated implementation and
  review commits are `0a307ab08` and `1eb2eabb6`.
- Post-merge `cargo test -p roko-cli tui` passed 245/245 and
  `cargo check -p roko-cli --all-targets`, format, and diff checks passed. The
  only warning is the pre-existing plan-validation test crate's missing crate
  documentation.
- Final status: `DONE` for this bounded CTRL-02 TUI precursor attribution only.
  `SH04-T06`, SH04, issue 11, remaining issue 14 findings, Wave 4, and the
  programme remain open.
