# 151 — TUI PNG Snapshot Rendering

**Priority**: P2 — enables automated visual assessment of TUI quality; text snapshots exist but pixel-level visual inspection requires PNG output
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/tui/snapshot.rs`, `crates/roko-cli/Cargo.toml`
**Depends on**: #111 (screenshot command completion — text snapshot infrastructure must be working first)
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` §0.1 (PNG rendering approach)

---

## Background

Roko is a Rust toolkit for building agents that build themselves. It includes an interactive TUI dashboard (`roko dashboard`) built with ratatui, accessible via ten tabs (F1-F10) covering system status, plans, agents, knowledge, learning, and more. The `roko screenshot` command (backlog #111) renders these tabs headlessly via ratatui's `TestBackend` and writes `.txt` files so that AI agents can inspect dashboard output without a real terminal.

However, text output cannot capture visual properties like color palette compliance, layout spacing, widget alignment, or style rendering. The predecessor system (Mori) had a polished TUI with the ROSEDUST color palette (warm rose-tinted greys), particle VFX, gradient progress bars, and information-dense layouts. The visual target screenshots are at `tmp/mori-old/screenshots/` (17 PNGs captured from a real terminal emulator). To assess whether roko's TUI matches that visual quality, Claude needs to compare PNG screenshots side-by-side using its vision capability.

The text snapshot infrastructure in `crates/roko-cli/src/tui/snapshot.rs` already handles tab iteration, tab filtering, `TestBackend` rendering, and `manifest.json` generation. The `capture_snapshots()` function iterates all tabs, renders each into a ratatui `Buffer` via `render_tab_to_text()`, and writes the result. This spec adds a parallel PNG rendering pipeline that converts the same ratatui `Buffer` into pixel images.

ratatui's `Buffer` contains per-cell data: `symbol` (the character), `fg` (Color), `bg` (Color), and `modifier` (bold/italic/underline/etc). All the information needed for pixel rendering is already present in the Buffer — this spec converts that structured data into actual pixels.

## Current State

- `crates/roko-cli/src/tui/snapshot.rs` — text rendering engine exists. Uses `TestBackend::new(width, height)` to render each tab into a `Buffer`, then extracts text via `buffer_to_text()`. The `SnapshotConfig` struct holds width/height/output_dir/tabs/label. The `capture_snapshots()` function writes `.txt` files and a `manifest.json` to the output directory.
- The `--format` flag on `roko screenshot` is planned (#111) to accept `text`, `ansi`, or `all`. Currently text-only. PNG is explicitly out of scope in #111 ("PNG remains future work").
- ratatui's `Buffer` (accessed via `terminal.backend().buffer()`) stores `content` as a flat `Vec<Cell>` where each `Cell` has `.symbol()`, `.fg`, `.bg`, and `.modifier` fields. The `buffer_to_text()` function in `snapshot.rs` already iterates this structure but only extracts the symbol text.
- No `image` or `fontdue` crate dependency exists in `crates/roko-cli/Cargo.toml`.
- The TUI module lives at `crates/roko-cli/src/tui/` and already contains `theme.rs` (color/style definitions), `snapshot.rs` (text capture), and the full rendering pipeline in `views/`.
- Mori reference screenshots at `tmp/mori-old/screenshots/` are 17 terminal-capture PNGs taken from a real terminal emulator (not programmatically rendered). Text descriptions of each are in `tmp/mori-old/MORI-TUI-SCREENSHOTS.md`.

## Implementation Plan

1. **Add `image` and `fontdue` crate dependencies** to `crates/roko-cli/Cargo.toml` behind a `snapshot-png` feature flag (default-enabled). `image` (crates.io) provides PNG encoding via `image::RgbaImage::save()`. `fontdue` (crates.io) provides CPU-only font rasterization without system font dependencies or C bindings — it parses TrueType/OpenType fonts and renders individual glyphs to bitmaps.

2. **Embed a monospace font**: Create a new module `crates/roko-cli/src/tui/font.rs`. Bundle JetBrains Mono Regular (SIL Open Font License, freely redistributable) as a static byte array using `include_bytes!()`. Use `fontdue::Font::from_bytes()` to parse it at runtime into a `fontdue::Font`. Only the Regular weight is needed; bold can be synthesized by rendering the glyph twice with a 1px horizontal offset and compositing. Store the font binary in `crates/roko-cli/assets/JetBrainsMono-Regular.ttf` (approximately 200KB).

3. **Define cell dimensions**: Use 8px wide x 16px tall per terminal cell as the default (matches typical terminal cell proportions for monospace fonts). A 240x60 terminal grid produces a 1920x960px PNG. Expose an optional `--snapshot-cell-size WxH` CLI flag for customization. The fontdue rasterizer should be configured to render glyphs at a size that fits within the cell dimensions (approximately 14px font size for an 8x16 cell).

4. **Implement color mapping**: Add a `ratatui_color_to_rgba(color: ratatui::style::Color) -> [u8; 4]` function in `font.rs` or `snapshot.rs`. Handle:
   - `Color::Rgb(r, g, b)` — direct mapping to `[r, g, b, 255]`
   - `Color::Indexed(n)` — map ANSI 256-color palette indices to RGB values (standard terminal palette lookup table)
   - Named colors (`Color::Red`, `Color::Green`, `Color::Blue`, etc.) — map to their standard ANSI bright/dark RGB equivalents
   - `Color::Reset` / `Color::default()` — use terminal defaults (white foreground `[204, 204, 204, 255]`, black background `[0, 0, 0, 255]`)

5. **Implement `render_buffer_to_png(buffer: &ratatui::buffer::Buffer, cell_w: u32, cell_h: u32) -> image::RgbaImage`**: This is the core rendering function.
   - Create an `RgbaImage` of size `(buffer.area.width as u32 * cell_w, buffer.area.height as u32 * cell_h)`
   - Load the embedded font via fontdue (cache the parsed `Font` in a `OnceLock<Font>` static)
   - For each cell in the Buffer at grid position `(col, row)`:
     a. Compute pixel rectangle: `x = col * cell_w`, `y = row * cell_h`
     b. Fill the cell rectangle with the cell's background color (mapped via `ratatui_color_to_rgba(cell.bg)`)
     c. Rasterize the cell's `symbol` character using `fontdue::Font::rasterize(char, size)`, which returns a `(Metrics, Vec<u8>)` where the `Vec<u8>` is a coverage bitmap
     d. Composite the glyph bitmap onto the cell rectangle using the foreground color: for each pixel in the glyph bitmap, blend `fg_color` with alpha = coverage value
     e. Handle modifier flags from `cell.modifier`:
        - `Modifier::BOLD` — render the glyph a second time offset 1px to the right and composite (synthetic bold)
        - `Modifier::UNDERLINE` — draw a 1px horizontal line at `y + cell_h - 2` across the cell width using the foreground color
        - `Modifier::ITALIC` — apply a simple horizontal shear transform to glyph pixel positions (shift x by `-(y - baseline) / 4`)
        - `Modifier::DIM` — reduce foreground alpha by 50%
   - Return the completed `RgbaImage`

6. **Wire into `capture_snapshots()`**: Modify the tab iteration loop in `crates/roko-cli/src/tui/snapshot.rs`. After the existing `.txt` file write, check whether the output format includes `png`. If so:
   - Get the raw `Buffer` from the terminal (the `render_tab_to_text()` function currently consumes it — refactor to return both text and the buffer, or re-render)
   - Call `render_buffer_to_png()` with the buffer and configured cell dimensions
   - Save via `image::RgbaImage::save()` to `<dir>/f01-dashboard.png`, etc.
   - The refactored function should be renamed to something like `render_tab()` that returns a `RenderedTab { text: String, buffer: Buffer }`, and the existing `render_tab_to_text()` should call it and return only the text.

7. **Extend `--format` flag**: Add `png` as a valid variant alongside `text` and `ansi`. `--format all` produces `.txt` + `.ansi` + `.png`. `--format png` produces only `.png`. When PNG is requested but the `snapshot-png` feature is disabled, print a clear error: "PNG snapshot support requires the `snapshot-png` feature. Rebuild with `cargo build --features snapshot-png`."

8. **Update manifest.json**: Extend the `TabEntry` struct (already in `snapshot.rs`) to include an optional `png` field. When PNG files are generated, populate it with the filename (e.g., `"png": "f01-dashboard.png"`). The `Manifest` struct already serializes to JSON via serde.

9. **Handle emoji and wide characters**: fontdue may not have glyphs for all Unicode code points. For missing glyphs (where `Font::rasterize()` returns an empty/zero-size bitmap), render the Unicode replacement character (U+FFFD) or leave the cell blank. For wide characters (those where `unicode_width::UnicodeWidthChar::width()` returns 2), render into a double-width cell spanning two grid positions — skip the next cell in the iteration.

## Acceptance Criteria

1. `roko screenshot --format png` produces one `.png` file per tab (10 minimum, matching the 10 TUI tabs: Dashboard, Plans, Agents, Knowledge, Signals, DeFi, Inspect, Queue, Config, Learning).
2. `roko screenshot --format all` produces `.txt` + `.png` + `.ansi` for each tab.
3. PNG files are valid images that can be opened by standard image viewers and read by Claude's vision capability via the `Read` tool.
4. Colors in the PNG match the ratatui Buffer's fg/bg colors — visually verified by comparing a tab that uses known RGB colors (e.g., the ROSEDUST palette defined in `crates/roko-cli/src/tui/theme.rs`).
5. Text in the PNG is legible at the default 8x16 cell size — individual characters are distinguishable and words are readable.
6. `manifest.json` includes PNG file paths (in the `png` field of each tab entry) when PNG output is enabled.
7. The `snapshot-png` feature flag can be disabled to skip the `image`/`fontdue` dependencies, producing a text-only build. `cargo build -p roko-cli --no-default-features` compiles without those crates.
8. PNG rendering completes in < 5 seconds for all 10 tabs at 240x60 terminal resolution (1920x960px per image).

## Verification Checklist

- [ ] `cargo build -p roko-cli --features snapshot-png` compiles without errors
- [ ] `roko screenshot --format png` produces `.png` files in the output directory (one per tab, 10 files minimum)
- [ ] Open a generated `.png` in an image viewer — text is readable, colors are correct, layout matches the `.txt` output structure
- [ ] `roko screenshot --format all` produces `.txt` + `.ansi` + `.png` for each tab
- [ ] `roko screenshot --format text` does NOT produce `.png` files
- [ ] `manifest.json` includes `"png": "f01-dashboard.png"` entries when format includes png
- [ ] `cargo build -p roko-cli --no-default-features` compiles without image/fontdue (no linker errors from missing crate)
- [ ] Compare PNG output to `tmp/mori-old/screenshots/` visually — layout structure (header bar, content area, tab indicators) should be recognizable even if styling differs
- [ ] Verify bold text appears thicker than regular text in the PNG
- [ ] Verify underlined text has a visible underline in the PNG
- [ ] Verify colored text (e.g., green for pass, red for fail) renders with correct hues

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/Cargo.toml` | Add `image` and `fontdue` dependencies behind `snapshot-png` feature flag |
| `crates/roko-cli/src/tui/snapshot.rs` | Add `render_buffer_to_png()` function; refactor `render_tab_to_text()` to expose the raw `Buffer`; wire PNG output into `capture_snapshots()` loop; extend `TabEntry` with optional `png` field |
| `crates/roko-cli/src/tui/font.rs` | New file: `include_bytes!()` embedded font, `fontdue::Font` initialization via `OnceLock`, `ratatui_color_to_rgba()` color mapping, glyph rasterization helpers |
| `crates/roko-cli/src/tui/mod.rs` | Add `pub mod font;` declaration (behind `#[cfg(feature = "snapshot-png")]`) |
| `crates/roko-cli/src/commands/screenshot.rs` | Extend `--format` CLI flag to include `png` variant; add `--snapshot-cell-size` flag |
| `crates/roko-cli/assets/JetBrainsMono-Regular.ttf` | New file: embedded monospace font binary (~200KB, SIL Open Font License) |
