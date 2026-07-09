# A — PAD and Temporal Model (Docs 00, 01, 02)

Topic `09` starts from a strong shipping base.

`PadVector` is real, shared, clamped, tested, and consumed across the
stack. The main mismatch in this section is narrower: Doc 02 still
reads like the full ALMA three-layer temporal model is live, while the
shipping runtime uses a single persisted PAD state with decay.

Generated: 2026-04-18

---

## Current Read

| Area | Status | Parity note |
|------|--------|-------------|
| mortality framing removed | DONE | the runtime has no death-state concepts |
| cyclical behavioral-state framing | DONE | six cyclical states ship in code |
| Daimon as control signal | DONE | routing, prompting, and retrieval all consume affect surfaces |
| PAD primitive and math | DONE | clamping, decay helpers, similarity, and tests all ship |
| octant / Plutchik labels | PARTIAL | useful explanatory mapping, not a live runtime type |
| ALMA three-layer model | FRONTIER | target-state only; not the shipping temporal contract |

---

## A.01-A.04 — The core framing already matches reality

These foundation claims are already strong and should stay strong:

- mortality framing is gone from the active runtime
- behavioral states are cyclical, not terminal
- Daimon is a real control signal, not cosmetic presentation
- `PadVector` is the live primitive and lives in `roko-core`

This is not a missing subsystem. It is a mature kernel surface with
cross-crate consumers.

---

## A.05 — Octants ship as explanatory labels, not as a runtime enum

**Status**: PARTIAL

Doc 01 still makes the eight PAD sign combinations easy to read as a
live `AffectOctant`-style runtime surface. The shipping code does not
expose that type.

Parity stance:

- keep the octant table as an informational mapping from PAD sign triples,
- do not describe it as a live enum,
- do not let Plutchik-adjacent labels overshadow the actual runtime types:
  `PadVector`, `BehavioralState`, `DaimonPolicy`, `EmotionalTag`.

This is a wording correction, not a runtime gap that blocks the affect
path.

---

## A.06-A.07 — The live temporal behavior is simple and real

Two details matter here:

- PAD similarity is a real helper with neutral fallback behavior
- decay is live and central to the runtime

The shipping temporal contract is:

- a single persisted PAD state,
- confidence on the same state,
- exponential decay over time,
- recomputed behavioral state after decay/appraisal.

Doc wording should make one nuance explicit:

- PAD decays toward neutral origin
- confidence decays toward its midpoint

That is the live behavior later docs should build on.

---

## A.08-A.09 — ALMA layering is the real frontier

**Status**: FRONTIER

Doc 02 remains the sharpest design/runtime mismatch in topic `09`.

What ships today:

- one persisted affect state
- one decay regime
- no separate mood layer
- no separate personality layer
- no layer-to-layer interaction math

What Doc 02 should say after parity:

- the ALMA framing is useful design motivation,
- the shipping runtime currently compresses that into a single-layer PAD model,
- true emotion/mood/personality separation is target-state work.

This is the right place for an explicit `Design — Phase 2+` or
`target-state` banner.

---

## A.10 — Domain-agnostic PAD is a shipping strength

**Status**: DONE

The core affect vocabulary remains domain-agnostic:

- PAD is shared,
- appraisal surfaces feed the same primitive,
- downstream projection changes by domain,
- the affect primitive itself does not fragment by domain.

That strengthens the "mostly shipping" story because the live primitive
already sits at the kernel boundary.

---

## Section Outcome

| Status | Count |
|--------|-------|
| DONE | 7 |
| PARTIAL | 1 |
| FRONTIER | 2 |

The section should read as:

- PAD is already live and reliable,
- octants are explanatory, not structural,
- ALMA layering is the main frontier edge.

---

## Edit Guidance

- strengthen the PadVector shipping story
- mark octant/Plutchik language as informational
- mark ALMA layering as frontier
- do not imply new temporal layers already exist
