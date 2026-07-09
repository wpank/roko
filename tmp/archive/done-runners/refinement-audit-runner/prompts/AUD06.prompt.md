# Batch AUD06: Fix integrator docs (REF31-35) — synergy, glossary, safety, roadmap

**Audit refs**: 05-integrator-audit.md (full file), 05-refinement-matrix.md (REF31-35 rows),
07-doc-quality-audit.md (sections 2.3, 2.4, 5). Applies the audit's "integrate code, not
plans" verdict to safety, synergy, glossary, and roadmap docs.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/05-integrator-audit.md` (full file -- REF31-35 verdicts)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF31-35 rows)
- `tmp/refinements-audit/06-codebase-reality-check.md` (section 6: Safety Layer Reality)
- `tmp/refinements-audit/07-doc-quality-audit.md` (sections 2.3, 2.4, 5)
- `docs/00-architecture/34-synergy-integration-map.md`
- `docs/00-architecture/35-consolidated-roadmap.md`
- `docs/00-architecture/24-cross-section-integration-map.md`
- `docs/00-architecture/01-naming-and-glossary.md`
- `docs/11-safety/INDEX.md`
- `docs/11-safety/00-defense-in-depth.md`
- `docs/11-safety/03-taint-tracking.md`
- `docs/11-safety/01-capability-tokens.md`

## Task

The integrator docs (REF31-35) try to stitch the previous 30 refinements into
a coherent whole. The audit found: the synergy matrix describes interactions
between things that mostly do not exist (3 of 10 primitives are real), the
glossary marks EventBus as "retired" despite it being the only live transport
code, the safety chapter ignores the existing AgentContract/Warrant system,
and the roadmap assumes 5-7 engineers. Fix these.

## Current state (evidence)

1. **Synergy matrix (REF31)**: 10 "load-bearing primitives" listed. Audit
   found: Engram (YES), Pulse (NO), Bus trait (NO -- EventBus<E> struct exists),
   Substrate (YES), HDC (PARTIAL), Demurrage (NO), Heuristics (MINIMAL),
   c-factor (PARTIAL), Replication ledger (NO), Plugin SPI (NO). Score: 3 of 10
   exist meaningfully.

2. **Safety (REF32)**: The existing safety layer at `roko-agent/src/safety/`
   has SafetyLayer, BashPolicy, GitPolicy, NetworkPolicy, PathPolicy,
   ScrubPolicy, RateLimiter, AgentContract, AgentWarrant, Capability enum --
   4,465 lines across 10 files. REF32 proposes replacing it without
   acknowledging it. The existing contract system is not mentioned.

3. **Glossary (REF34)**: Marks EventBus as "retired" despite it being the only
   live transport code. Defines terms for things that do not exist (Datum,
   Custody, Claim, Paper, Fleet, Graduation). Audit verdict: **REWRITE** --
   split into exists/planned.

4. **Roadmap (REF35)**: Assumes 5-7 engineers and quarterly milestones. Reality:
   1 developer + AI agents. Audit verdict: **REWRITE** -- calibrate for actual
   team.

## Implementation

### 1. Strip synergy matrix of unbuilt features

In `docs/00-architecture/34-synergy-integration-map.md`:
- Add an implementation-status callout at the top listing which primitives
  actually exist:
  `> **Implementation status**: Of the 10 primitives in this matrix, 3 exist
  > fully (Engram, Substrate, EventBus), 2 partially (HDC fingerprint,
  > c-factor), and 5 are target-state only (Pulse, Bus trait, Demurrage,
  > Heuristic commons, Replication ledger, Plugin SPI). Synergy cells
  > involving unbuilt primitives are aspirational.`
- In the matrix itself, mark each cell with a status indicator:
  - Cells where both primitives exist: leave as-is
  - Cells where one or both primitives do not exist: add `[target-state]` tag
- Keep the "Non-Synergies Worth Naming" section intact -- the audit called it
  the best part

In `docs/00-architecture/24-cross-section-integration-map.md`:
- Apply the same treatment: mark integration points involving unbuilt
  primitives as target-state

### 2. Fix glossary: split into exists vs. planned

In `docs/00-architecture/01-naming-and-glossary.md`:
- **Do NOT delete entries.** Instead, add a status column or tag to the A-Z
  glossary:
  - `[shipping]` -- term corresponds to a working type/module in the codebase
  - `[built]` -- code exists but not fully wired
  - `[planned]` -- target-state design, no code
  - `[retired]` -- deliberately replaced
