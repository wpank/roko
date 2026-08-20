# 71 — TUI Theme Alignment with ROSEDUST Design System

**Priority**: P3 — visual polish: the TUI uses a different set of RGB values than the reference design system, and 82 inline color literals are scattered across widget files instead of using the canonical theme constants
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli/src/tui/` (roko-cli crate only)
**Depends on**: None

---

## Background

Roko's TUI (`roko dashboard`) uses the ROSEDUST design system, a warm rose/indigo aesthetic defined in `tmp/archive/08-17-26/design-systems/01-ROSEDUST-DESIGN-SYSTEM.md`. The demo web application (`demo/demo-app/`) implements the same system via CSS custom properties in `demo/demo-app/src/styles/tokens.css`. The TUI theme is defined in `crates/roko-cli/src/tui/theme.rs`.

The three documents — design spec, CSS tokens, and TUI theme — have drifted apart. Every named color in `theme.rs` differs from the canonical spec values. More significantly, 82 inline `Color::Rgb(...)` literals are scattered across 11 widget and view files outside `theme.rs`. Many of these inline values are ad-hoc approximations that match neither the theme constants nor any official ROSEDUST token. This creates two problems: visual inconsistency across tabs, and fragility when updating colors (each file must be found and updated independently).

There is also a structural inconsistency: `agents_view.rs` defines its own `role_accent()` function (line 1120) with hardcoded inline RGB values for each role, while the global `Theme::role_accent()` method also exists but is not used by this file.

---

## Current State

All constants are verified from reading `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs`.

**Current TUI constants (in `theme.rs`):**

| Constant | Current TUI Value |
|---|---|
| `VOID` | `Rgb(0, 0, 0)` — pure black (diverges from spec's tinted `#060608`) |
| `ROSE` | `Rgb(185, 120, 148)` |
| `ROSE_BRIGHT` | `Rgb(220, 155, 180)` |
| `ROSE_DIM` | `Rgb(140, 96, 112)` |
| `BONE` | `Rgb(215, 198, 158)` |
| `BONE_DIM` | `Rgb(160, 142, 108)` |
| `TEXT` / `FG` | `Rgb(165, 142, 158)` |
| `TEXT_DIM` / `FG_DIM` | `Rgb(145, 120, 138)` |
| `TEXT_GHOST` | `Rgb(110, 85, 105)` |
| `TEXT_PHANTOM` | `Rgb(55, 42, 55)` |
| `DREAM` | `Rgb(120, 115, 165)` |
| `SAGE` | `Rgb(125, 158, 140)` |
| `EMBER` | `Rgb(195, 110, 85)` |
| `WARNING` | `Rgb(195, 155, 95)` |
| `BG` | `Rgb(0, 0, 0)` — same as VOID, pure black |
| `BG_SECONDARY` | `Rgb(14, 12, 16)` |
| `BG_HIGHLIGHT` | `Rgb(34, 28, 36)` |

**Target values (from live CSS `tokens.css`, which is the canonical reference per the recommendation below):**

| CSS Token | Target RGB |
|---|---|
| `--rose` | `Rgb(184, 122, 148)` |
| `--rose-bright` | `Rgb(216, 154, 178)` |
| `--rose-dim` | `Rgb(138, 90, 112)` |
| `--bone` | `Rgb(212, 200, 156)` |
| `--bone-dim` | `Rgb(154, 138, 104)` |
| `--text-primary` | `Rgb(232, 220, 232)` |
| `--text-dim` | `Rgb(154, 138, 152)` |
| `--text-ghost` | `Rgb(96, 80, 96)` |
| `--dream` | `Rgb(136, 136, 168)` |
| `--success` | `Rgb(138, 156, 134)` |
| `--warning` | `Rgb(216, 168, 120)` |
| `--danger` | `Rgb(204, 85, 85)` |
| `--bg-void` | `Rgb(8, 8, 12)` |
| `--bg-raised` | `Rgb(18, 16, 26)` |

**Missing constants** (referenced in the design system but not yet defined in `theme.rs`):

