+++
id = "gap-38e925"
kind = "gap"
title = "Research paper: the ~190 lower-ranked cited keys have not been checked against their full texts"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "M"
subsystem = ["tmp/cybernetic-harness/paper"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's gap-f052aa result: top 95 keys ranked, 81 read, 9 minor corrections)"
anchors = ["tmp/cybernetic-harness/paper/sections/", "tmp/cybernetic-harness/paper/bibliography/content-audit.json"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-f052aa", "gap-6e0a95"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('tmp/cybernetic-harness/paper/bibliography/content-audit.json'));assert len(d['works'])>=200\""
+++

## Problem

gap-f052aa ranked the research paper's 288 cited keys by how much the sections say about them and read the top 95 (81 in full, 14 paywalled or books). The other ~190 keys, cited in one or two sentences each, have not been read against the claims made about them.

## Why it matters

The paper goes to review after the pilot GO. gap-f052aa's rate was low (11%, all minor), so this is a pre-submission sweep, not a blocker.

## Plan

1. Read the remaining open-access keys against their citing sentences, in rank order; list paywalled ones as unread.
2. Correct unsupported claims in place, keep paperlint clean, and regenerate CLAIMS-EVIDENCE if a claim row changes.
3. Extend `content-audit.json` with every checked key.

## Done when

- [ ] At least 200 keys are recorded in the audit (read or listed as unreadable).
- [ ] The `[[verify]]` command passes.
