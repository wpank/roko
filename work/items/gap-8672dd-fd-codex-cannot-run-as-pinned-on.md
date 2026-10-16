+++
id = "gap-8672dd"
kind = "gap"
title = "fd_codex cannot run as pinned on the ChatGPT login: gpt-5.5 is refused, and the models it serves (gpt-5.6-terra, gpt-5.6-luna) have no snapshot row"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["config/prices", "benchmarks/viabilitybench/arms"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "task 6107 Codex probes, 2026-10-16 (gap-8c2535, config/prices/2026-09-28.probes.md)"
discovered_from = "gap-8c2535"
anchors = ["config/prices/2026-09-28.toml", "benchmarks/viabilitybench/arms/fd_codex.toml", "benchmarks/viabilitybench/driver/run_codex.py::price_turns"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-8c2535"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import glob,sys,tomllib; arm=tomllib.load(open('benchmarks/viabilitybench/arms/fd_codex.toml','rb'))['arm']; snap=tomllib.load(open(sorted(glob.glob('config/prices/*.toml'))[-1],'rb')); pin=arm['models_allow'][0]; sys.exit(0 if snap['id']!='prices-2026-09-28' and any(r['slug']==pin for r in snap['model']) and (arm.get('billed') or pin!='gpt-5.5') else 1)\""
+++

## Problem

The 2026-10-16 Codex cost probes (task 6107, gap-8c2535, `config/prices/2026-09-28.probes.md`) show that the
fd_codex arm cannot run as pinned on the ChatGPT login, and that the price snapshot cannot price what that login
serves:

- `codex exec --json -m gpt-5.5` (fd_codex's D2 pin) fails before any usage with 400
  `The 'gpt-5.5' model is not supported when using Codex with a ChatGPT account.`
- The login's model list (fetched by codex-cli 0.152.0 on 2026-10-16) offers gpt-5.6-terra and gpt-5.6-luna, plus
  two hidden models (gpt-reserve, codex-auto-review). It does not offer gpt-5.5, or gpt-5.4, the fallback that S08
  decision 6 names.
- `prices-2026-09-28` has no row for gpt-5.6-terra or gpt-5.6-luna, so `run_codex.price_turns` prices every run this
  login can make as unknown (U′ null). The two measured probe runs (gpt-5.6-terra at high effort and gpt-5.6-luna
  at xhigh) used tokens and have no cost.

## Why it matters

fd_codex is LOG1's "could" arm (BL1) and P1's second frontier-direct comparator from another vendor, billed $0 on
the subscription with U′ from the snapshot. As configured, every fd_codex task would fail at its first turn, and with
another model its cost coverage would be 0, failing S04's cost acceptance (`cost_coverage` 1.0 for every arm in H1).

## Where

- `config/prices/2026-09-28.toml`: immutable; a rate change or a new model goes in a new dated file with a new id.
- `benchmarks/viabilitybench/arms/fd_codex.toml`: `models_allow = ["gpt-5.5"]`, `billed = false`.
- `benchmarks/viabilitybench/driver/run_codex.py::price_turns`, which prices turns at the snapshot.

## Current state

The probe facts that the snapshot does encode hold: Codex counts `reasoning_output_tokens` inside `output_tokens`,
and Claude Code's subscription figures match the snapshot (see the probe table). Only the Codex model coverage is
missing.

## Plan

1. Ask Will to pick fd_codex's model (a D2/S08 decision 6 question): a model the ChatGPT login serves (gpt-5.6-terra
   or gpt-5.6-luna, whichever is the frontier one), or keep gpt-5.5 and bill the arm through an API key
   (`billed = true`, a funded budget line).
2. Add a new dated snapshot (`config/prices/<date>.toml`, new id) with rows for the chosen model, from the provider's
   published API prices with `source_url` and `verified`, and `reasoning_in_output = true` (the 2026-10-16 probes
   observed it for both gpt-5.6 models).
3. Point fd_codex's `models_allow` (and S09's snapshot id, if the lock has not been taken) at it, and re-run one
   fd_codex probe of the egress list before LOG1.

## Done when

- [ ] The newest dated snapshot prices fd_codex's pin, and that pin is not gpt-5.5 unless the arm is billed (the
  `[[verify]]` command).
- [ ] One live fd_codex probe on the new pin reports `turn.completed` usage and a known U′.

## Notes

- Evidence: `config/prices/2026-09-28.probes.md`; raw events (scrubbed) under
  `tmp/status-quo/2026-10-16-live/vb-results/PROBE-6107/probes-6107-20261016T1404Z/raw/`.
- Never edit `2026-09-28.toml`; it is immutable.
