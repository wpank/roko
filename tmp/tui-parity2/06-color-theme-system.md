# 06 Color & Theme System Audit: ROSEDUST Palette

**Audit date:** 2026-09-01
**Source files:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs` (canonical theme, 518 LOC)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/rosedust.rs` (compatibility shim, 10 LOC)
- 12 view files in `crates/roko-cli/src/tui/views/`
- 18 widget files in `crates/roko-cli/src/tui/widgets/`
- Bardo design system: `/Users/will/dev/uniswap/bardo/prd/18-interfaces/rendering/00-design-system.md`

---

## 1. Complete Color Constant Catalog

### 1.1 Primary Rose Spectrum

| Constant | RGB | Hex | Luminance | Intended Use | Lines |
|---|---|---|---|---|---|
| `VOID` | (0, 0, 0) | `#000000` | 0.000 | True black canvas, matching Mori | L34 |
| `ROSE` | (185, 120, 148) | `#B97894` | 0.247 | Primary accent, active plans, app name | L35 |
| `ROSE_BRIGHT` | (220, 155, 180) | `#DC9BB4` | 0.376 | Focused borders, bright highlights | L36 |
| `ROSE_GLOW` | (228, 172, 196) | `#E4ACC4` | 0.428 | Soft glow effects (currently unused in views) | L37 |
| `ROSE_DIM` | (140, 96, 112) | `#8C6070` | 0.155 | Heartbeat dots, separators, scrollbar thumb | L38 |
| `ROSE_DEEP` | (65, 36, 52) | `#412434` | 0.028 | Background accents (currently unused in views) | L39 |
| `ROSE_EMBER` | (80, 45, 62) | `#502D3E` | 0.039 | Warning bar background | L57 |

### 1.2 Bone (Emphasis/Highlight)

| Constant | RGB | Hex | Luminance | Intended Use | Lines |
|---|---|---|---|---|---|
| `BONE` | (215, 198, 158) | `#D7C69E` | 0.567 | Focused titles, selection FG, wave labels, cost, branch names | L40 |
| `BONE_BRIGHT` | (228, 213, 176) | `#E4D5B0` | 0.650 | Bright bone variant (currently unused in views) | L41 |
| `BONE_DIM` | (160, 142, 108) | `#A08E6C` | 0.316 | Default titles, plan names, cost info, F-key labels | L42 |

### 1.3 Text Hierarchy

| Constant | RGB | Hex | Luminance | Intended Use | Lines |
|---|---|---|---|---|---|
| `TEXT` / `FG` | (165, 142, 158) | `#A58E9E` | 0.319 | Standard foreground, body text | L45, L77 |
| `TEXT_STRONG` | (215, 198, 208) | `#D7C6D0` | 0.576 | Highlighted content, dream replay episode IDs | L46 |
| `TEXT_SOFT` | (200, 184, 196) | `#C8B8C4` | 0.492 | Secondary text, dream progress counts | L63 |
| `TEXT_DIM` / `FG_DIM` | (145, 120, 138) | `#91788A` | 0.224 | Muted text, elapsed time, token counts, labels | L47, L78 |
| `TEXT_GHOST` | (110, 85, 105) | `#6E5569` | 0.121 | Barely visible, idle states, unfocused titles, commit hashes | L48 |
| `TEXT_PHANTOM` | (55, 42, 55) | `#372A37` | 0.033 | Column separators, unfocused borders, empty track segments | L49 |

### 1.4 Semantic Accents

| Constant | RGB | Hex | Luminance | Intended Use | Lines |
|---|---|---|---|---|---|
| `SAGE` / `STATUS_OK` | (125, 158, 140) | `#7D9E8C` | 0.334 | Success, completed, healthy, pass rate >80% | L54, L81 |
| `EMBER` / `STATUS_ERROR` | (195, 110, 85) | `#C36E55` | 0.227 | Danger, failed, error, critical errors | L55, L82 |
| `WARNING` | (195, 155, 95) | `#C39B5F` | 0.358 | Warning, gating, compile/test phases, amber | L56 |
| `DREAM` | (120, 115, 165) | `#7873A5` | 0.212 | Informational, dream state, ETA, research, learning, MCP | L52 |
| `DREAM_BRIGHT` | (150, 145, 192) | `#9691C0` | 0.325 | NREM replay phase, replay candidate bars | L53 |
| `DREAM_REM` | (180, 100, 200) | `#B464C8` | 0.211 | REM imagination, creative purple, hypotheses titles | L59 |
| `DREAM_DEEP` | (40, 40, 72) | `#282848` | 0.024 | Deep accent backgrounds (matches bardo spec exactly) | L61 |

