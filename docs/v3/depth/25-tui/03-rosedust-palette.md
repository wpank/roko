# 25.03 -- ROSEDUST Design Language

> Depth file for [25-TUI.md](../../25-TUI.md) section 4.

---

## Palette Overview

ROSEDUST is Roko's visual identity: rose tones on a dark ground plane, deliberate
contrast, glass morphism approximation, and luxury motion. The name evokes rose light
on dust -- viewing the system through a faintly glowing lens.

## TUI Palette (canonical v2 constants from `Theme`)

### Backgrounds

| Constant | RGB | Usage |
|---|---|---|
| `VOID` | (0, 0, 0) | Deepest background (true black) |
| `BG_RAISED` | (14, 12, 18) | Elevated panel backgrounds |
| `BG_SECONDARY` | (14, 12, 16) | Secondary panel backgrounds |
| `BG_HIGHLIGHT` | (34, 28, 36) | Selection highlight |

### Rose Accent Family

| Constant | RGB | Usage |
|---|---|---|
| `ROSE_DEEP` | (65, 36, 52) | Background accent |
| `ROSE_DIM` | (155, 106, 124) | Muted elements, inactive borders |
| `ROSE` | (185, 120, 148) | Primary accent: active elements |
| `ROSE_BRIGHT` | (220, 155, 180) | Highlighted elements, active borders |
| `ROSE_GLOW` | (228, 172, 196) | Maximum emphasis, notifications |

### Text Hierarchy

| Constant | RGB | Usage |
|---|---|---|
| `TEXT_PHANTOM` | (55, 42, 55) | Barely visible structural elements |
| `TEXT_GHOST` | (110, 85, 105) | Timestamps, decorative text |
| `TEXT_DIM` | (145, 120, 138) | Secondary text |
| `TEXT` | (165, 142, 158) | Primary body text |
| `TEXT_STRONG` | (215, 198, 208) | Emphasized text |
| `TEXT_SOFT` | (200, 184, 196) | Soft secondary text |

### Semantic Accents

| Constant | RGB | Meaning |
|---|---|---|
| `SAGE` | (125, 158, 140) | Success, passing gates |
| `WARNING` | (195, 155, 95) | Warnings, approaching thresholds |
| `EMBER` | (195, 110, 85) | Errors, failed gates |
| `DREAM` | (120, 115, 165) | Knowledge, violet accent |
| `DREAM_BRIGHT` | (150, 145, 192) | Active knowledge, highlights |
| `DREAM_REM` | (180, 100, 200) | REM imagination, creative purple |
| `BONE` | (215, 198, 158) | Warm foreground data values |
| `TEAL` | (100, 150, 170) | Researcher roles |
| `LAVENDER` | (155, 130, 175) | Conductor/orchestrator roles |

## OKLab Color Science

ROSEDUST uses OKLab (Ottosson 2020) for perceptually uniform gradient interpolation.
The `gradient()` function in `color.rs` interpolates between two colors in OKLab
space, producing clean midpoints without the muddy results of sRGB linear
interpolation.

OKLCH (cylindrical OKLab): the rose anchor sits at approximately
`OKLCH(0.65, 0.13, 12 degrees)`.

## APCA Contrast Verification

All text/background pairings are verified against APCA (Advanced Perceptual Contrast
Algorithm, WCAG 3.0 candidate). APCA is polarity-aware, accounting for
light-text-on-dark-bg perception. Targets: |Lc| >= 75 for body text, >= 60 for
large/bold, >= 45 for non-text.

## Terminal Color Quantization

For 256-color terminals, ROSEDUST maps via perceptual distance in OKLab space.
The `nearest_256()` function finds the closest xterm-256 index using OKLab
Euclidean distance, preserving perceptual intent.

## NO_COLOR Compliance

The `active_theme()` function checks `$NO_COLOR` and returns a fully reset palette
when set. This follows the https://no-color.org/ standard.

## Color Blindness Safety

Under deuteranopia simulation, rose shifts toward brownish-yellow and jade toward
blue-gray. The pair remains distinguishable due to OKLab lightness difference
(L=0.65 vs L=0.84). All status indicators use symbol + color encoding.
