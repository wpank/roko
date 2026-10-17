+++
id = "gap-d2c478"
kind = "gap"
title = "The research paper's figures print their labels at 5.5 to 6.9 pt, below the 8 pt its review checklist asks for"
status = "done"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["tmp/cybernetic-harness/paper"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "3295008f2"
source = "paper-review-2026-10-16 (gap-aed6f1)"
discovered_from = "gap-aed6f1"
anchors = ["tmp/cybernetic-harness/paper/figures/f1-nested-loops.svg", "tmp/cybernetic-harness/paper/OUTLINE.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-aed6f1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c 'import re,glob,sys; fs=sorted(glob.glob(\"tmp/cybernetic-harness/paper/figures/f*.svg\")); low=[(f,round(s,1)) for f in fs for t in [open(f).read()] for w in [float(re.search(r\"viewBox=\\\"[-0-9.]+ [-0-9.]+ ([0-9.]+)\",t).group(1))] for s in [float(x)*468/w for x in re.findall(r\"font-size[=:]\\s*\\\"?([0-9.]+)\",t)] if s<8]; print(low); sys.exit(1 if low or not fs else 0)'"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T20:17:40Z"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:37Z"
forced = false
evidence = "Raised every label font-size in f1-f5 from 13-16 to 19-22 units (viewBox widths unchanged: f1 1080, f2-f5 1100), printing 8.08-9.97pt at the 468pt text width (was 5.5-6.9pt). Re-flowed each figure for the bigger type: taller boxes, wider line spacing, rewrapped long lines, and widened a few boxes that were packed tight (f1's verdict-rail box, f2's 5-across Surfaces row and Records column, f3's 6-box main row plus its 'ways back' labels, f4's Delivery/Run-supervisor boxes, f5's four audit-level boxes and the two loop-level inset boxes). No label wording changed, only line breaks and geometry. Verified three ways: (1) the item's verify command (python3 -c '...') exits 0 with an empty low-font list across all 5 SVGs; (2) each figure rendered standalone with rsvg-convert and inspected for overlaps, fixing several found this way (f1's clock label colliding with a long loop name; f3's climb/retry feedback labels colliding; f4's conflict-retry label crossing an arrow and a stray blank line; f5's gap-label/box collisions); (3) built the actual paper PDF via tools/build.sh (pandoc+tectonic+rsvg-convert, all available) and inspected the real pages (3, 10, 16, 18, 26) at 200-600 DPI, which caught 3 overlaps invisible in the isolated renders: f2's 'Records / durable, attributable' subtitle overflowing its box, f5's false-green-rate/sets-audit-rate cross-link labels overlapping the Bounded-controller box border, and f5's 'keyed-hash holdouts;' overflowing the Loop-audit box; all fixed and reconfirmed at 600 DPI crops. Pre-edit figures (already carrying gap-aed6f1's content fixes) archived to tmp/cybernetic-harness/archive/2026-10-16-figures/paper/figures/ before any change, per the item's plan."
+++

## Problem

The research paper's review checklist (`tmp/cybernetic-harness/paper/REVIEW-SKELETON.md` §7) asks that figure text be
at least 8 pt at the printed width. `paper/tools/build.sh` sets every figure to the full text width (468 pt: letter
paper with 1-inch margins), and the five SVGs are 1,080–1,100 units wide with font sizes of 13–16 units, so every label
prints at 5.5–6.9 pt (f1 5.6–6.9, f2 5.5–6.4, f3 and f4 5.5–6.0, f5 5.5–6.4). Legibility in greyscale has not been
checked either.

## Why it matters

Goal whitepaper. The figures carry the paper's structure (the nested loops, the architecture, the attempt and plan loops,
the audit level); below 7 pt they are hard to read on paper and in the arXiv PDF. The independent review of the paper
(gap-aed6f1, 2026-10-16) left this checklist item open.

## Where

- `tmp/cybernetic-harness/paper/figures/f1-nested-loops.svg` … `f5-audit-level.svg` (untracked).
- `tmp/cybernetic-harness/paper/tools/build.sh`: the figure filter that sets `width = "100%"`, and `LINE_PT` (468).

## Current state

Labels fit their boxes at the current sizes; the review changed some labels in f1–f4 and checked the fit by rendering
each figure with `rsvg-convert`.

## Plan

1. Before changing a figure, copy it into `tmp/cybernetic-harness/archive/<date>-figures/` (the paper has no history).
2. Raise the type to at least 19 units in a 1,100-unit-wide viewBox (8 pt at 468 pt), and re-flow each figure; where
   labels no longer fit, shorten them rather than shrinking the type.
3. Render each figure with `rsvg-convert` and check the PDF from `build.sh` at 100% zoom; check a greyscale render for
   contrast.
4. Keep every label's wording: it matches the text's terms.

## Done when

The `[[verify]]` command passes: every font size in f1–f5 prints at 8 pt or more at the 468 pt text width.

## Notes

The verify command assumes the figures stay at the full text width. Publication itself (authors, licence, arXiv) is
Will's decision.
