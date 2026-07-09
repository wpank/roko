# tui/, tui-parity/ — TUI Gap Audit & Parity Batches

**Status**: DONE — 19/19 parity batches merged; 243/270 gap items fixed

---

## tui/ — TUI Gap Audit (270 Items)

**Directory**: `tmp/tui/`
**Generated**: 2026-04-14
**Files**: 7 gap analysis docs + results

Documented 270 issues across 7 categories. As of Apr 20, 90% fixed.

| Category | File | Planned | Fixed | Partial |
|----------|------|---------|-------|---------|
| Data Flow | 01-data-flow-gaps.md | 44 | 43 | 1 |
| Input & Keys | 03-input-gaps.md | 41 | 39 | 2 |
| Modals | 04-modal-gaps.md | 19 | 18 | 1 |
| Views/Tabs | 05-view-gaps.md | 37 | 33 | 4 |
| Rendering | 07-rendering-gaps.md | 32 | 26 | 6 |
| Runtime | 08-runtime-gaps.md | 41 | 34 | 7 |
| Stubs/Dead Code | 09-stubs-placeholders.md | 56 | 50 | 6 |
| **TOTAL** | | **270** | **243** | **27** |

### Remaining 27 Partial Items

- [ ] D25: Context limits per-model still has hardcoded fallback
- [ ] K33-K41: 3 more mori key bindings needed
- [ ] M17: Some ConfirmAction variants still unhandled
- [ ] V19-V24: Dynamic per-plan agent tabs incomplete
- [ ] V37: Queue overlay data population pending
- [ ] R4-R13: PostFX pipeline hard to enable (needs roko.toml `[tui.effects]` entry)
- [ ] R17-R19: Advanced plasma/cellular background VFX pending
- [ ] RT4-RT23: Some tabs show "waiting..." on cold startup
- [ ] S29-S41: Structured error logging incomplete on some `.ok()` paths

### Key Source Files

- State: `crates/roko-cli/src/tui/state.rs`
- App: `crates/roko-cli/src/tui/app.rs`
- Input: `crates/roko-cli/src/tui/input.rs`
- Modals: `crates/roko-cli/src/tui/modals/`
- Views: `crates/roko-cli/src/tui/views/`
- Git watch: `crates/roko-cli/src/tui/git_watch.rs`
- Approval IPC: `crates/roko-cli/src/tui/approval_ipc.rs`
- Scroll accel: `crates/roko-cli/src/tui/scroll.rs`

---

## tui-parity/ — 19-Batch Execution

**Directory**: `tmp/tui-parity/`
**Status**: DONE — 19/19 batches completed and merged
**Merged**: `235d57b9` (T1-T8), `e792e649` (T9-T19)

| Batch | Title | Status |
|-------|-------|--------|
| T1 | StateHub subscription (replace polling) | DONE |
| T2 | Agent output segment parsing | DONE |
| T3 | Approval flow IPC | DONE |
| T4 | Process supervision display | DONE |
| T5 | Parallel pool + wave ribbon | DONE |
| T6 | Context metrics + route display | DONE |
| T7 | Dead field cleanup | DONE |
| T8 | Visual effects (NervViz + particles) | DONE |
| T9 | Agent-server messaging: real LLM dispatch | DONE |
| T10 | TUI snapshot bridging | DONE |
| T11 | Plan nested tasks + failures | DONE |
| T12 | Inject/filter input line visibility | DONE |
| T13 | Modal data + PlanDetail + key intercepts | DONE |
| T14 | Modal system consolidation | DONE |
| T15 | Dead widgets + dual theme merge | DONE |
| T16 | Duplicate fields + types consolidation | DONE |
| T17 | Scroll + PageUp/Down + ScrollAccel | DONE |
| T18 | Route tests + learning refactor | DONE |
| T19 | Agent-server messaging integration tests | DONE |

### New Files Created

- `crates/roko-cli/src/tui/segment.rs` — Semantic output parsing
- `crates/roko-cli/src/tui/approval_ipc.rs` — Approval IPC
- `crates/roko-cli/src/tui/postfx.rs` — Visual effects
- `crates/roko-cli/src/tui/postfx_pipeline.rs` — Effect chain

### Metrics

- ~2,200 LOC removed (dead code cleanup)
- ~350 tests written/updated
- 14/19 batches passed first try (73.7%)
- 5 batches required 1-2 retries

No remaining action on either directory.