| Token | Purpose |
|---|---|
| `ROSE_GLOW` | Signature emphasis color `Rgb(232, 181, 206)` |
| `ROSE_DEEP` | Background tints, hover states `Rgb(58, 32, 48)` |
| `TEXT_STRONG` | Maximum text brightness `Rgb(248, 240, 248)` |
| `TEXT_SOFT` | Secondary text `Rgb(200, 184, 196)` |
| `DREAM_BRIGHT` | Knowledge highlight `Rgb(164, 164, 200)` |
| `DREAM_DEEP` | Deep accent `Rgb(40, 40, 72)` |
| `BONE_BRIGHT` | Metrics, important numbers `Rgb(228, 216, 176)` |

**Inline `Color::Rgb` literals outside `theme.rs`** (verified by counting):

| File | Count | Nature |
|---|---|---|
| `widgets/dream_view.rs` | 22 | Phase colors, label styles |
| `postfx.rs` | 21 | Shadow/overlay/effect colors |
| `views/agents_view.rs` | 12 | Role accent colors (in `role_accent()` fn at line 1120), separators |
| `views/plans_view.rs` | 11 | Border/separator colors |
| `dashboard.rs` | 5 | Test assertions (update to new values) |
| `widgets/task_progress.rs` | 2 | Progress colors |
| `widgets/sys_metrics.rs` | 2 | Metric colors |
| `widgets/phase_compact.rs` | 2 | Phase indicator colors |
| `postfx_pipeline.rs` | 2 | Pipeline effect colors |
| `atmosphere.rs` | 2 | Atmospheric brightness values |
| `widgets/header_bar.rs` | 1 | Header accent |

Total: **82 inline literals** in 11 files.

**Exception**: `ansi.rs` has 4 inline `Color::Rgb` literals that are ANSI parsing test assertions. These should NOT be changed.

**Duplicate `role_accent()` function**: `views/agents_view.rs` line 1120 defines its own `role_accent(role: &str, theme: &Theme) -> Color` with inline RGB values for "implementer", "strategist", "architect", "auditor", "critic", "conductor", "researcher". A `Theme::role_accent()` method exists at the Theme level. The file uses its own local function instead of the theme method.

---

## Implementation Plan

### Step 1: Decide canonical source

The recommendation is to align TUI to the **live CSS tokens** (`demo/demo-app/src/styles/tokens.css`). The CSS was deliberately brightened from the original spec for readability and has been battle-tested in the demo app. The design spec document is the original intent but has lower text contrast than what users see in practice.

This means: for `--text-primary`, the CSS value `Rgb(232, 220, 232)` is used rather than the spec's `Rgb(200, 184, 192)`. Other tokens follow the same CSS-first rule.

### Step 2: Update `theme.rs` color constants

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs`, update the 17 existing constants and add 7 new ones:

```rust
// -- Primaries --
pub(crate) const VOID: Color = Color::Rgb(8, 8, 12);         // was Rgb(0,0,0); CSS --bg-void
pub(crate) const ROSE: Color = Color::Rgb(184, 122, 148);    // was Rgb(185,120,148)
pub(crate) const ROSE_BRIGHT: Color = Color::Rgb(216, 154, 178); // was Rgb(220,155,180)
pub(crate) const ROSE_DIM: Color = Color::Rgb(138, 90, 112); // was Rgb(140,96,112)
// NEW:
pub(crate) const ROSE_GLOW: Color = Color::Rgb(232, 181, 206);
pub(crate) const ROSE_DEEP: Color = Color::Rgb(58, 32, 48);

pub(crate) const BONE: Color = Color::Rgb(212, 200, 156);    // was Rgb(215,198,158)
pub(crate) const BONE_DIM: Color = Color::Rgb(154, 138, 104); // was Rgb(160,142,108)
// NEW:
pub(crate) const BONE_BRIGHT: Color = Color::Rgb(228, 216, 176);

// -- Text --
pub(crate) const TEXT: Color = Color::Rgb(232, 220, 232);    // was Rgb(165,142,158); CSS --text-primary
pub(crate) const TEXT_DIM: Color = Color::Rgb(154, 138, 152); // was Rgb(145,120,138)
pub(crate) const TEXT_GHOST: Color = Color::Rgb(96, 80, 96); // was Rgb(110,85,105)
pub(crate) const TEXT_PHANTOM: Color = Color::Rgb(55, 42, 55); // unchanged
// NEW:
pub(crate) const TEXT_STRONG: Color = Color::Rgb(248, 240, 248);
pub(crate) const TEXT_SOFT: Color = Color::Rgb(200, 184, 196);

