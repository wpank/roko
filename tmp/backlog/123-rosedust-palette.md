# 123 — ROSEDUST Color Palette Port

**Priority**: P2 — The default ratatui color scheme makes roko visually distinct from Mori's production reference, complicating visual assessment and giving a less polished feel to long-running operator sessions.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/tui/`
**Depends on**: None (self-contained to the theme/color module)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.3, `tmp/backlog/_mori-old-gaps.md` MO-08

---

## Background

Mori's TUI used a custom color scheme called ROSEDUST: warm rose-tinted greys with semantic color roles. The palette was designed for long operator sessions — avoiding pure white (which causes eye strain), using a true black background (works on both dark terminals and OLED displays), and using HSV-derived gradient tables for progress bars.

Roko's TUI currently uses ratatui's default palette, which includes pure white, bright primary colors, and high-contrast combinations that are visually noisy during long runs.

The ROSEDUST palette from the Mori reference:
- **Background**: True black `#000000`
- **SAGE**: Green `#7FAF7F` — success, pass, active
- **WARNING**: Amber `#AF8F00` — warnings, slow progress
- **EMBER**: Red-orange `#CF5A00` — errors, failures
- **BONE**: Off-white `#D4C8B0` — primary text
- **DREAM**: Indigo `#7070AF` — special states, learning indicators
- **MUTED**: Dark grey `#404040` — secondary text, borders
- HSV gradient tables for progress bars (green-to-amber-to-red progression)

Porting this is straightforward: create a `theme.rs` module that defines the color constants and gradient functions, then replace inline `Color::*` literals throughout the rendering code with theme references.

## Current State

- `crates/roko-cli/src/tui/` — all tab rendering files use inline `Color::*` literals from ratatui.
- No `theme.rs` or `palette.rs` module exists.
- Mori's exact color values are documented in the implementation checklist.
- The change is purely cosmetic; no logic changes are required.

## Implementation Plan

1. **Create `crates/roko-cli/src/tui/theme.rs`**:
   ```rust
   use ratatui::style::Color;

   pub const BG: Color = Color::Black;
   pub const SAGE: Color = Color::Rgb(127, 175, 127);
   pub const WARNING: Color = Color::Rgb(175, 143, 0);
   pub const EMBER: Color = Color::Rgb(207, 90, 0);
   pub const BONE: Color = Color::Rgb(212, 200, 176);
   pub const DREAM: Color = Color::Rgb(112, 112, 175);
   pub const MUTED: Color = Color::Rgb(64, 64, 64);
   pub const BORDER: Color = Color::Rgb(80, 80, 80);

   /// HSV gradient from SAGE to EMBER to WARNING, t in [0.0, 1.0]
   pub fn progress_color(t: f32) -> Color { ... }
   ```

2. **Replace inline colors across tab rendering files**: For each tab rendering file in `crates/roko-cli/src/tui/`, replace:
   - `Color::Green` → `theme::SAGE`
   - `Color::Yellow` → `theme::WARNING`
   - `Color::Red` → `theme::EMBER`
   - `Color::White` → `theme::BONE`
   - `Color::White` in borders → `theme::BORDER`
   - `Color::Blue` / `Color::Cyan` → `theme::DREAM`
   - `Color::DarkGray` / `Color::Gray` → `theme::MUTED`

3. **Progress bar gradient**: Replace the static green progress bar color with `theme::progress_color(completed_fraction)` so bars transition from SAGE (fresh) to WARNING (near end) to EMBER (stalled/overdue).

4. **Persist theme choice**: Add `[tui] theme = "rosedust"` to `roko.toml` schema, defaulting to `"rosedust"`. For now, no other theme is implemented; the field is a stub for future extensibility.

5. **Verify ANSI terminal compatibility**: Test on macOS Terminal.app and iTerm2 (the most common terminals for roko operators). Verify RGB colors fall back to 256-color approximations on terminals that don't support 24-bit color.

## Acceptance Criteria

1. A new `theme.rs` module exports all ROSEDUST color constants.
2. All tab rendering files use theme constants instead of inline `Color::*` literals.
3. Progress bars use HSV gradient from green through amber to red based on completion fraction.
4. `roko dashboard` on a standard macOS terminal displays the ROSEDUST palette visually.
5. `cargo clippy -p roko-cli -- -D warnings` passes.
6. TUI renders correctly on terminals without RGB color support (graceful fallback).

## Verification Checklist

- [ ] `grep -rn 'Color::Green\|Color::Red\|Color::White' crates/roko-cli/src/tui/` returns no results after the port (only legitimate exceptions like explicit terminal color detection code).
- [ ] `roko screenshot` produces text output with ANSI colour codes that match the ROSEDUST palette when inspected.
- [ ] Launch `roko dashboard` on macOS Terminal.app; verify no bright pure-white text.
- [ ] Progress bar at 100%: SAGE. Progress bar at 50%: transitioning. Progress bar at 0% with elapsed time: EMBER.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/theme.rs` | New file: ROSEDUST constants, progress gradient |
| `crates/roko-cli/src/tui/mod.rs` | Export `theme` module |
| All `crates/roko-cli/src/tui/*.rs` rendering files | Replace inline `Color::*` with `theme::*` |
