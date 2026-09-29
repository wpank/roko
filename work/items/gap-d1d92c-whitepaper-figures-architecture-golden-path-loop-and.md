+++
id = "gap-d1d92c"
kind = "gap"
title = "Whitepaper figures: architecture, golden-path loop and status matrix"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper", "tools/status_matrix"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d12ab8a79"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/paper/FIGURES-TABLES.md (F1, T1)"
anchors = ["docs/whitepaper/figures", "tools/status_matrix.py", "docs/whitepaper/03-architecture.md", "docs/whitepaper/04-golden-path.md", "docs/whitepaper/09-status-and-roadmap.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-4161ea", "gap-ac4646"], blocks = [], related = ["gap-35a614", "gap-c19902", "gap-8117a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/figures/fig1-architecture.svg && test -f docs/whitepaper/figures/fig2-golden-path.svg && test -f docs/whitepaper/figures/fig3-status-matrix.svg && grep -q 'figures/fig1-architecture.svg' docs/whitepaper/03-architecture.md && grep -q 'figures/fig2-golden-path.svg' docs/whitepaper/04-golden-path.md && grep -q 'figures/fig3-status-matrix.svg' docs/whitepaper/09-status-and-roadmap.md"

[closed]
at = 2026-09-29
commit = "d12ab8a79"
by = "commit trailer"
evidence = "docs/whitepaper/figures/ holds the three SVGs: Figures 1 and 2 are maintained by hand, with the sections' text diagrams beside them as .txt, and Figure 3 is rendered from mechanisms.toml by tools/status_matrix.py. §3, §4 and §9 include them with alt text. Proven by the item's verify; status_matrix.py --check (Figure 3 current, and the hand-drawn marks and pins agree with the matrix at a17d4dadd); test_status_matrix.py, 26 pass; paperlint --strict on §3, §4 and §9, clean."
+++

## Problem

§3 and §4 will carry text drafts of Figures 1 and 2 (gap-4161ea, gap-ac4646), and nothing draws the status matrix.
The whitepaper needs three final figures, in one consistent style, that render both on GitHub and in the PDF.

## Why it matters

The three figures carry the thesis at a glance:
- **Figure 1:** the architecture and control stack.
- **Figure 2:** the golden-path loop, with each step marked by its status.
- **Figure 3:** how much of the design runs today.

## Where

- **New `docs/whitepaper/figures/`:** `fig1-architecture.svg`, `fig2-golden-path.svg` and `fig3-status-matrix.svg`.
- **`tools/status_matrix.py`** (from gap-35a614) gains an `--svg` option for Figure 3.
- **§3 and §4** replace their text drafts with the images. Figure 3 goes into §9.

## Current state

Checked at `41c7ffbd6` on this machine.
- **Installed:** `pandoc` and `tectonic`.
- **Not installed:** `rsvg-convert`, Graphviz and the Mermaid CLI. Pandoc needs `rsvg-convert` to put SVG figures into
  a LaTeX PDF.
- **The research draft's F1 and T1** exist only as specifications. F1 is a VSM map of the harness; T1 summarises the
  mechanisms.

## Plan

1. **Draw Figures 1 and 2 as hand-written SVG** from the section drafts. Show status by label and pattern as well as
   colour, so the figures read in greyscale and in dark mode.
2. **Generate Figure 3 from `docs/whitepaper/data/mechanisms.toml`** with `status_matrix.py --svg`, so it cannot drift
   from the appendix. Extend `--check` to cover it.
3. **Add alt text and captions** in the sections. Each caption cites the matrix's commit.
4. **Settle the PDF path with gap-8117a8:** install `librsvg`, or commit PDF exports next to the SVGs.

## Done when

- [x] The three SVGs exist and the sections reference them.
- [x] Figure 3 regenerates identically.
- [x] The `[[verify]]` command passes.

## Notes

- **`paperlint --strict` on §3, §4 and §9 must still pass after the swap.** Its link rule checks image paths.
- **Installing tools is the author's call.**
- Lane `paper`; no hot files.
- **PDF path (plan step 4):** librsvg, as Will decided on 2026-09-29 (gap-8117a8's notes). `figures/README.md` asks `build.sh` to read the sections with `-f markdown-implicit_figures`: each section prints its caption under the image, so pandoc's implicit figures would print it twice.