### 1.5 Backgrounds

| Constant | RGB | Hex | Luminance | Intended Use | Lines |
|---|---|---|---|---|---|
| `BG` | (0, 0, 0) | `#000000` | 0.000 | Default background (= VOID) | L66 |
| `BG_RAISED` | (14, 12, 18) | `#0E0C12` | 0.003 | Header/status bar backgrounds, raised panels | L67 |
| `BG_SECONDARY` | (14, 12, 16) | `#0E0C10` | 0.003 | Header bar cell backgrounds, scrollbar track | L68 |
| `BG_HIGHLIGHT` | (34, 28, 36) | `#221C24` | 0.013 | Selected-item row highlight | L69 |

### 1.6 Structural

| Constant | RGB | Hex | Luminance | Intended Use | Lines |
|---|---|---|---|---|---|
| `SEPARATOR` | (40, 35, 42) | `#28232A` | 0.016 | Panel separators (defined but unused directly) | L72 |
| `SHADOW` | (30, 30, 30) | `#1E1E1E` | 0.010 | Shadow effects (defined but unused directly) | L73 |
| `SHADOW_FG` | (50, 50, 50) | `#323232` | 0.028 | Shadow foreground (defined but unused directly) | L74 |

### 1.7 Gradients

| Gradient | Start | Mid | End | Use |
|---|---|---|---|---|
| `gradient_fire` | (120, 30, 20) `#781E14` | (195, 110, 45) `#C36E2D` | (215, 198, 80) `#D7C650` | Progress bars in header |
| `gradient_ocean` | (30, 40, 120) `#1E2878` | (40, 120, 150) `#287896` | (80, 190, 210) `#50BED2` | Wave progress bars, plan tree gradient bars |

---

## 2. Contrast Ratio Analysis

Relative luminance is calculated per WCAG 2.1 using sRGB linearization:
`L = 0.2126*R + 0.7152*G + 0.0722*B` (after gamma correction).

