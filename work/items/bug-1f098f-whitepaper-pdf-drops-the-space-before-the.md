+++
id = "bug-1f098f"
kind = "bug"
title = "Whitepaper PDF drops the space before the status tag in figure 2's step-9 chips"
status = "open"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "wk-wp-figures report on gap-daa246 (2026-09-30)"
anchors = ["docs/whitepaper/figures/fig2-golden-path.svg"]
lane = "docs"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-daa246", "gap-08d9b2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '<tspan[^>]*> · ' docs/whitepaper/figures/fig2-golden-path.svg && python3 tools/status_matrix.py --check"
+++

## Problem

In `docs/whitepaper/figures/fig2-golden-path.svg`, step 9 has two mini-chips whose status tag sits in a `<tspan>` that starts with a space: `merge queue<tspan …> · ORPHANED</tspan>` and `whole-plan gate<tspan …> · MISSING</tspan>`. librsvg (`rsvg-convert`, which `build.sh` uses to make the PDF figures) drops that leading space, so the PDF reads "merge queue· ORPHANED" and "whole-plan gate· MISSING". Browsers keep the space.

## Why it matters

The PDF is the whitepaper's published form (epic spec-ce1484). The glitch is small but visible in the golden-path figure.

## Where

The two `<tspan class="note">` elements in step 9 of `fig2-golden-path.svg` (the IS2 and IS3 marks, near lines 96–97).

## Current state

Found while checking gap-daa246's build: `build.sh` renders 32 pages, and every other label is correct.

## Plan

1. Write the leading space as `&#160;` (or move it before the `<tspan>`) in both chips. Keep the `data-row` and `data-tag` attributes unchanged.
2. Rebuild the PDF with `docs/whitepaper/build.sh` and check both chips.

## Done when

- [x] Both chips render with the space in the PDF.
- [x] `python3 tools/status_matrix.py --check` still passes.
- [x] The `[[verify]]` command passes.

## Notes

- **`&#160;` does not work.** librsvg 2.63.2 drops a no-break space at the start of a `<tspan>` as well as a plain space, so the PDF still read "merge queue· ORPHANED". The fix moves the space before the `<tspan>`, where librsvg keeps it (Figure 1's "records under `.roko/`" already relies on that).