// -- Accents --
pub const DREAM: Color = Color::Rgb(136, 136, 168);          // was Rgb(120,115,165)
pub(crate) const SAGE: Color = Color::Rgb(138, 156, 134);   // was Rgb(125,158,140)
pub(crate) const EMBER: Color = Color::Rgb(204, 85, 85);    // was Rgb(195,110,85); CSS --danger
pub(crate) const WARNING: Color = Color::Rgb(216, 168, 120); // was Rgb(195,155,95)
// NEW:
pub(crate) const DREAM_BRIGHT: Color = Color::Rgb(164, 164, 200);
pub(crate) const DREAM_DEEP: Color = Color::Rgb(40, 40, 72);

// -- Backgrounds --
pub(crate) const BG: Color = Color::Rgb(8, 8, 12);           // was Rgb(0,0,0); tinted dark
pub(crate) const BG_SECONDARY: Color = Color::Rgb(18, 16, 26); // was Rgb(14,12,16); CSS --bg-raised
pub(crate) const BG_HIGHLIGHT: Color = Color::Rgb(34, 28, 36); // unchanged
```

### Step 3: Eliminate inline `Color::Rgb` literals in widget files

For each of the 11 affected files, replace every `Color::Rgb(r, g, b)` literal with the nearest `Theme::` constant. Use these mappings:

| Common inline value | Replace with |
|---|---|
| `Color::Rgb(185, 120, 148)` or nearby rose | `Theme::ROSE` |
| `Color::Rgb(220, 155, 180)` or nearby rose-bright | `Theme::ROSE_BRIGHT` |
| `Color::Rgb(140, 96, 112)` or nearby rose-dim | `Theme::ROSE_DIM` |
| `Color::Rgb(120, 115, 165)` or nearby indigo | `Theme::DREAM` |
| `Color::Rgb(125, 158, 140)` or nearby sage | `Theme::SAGE` |
| `Color::Rgb(195, 110, 85)` or nearby ember | `Theme::EMBER` |
| `Color::Rgb(195, 155, 95)` or nearby amber | `Theme::WARNING` |
| `Color::Rgb(155, 130, 175)` (lavender — conductor) | `Theme::DREAM_BRIGHT` |
| `Color::Rgb(100, 150, 170)` (teal — researcher) | `Theme::SAGE` (closest) |
| Dark background variants near `Rgb(14, 12, 16)` | `Theme::BG_SECONDARY` |
| Dark background variants near `Rgb(34, 28, 36)` | `Theme::BG_HIGHLIGHT` |
| Dim text near `Rgb(110, 85, 105)` | `Theme::TEXT_GHOST` |

For any inline color that does not closely match an existing constant, either: (a) create a new named constant in `theme.rs` if the color has semantic meaning, or (b) use the closest existing constant if the difference is minor (< 15 on any channel).

In `postfx.rs` (21 occurrences), shadow and overlay colors are often semi-transparent blends. For these, create named constants with doc comments explaining the effect (e.g., `SHADOW_OVERLAY: Color = Color::Rgb(4, 4, 8)`).

### Step 4: Consolidate the duplicate `role_accent()` function

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/agents_view.rs` (line 1120), delete the local `role_accent()` function. Update all three call sites in the file (lines 246, 548, 598) to use `Theme::role_accent(role)` or equivalently define the role mapping inside the `Theme` impl.

Add to `theme.rs`:
```rust
/// Role-specific accent color for agent role labels.
pub fn role_accent(role: &str) -> Color {
    match role.to_lowercase().as_str() {
        "implementer" | "impl" => Self::ROSE,
        "strategist" | "strat" => Self::DREAM,
        "architect" | "arch" => Self::SAGE,
        "auditor" | "audit" => Self::WARNING,
        "critic" | "crit" => Self::EMBER,
        "conductor" | "cond" => Self::DREAM_BRIGHT,
        "researcher" | "res" => Self::SAGE,  // closest available
        _ => Self::ROSE,
    }
}
```

Note: the previous inline version used distinct `Rgb(155,130,175)` for "conductor" and `Rgb(100,150,170)` for "researcher". Map "conductor" to `DREAM_BRIGHT` (new constant) and "researcher" to `SAGE` as the closest built-in options. This consolidates the color vocabulary.

