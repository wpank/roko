+++
id = "gap-8117a8"
kind = "gap"
title = "Publish the whitepaper: PDF build, release tag and venue"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§3 E1, item 17)"
anchors = ["docs/whitepaper/build.sh", "docs/whitepaper/README.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-8d2c79", "dec-2cd76a"], blocks = [], related = ["spec-ae5f94", "gap-d1d92c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x docs/whitepaper/build.sh && git rev-parse -q --verify refs/tags/whitepaper-v1 && grep -q '^Published: whitepaper-v1' docs/whitepaper/README.md"
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

- **Tagging, pushing and releasing are git and GitHub actions.** Ask the author before each one.
- **Don't commit the PDF;** attach it to the release instead. Any change to `.gitignore` needs the author's OK.
- **Waits** for gap-8d2c79, dec-2cd76a and the release goal. Lane `paper`; no hot files.
- **Decided 2026-09-29 (Will):** publish in the repo with a generated PDF, with no arXiv for now. Install librsvg (`brew install librsvg`, with Will's OK) so pandoc can embed SVG figures. Tagging still needs Will's approval.
