+++
id = "gap-129862"
kind = "gap"
title = "G0's SC2 spot check needs a person's labels on 20 pilot outputs; Pilot A's worksheet holds 18 and waits for 2 VS = 1 outputs from Pilot B"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "gap-c33709 worker (claude-agent), 2026-10-19: Pilot A's G0 page, benchmarks/viabilitybench/reports/pilot_a/go-no-go.md"
discovered_from = "gap-c33709"
anchors = ["benchmarks/viabilitybench/reports/pilot_a/sc2.toml", "benchmarks/viabilitybench/analysis/gates.py::_sc2"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-327242"], blocks = [], related = ["gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json, sys; checks = {c['id']: c['verdict'] for c in json.load(open('benchmarks/viabilitybench/reports/pilot_a/g0.json'))['checks']}; sys.exit(0 if checks.get('sc2_spot_check') == 'pass' else 1)\""
+++

## Problem

G0's SC2 spot check (S08 SC2; S09 §4.7) is not evaluated. A person has to read 20 pilot outputs, 10 the census
labelled VS = 1 and 10 it labelled VS = 0, and at least 19 must agree with the census. Pilot A could not supply
that. It has only 8 outputs with VS = 1 (3 cheap_direct gpt-oss-120b, 5 fd_api gpt-5.4), and no agent may stand
in for the person.

A worksheet is ready: `benchmarks/viabilitybench/reports/pilot_a/sc2.toml` (`vb.sc2/1`). It holds 18 items: all 8
VS = 1 outputs, and 10 completed cheap_direct outputs with VS = 0 (5 F1, 5 F4), drawn by a keyed hash. The items are
in a keyed order, and the sheet shows no census label, so the reader judges blind. Its header says where each
item's spec, diff, final tree and transcript are, in `~/.roko-bench/viability/PILOT-A/<run>/`.

## Why it matters

- **G0 needs it:** it cannot be GO without this check.
- **It is the only check of the census labels against a human's judgement on real model output** (S08 SC2).
  Pilot A's 37 false greens (visible check passed, truth suite failed) make it the more important: they are
  the census's claim that the hidden suites catch what the visible tests miss.

## Where

- `benchmarks/viabilitybench/reports/pilot_a/sc2.toml`: the worksheet.
- `benchmarks/viabilitybench/analysis/gates.py`, `_sc2`: what the check needs. That is at least 20 items, 10 of
  each census label, each with a 0/1 `human_label`, and at least 95% agreeing.
- The same file's `[direct_loop]` table holds S08 decision 3's choice, which `_direct_loop` reads.

## Current state

The worksheet holds 18 items with no labels yet. Two more VS = 1 outputs are needed, which Pilot B
(gap-327242) will supply: its `fd_claude` and `roko_fixed` runs are on the same 20 instances.

## Plan

1. **After Pilot B, add 2 of its VS = 1 outputs to the worksheet**, drawn by the same keyed hash
   (`pilot-a-sc2-1`), or rebuild the sheet over both pilots. The G0 page is computed over both pilots anyway.
2. **A person labels every item:** read the spec and the diff, and set `human_label` to 1 when the output meets the
   spec's acceptance criteria, else 0. Then fill in `checked_by` and `date`.
3. **Once Pilot B's `mini-swe` block has run,** fill `[direct_loop]` with S08 decision 3's choice and its reason: the
   loop with the higher VS on gpt-oss-120b.
4. **Recompute the G0 page** with `--sc2` and both pilots' runs.

## Done when

- [ ] 20 or more items are labelled by a person, with 10 or more of each census label.
- [ ] The G0 page's SC2 check passes: the `[[verify]]` command.

## Notes

- **The labels must be a person's.** An agent may prepare the sheet but must not fill `human_label`.
- **The census label is not on the sheet,** so keep the reading blind: look at the records only afterwards.
