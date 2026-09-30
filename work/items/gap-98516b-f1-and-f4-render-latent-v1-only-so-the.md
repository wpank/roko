+++
id = "gap-98516b"
kind = "gap"
title = "F1 and F4 render latent v1 only, so the convention_flip disturbance is refused"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families", "benchmarks/viabilitybench/driver"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench's report)"
anchors = ["benchmarks/viabilitybench/driver/disturb.py", "benchmarks/viabilitybench/families/f1_pyconv/gen.py", "benchmarks/viabilitybench/families/f4_kvtool/gen.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-15bb83"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_convention_flip_renders_a_v2_latent' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_convention_flip_renders_a_v2_latent -q"
+++

## Problem

The `convention_flip` disturbance renders the tasks it covers with `gen.py --latent <params.latent>`, v2 by default (`driver/disturb.py:27`, :65, :138). Both families build only latent v1: F4's `--latent` accepts only `v1` (`families/f4_kvtool/gen.py:288`), and F1's help says "only v1 is built" (`families/f1_pyconv/gen.py:585`). So the disturbance is refused.

## Why it matters

Pilot benchmark (epic spec-567e52): H6 can't measure how the arms cope with a changed convention. gap-15bb83 built the hooks, and this is one it couldn't enable. p3.

## Where

The families' latent handling, and `disturb.py`.

## Plan

1. Design and build latent v2 for F1 and F4 (a changed convention the spec states and the hidden suite checks), with the verifier CI covering both latents.
2. Add `test_convention_flip_renders_a_v2_latent`.

## Done when

- [ ] `convention_flip` runs on F1 and F4.
- [ ] The `[[verify]]` command passes.
