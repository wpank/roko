+++
id = "gap-f052aa"
kind = "gap"
title = "Research paper: claims about cited works (216 keys, related work above all) have not been checked against the papers' full text"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "L"
subsystem = ["tmp/cybernetic-harness/paper"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ea5fbe4b2"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (coordinator, after gap-11845c's 62% rate in docs/v3)"
anchors = ["tmp/cybernetic-harness/paper/sections/03a-related-work.md", "tmp/cybernetic-harness/paper/sections/03b-related-work.md", "tmp/cybernetic-harness/paper/sections/02-background.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-11845c", "gap-1f72ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('tmp/cybernetic-harness/paper/bibliography/content-audit.json'));assert len(d['works'])>=60\""

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:08:49Z"
by = "coordinator (session 7622b882)"
size = "L"
claimed_at = "2026-10-01T08:47:38Z"
forced = false
evidence = "wk-rp-cite ranked the paper s 288 cited keys (540 citing sentences, related work and background weighted x3) and read 81 of the top 95 in full (14 paywalled or books listed as unread): 72 supported, 9 corrected (11.1%, all minor), every checked number matched its source. Audit: tmp/cybernetic-harness/paper/bibliography/content-audit.json (81 works). Edits are untracked in tmp/cybernetic-harness/paper/sections; CLAIMS-EVIDENCE regenerated (432 claims, --check up to date, 21 claims tests OK); paperlint --budget 1.2 --check-identifiers clean on all 18 sections."
+++

## Problem

The research paper's sections cite 216 distinct keys. Their metadata was verified (BIB-QA, CITATION-ERRATA), but what the sections say about each work was not read against the works. Agent-written descriptions in docs/v3 were unsupported for 62% of the works read in full (gap-11845c).

## Why it matters

The research paper goes to review once the pilot gives a GO. Related-work and background claims are where reviewers look for misreadings.

## Plan

1. Rank the cited works by how much the sections say about them (related work §3a/§3b and background §2 first).
2. Read at least the top 60 in full and correct unsupported claims in place (the paper lives in tmp/, untracked; edit by absolute path).
3. Record each work and its verdict in `tmp/cybernetic-harness/paper/bibliography/content-audit.json`, with the error rate and a recommendation for the rest.

## Done when

- [ ] At least 60 of the most-described works are checked and recorded.
- [ ] The `[[verify]]` command passes.
