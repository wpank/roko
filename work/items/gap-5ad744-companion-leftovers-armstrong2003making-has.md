+++
id = "gap-5ad744"
kind = "gap"
title = "Companion leftovers: armstrong2003making has no institution, ledger_classify_commits.py misreads --json OUT, and E10 has 28 words of headroom"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-companion-tidy's report)"
anchors = ["tmp/cybernetic-harness/companion-audit/references.bib", "tmp/cybernetic-harness/companion-audit/telemetry/scripts/ledger_classify_commits.py", "tmp/cybernetic-harness/companion-audit/E10-DRAFT.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-4ec886", "gap-cb86e4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/@[a-z]+\\{armstrong2003making,/,/^\\}/' tmp/cybernetic-harness/companion-audit/references.bib | grep -qE 'school|institution' && grep -q 'argparse' tmp/cybernetic-harness/companion-audit/telemetry/scripts/ledger_classify_commits.py && python3 tools/paperlint.py --budget 0.97 tmp/cybernetic-harness/companion-audit/E10-DRAFT.md"
+++

## Problem

Three companion leftovers:

- **`armstrong2003making`** (`references.bib`) is a `@misc` with `howpublished = {Doctoral dissertation}` and no institution. It is Joe Armstrong's PhD thesis (Royal Institute of Technology, KTH, Stockholm, 2003), so it should be a `@phdthesis` with `school`.
- **`ledger_classify_commits.py`** (`telemetry/scripts/`) takes its positional arguments as every argument that doesn't start with `--` (:31) and reads `--json`'s value separately (:38). With `--json OUT` and no REPO argument, OUT is taken as the repo.
- **E10-DRAFT.md** is at 7,672 of its 7,700-word budget. E13's marker, when filled, will push it over.

## Why it matters

Paper, companion and TL;DR (epic spec-f8d196): a wrong entry type, a tool that misreads its arguments, and a draft that will fail its budget as soon as E13 lands.

## Where

The anchors, all untracked under `tmp/cybernetic-harness/companion-audit/`. Edit them in place.

## Plan

1. Make the entry `@phdthesis{armstrong2003making, …, school = {Royal Institute of Technology (KTH), Stockholm}}`.
2. Parse the script's arguments with `argparse`: a required REPO positional and `--json OUT`.
3. Make room for E13 in E10: trim at least the words E13's fill needs (the verify asks for 3% headroom, about 230 words), or record a larger budget in the header, with the reason.

## Done when

- [ ] The entry has its institution, the script refuses `--json OUT` without a REPO, and E10 has room for E13.
- [ ] The `[[verify]]` command passes. Adjust its 0.97 once E13's fill size is known.
