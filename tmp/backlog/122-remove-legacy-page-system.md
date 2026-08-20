# 122 — Remove Legacy Page System (`PageId` / `PageScaffold`)

**Priority**: P2 — Two parallel page/tab systems (`PageId`/`PageScaffold` for text mode, `Tab`/`SubView` for ratatui) create maintenance overhead and confusion about which system is authoritative.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/tui/`
**Depends on**: #121 (TUI data model unification should be done first to avoid merging two large concurrent refactors)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.2, `tmp/backlog/_mori-old-gaps.md` MO-07

---

## Background

The roko TUI has two overlapping systems for representing screens:
1. `Tab` / `SubView` — the ratatui-native system used by all rendering code (F1–F10 tabs, sub-views within each tab).
2. `PageId` / `PageScaffold` — a text-mode abstraction likely introduced before ratatui was fully wired; it appears in the TUI module but is not used by any rendering code path.

The dual-system creates confusion: a new contributor looking at the `PageId` enum might think it is authoritative and implement new features against it, only to find they have no rendering effect because all actual rendering goes through `Tab`. Removing `PageId`/`PageScaffold` eliminates this confusion and reduces the TUI module size.

This is a dead-code removal, not a behaviour change. The ratatui `Tab`/`SubView` system remains and is the only system used going forward.

## Current State

- `Tab::ALL` — canonical list of all 10 tabs, used by rendering code.
- `SubView` — sub-screens within each tab, used by rendering code.
- `PageId` — an enum (exact variants unknown without reading the file) that appears alongside `Tab` but is not used by rendering.
- `PageScaffold` — a struct providing text-mode scaffolding; not used by any ratatui rendering code.
- Exact file locations need inspection but likely within `crates/roko-cli/src/tui/app.rs` or `tui/mod.rs`.

## Implementation Plan

1. **Audit usage**: Run `cargo check -p roko-cli 2>&1` with `#[allow(dead_code)]` removed from `PageId` and `PageScaffold`. Note every usage site. If there are zero usage sites outside of their own definition, they are confirmed dead.

2. **Delete `PageId` enum**: Remove the enum and any `impl` blocks. Run `cargo check` and fix any compilation errors (likely only the dead code was removed).

3. **Delete `PageScaffold` struct**: Same process.

4. **Delete associated conversion functions**: Any `fn page_to_tab()`, `fn tab_to_page()`, or similar bridge functions should also be deleted.

5. **Verify no rendering regression**: Run `roko screenshot` before and after and confirm output is identical.

6. **Update any documentation comments**: If `CLAUDE.md` or any inline doc comment references `PageId`/`PageScaffold` as part of the TUI architecture, update to reference only `Tab`/`SubView`.

## Acceptance Criteria

1. `PageId` and `PageScaffold` types do not exist in the compiled binary.
2. `cargo test -p roko-cli` passes after deletion.
3. TUI renders all ten tabs correctly after deletion (verified with snapshot comparison).
4. No compilation warnings about dead code for the removed types.

## Verification Checklist

- [ ] Confirm zero usage sites for `PageId` outside its own definition.
- [ ] Delete `PageId` and `PageScaffold`; verify `cargo check` passes.
- [ ] Run `roko screenshot` before and after; verify rendered text is identical.
- [ ] `cargo clippy -p roko-cli -- -D warnings` passes.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/app.rs` or `tui/mod.rs` | Delete `PageId` enum, `PageScaffold` struct, associated impls |
| Any file importing `PageId` or `PageScaffold` | Remove imports |
