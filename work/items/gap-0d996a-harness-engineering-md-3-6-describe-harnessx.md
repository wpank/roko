+++
id = "gap-0d996a"
kind = "gap"
title = "harness-engineering.md §3–§6 describe HarnessX, Harness-Bench and Belief Divergence's dimensions and taxonomies without a check against the papers"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs/v3"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report on gap-785a4f, branch work/gap-785a4f at 4201615de)"
anchors = ["docs/v3/depth/05-agent/harness-engineering.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-785a4f", "gap-212b75", "gap-fc5d3d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'content-checked: §3–§6' docs/v3/depth/05-agent/harness-engineering.md && python3 tools/docs_integrity/check_citation_errata.py --prose"
+++

## Problem

gap-785a4f rewrote §1–§2 of `docs/v3/depth/05-agent/harness-engineering.md` from the full Meta-Harness paper. §3–§6 still describe three other works' content (HarnessX, Harness-Bench and the Belief Divergence paper): their dimensions, taxonomies and findings. Nobody has read those papers against the text, and the citation checker checks titles, authors and ids, not what a paper says (wk-rp-cite).

## Why it matters

Release: the docs are public, and the same section already credited Meta-Harness with principles it never states.

## Where

`docs/v3/depth/05-agent/harness-engineering.md` §3–§6, plus any docs/v1 copies, and `tools/docs_integrity/citation_errata.json` for phrases to flag.

## Current state

At 766ad6270 the citations' metadata is correct (the checker is clean), but the sections' descriptions of each work are unverified.

## Plan

1. Read each paper in full and compare the sections' claims: names of dimensions, taxonomies, numbers.
2. Rewrite what the papers don't support. Where the text is Roko's own synthesis, say so.
3. Record invented phrases in `citation_errata.json` (a `harness-engineering` note or phrase entries) so `--prose` catches them coming back.

## Done when

- [ ] Every claim §3–§6 makes about a cited work is supported by that work, with a section reference, or marked as Roko's synthesis.
- [ ] The doc carries an HTML comment `<!-- content-checked: §3–§6 against HarnessX, Harness-Bench and Belief Divergence, <date> -->` once the check is done, and the `[[verify]]` command passes.
