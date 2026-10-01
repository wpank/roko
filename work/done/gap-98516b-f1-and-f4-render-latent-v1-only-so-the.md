+++
id = "gap-98516b"
kind = "gap"
title = "F1 and F4 render latent v1 only, so the convention_flip disturbance is refused"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families", "benchmarks/viabilitybench/driver"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "292586654"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench's report)"
anchors = ["benchmarks/viabilitybench/driver/disturb.py", "benchmarks/viabilitybench/families/f1_pyconv/gen.py", "benchmarks/viabilitybench/families/f4_kvtool/gen.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-15bb83"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_convention_flip_renders_a_v2_latent' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_convention_flip_renders_a_v2_latent -q"

[closed]
at = 2026-09-30
commit = "292586654"
by = "wk-bench-fix1"
evidence = "292586654: F1 and F4 render latent v2 (gen.py --latent v2). F1 v2 registers E-1234 codes in app/registry.toml with messages 'E-1234: ...'; F4 v2 is kvtool 2.0, which writes by default but asks for confirmation and waits (hangs) unless --yes, with --dry-run previews. Truth suites, gaming detectors, solutions, help/docs/exemplars follow the latent; v1 instances are byte-identical to BASE's (15 cells per family: tree, manifest minus versions, spec). Verify passes (test_convention_flip_renders_a_v2_latent: a vb run over F1 and F4 with convention_flip from position 3 records v1 then v2 in task.latent_version, perturbations_active, and each manifest states its latent's convention). verify_verifiers.py judges every latent a family builds (--latents): --families f1,f4 --seeds 2 is 40/40 green, and --latents v2 --seeds 10 is 100/100 green. Full bench suite: 346 passed, 2 skipped."
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
