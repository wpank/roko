# Gaps Summary — Post-Audit Posture

## Main Parity Problems After The Audit

### 1. Current Vs Planned Was Still Blurry

The earlier pass often described target-state architecture as if it already existed. This refresh
must keep three categories distinct:

- shipped code
- partial runtime wiring
- future design

### 2. Several Baseline Facts Drifted

- `roko-serve` is not a scaffold stub; it is wired
- the TUI is not a text-only placeholder; it is wired
- the corrected workspace baseline is 36 workspace members and 322,088 Rust LOC
- the live event reality is not a broad Pulse fabric; it is exactly two live RokoEvent variants

### 3. Zero-Code Concepts Were Promoted Too Far

These must stay explicit future work:

- `Pulse`
- `Datum`
- `Demurrage`
- `Worldview`
- `Custody`

### 4. Meta Docs Overstated Runtime Reality

The highest rewrite pressure is in docs `30-35`:

- synergy matrix
- readiness/meta scorecards
- test strategy scale
- phased refactor plan
- consolidated roadmap

These should remain planning/reference material, not direct claims about current architecture
parity.

Treat those chapters as planning artifacts with dependency ordering, not as a live moat or staffing
plan.

## What This Batch Can Fix Directly

- wording discipline
- stale counts and status labels
- Engram-centered terminology
- explicit `planned` / `deferred` markers
- source-index usefulness as a verification aid

## What This Batch Must Not Pretend To Fix

- missing runtime wiring in `crates/`
- speculative type systems
- multi-quarter implementation programs
- feature work that belongs to later parity topics
- architecture claims that need new code before they become true
