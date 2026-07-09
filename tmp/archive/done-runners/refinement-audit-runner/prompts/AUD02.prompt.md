# Batch AUD02: Narrow foundation concepts (REF01-09) to target-state in architecture docs

**Audit refs**: 01-foundation-audit.md, 02-foundation-learning.md, 05-refinement-matrix.md
(REF01-09 rows). Applies the audit's "keep diagnosis, narrow prescription" verdict to
`docs/00-architecture/`.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/01-foundation-audit.md` (full file -- verdict per REF)
- `tmp/refinements-audit/02-foundation-learning.md` (foundation section: what to keep, what to narrow)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF01-09 rows)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 10 Things to Defer" section)
- `docs/00-architecture/02b-pulse-ephemeral-event.md`
- `docs/00-architecture/07b-bus-transport-fabric.md`
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md`
- `docs/00-architecture/09-universal-cognitive-loop.md`
- `docs/00-architecture/12-five-layer-taxonomy.md`
- `docs/00-architecture/15-crate-map.md`
- `docs/00-architecture/01-naming-and-glossary.md` (first 100 lines for orientation)

## Task

The refinements-runner wrote Pulse, Datum, Bus-as-trait, generalized operators,
and the seven-step loop into the architecture docs as if they were current
architecture. The audit found that Pulse has 0 lines of code, Datum has 0 lines,
Bus exists only as a concrete `EventBus<E>` struct (not a kernel trait), and
the operator generalization is premature. Mark these concepts as
**target-state** rather than **current architecture**.

## Current state (evidence)

The audit found these specific problems in the architecture docs:

1. **`02b-pulse-ephemeral-event.md`** describes Pulse as a current kernel type.
   Reality: no `Pulse` struct exists anywhere in the codebase. Zero lines.

2. **`07b-bus-transport-fabric.md`** describes `trait Bus` as a kernel trait.
   Reality: `EventBus<E>` exists as a concrete struct in `roko-runtime/src/event_bus.rs`
   with 2 event types (PlanRevision, PrdPublished). It is NOT a kernel trait.

3. **`08-scorer-gate-router-composer-policy.md`** describes `Datum` as the
   universal input type for operators. Reality: no `Datum` type exists. Operators
   take `&[Engram]` today.

4. **`12-five-layer-taxonomy.md`** line 221 says `roko-core, roko-bus, roko-hdc,
   and roko-spi are the only kernel-tier crates` (present tense). Reality:
   `roko-bus`, `roko-hdc`, and `roko-spi` do not exist as crates.

5. **`15-crate-map.md`** describes target crates (`roko-bus`, `roko-hdc`,
   `roko-spi`, `roko-defaults`, `roko-tools`, `roko-compose-core`,
   `roko-templates`). The crate-map doc itself is honest about the gap, but
   other docs reference it without qualification.

6. **`09-universal-cognitive-loop.md`** describes the seven-step loop with
   co-equal PERSIST/BROADCAST as current architecture. Reality: the loop exists
   as `loop_tick` in `roko-core` but BROADCAST (Bus-mediated) is not wired.

## Implementation

### 1. Add target-state markers to Pulse doc

In `docs/00-architecture/02b-pulse-ephemeral-event.md`:
- Add a prominent callout near the top (after the abstract) stating:
  `> **Implementation status**: Target-state design. No `Pulse` type exists in
  > the codebase yet. The current transport mechanism is `EventBus<RokoEvent>`
  > in `roko-runtime/src/event_bus.rs` with 2 event types.`
- Do NOT delete the Pulse design content. It is useful as a target spec.

### 2. Add target-state markers to Bus doc

In `docs/00-architecture/07b-bus-transport-fabric.md`:
- Add a prominent callout near the top:
  `> **Implementation status**: Target-state design. The current transport is
  > `EventBus<E>` (a concrete generic struct in roko-runtime, not a kernel
  > trait). It has 2 event types: PlanRevision and PrdPublished. The trait-based
  > Bus described here is the target architecture.`

### 3. Mark Datum as target-state in operator doc

In `docs/00-architecture/08-scorer-gate-router-composer-policy.md`:
- Where `Datum` is introduced as the operator input type, add a note:
  `> **Note**: `Datum` is a target-state abstraction. Current operators accept
  > `&[Engram]` directly. The medium-polymorphic `Datum` wrapper is planned
  > but not yet implemented.`

### 4. Fix five-layer taxonomy crate claims

In `docs/00-architecture/12-five-layer-taxonomy.md`:
- Change "roko-core, roko-bus, roko-hdc, and roko-spi **are** the only
  kernel-tier crates" to "roko-core is the current kernel-tier crate;
  roko-bus, roko-hdc, and roko-spi are **target** kernel crates proposed by
  REF20"
- Apply the same treatment to any other present-tense claims about crates that
  do not exist

### 5. Verify crate-map qualification

In `docs/00-architecture/15-crate-map.md`:
- Check that target crates are marked as "Target" or "Proposed" consistently
- If any target crates are described in present tense ("roko-bus provides..."),
  change to future tense or add a "(target)" qualifier

### 6. Mark BROADCAST step as target-state in loop doc

In `docs/00-architecture/09-universal-cognitive-loop.md`:
- Where the BROADCAST step is described as co-equal with PERSIST, add a note:
  `> **Implementation status**: PERSIST is wired (FileSubstrate). BROADCAST
  > (Bus-mediated event emission) exists only for PlanRevision and
  > PrdPublished events. Full Bus-mediated broadcast is target-state.`

## Write scope

- `docs/00-architecture/02b-pulse-ephemeral-event.md`
- `docs/00-architecture/07b-bus-transport-fabric.md`
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md`
- `docs/00-architecture/09-universal-cognitive-loop.md`
- `docs/00-architecture/12-five-layer-taxonomy.md`
- `docs/00-architecture/15-crate-map.md`

## Rules

1. **Mark, do not delete.** The target-state designs are valuable specs. Add
   implementation-status callouts; do not remove design content.
2. **Use consistent callout format.** Every target-state marker should be a
   blockquote starting with `> **Implementation status**:` followed by what
   exists today and what is target-state.
3. **Distinguish three levels**: "Shipping" (wired, tested, CLI-accessible),
   "Built" (code exists, not fully wired), "Target-state" (described in docs,
   no code).
4. **Do not touch the glossary.** Glossary fixes are AUD06's scope.
5. **Do not fix Signal->Engram references.** That is AUD07's scope.
6. **Do not change the architecture narrative.** The two-medium, two-fabric
   story is the intended target architecture. Just qualify what is current vs.
   what is planned.

## Done when

- Every architecture doc that describes Pulse, Datum, Bus-as-trait, or target
  crates has a visible implementation-status callout
- `12-five-layer-taxonomy.md` no longer claims target crates exist in present
  tense
- No architecture doc was deleted or had its design content removed
- The distinction between "current" and "target-state" is clear to a reader
  who opens any single doc in the set
- Final message lists every file edited and the specific callouts added
