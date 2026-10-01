+++
id = "gap-785a4f"
kind = "gap"
title = "harness-engineering.md builds on six harness principles credited to Meta-Harness, which the paper's abstract doesn't name"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs/v3"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = ["docs/v3/depth/05-agent/harness-engineering.md", "tools/docs_integrity/citation_errata.json"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-212b75", "gap-b23ebd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qi 'six harness principles' docs/v3/depth/05-agent/harness-engineering.md || grep -iE 'six harness principles' docs/v3/depth/05-agent/harness-engineering.md | grep -qE '§|[Ss]ection [0-9]'"
+++

## Problem

`docs/v3/depth/05-agent/harness-engineering.md` builds its section on "six harness principles" credited to Meta-Harness (Lee et al., 2026, "Meta-Harness: End-to-End Optimization of Model Harnesses"; :3, :9, :23). The paper's abstract describes an outer-loop search over harness code and names no principles (wk-rp-cite). Whether the principles come from the paper needs a reading of the full text.

## Why it matters

Release: if the principles aren't the paper's, the docs credit it with claims it doesn't make, which is the kind of error gap-212b75 removed elsewhere. p3.

## Plan

1. Read the full paper. If it states the principles, cite the section. If not, reattribute them (to roko's own synthesis) or rewrite the section.
2. `citation_errata.json` already has a Meta-Harness entry, and `--prose` passes, so it doesn't catch this claim. Add the adjudication to the entry, so the checker enforces it from now on.

## Done when

- [ ] The section attributes only what the paper says.
- [ ] The `[[verify]]` command passes: the phrase is gone, or each mention cites the paper's section.