- Specific fixes:
  - `EventBus`: Remove "retired" tag. It is the ONLY live transport code.
    Mark as `[shipping]` with a note: "The current transport mechanism.
    Target-state: evolve into Bus trait."
  - `Datum`: Mark as `[planned]`
  - `Custody`: Mark as `[planned]`
  - `Claim`: Mark as `[planned]`
  - `Paper`: Mark as `[planned]`
  - `Fleet`: Mark as `[planned]`
  - `Graduation`: Mark as `[planned]`
  - `Demurrage`: Mark as `[planned]`
  - `Worldview`: Mark as `[planned]`
  - `Falsifier`: Mark as `[planned]`
  - `Pulse`: Mark as `[planned]` (0 lines of code)
  - `Bus` (trait): Mark as `[planned]` with note that `EventBus<E>` struct
    is the current implementation
  - Keep all terms that match real code as `[shipping]` or `[built]`

### 3. Acknowledge existing safety system

In `docs/11-safety/INDEX.md`:
- Ensure the overview acknowledges the EXISTING safety system:
  `The safety layer at roko-agent/src/safety/ is **Shipping**: SafetyLayer
  (5-policy chain), BashPolicy, GitPolicy, NetworkPolicy, PathPolicy,
  ScrubPolicy, RateLimiter, AgentContract (with Invariant/GovernanceRule),
  AgentWarrant (OCaps-style), and Capability enum (Tool, ReadPath, WritePath,
  Exec, Network). 4,465 lines across 10 files, plus role-specific YAML
  contracts.`

In `docs/11-safety/00-defense-in-depth.md`:
- If it proposes `authorize(principal, action, target, ctx)` without
  acknowledging the existing `check_pre_execution()` chain, add a note:
  `> **Note**: The current implementation uses `SafetyLayer::check_pre_execution()`
  > which chains 5 policy checks. The `authorize()` signature described here
  > is a target-state API that would replace or wrap the current chain.`

In `docs/11-safety/03-taint-tracking.md`:
- Note that taint currently exists as `Provenance.tainted: bool` and the
  rich `Taint` enum is target-state

In `docs/11-safety/01-capability-tokens.md`:
- Acknowledge that `AgentWarrant` and `Capability` enum already exist and work

### 4. Qualify roadmap for actual team size

In `docs/00-architecture/35-consolidated-roadmap.md`:
- Add a callout:
  `> **Team calibration note**: This roadmap was drafted assuming 5-7
  > engineers. The actual project has 1 developer + AI agents. Timeline
  > estimates should be multiplied accordingly, and the number of
  > simultaneous work streams should be reduced.`
- Where "quarterly" milestones are specified, add: "estimated for a
  full team; single-developer timelines will differ"

## Write scope

- `docs/00-architecture/34-synergy-integration-map.md`
- `docs/00-architecture/35-consolidated-roadmap.md`
- `docs/00-architecture/24-cross-section-integration-map.md`
- `docs/00-architecture/01-naming-and-glossary.md`
- `docs/11-safety/INDEX.md`
- `docs/11-safety/00-defense-in-depth.md`
- `docs/11-safety/03-taint-tracking.md`
- `docs/11-safety/01-capability-tokens.md`

## Rules

1. **Mark, do not delete.** The synergy matrix and roadmap are useful as
   planning tools. Add reality markers; do not gut them.
2. **Credit existing safety code.** The AgentContract/Warrant system is real
   and working. REF32 should build on it, not replace it.
3. **The glossary is the highest-impact fix in this batch.** Every other batch
   depends on the glossary being honest about what exists. Get this right.
4. **Do not touch learning docs** -- that is AUD03's scope.
5. **Do not touch architecture foundation docs** (02b, 07b, 08, 09) -- those
   are AUD02's scope.
6. **Do not touch interfaces/deployment docs** -- those are AUD05's scope.
7. **Do not fix Signal->Engram references** -- that is AUD07's scope.

## Done when

- Synergy matrix has status indicators on every cell involving unbuilt
  primitives
- Glossary entries for unbuilt concepts are marked `[planned]`
- EventBus is NOT marked as retired in the glossary
- Safety INDEX acknowledges the existing 4,465-line safety system
- Safety docs reference existing `check_pre_execution()`, `AgentContract`,
  `AgentWarrant`
- Roadmap has a team-calibration note
- No content was deleted
- Final message lists: (a) how many glossary terms changed status, (b) how many
  synergy cells were marked target-state, (c) the safety acknowledgments added
