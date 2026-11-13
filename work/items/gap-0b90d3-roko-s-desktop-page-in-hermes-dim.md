+++
id = "gap-0b90d3"
kind = "gap"
title = "roko's desktop page in Hermes: dim text on a module's sunk area reads 4.25-4.47:1 on three non-default themes (Everforest light and dark, Nous Alt dark), under design system §3's 4.5:1"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["nous/plugin"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "plugin-dash-v8, round v8 2026-11-13: measured after the contrast fix (hermes-plugin-roko 4175f29, 0ac7527); the lead asked for a p3 if the same rule could not fix them"
anchors = ["../hermes-plugin-roko/desktop/plugin.js"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

roko's desktop page in Hermes (hermes-plugin-roko 0.9, `desktop/plugin.js`) reads at least 4.5:1 for every ink on the
page and on a module, on all 20 Hermes desktop theme × mode paintings (round v8, 4175f29 and 0ac7527). On a module's
sunk area it does not on three non-default themes:

| Theme | Ink | On the sunk area | Why |
|---|---|---|---|
| Everforest light | ink-1, the theme's own `#5c6a72` at 94% | 4.41:1 | the host's ink passes on the page (4.73) and is kept |
| Everforest dark | ink-3, the panel's night `#8a8c8b` | 4.25:1 | the night ink was made for graphite; this ground is lighter (`#1f2225`) |
| Nous Alt dark | ink-3, Hermes' 54% step of `#ffe6cb` | 4.47:1 | a mid-blue ground (`#142345`) |

The sunk areas are `--rk-module-2` (`color-mix(in oklab, var(--rk-module) 94%, var(--rk-ink))`, table heads and folded
detail) and `--rk-sunk` (95% of the page). On a dark host they mix in 5-6% of a light ink, so they are lighter than
the module and the dim ink reads lower on them. The design system's own night pair is at the edge too: `#8a8c8b` on
`#232427` is 4.56:1.

## Why it matters

Goal hermes (polish): design system §3 asks 4.5:1 for text on every surface it is used on. Labels and times (ink-3,
and since 0ac7527 what ink-4 painted) sit on these areas.

## Where

- `hermes-plugin-roko/desktop/plugin.js`: `hostTone()` (the probe and the weak-ink check, page and module only),
  `TONE_INKS`, the `.roko{…}` rule's `--rk-module-2` and `--rk-sunk`.
- `hermes-plugin-roko/tests/desktop/test_desktop_contrast.py`: checks page and module, not the sunk areas.
- The dashboard is not affected: its sunk area is an srgb tint, and its contrast test covers it (all pass).

## Current state

The weak check falls back to the design system's fixed inks. Extending it to the sunk areas would fix Everforest
light (the day ink-1 reads 15.7:1) and Midnight dark's borderline 4.51, but it would make Nous Alt dark worse: the
night ink-3 reads 4.11:1 there, below its own 54% step's 4.47. Everforest dark already has the night inks, and they
read 4.25:1. Fixed inks cannot reach every ground. Numbers are from `tmp/hermes-roko-v8/plugin-dash-v8/tools/`
(`desktop_themes.py` and an oklab mix; Hermes' surfaces confirmed in Chromium by `tone_chrome.py`).

## Plan

Pick one:
1. Measure the sunk areas too (probe a `color-mix(in oklab, …)` swatch; Chromium returns `oklab(…)`, so `paintOf`
   needs an oklab-to-sRGB step) and, where the chosen dim ink still reads under 4.5:1 there, let ink-3 take ink-2.
   This is what prefers-contrast: more already does. It is cheap, but on those themes dim text looks like body text.
2. Compute the ink per host: lift the chosen ink-3 toward the pole (white on dark, black on light) just until it
   reads 4.5:1 on the worst surface, like Hermes' `ensureContrast` in `apps/shared/src/color.ts`. This keeps the
   hierarchy, but the tone's inks are no longer only §3's.
Either way, extend `test_desktop_contrast.py` to the sunk areas (an oklab mix of the module and the ink) for all 20
paintings.

## Done when

`tests/desktop/test_desktop_contrast.py` checks --rk-ink to -4 on the page, a module, `--rk-module-2` and `--rk-sunk`
for every theme, and passes.

## Notes

Found by plugin-dash-v8 in round v8 (2026-11-13), measuring the contrast fix 4175f29. The lead asked for a fix if the
same rule made it cheap, and a p3 if not. It does not: the same rule fixes Everforest light and worsens Nous Alt dark.
