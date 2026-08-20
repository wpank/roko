# 111 — Screenshot Command Completion (Flags, Symlink, Sub-Views)

**Priority**: P1 — Self-assessment of TUI output currently requires a real terminal; the screenshot command skeleton exists but critical flags and output modes are unimplemented.
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli/src/commands/screenshot.rs`, `crates/roko-cli/src/tui/snapshot.rs`
**Depends on**: None (skeleton already exists)
**Sources**: `tmp/backlog/_checklist-gaps.md` §0.2, `tmp/backlog/_mori-old-gaps.md` MO-01

---

## Background

Claude and other automated agents need a way to inspect the TUI's rendered output without an attached terminal. The `roko screenshot` command was introduced to fill this role: it spins up a headless ratatui `TestBackend`, renders all ten tabs, and writes the results to disk.

The infrastructure is partially in place. `crates/roko-cli/src/tui/snapshot.rs` implements a text-rendering engine and `crates/roko-cli/src/commands/screenshot.rs` contains the `SnapshotConfig`/`capture_snapshots` skeleton. However, the flags documented in the implementation checklist (`--compare`, `--format`, `--pages`, auto-symlink to `.roko/screenshots/latest/`) are not wired, and sub-view rendering (rendering each `SubView` within a tab, not just the top-level tab) is missing.

Without these gaps closed, a caller receives only top-level tab text files with no canonical "latest" pointer, making scripting fragile. The comparison path (`--compare`) is needed for automated visual regression testing against Mori reference screenshots.

## Current State

- `crates/roko-cli/src/tui/snapshot.rs` — text rendering engine exists and produces `.txt` files per tab.
- `crates/roko-cli/src/commands/screenshot.rs` — `SnapshotConfig` struct and `capture_snapshots()` function exist (untracked new file as of 2026-08-19 git status).
- The `--snapshot` flag on `roko dashboard` is NOT wired (only `roko screenshot` as a top-level command is hooked in).
- `--compare` flag: not implemented.
- `--format` flag: not implemented (text-only; no `png` or `all` variant).
- `--pages` flag: not implemented (no per-page-slug capture for SubViews).
- Auto-symlink `.roko/screenshots/latest/` → most recent capture directory: not implemented.
- PNG rendering is explicitly out of scope for this item (requires image crate / font atlas; tracked separately).

## Implementation Plan

1. **Verify main.rs registration**: Confirm `roko screenshot` is wired as a top-level subcommand in `crates/roko-cli/src/main.rs`. If not, add it alongside existing top-level commands.

2. **Sub-view iteration in `snapshot.rs`**: After rendering each `Tab`, iterate its `SubView` variants and render each to `<dir>/<tab>/<subview>.txt`. Update the `manifest.json` schema to include a `sub_views` array per tab entry.

3. **`--pages` flag**: Accept a comma-separated list of tab slugs (e.g., `--pages dashboard,plans,agents`). When provided, only render the specified tabs. Default: all tabs.

4. **`--format` flag**: Accept `text` (default), `ansi`, or `all`. For `ansi`, write a `.ansi` file alongside `.txt` that includes ANSI escape codes for colour. PNG remains future work — for now, `--format png` should error with a clear message.

5. **Auto-symlink**: After writing to the timestamped directory (e.g., `.roko/screenshots/2026-08-19T14-23-00/`), atomically update the `.roko/screenshots/latest/` symlink to point to it using `std::os::unix::fs::symlink` after removing the old symlink.

6. **`--compare` flag (text diff)**: Accept a reference directory path. After capturing, run a line-by-line diff against the reference for each overlapping file. Emit a structured `diff-report.json` in the output directory with per-file `added`/`removed`/`changed` line counts and the full unified diff for each file that has changes.

7. **Wire `--snapshot <dir>` onto `roko dashboard`**: Add the flag to the `Dashboard` subcommand variant and invoke `capture_snapshots` with the given directory before entering the live TUI loop.

8. **Update manifest.json schema**: Include `captured_at`, `roko_version`, `tab_count`, `subview_count`, per-tab `{slug, path, subviews: [{slug, path}]}`.

## Acceptance Criteria

1. `roko screenshot` produces a timestamped directory with one `.txt` file per tab (ten tabs minimum).
2. `.roko/screenshots/latest/` symlink is created and points to the most recent capture.
3. `roko screenshot --pages dashboard,plans` produces only two tab files.
4. `roko screenshot --compare <ref-dir>` produces a `diff-report.json` with per-file diffs.
5. `roko screenshot --format ansi` produces `.ansi` files alongside `.txt`.
6. `roko dashboard --snapshot <dir>` exits immediately after writing the snapshot without entering the live TUI.
7. `manifest.json` in the output directory lists all captured files with tab/subview metadata.
8. The command exits 0 on success and non-zero if any tab fails to render.

## Verification Checklist

- [ ] Run `roko screenshot` in CI with `TERM=dumb` and verify ten `.txt` files are created.
- [ ] Verify `.roko/screenshots/latest/` symlink is updated on each invocation.
- [ ] Run `roko screenshot --pages dashboard,agents` and verify exactly two tab files are written.
- [ ] Run `roko screenshot --compare <prev-dir>` and verify `diff-report.json` is non-empty when TUI changed.
- [ ] Run `roko dashboard --snapshot /tmp/snap` and verify it exits without entering live mode.
- [ ] Verify `manifest.json` is valid JSON with correct field names and file paths.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/screenshot.rs` | Add `--compare`, `--format`, `--pages` flags; implement auto-symlink; implement text diff |
| `crates/roko-cli/src/tui/snapshot.rs` | Add sub-view iteration; add ANSI output mode |
| `crates/roko-cli/src/commands/dashboard.rs` | Wire `--snapshot <dir>` flag |
| `crates/roko-cli/src/main.rs` | Verify `roko screenshot` is registered as top-level subcommand |