### Step 5: Update test assertions in `dashboard.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs`, the test `theme_defaults_to_rosedust_palette` at line 6175 asserts specific RGB values. Update these assertions to reflect the new canonical values from Step 2.

### Step 6: Update `BG` alias

`Theme::BG` and `Theme::VOID` both resolved to `Rgb(0, 0, 0)` before. After this change both become `Rgb(8, 8, 12)`. Verify that no code path relied on `BG == Color::Black` for special terminal behavior (e.g., transparency). The change from pure black to a dark tinted value is intentional and matches the ROSEDUST spec.

---

## Acceptance Criteria

1. All 17 `Theme::` color constants that existed before this change have updated RGB values matching the CSS `tokens.css` source. Deviations are documented in comments.

2. Seven new named constants (`ROSE_GLOW`, `ROSE_DEEP`, `BONE_BRIGHT`, `TEXT_STRONG`, `TEXT_SOFT`, `DREAM_BRIGHT`, `DREAM_DEEP`) are added to `theme.rs` with doc comments explaining their purpose.

3. The number of inline `Color::Rgb(...)` literals outside `theme.rs` is at most 4 (the `ansi.rs` test assertions). All others have been replaced with `Theme::` constants.

4. `Theme::role_accent(role: &str) -> Color` is defined in `theme.rs` and is the only role-to-color mapping used across the TUI. The local `role_accent()` in `agents_view.rs` is removed.

5. `Theme::BG` and `Theme::VOID` use `Rgb(8, 8, 12)` (tinted dark) instead of pure black `Rgb(0, 0, 0)`.

6. The `theme_defaults_to_rosedust_palette` test in `dashboard.rs` is updated to assert the new canonical values and passes.

7. `cargo test --workspace` passes.

8. `cargo clippy --workspace --no-deps -- -D warnings` passes.

9. `roko dashboard` launches without visual errors (manual verification on a dark terminal).

---

## Verification Checklist

- [ ] `grep -rn "Color::Rgb" crates/roko-cli/src/tui/ | grep -v theme.rs | grep -v ansi.rs | wc -l` returns 0
- [ ] `grep -n "VOID\|BG" crates/roko-cli/src/tui/theme.rs | grep "Rgb(0, 0"` returns 0 (pure black eliminated)
- [ ] `grep -n "fn role_accent\|role_color" crates/roko-cli/src/tui/views/agents_view.rs | grep -v "Theme"` returns 0 (local function deleted)
- [ ] `cargo test -p roko-cli -- theme` passes (theme tests pass with new values)
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] Launch `roko dashboard` and visually check: F1 (plans), F6 (agents), F7 (dream) tabs for correct rose/indigo palette and no pure-black backgrounds

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs` | Update 17 constants; add 7 new constants; add `role_accent()` method |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/agents_view.rs` | Delete local `role_accent()` at line 1120; update 3 call sites; replace 12 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` | Replace 11 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/dream_view.rs` | Replace 22 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/postfx.rs` | Replace 21 inline literals; add named shadow/overlay constants to `theme.rs` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs` | Update 5 test assertions in `theme_defaults_to_rosedust_palette` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/task_progress.rs` | Replace 2 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/sys_metrics.rs` | Replace 2 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/phase_compact.rs` | Replace 2 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/postfx_pipeline.rs` | Replace 2 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/atmosphere.rs` | Replace 2 inline literals |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs` | Replace 1 inline literal |

## Files NOT to Modify

| File | Why |
|---|---|
| `crates/roko-cli/src/tui/ansi.rs` | Contains ANSI parsing test assertions; leave as-is |
| `demo/demo-app/src/styles/tokens.css` | Web frontend; separate concern |
| `tmp/archive/08-17-26/design-systems/01-ROSEDUST-DESIGN-SYSTEM.md` | Read-only reference; do not modify |

---

## Not in Scope

- Implementing Spectre avatars (braille dot-cloud), phosphor decay trails, crystallization reward animations, or progressive intensity bands. These are complex visual effects that would require significant new infrastructure. They are catalogued in the design spec but not part of this alignment task.
- Changes to the `high_contrast()` or `no_color()` theme palettes. These are accessibility features with different requirements.
- Reconciling the ROSEDUST spec document with the CSS tokens. That is a documentation task.
- Typography or spacing alignment (terminal cells vs. CSS pixels are incommensurable).
- Motion/animation timing alignment.
