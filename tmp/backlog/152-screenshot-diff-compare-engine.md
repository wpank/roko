# 152 — Screenshot Diff/Compare Engine

**Priority**: P2 — enables automated visual regression testing of TUI changes against baselines and Mori reference screenshots
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/tui/snapshot.rs`, `crates/roko-cli/src/commands/screenshot.rs`
**Depends on**: #111 (screenshot command — `--compare` flag skeleton), #151 (PNG rendering — needed for PNG diff mode)
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` §0.7

---

## Background

The `roko screenshot` command captures headless TUI snapshots. Backlog #111 mentions a `--compare <dir>` flag for comparing against a previous snapshot, but only defines the flag — not the diff engine itself.

To enable automated visual regression testing, roko needs a diff engine that can:
1. Compare text snapshots line-by-line and report structured diffs
2. Compare PNG snapshots pixel-by-pixel and generate visual diff images (highlighting changed regions)
3. Support a `--reference-mode` for comparing against Mori reference screenshots (which have PNGs but no matching text files)

This is how the self-development loop verifies visual changes: capture baseline → make changes → capture after → compare → verify improvement.

## Current State

- `crates/roko-cli/src/commands/screenshot.rs` — `--compare` flag exists in #111's spec but is not implemented yet. The flag accepts a directory path.
- `crates/roko-cli/src/tui/snapshot.rs` — text rendering engine exists. Each snapshot directory contains `manifest.json` listing all captured files.
- Mori reference screenshots at `tmp/mori-old/screenshots/` (17 PNGs from real terminal captures). These have no matching text files.
- No diff library is currently used in roko-cli.

## Implementation Plan

1. **Text diff engine**: Implement a line-by-line diff for `.txt` files.
   - Load `manifest.json` from both the current and reference snapshot directories
   - For each matching tab/sub-view file pair, compute a unified diff (use the `similar` crate or implement a simple LCS-based diff)
   - Generate a structured diff report:
     ```json
     {
       "compared": { "current": "/path/to/current", "reference": "/path/to/reference" },
       "tabs": [
         {
           "tab": "f01-dashboard",
           "changed": true,
           "lines_added": 3,
           "lines_removed": 2,
           "lines_modified": 5,
           "diff_preview": "- Wave 1/7  Queue: Sprint-1\n+ Wave 2/7  Queue: Sprint-1"
         },
         { "tab": "f02-plans", "changed": false }
       ],
       "summary": { "tabs_changed": 4, "tabs_unchanged": 6, "total_lines_changed": 23 }
     }
     ```
   - Write the report to `<output-dir>/diff-report.json`

2. **PNG diff engine**: For each matching `.png` file pair:
   - Load both images using the `image` crate
   - Resize reference image to match current dimensions if they differ (using nearest-neighbor to avoid blurring text)
   - Compute per-pixel absolute difference across RGB channels
   - Generate a diff image: unchanged pixels rendered at 30% opacity, changed pixels highlighted in red with full opacity
   - Write to `<output-dir>/diff-f01-dashboard.png`
   - Calculate a similarity percentage: `(1.0 - changed_pixels / total_pixels) * 100`
   - Include similarity scores in `diff-report.json`

3. **`--reference-mode` flag**: When set, the comparison treats the reference directory as containing raw image files (no manifest.json). Match files by name pattern (e.g., reference `01-dashboard.png` matches current `f01-dashboard.png` using fuzzy tab name matching). Skip text diffs entirely — only PNG diffs are computed.

4. **Unified diff output**: For text diffs, also write a human-readable `diff-summary.txt` that shows the unified diff for all changed tabs, suitable for reading in a terminal or by an agent.

5. **Threshold-based assessment**: Add a `--threshold <percent>` flag (default: 95). If overall similarity is below the threshold, exit with non-zero status. This enables CI/scripting:
   ```bash
   roko screenshot --compare .roko/screenshots/baseline --threshold 90 || echo "TUI regression detected"
   ```

6. **Wire into capture pipeline**: When `--compare <dir>` is provided alongside a capture:
   - First capture the new snapshot
   - Then run the diff engine against the reference
   - Output the diff report alongside the captured files

## Acceptance Criteria

1. `roko screenshot --compare <ref-dir>` produces a `diff-report.json` with per-tab change summaries
2. Text diffs show added/removed/modified line counts and a preview of changes
3. PNG diffs generate visual diff images highlighting changed regions in red
4. `--reference-mode` works with Mori screenshots at `tmp/mori-old/screenshots/`
5. `--threshold 95` exits non-zero when similarity drops below 95%
6. `diff-summary.txt` is human-readable and shows unified diffs
7. The diff engine handles missing tabs gracefully (tabs only in one set are reported as "added" or "removed")

## Verification Checklist

- [ ] `roko screenshot --compare <same-dir>` reports 0 changes (self-comparison)
- [ ] Modify a TUI widget, capture, compare to previous — diff-report.json shows the changed tabs
- [ ] PNG diff images show changed regions in red
- [ ] `--reference-mode` against `tmp/mori-old/screenshots/` produces diff images
- [ ] `--threshold 50` passes, `--threshold 100` fails (on any non-identical comparison)
- [ ] Missing tabs in reference are reported as "new" tabs in the report

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/Cargo.toml` | Add `similar` crate for text diffing |
| `crates/roko-cli/src/tui/snapshot.rs` | Add `diff_snapshots()`, `diff_images()`, `DiffReport` struct |
| `crates/roko-cli/src/commands/screenshot.rs` | Wire `--compare`, `--reference-mode`, `--threshold` flags |
