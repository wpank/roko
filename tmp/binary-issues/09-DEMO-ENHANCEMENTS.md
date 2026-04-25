# 09 — Demo & Showcase Enhancements

**Status**: spec
**Scope**: `demo/demo-web/`, `crates/roko-cli/src/demo_cmd.rs`

## Overview

The web demo is the first impression. These enhancements make it feel like
a product launch, not a dev tool walkthrough.

---

## Feature 9A: Guided Walkthrough Mode

Add a step-by-step guided mode where the demo narrates what's happening:

```
┌─────────────────────────────────────────────────────┐
│  Step 2 of 5: Generating Implementation Plan        │
│                                                     │
│  Roko is analyzing the PRD and creating a DAG of    │
│  tasks with dependencies. Watch the terminal...     │
│                                                     │
│  [Previous]  [Next]  [Skip to end]                  │
└─────────────────────────────────────────────────────┘
```

- Overlay narration cards that explain each step
- Auto-advance when the terminal output reaches a checkpoint
- Manual advance with Next/Previous buttons
- Keyboard: left/right arrows, space to advance

**Implementation**: JavaScript overlay system in demo.html. ~100 lines.
Narration content as JSON array of `{step, title, description, checkpoint}`.

---

## Feature 9B: Live Metrics Dashboard Pane

Add a real-time metrics pane to the demo that shows what's happening internally:

```
┌─ Metrics ─────────────────┐
│ Tokens/s:  ████░░  142    │
│ Cost:      $0.0041         │
│ Model:     haiku           │
│ Gates:     ✔✔✖ (2/3)      │
│ Context:   ███░░░  34%    │
│ Latency:   1.2s avg       │
└───────────────────────────┘
```

- Updates in real-time by scraping terminal output
- Shows progress bars for throughput and context
- Gate status icons update as gates run

**Implementation**: Parse roko output with regex, update DOM elements.
~80 lines of JavaScript.

---

## Feature 9C: Interactive Prompt Playground

Let users type their own prompts in the demo (not just presets):

```
┌─ Try Roko ────────────────────────────────────────┐
│                                                    │
│  > Type a prompt and see roko in action            │
│                                                    │
│  [implement a fibonacci function in rust]     [Go] │
│                                                    │
│  Popular prompts:                                  │
│  · "add error handling to main.rs"                 │
│  · "explain the gate pipeline"                     │
│  · "generate a PRD for X"                          │
│                                                    │
└────────────────────────────────────────────────────┘
```

- Text input that feeds into the terminal pane
- Popular prompt suggestions below
- Output streams in the terminal in real-time

**Implementation**: Connect the demo prompt bar to the terminal WebSocket.
~50 lines.

---

## Feature 9D: Before/After Code Comparison

Show the impact of roko's work with a side-by-side diff:

```
┌─ Before ──────────────────┐  ┌─ After ───────────────────┐
│ fn main() {               │  │ fn main() {               │
│     println!("hello");    │  │     let args = parse();   │ +
│                           │  │     println!("{}", args); │ ~
│ }                         │  │ }                         │
└───────────────────────────┘  └───────────────────────────┘
```

- Triggered after agent completes a code modification task
- Syntax highlighted (using highlight.js in the web demo)
- Line-level diff coloring (green = added, yellow = modified)

**Implementation**: Capture before/after file content, render with highlight.js
diff view. ~80 lines.

---

## Feature 9E: Speed Control with Visual Feedback

Enhance the existing speed control with visual feedback:

```
Speed: ◀ ■■■□□ ▶  1.5x
```

- Slider with 5 steps: 0.5x, 1x, 1.5x, 2x, 4x
- Visual blocks showing current speed
- Keyboard: +/- to adjust
- Apply to: command execution delay, typing animation speed

**Implementation**: ~30 lines of CSS + JS for the slider widget.

---

## Feature 9F: Scenario Thumbnails

Show small preview thumbnails for each scenario tab:

```
┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐
│ 1 │ │ 2 │ │ 3 │ │ 4 │ │ 5 │ │ 6 │ │ 7 │
│ ⚙ │ │ 🔨│ │ 💰│ │ 🔗│ │ 🔍│ │ 💬│ │ ⛓ │
│   │ │   │ │   │ │   │ │   │ │   │ │   │
└───┘ └───┘ └───┘ └───┘ └───┘ └───┘ └───┘
Self  Build  Cost  Prov  Expl  Chat  Chain
Host                ider  ore
```

- Each tab shows a small icon/glyph representing the scenario
- Hover shows a one-line description
- Active tab gets a rose border glow

**Implementation**: CSS grid + icon font or Unicode glyphs. ~40 lines.

---

## Priority Order

1. **9B** Live metrics — makes demo feel dynamic and real
2. **9A** Guided walkthrough — accessibility for new users
3. **9C** Interactive playground — engagement
4. **9E** Speed control — usability
5. **9D** Before/after comparison — wow factor
6. **9F** Scenario thumbnails — navigation polish
