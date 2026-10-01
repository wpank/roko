+++
id = "gap-f052aa"
kind = "gap"
title = "Research paper: claims about cited works (216 keys, related work above all) have not been checked against the papers' full text"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "L"
subsystem = ["tmp/cybernetic-harness/paper"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (coordinator, after gap-11845c's 62% rate in docs/v3)"
anchors = ["tmp/cybernetic-harness/paper/sections/03a-related-work.md", "tmp/cybernetic-harness/paper/sections/03b-related-work.md", "tmp/cybernetic-harness/paper/sections/02-background.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-11845c", "gap-1f72ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('tmp/cybernetic-harness/paper/bibliography/content-audit.json'));assert len(d['works'])>=60\""
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
