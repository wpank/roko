+++
id = "gap-daa246"
kind = "gap"
title = "Whitepaper figures README still says pandoc converts the SVGs, which build.sh now does"
status = "open"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-wp-publish's report on gap-8117a8, branch work/gap-8117a8)"
anchors = ["docs/whitepaper/figures/README.md", "docs/whitepaper/build.sh"]
lane = "docs"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-8117a8", "gap-d1d92c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'pandoc converts each SVG' docs/whitepaper/figures/README.md && grep -qF 'build.sh' docs/whitepaper/figures/README.md"
+++

## Problem

The "Rendering" section of `docs/whitepaper/figures/README.md` says: "For the PDF, pandoc converts each SVG through librsvg (`rsvg-convert`)", and that "the build should turn off pandoc's implicit figures". Since gap-8117a8, `docs/whitepaper/build.sh` converts each SVG to PDF with `rsvg-convert` itself, has its Lua filter point the LaTeX at those PDFs, and already runs pandoc with `--from markdown-implicit_figures`.

## Why it matters

Whitepaper v1 (epic spec-ce1484): this README is where someone editing a figure learns how it reaches the PDF.

## Where

- `docs/whitepaper/figures/README.md`, "Rendering" (:44-52).
- `docs/whitepaper/build.sh`: the "Figures" block (about :64-69) and the pandoc call (about :314).

## Current state

At BASE the README describes the plan from before `build.sh` existed, and never names `build.sh`.

## Plan

Rewrite the paragraph: `build.sh` converts `figures/*.svg` with `rsvg-convert --format pdf` in its work directory, the Lua filter points the LaTeX at the PDFs, and implicit figures are off. Name the tools `build.sh` checks for (pandoc, tectonic, rsvg-convert).

## Done when

- [x] The "Rendering" section describes what `build.sh` does.
- [x] The `[[verify]]` command passes.