All ratios computed against `BG` (#000000, L=0.000). For ratios against BG, the formula simplifies to `(L_fg + 0.05) / 0.05`.

### 2.1 Text on BG (#000000) -- Key Ratios

| Color | Hex | Contrast Ratio | WCAG AA (4.5:1) | WCAG AAA (7:1) | Usage Context |
|---|---|---|---|---|---|
| BONE | `#D7C69E` | **12.3:1** | PASS | PASS | Focused titles, selection FG |
| BONE_BRIGHT | `#E4D5B0` | **14.0:1** | PASS | PASS | (unused) |
| TEXT_STRONG | `#D7C6D0` | **12.5:1** | PASS | PASS | Highlighted content |
| TEXT_SOFT | `#C8B8C4` | **10.8:1** | PASS | PASS | Secondary text |
| ROSE_BRIGHT | `#DC9BB4` | **8.5:1** | PASS | PASS | Focused borders, active highlights |
| ROSE_GLOW | `#E4ACC4` | **9.6:1** | PASS | PASS | (unused) |
| WARNING | `#C39B5F` | **8.2:1** | PASS | PASS | Amber warnings |
| SAGE | `#7D9E8C` | **7.7:1** | PASS | PASS | Success/completed |
| TEXT / FG | `#A58E9E` | **7.4:1** | PASS | PASS | Body text |
| BONE_DIM | `#A08E6C` | **7.3:1** | PASS | PASS | Titles, plan names |
| DREAM_BRIGHT | `#9691C0` | **7.5:1** | PASS | PASS | NREM phase |
| ROSE | `#B97894` | **6.0:1** | PASS | FAIL | Primary accent |
| EMBER | `#C36E55` | **5.5:1** | PASS | FAIL | Error/danger text |
| DREAM | `#7873A5` | **5.2:1** | PASS | FAIL | Info, ETA, dream state |
| TEXT_DIM | `#91788A` | **5.5:1** | PASS | FAIL | Muted labels, elapsed |
| DREAM_REM | `#B464C8` | **5.2:1** | PASS | FAIL | REM imagination |
| ROSE_DIM | `#8C6070` | **4.1:1** | FAIL | FAIL | Heartbeat, separators |
| TEXT_GHOST | `#6E5569` | **3.4:1** | FAIL | FAIL | Unfocused titles, idle states |
| TEXT_PHANTOM | `#372A37` | **1.7:1** | FAIL | FAIL | Borders, separators |
| ROSE_EMBER | `#502D3E` | **1.8:1** | FAIL | FAIL | Warning bar BG |
| ROSE_DEEP | `#412434` | **1.6:1** | FAIL | FAIL | Background accents |

### 2.2 Text on BG_HIGHLIGHT (#221C24) -- Selection Context

| Color | Hex | Contrast Ratio | WCAG AA |
|---|---|---|---|
| BONE (selection FG) | `#D7C69E` | **8.7:1** | PASS |
| ROSE_BRIGHT | `#DC9BB4` | **6.0:1** | PASS |
| TEXT | `#A58E9E` | **5.2:1** | PASS |
| TEXT_DIM | `#91788A` | **3.9:1** | FAIL |

### 2.3 Text on ROSE_EMBER (#502D3E) -- Warning Bar

| Color | Hex | Contrast Ratio | WCAG AA |
|---|---|---|---|
| BONE | `#D7C69E` | **6.9:1** | PASS |
| WARNING | `#C39B5F` | **4.6:1** | PASS (borderline) |
| TEXT_GHOST | `#6E5569` | **1.9:1** | FAIL -- "[n] dismiss" hint on warning bar |

### 2.4 Critical Failure: PAUSED Indicator

The PAUSED badge uses `VOID` (#000000) text on `WARNING` (#C39B5F) background:
- Contrast ratio: **8.2:1** -- PASS (good choice)

### 2.5 Active Tab F-Key Indicators

Active tab uses `VOID` (#000000) text on the tab's color as background. All tab colors are light enough:
- VOID on ROSE: **6.0:1** PASS
- VOID on SAGE: **7.7:1** PASS
- VOID on DREAM: **5.2:1** PASS
- VOID on BONE_DIM: **7.3:1** PASS

### 2.6 Summary

- **14/20 palette colors pass WCAG AA** (4.5:1) against the black background
- **10/20 pass WCAG AAA** (7:1) against black
- **6 colors fail AA**: ROSE_DIM, TEXT_GHOST, TEXT_PHANTOM, ROSE_EMBER, ROSE_DEEP, DREAM_DEEP
- The lowest 4 (TEXT_PHANTOM, ROSE_EMBER, ROSE_DEEP, DREAM_DEEP) are intentionally sub-perceptual for structural/atmospheric use -- acceptable by design
- **ROSE_DIM at 4.1:1 is the most concerning AA failure** -- it carries meaningful information (heartbeat dot, separators, in-flight agent count) and should be readable

---

## 3. WCAG Compliance Assessment

### 3.1 Passing Elements (AA or better)

All primary content text meets WCAG AA:
- Body text (`TEXT` 7.4:1)
- Titles (`BONE` 12.3:1, `BONE_DIM` 7.3:1)
- Semantic status colors (`SAGE` 7.7:1, `EMBER` 5.5:1, `WARNING` 8.2:1)
- Selection foreground (`BONE` on `BG_HIGHLIGHT` 8.7:1)
- Highlighted content (`TEXT_STRONG` 12.5:1)

### 3.2 Marginal/Failing Elements

| Element | Colors | Ratio | Risk |
|---|---|---|---|
| Warning bar dismiss hint | TEXT_GHOST on ROSE_EMBER | 1.9:1 | **HIGH** -- interactive text is invisible to many users |
| Selected-row muted text | TEXT_DIM on BG_HIGHLIGHT | 3.9:1 | **MEDIUM** -- secondary info in selections hard to read |
| Unfocused panel titles | TEXT_GHOST on BG | 3.4:1 | **MEDIUM** -- tab names for unfocused panels are dim |
| Heartbeat/separator dots | ROSE_DIM on BG | 4.1:1 | **LOW** -- decorative, but carries in-flight agent count |
| Column separators | TEXT_PHANTOM on BG | 1.7:1 | **NONE** -- intentionally structural, not content |

### 3.3 High-Contrast Mode

The `high_contrast()` palette variant is well-constructed:
- White (#FFFFFF) text on Black (#000000): 21:1
- All semantic colors are bright variants (bright green, bright yellow, bright red, bright blue)
- Environment variable activation (`ROKO_HIGH_CONTRAST=1`) is clean
- The `no_color()` mode correctly uses `Color::Reset` throughout

---

## 4. Semantic Color Consistency

### 4.1 Semantic Mapping

| Semantic | Expected Color | Actual Colors Used | Consistent? |
|---|---|---|---|
| Success/Complete/Pass | Green/SAGE | `SAGE` throughout | YES |
| Error/Fail/Critical | Red/EMBER | `EMBER` throughout | YES |
| Warning/Gating/Compile | Amber/WARNING | `WARNING` throughout | YES |
| Info/Active/In-flight | Blue-purple/DREAM | `DREAM` throughout | YES |
| Muted/Inactive/Secondary | Grey/TEXT_DIM | `TEXT_DIM`, `TEXT_GHOST` | YES |

### 4.2 Semantic Color Functions

Three layers of semantic color abstraction exist:

1. **Theme struct fields**: `success`, `warning`, `danger`, `info`, `muted` (lines 19-25)
2. **Style methods**: `theme.success()`, `theme.warning()`, `theme.danger()`, `theme.info()` (lines 197-225)
3. **Derived semantic functions**: `semantic_color(t)`, `progress_color(fraction)`, `role_accent(role)`, `phase_accent(phase)` (lines 283-330)

All three layers are internally consistent. `semantic_color()` correctly maps:
- 0.0-0.4 -> EMBER (danger)
- 0.4-0.8 -> WARNING (caution)
- 0.8-1.0 -> SAGE (success)

### 4.3 Consistency Violations Found

**learning_view.rs uses raw `Color::` constants instead of theme colors.** This is the most significant consistency violation:

| Location | Raw Color | Should Be |
|---|---|---|
| `learning_view.rs:86` | `Color::Yellow` | `Theme::WARNING` |
| `learning_view.rs:88` | `Color::Cyan` | `Theme::DREAM` or similar |
| `learning_view.rs:90` | `Color::Green` | `Theme::SAGE` |
| `learning_view.rs:151-155` | `Color::Green/Yellow/Red` | `Theme::SAGE/WARNING/EMBER` |
| `learning_view.rs:240-245` | `Color::Blue/Cyan/Green/Yellow/Magenta/Red` | Theme bar chart palette needed |
| `learning_view.rs:318-320` | `Color::Yellow/Cyan/Green` | Same as L86-90 |
| `learning_view.rs:381-396` | `Color::Yellow/Cyan/Green` | Same stage colors, also raw |
| `learning_view.rs:485-489` | `Color::Green/Yellow/Red` | Same pass-rate colors |
| `learning_view.rs:526-531` | `Color::Blue/Cyan/Green/Yellow/Magenta/Red` | Same bar chart array |

**diff_panel.rs:71** uses `Color::Cyan` for `@@` hunk headers instead of a theme constant.

**Impact:** These raw colors will NOT respond to theme switching (high-contrast, no-color). The learning view effectively has its own unthemed sub-palette. Bar charts in particular use the 16-color ANSI palette which clashes visually with the 24-bit ROSEDUST colors.

---

## 5. Color Similarity / Confusion Risk

### 5.1 Perceptually Close Pairs

| Color A | Color B | Delta-E (approx) | Risk |
|---|---|---|---|
| `BG_RAISED` (14,12,18) | `BG_SECONDARY` (14,12,16) | ~0.5 | **HIGH** -- functionally identical, 2-value blue channel difference only |
| `TEXT` (165,142,158) | `TEXT_DIM` (145,120,138) | ~8 | **MEDIUM** -- close enough to blur hierarchy in peripheral vision |
| `SAGE` (125,158,140) | `TEXT` (165,142,158) | ~10 | **MEDIUM** -- both desaturated mid-tones; success can blend with body text |
| `DREAM` (120,115,165) | `DREAM_REM` (180,100,200) | ~20 | LOW -- different hue and saturation, visually distinct |
| `EMBER` (195,110,85) | `WARNING` (195,155,95) | ~12 | **MEDIUM** -- both warm, same R channel; danger vs warning can blur on some displays |
| `BONE` (215,198,158) | `BONE_DIM` (160,142,108) | ~18 | LOW -- clearly different brightness |
| `ROSE` (185,120,148) | `ROSE_BRIGHT` (220,155,180) | ~14 | LOW -- distinct brightness step |

### 5.2 Highest-Risk Confusion

1. **BG_RAISED vs BG_SECONDARY**: These are effectively the same color. The 2-unit blue channel difference is imperceptible on any monitor. They should either be merged or given more meaningful separation. Currently `BG_RAISED` is used for generic raised panels while `BG_SECONDARY` is used for header/status bars -- the distinction is semantic but not visual.

2. **SAGE vs TEXT**: On a slightly warm-calibrated monitor, completed-plan text (SAGE) and body text (TEXT) can look confusingly similar. The hue difference (green-grey vs mauve-grey) is subtle at these saturation levels.

3. **EMBER vs WARNING**: Both warm tones with the same R=195. Under reduced color perception (deuteranomaly affects ~8% of males), these two are very hard to distinguish. The semantic gap (error vs warning) is critical but the visual gap is narrow.

---

## 6. Terminal Background Compatibility

### 6.1 Dark Terminal Backgrounds (default, expected)

The palette is designed exclusively for dark terminals. Against `#000000` or near-black backgrounds, all colors render as intended. The BG constant IS #000000.

### 6.2 Light Terminal Backgrounds

**The ROSEDUST palette is completely unsuitable for light terminal backgrounds.** All colors are designed for dark-on-light contrast:

- `TEXT_PHANTOM` (#372A37) would be invisible against white
- `BG_RAISED` and `BG_SECONDARY` would render as black boxes
- `ROSE_DIM` and `TEXT_GHOST` would vanish
- The fire and ocean gradients start from near-black and would lose all definition

There is no light theme variant. This is acceptable for a specialized dashboard tool, but notable.

### 6.3 Semi-Dark Backgrounds (#1a1a1a - #2a2a2a)

Common "dark but not black" terminal themes (Solarized Dark, Dracula, One Dark) use backgrounds in the #1a1a1a to #2a2a2a range. Against these:

- `BG_HIGHLIGHT` (#221C24) would be invisible or inverted
- `TEXT_PHANTOM` (#372A37) would blend into the terminal's native background
- `ROSE_EMBER` (#502D3E) would lose contrast distinction from the terminal background
- Panel borders (TEXT_PHANTOM) would disappear entirely

**Recommendation:** Either enforce true-black background with a terminal config note, or detect and warn when the terminal background is not #000000. The `BG` constant being exactly #000000 means any non-black terminal theme will break the depth model.

### 6.4 NO_COLOR Compliance

The `no_color()` mode correctly resets all colors to `Color::Reset`, complying with the NO_COLOR standard. The `from_env()` method checks `NO_COLOR` and `ROKO_HIGH_CONTRAST` environment variables.

---

## 7. Visual Hierarchy Through Color

### 7.1 Hierarchy Levels (Brightness-Based)

The palette establishes a clear 7-level brightness hierarchy:

| Level | Color(s) | Luminance Range | Purpose |
|---|---|---|---|
| **1. Maximum** | BONE, BONE_BRIGHT, TEXT_STRONG | 0.57-0.65 | The ONE important element, focused titles |
| **2. High** | ROSE_BRIGHT, ROSE_GLOW | 0.38-0.43 | Active highlights, focused borders |
| **3. Primary** | TEXT, BONE_DIM, SAGE, WARNING | 0.22-0.36 | Standard content, semantic status |
| **4. Secondary** | ROSE, DREAM, TEXT_DIM, EMBER | 0.21-0.25 | Accents, info, muted labels |
| **5. Tertiary** | ROSE_DIM, TEXT_GHOST | 0.12-0.16 | Background decorations, idle states |
| **6. Structural** | TEXT_PHANTOM, ROSE_EMBER | 0.03-0.04 | Borders, separators, track |
| **7. Subliminal** | BG_HIGHLIGHT, BG_RAISED, BG_SECONDARY | 0.003-0.013 | Background depth |

This hierarchy is well-designed. The jump from Level 6 to Level 5 (structural to tertiary) is the largest perceptual gap, which correctly makes panel borders feel separate from content.

### 7.2 Hierarchy Application in Practice

The header bar (header_bar.rs) demonstrates good hierarchy usage:
- App name "roko" in ROSE (Level 4) -- distinctive but not shouting
- Wave indicator in BONE (Level 1) -- the key progress metric
- Progress bar uses gradient_fire -- draws the eye
- ETA in DREAM (Level 4) -- informational
- System metrics use semantic coloring (SAGE/WARNING/EMBER)
- F-key labels are DIM (Level 4) with the active tab inverted (VOID on color)

The plan tree (plan_tree.rs) similarly uses:
- Active plans: ROSE_BRIGHT (Level 2) with BOLD
- Completed plans: SAGE (Level 3)
- Failed plans: EMBER (Level 3)
- Pending plans: TEXT_DIM (Level 4)
- Structural separators: TEXT_PHANTOM (Level 6)

### 7.3 Hierarchy Weakness

**No "screaming emergency" tier exists.** The brightest color (BONE at ~0.57 luminance) is a warm beige. The bardo design system uses `rose_bright` (#CC90A8) as the danger/alert color, mapping to roko's ROSE_BRIGHT -- but roko uses EMBER for errors, which is actually dimmer than BONE. A critical error (EMBER, 0.227 luminance) is less bright than a focused panel title (BONE, 0.567 luminance). This means errors blend into the background while titles shout -- the opposite of optimal emergency visibility.

---

## 8. Interactive vs Non-Interactive Element Distinction

### 8.1 Focus System

The focused/unfocused panel distinction is clear and well-implemented:

| State | Border Color | Title Color | Method |
|---|---|---|---|
| **Focused** | ROSE_BRIGHT + BOLD | BONE + BOLD | `focused_border_style()`, `focused_title_style()` |
| **Unfocused** | TEXT_PHANTOM | TEXT_GHOST | `unfocused_border_style()`, `unfocused_title_style()` |

The contrast between focused (bright rose) and unfocused (near-invisible phantom) is dramatic -- a ~16x brightness ratio. This is effective.

### 8.2 Selection Highlighting

Selected rows use `BG_HIGHLIGHT` (#221C24) background with BONE foreground and BOLD modifier. The selection style is defined centrally:

```rust
pub fn selection(self) -> Style {
    Style::default()
        .fg(self.selection_foreground)  // BONE
        .bg(self.selection_background)  // BG_HIGHLIGHT
        .add_modifier(Modifier::BOLD)
}
```

This is consistent across plan_tree, git_view, and logs_view.

### 8.3 Interactive Affordances

**Weakness: No cursor/hover distinction.** The TUI uses selection highlighting but has no visual cue for "this element is clickable/actionable" vs "this element is display-only." The keybind hints in the status bar are the only interactive affordance, and they use TEXT_DIM (the same color as muted labels). The "[n] dismiss" hint on warning bars uses TEXT_GHOST, which is nearly invisible.

**Recommendation:** Keybind hints and interactive affordances should use a brighter color than non-interactive labels. Using DREAM or ROSE_DIM for hint keys and TEXT_DIM for hint descriptions would create a visual hierarchy within the hints themselves.

---

## 9. Comparison with Bardo Design System

### 9.1 Palette Drift

| Token (Bardo Spec) | Bardo Value | Roko Value | Delta |
|---|---|---|---|
| `bg_void` | #060608 (6,6,8) | #000000 (0,0,0) | **Diverged: roko uses true black, bardo uses violet-black** |
| `bg_raised` | #0C0A0E (12,10,14) | #0E0C12 (14,12,18) | Close, minor shift |
| `rose` | #AA7088 (170,112,136) | #B97894 (185,120,148) | **Brightened +15 R, +8 G, +12 B** |
| `rose_bright` | #CC90A8 (204,144,168) | #DC9BB4 (220,155,180) | **Brightened +16 R, +11 G, +12 B** |
| `rose_dim` | #7A5060 (122,80,96) | #8C6070 (140,96,112) | **Brightened +18 R, +16 G, +16 B** |
| `rose_deep` | #3A2030 (58,32,48) | #412434 (65,36,52) | Minor shift |
| `rose_ember` | #482838 (72,40,56) | #502D3E (80,45,62) | Minor shift |
| `bone` | #C8B890 (200,184,144) | #D7C69E (215,198,158) | **Brightened +15 R, +14 G, +14 B** |
| `bone_dim` | #8A7A5A (138,122,90) | #A08E6C (160,142,108) | **Brightened +22 R, +20 G, +18 B** |
| `text_primary` | #988090 (152,128,144) | #A58E9E (165,142,158) | **Brightened +13 all channels** |
| `text_dim` | #584858 (88,72,88) | #91788A (145,120,138) | **MAJOR divergence: +57 R, +48 G, +50 B** |
| `text_ghost` | #302830 (48,40,48) | #6E5569 (110,85,105) | **MAJOR divergence: +62 R, +45 G, +57 B** |
| `text_phantom` | #201820 (32,24,32) | #372A37 (55,42,55) | Significant brightening |
| `dream` | #585878 (88,88,120) | #7873A5 (120,115,165) | **Brightened +32 R, +27 G, +45 B** |
| `dream_deep` | #282848 (40,40,72) | #282848 (40,40,72) | **Exact match** |
| `warning` | #AA8855 (170,136,85) | #C39B5F (195,155,95) | **Brightened +25 R, +19 G, +10 B** |
| `success` | #70887A (112,136,122) | #7D9E8C (125,158,140) | Brightened +13 R, +22 G, +18 B |

### 9.2 Systematic Pattern

Roko's palette is systematically brighter than bardo's spec by roughly 10-20% across all colors, with `text_dim` and `text_ghost` having the largest drift (~60-65%). This was likely a deliberate readability adjustment -- bardo's spec targets a more atmospheric, film-like darkness where much text is intentionally hard to read (the "terminal existentialism" aesthetic), while roko needs practical readability for long monitoring sessions.

### 9.3 Missing from Roko (Present in Bardo Spec)

1. **bg_void (#060608)** -- Roko uses pure black, losing the violet undertone that gives bardo's void depth
2. **bg_mid (#080810)** -- No intermediate background depth
3. **bg_warm (#0A0808)** -- No warm-shifted void for degraded states
4. **border (#181420)** and **border_active (#AA708844)** -- Roko uses TEXT_PHANTOM and ROSE_BRIGHT instead
5. **Palette degradation by lifecycle phase** -- Not implemented; palette is static
6. **PAD-driven micro-shifts** -- Not implemented at the color level (only behavioral state labels in affect_view)
7. **CRT materiality tokens** (scanline_dark, phosphor_res, bleed_rose, halftone_bg, noise_warm, noise_cool) -- Not implemented
8. **oklch color space** for perceptually uniform interpolation -- Roko uses linear RGB interpolation in gradients and `brighten()`
9. **Bone scarcity rule** ("used once per screen, max") -- Roko uses BONE freely across headers, titles, and branch names
10. **dream_dim (#383858)** -- No dimmed dream variant

### 9.4 Added in Roko (Not in Bardo Spec)

1. **ROSE_GLOW** (#E4ACC4) -- Softer rose variant
2. **TEXT_STRONG** (#D7C6D0) -- Brighter text for highlighted content
3. **TEXT_SOFT** (#C8B8C4) -- Warm secondary text
4. **DREAM_BRIGHT** (#9691C0) -- Brighter dream for NREM phase
5. **DREAM_REM** (#B464C8) -- Creative purple accent
6. **BONE_BRIGHT** (#E4D5B0) -- Brighter bone variant
7. **High-contrast mode** -- Full WCAG-compliant alternate palette (not in bardo spec)
8. **NO_COLOR mode** -- Color::Reset fallback (not in bardo spec)

### 9.5 Design Philosophy Gap

Bardo's design system is fundamentally about **atmosphere over data** -- the 8-layer atmospheric stack, the 50% empty space rule, the CRT materiality, the "light follows significance" law. Roko's implementation is **data-dense and functional** -- every pixel serves an information purpose. This is not a deficiency; it's a different product goal. Roko is a build-monitoring dashboard, not an existential meditation tool.

However, the **fire and ocean gradients** and the **heartbeat/breathing animations** in roko are direct descendants of bardo's atmospheric approach and work well as controlled injections of visual life into the data-dense layout.

---

## 10. Improvement Recommendations for a More Visually Striking, Demoscene-Inspired Aesthetic

### 10.1 Immediate Fixes (No Visual Change, Just Correctness)

1. **Replace all raw `Color::` constants in learning_view.rs** with theme constants. Create `Theme::BAR_CHART_COLORS: [Color; 6]` as a themed array for bar chart segments. This is the most urgent fix -- 32 instances of raw ANSI colors that bypass theming.

2. **Replace `Color::Cyan` in diff_panel.rs:71** with `Theme::DREAM` or a new `Theme::HUNK` constant.

3. **Merge BG_RAISED and BG_SECONDARY** or give them meaningful visual separation (+5 to one of the channels at minimum). Currently they are visually identical.

### 10.2 Contrast Fixes

4. **Bump ROSE_DIM** from (140, 96, 112) to (155, 106, 124) to clear the 4.5:1 AA threshold. It carries meaningful information (in-flight count, separators).

5. **Warning bar dismiss hint**: Change from TEXT_GHOST to TEXT_DIM on ROSE_EMBER, or use BONE_DIM. The current 1.9:1 ratio makes "[n] dismiss" invisible.

6. **Unfocused panel titles**: Consider bumping TEXT_GHOST from (110, 85, 105) to (125, 98, 118) for better readability, or accept the intentional dimness as part of the focus model.

### 10.3 Demoscene-Inspired Enhancements

7. **Void depth**: Change BG from #000000 to #040306 (matching bardo's concept of "not pure black"). True black is a hole; a very-dark-violet-black has depth. This single change would shift the entire feel from "terminal" to "CRT phosphor space."

8. **Plasma substrate on idle screens**: When no plans are running and the dashboard is idle, render a very subtle braille-based plasma effect in the empty space using ROSE_DEEP and DREAM_DEEP colors at ~0.3% density. The `braille.rs` widget already exists; the infrastructure is there. This would make the idle state feel alive rather than dead.

9. **Gradient progress bar enhancement**: The fire gradient in the header is good but could use a trailing glow effect. When the progress bar advances, the cell immediately behind the fill edge should briefly flash to a brighter version of the gradient color (100ms decay). This is a classic demoscene "comet tail" effect and would make progress feel more dynamic.

10. **Scanline modulation**: Add a subtle alternating-row background dimming (render every even row with BG at +2 luminance and odd rows at -1). This CRT scanline effect requires only a post-processing pass over the rendered buffer and would immediately distinguish roko's TUI from every other ratatui application. The `postfx.rs` file already exists with color utility functions.

11. **Phase-colored breathing**: Currently the heartbeat dot brightness pulses uniformly. Tie the pulse color to the health status so:
    - Healthy: breathing shifts between SAGE and a slightly-brighter SAGE
    - Gating: breathing shifts between WARNING and ROSE
    - Error: breathing becomes faster and shifts toward EMBER
    - This creates a "living" status indicator that communicates health through rhythm AND color.

12. **Dream-state palette shift**: When dream state data is populated (DreamPhaseLabel != Idle), shift the entire panel's background from BG to BG with a +4 blue channel offset (#000004). This subtle cool shift is bardo's "dreaming" palette in its simplest form.

13. **Per-cell gradient in wave progress**: The wave_progress.rs already does per-cell ocean gradient for the current wave, but the gradient phase is time-based. Add a velocity component: when a task completes, the gradient phase jumps by 0.3 radians, creating a visual "pulse" through the bar. Completions would visually ripple through the progress indicator.

14. **Phosphor persistence for BONE elements**: When a BONE-colored element (the "most important number") changes value, keep a dim ROSE_DIM echo of the previous value for 500ms, fading to BG. This phosphor-persistence effect from bardo's spec is achievable with a small HashMap of recently-changed cells in the render loop.

15. **Add a theme constant palette for category/series differentiation**: Currently bar charts use raw ANSI `Color::Blue, Cyan, Green, Yellow, Magenta, Red`. Define a themed palette:
    ```
    SERIES_1: ROSE           -- primary data series
    SERIES_2: DREAM          -- secondary
    SERIES_3: SAGE           -- tertiary
    SERIES_4: WARNING        -- quaternary
    SERIES_5: BONE_DIM       -- fifth
    SERIES_6: DREAM_REM      -- sixth
    ```
    This keeps bar charts in the ROSEDUST world rather than the ANSI 16-color world.

### 10.4 Structural Improvements

16. **Formalize the depth model**: Currently backgrounds are used ad-hoc. Define explicit depth levels as bardo does:
    ```
    DEPTH 0: BG           (#000000) -- void
    DEPTH 1: BG_RAISED    (#0E0C12) -- panels
    DEPTH 2: BG_HIGHLIGHT (#221C24) -- selection/active
    DEPTH 3: (missing)    (#2A2230) -- overlays/modals
    ```
    Add the missing overlay depth level for future modal/popup support.

17. **Add oklch-based color interpolation**: The current `brighten()` function and `Gradient` struct use linear RGB, which produces perceptually non-uniform transitions. For a 3-stop gradient, oklch interpolation would produce smoother, more natural color ramps. This matters most for the fire and ocean gradients where mid-range tones can look muddy in linear RGB.

### 10.5 Unused Constants

The following constants are defined but never referenced in any view or widget file:

| Constant | Status |
|---|---|
| `ROSE_GLOW` | Unreferenced in views/widgets |
| `ROSE_DEEP` | Unreferenced in views/widgets |
| `BONE_BRIGHT` | Unreferenced in views/widgets |
| `SEPARATOR` | Unreferenced (TEXT_PHANTOM used instead) |
| `SHADOW` | Unreferenced |
| `SHADOW_FG` | Unreferenced |

These should either be put to use (ROSE_GLOW for breath effects, BONE_BRIGHT for critical alerts, SEPARATOR for actual separators) or removed to reduce the surface area of the palette.

---

## Summary

The ROSEDUST palette is a well-designed, warm-toned dark theme with clear semantic assignments and a functional visual hierarchy. Its primary strengths are the consistent semantic color system (SAGE/EMBER/WARNING/DREAM mapping to success/error/warning/info) and the centralized Theme struct with style methods.

**Critical issues:**
1. 32 raw `Color::` constants in learning_view.rs bypass theming entirely
2. BG_RAISED and BG_SECONDARY are visually indistinguishable
3. Warning bar dismiss hint (TEXT_GHOST on ROSE_EMBER) has 1.9:1 contrast -- effectively invisible

**Strategic observations:**
- The palette has systematically brightened 10-20% from bardo's spec, trading atmosphere for readability
- True-black background (#000000) loses the violet-depth feel that bardo's #060608 provides
- The atmospheric/demoscene features (scanlines, phosphor persistence, plasma) from bardo's spec are entirely absent -- but the infrastructure (postfx.rs, braille.rs, atmosphere module) exists to implement them incrementally
- 6 defined color constants are never used in any view or widget
