+++
id = "gap-cf3940"
kind = "gap"
title = "roko's CLI has no one terminal style: --color, NO_COLOR and CLICOLOR reach none of run, plan, status or diagnose; roko run writes ANSI into pipes and under NO_COLOR; CI is read nowhere; three glyph sets and several time and money formats"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/output"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/main.rs::ColorMode", "crates/roko-cli/src/output_format.rs", "crates/roko-cli/src/inline/terminal.rs::should_use_inline"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn colour_follows_the_flag_then_no_color_then_the_terminal' crates/roko-cli/src && cargo test -p roko-cli --lib term_style"
+++

## Problem

roko's CLI has no single terminal style:
- `--color`, `NO_COLOR` and `CLICOLOR` reach only tracing and `roko show --follow`.
- `roko run` writes ANSI escapes into pipes and under `NO_COLOR`: 51 SGR codes in each mode of round v8's
  captures. `output_format.rs` never checks for a TTY, and its `is_tty` has no caller.
- `plan run`, `plan validate`, `plan status`, `status` and `diagnose` never colour.
- `CI` is read by no output code.
- Glyphs and formats vary from place to place:
  - three glyph sets (`inline/symbols.rs`, `tui/icons.rs`, and hard-coded `✓ ✗ ▸`), plus emoji in plan writing;
  - durations as `12.3s`, `12.3 s` and `44 s`;
  - money as `$0.44`, `$0.4363`, and `0.1007` with no dollar sign.

Captures: `tmp/hermes-roko-v8/design/terminal/before/`. The proposal: `tmp/hermes-roko-v8/design/terminal/README.md`.

## Why it matters

Goal tooling. Round v8 redesigns what `roko run`, `roko plan …`, `roko status` and `roko diagnose` print (items
3–5). They need one style first: one colour decision, one set of colour roles from roko's design system
(`tmp/hermes-roko-v8/design/roko-design-system.md`), one glyph set, one set of number formats. It must degrade
cleanly in pipes, in CI and under NO_COLOR.

## Where

- `crates/roko-cli/src/main.rs::ColorMode` (the `--color` flag).
- `crates/roko-cli/src/output_format.rs` (clack-style symbols, always ANSI).
- `crates/roko-cli/src/inline/terminal.rs::should_use_inline`.
- `crates/roko-cli/src/tui/theme.rs::Theme::from_env`.

## Plan

1. Add `crates/roko-cli/src/term_style.rs`, one module for:
   - the colour decision: `--color always|never`, else `NO_COLOR` (non-empty) off, else `CLICOLOR_FORCE` on, else
     `CLICOLOR=0` off, else `TERM=dumb` off, else the stream is a TTY;
   - the live decision: redraw only when stderr is a TTY, TERM isn't dumb and `CI` is unset;
   - colour roles: mark, live, pass, fail, you, learn, ink, dim, faint. Each is truecolor, falling back to xterm-256
     and then the 16 ANSI colours by `COLORTERM`/`TERM`;
   - glyphs: ✓ ✗ ↻ · ◆ and a braille spinner, ASCII under TERM=dumb;
   - formatters: duration `5.3s` / `4m 05s` / `1h 02m`; money `$0.44` (`$0.0042` below a cent); plurals; width
     (the terminal's, clamped to 60–120) and truncation with `…`.
2. Resolve `--color` once in `main` and hand it to the module (a process-wide setting), so every command honours
   it.
3. `output_format.rs` uses the module's colour decision, so `roko run` stops writing ANSI into pipes and under
   NO_COLOR.

## Done when

- `roko run "<prompt>" | cat` and `NO_COLOR=1 roko run "<prompt>"` print no escape codes.
- The module's tests pass: the colour decision for each env combination, the live decision, the formatters.
- The `[[verify]]` passes.

## Notes

- `--json` output never changes.
- The TUI keeps its own theme (`tui/theme.rs`).
