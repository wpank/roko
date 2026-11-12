+++
id = "gap-8117a8"
kind = "gap"
title = "Publish the whitepaper: PDF build, release tag and venue"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "815f512e2"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§3 E1, item 17)"
anchors = ["docs/whitepaper/build.sh", "docs/whitepaper/README.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-8d2c79", "dec-2cd76a"], blocks = [], related = ["spec-ae5f94", "gap-d1d92c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x docs/whitepaper/build.sh && git rev-parse -q --verify refs/tags/whitepaper-v1 && grep -q '^Published: whitepaper-v1' docs/whitepaper/README.md"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T10:33:13Z"
commit = "815f512e2"
by = "coordinator 2026-10-20"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Published 2026-10-20 on Will's go (decision 2026-10-18 #5, re-confirmed 2026-10-20 after PR #87 merged as dd11b6700): annotated tag whitepaper-v1 on dd11b6700, GitHub release https://github.com/wpank/roko/releases/tag/whitepaper-v1 with roko-whitepaper-v1.pdf (231,622 bytes, sha256 9652c1543299b2ebe22f835b345f5eecff2b2035d3606c49af17a2b43c9ffd4d), built by docs/whitepaper/build.sh from 5f7ff82ab (whitepaper tree identical to dd11b6700). docs/whitepaper/README.md carries the Published line (815f512e2); paperlint --strict --require-status reviewed: 11 files clean."
+++

## Problem

Once reviewed, the whitepaper has to become something people can read and cite. That takes three things:
- a PDF built from the tracked sources;
- a tag that fixes the exact text, and the commit its status tags refer to;
- a venue.

## Why it matters

This completes the `whitepaper` goal. The tag lets any reader check the status tags and the numbers against the code
they describe.

## Where

- **New `docs/whitepaper/build.sh`:** the PDF build.
- **`docs/whitepaper/README.md`:** a `Published:` line.
- **The tag** `whitepaper-v1`, and the venue that dec-2cd76a chose.

## Current state

Checked at `41c7ffbd6`: no build script exists.
- **Installed here:** `pandoc` and `tectonic`.
- **Not installed:** `rsvg-convert`, which pandoc needs to put SVG figures into a LaTeX PDF.
- **The repo is not public yet** (goal `release`; epic spec-ae5f94).

## Plan

1. **`build.sh`:**
   - pandoc with citeproc and `references.bib`;
   - the sections in order, then the appendix;
   - the PDF through tectonic, with the figures converted as gap-d1d92c settled.

   Write the output outside the tracked tree, or under a path the author agrees to ignore.
2. **Tag** the reviewed commit `whitepaper-v1` as an annotated tag, with the PDF's sha256 in the message.
3. **Publish** at the venue from dec-2cd76a. The default is a GitHub release with the PDF attached, once the repo is
   public.
4. **Add** `Published: whitepaper-v1 · <date> · <sha256> · <link>` to the README.

## Done when

- [ ] `build.sh` produces the PDF from a clean checkout of the tag.
- [ ] The `[[verify]]` command passes.

## Notes

- **Build done (2026-09-29, wk-wp-publish):** `docs/whitepaper/build.sh` at `35e06c980` builds the PDF from a
  `git archive` export. Result: 30 letter pages; §0–§10 on pages 1–17, the references on 17–20, and the appendix
  on 10 landscape pages (21–30).
  - **sha256** `a8962c6fbadedba99f4400fa750c2f3ac14a5704fc2fd48980486ca0d635b26e`, the same from a checkout and an
    export of `35e06c980` (pandoc 3.9, tectonic 0.15.0, librsvg 2.63.2).
  - **Checks:** the three figures are embedded as vector art, all 46 references resolve, no status header prints,
    and the build fails on an unknown citation key.
  - **Reproducible:** the PDF's dates are the commit time, so the tagged commit's own build is the one to record.
  - **Length:** the body runs past the planned 10–14 pages, at 7,123 counted words plus footnotes, figures and
    tables. 0.75in margins save only one page, so meeting the plan would take cuts to the text.
  - **Still open:** the `whitepaper-v1` tag, the release and the `Published:` line wait for Will. The `[[verify]]`
    needs the tag.
- **Tagging, pushing and releasing are git and GitHub actions.** Ask the author before each one.
- **Don't commit the PDF;** attach it to the release instead. Any change to `.gitignore` needs the author's OK.
- **Waits** for gap-8d2c79, dec-2cd76a and the release goal. Lane `paper`; no hot files.
- **Decided 2026-09-29 (Will):** publish in the repo with a generated PDF, with no arXiv for now. Install librsvg (`brew install librsvg`, with Will's OK) so pandoc can embed SVG figures. Tagging still needs Will's approval.
- **From gap-d1d92c (2026-09-29):** `build.sh` must call pandoc with `-f markdown-implicit_figures`, or the PDF prints each figure caption twice (see `docs/whitepaper/figures/README.md`).
- **Before tagging (coordinator, 2026-09-29):** the matrix re-pin (gap-fcea53, merged 366a7ccbe) changed tags and text in §1, §3, §4, §5, §6 and §9 after the review (REVIEW.md is at a17d4dadd). Do a short independent read of those sections first. The PDF rebuilt cleanly after the re-pin (sha256 6c7fd3f6acce…).

## Decision (2026-10-18, Will)

Publish the whitepaper once main is pushed and the release PR's CI is green: tag whitepaper-v1 (annotated, the PDF's sha256 in the message) and attach the PDF to a GitHub release. The research paper waits for the pilots (decision 9539 asked again then).
