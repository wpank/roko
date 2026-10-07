+++
id = "spec-c4fd9f"
kind = "spec"
title = "S06 names gates.max_rung as B3's ceiling; the shipped ceiling is verify.max_floor"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/homeostasis"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-16 follow-up reports 2026-10-04 (gap-86286e, gate 16b)"
discovered_from = "gap-86286e (closed; own closing evidence names this as a queued spec fix)"
anchors = ["tmp/cybernetic-harness/specs/S06-ultrastable-controller.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

S06's knob table names `gates.max_rung` as B3's ceiling, but that key indexes gate rungs (the
compile/lint/test pipeline, `roko-gate`'s rung-numbered pipeline), not S05's V0-V4 verify-depth
scale, and does nothing to bound B3 on the Graph engine. `tmp/cybernetic-harness/specs/S06-ultrastable-controller.md:156`
describes B3 as "a verify-depth floor request on S05 §4.6's scale: V0 → V1 (+clippy, tamper
diff) → V2 (+clean full-test re-run) → V3 (+hidden tests) → V4 (+mutation, LLM review); ceiling
`gates.max_rung`," and line 170's Bounds section repeats it: "`gates.max_rung` (B3 ceiling)."

`gates.max_rung` (`crates/roko-core/src/config/gates.rs::GatesConfig::max_rung`) is a real,
working config key — but it caps which numbered *gate* rung (compile=0, lint=1, ...; see
`production_service.rs`'s own test comment "Only Compile (0) and Lint (1)") the pipeline runs
at all (`roko-gate/src/production_service.rs`, `gate_service.rs`), an entirely different,
coarser axis than S05's V0-V4 verify-depth scale (clippy/tamper-diff, full-test-re-run,
hidden-tests, mutation/LLM-review). It has no reference anywhere in `graph_task_dispatch.rs` or
`runner/gate_dispatch.rs` — the Graph engine's own per-task gate entry point — so setting
`gates.max_rung` has no effect on B3's actual ceiling on the live path.

The real ceiling was implemented, under `gap-86286e` (closed at gate 16b), as
`verify.max_floor` in `.roko/policy/viability.toml`'s homeostasis policy (default `V4`):
`crates/roko-learn/src/homeostasis/policy.rs::InclusionParams`/`ViabilityPolicy` (`max_floor:
VerifyDepth`, lines 95, 390, 446), which directly bounds `new.extra_rungs` (B3) the same way
`p_max` bounds B7 (the code's own comment, line 876: "gap-86286e: S5's `verify.max_floor`
bounds B3 as `p_max` bounds B7"). `gap-86286e`'s own title makes the contrast explicit: "B3
(verify-depth floor) has no S5 ceiling field, unlike its B7/B8 siblings" — i.e. B7/B8 already
had correctly-documented ceilings in S06; B3's entry just names the wrong key.

## Why it matters

Goal: cybernetic, S06 accuracy (M1 controller knob table). An operator reading S06 to configure
B3's ceiling would set `gates.max_rung`, which silently does nothing for this purpose (it still
does its own, unrelated thing to the gate pipeline) — the controller's actual verify-depth
ceiling would stay at its `verify.max_floor` default (V4) regardless, with no error or warning
pointing at the mismatch.

## Where

- `tmp/cybernetic-harness/specs/S06-ultrastable-controller.md:156,170` (the lines to amend).
- `crates/roko-learn/src/homeostasis/policy.rs::ViabilityPolicy` (`verify.max_floor`, the real
  ceiling; read-only reference).
- `crates/roko-core/src/config/gates.rs::GatesConfig::max_rung` (the real, unrelated gate-rung
  cap; read-only reference, to make the distinction concrete in the amended text).

## Plan

1. Replace S06 line 156's "ceiling `gates.max_rung`" with "ceiling `verify.max_floor`
   (`.roko/policy/viability.toml`, default V4)."
2. Replace line 170's Bounds-section entry `gates.max_rung` (B3 ceiling) with `verify.max_floor`
   (B3 ceiling), keeping `gates.max_rung`'s own, separate entry (if S06 documents the gate-rung
   pipeline elsewhere) clearly distinct from this one.

## Done when

- S06 names `verify.max_floor` as B3's ceiling, matching the shipped code and B7/B8's own
  already-correct entries.

## Notes

- 2026-10-04 (wave-16 follow-up, gap-86286e, gate 16b): confirmed at main HEAD `70fc09313`.
  `gap-86286e`'s own closing evidence names this exact follow-up: "S06's `gates.max_rung`
  wording is queued as a spec fix." Filed as `kind = "spec"`, editing
  `tmp/cybernetic-harness/specs/` only (not `docs/whitepaper/*` or
  `tmp/cybernetic-harness/paper/*`, both held for the paper-rewrite session).
