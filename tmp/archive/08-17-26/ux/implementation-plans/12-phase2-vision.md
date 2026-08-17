# 12 — Phase 2 Vision (Parking Lot)

> **Source plan**: `tmp/ux/ux-followup/08-phase-2-vision.md` items 49-54.
>
> **Status as of 2026-05-01**: All six items remain P2 / strategic. None
> are blocked or unblocked by the recent post-PR-13 work. The catalogue
> has confirmed they "are aligned with CLAUDE.md 'Self-hosting workflow'
> priorities (Phase 1 must close before Phase 2 starts)."
>
> **Effort**: Multi-week, **parked**.
>
> **Risk**: High if started prematurely. These items have wide blast
> radius (chain protocol changes, new contracts, full-TUI rewrites,
> public-cloud security boundaries) and require Phase 1 to be fully
> green first.

---

## What this plan is

This is a **parking document**, not an active implementation plan.
It captures (a) the entry conditions each Phase 2 item requires before
it can start, (b) the open design questions that must be resolved
before scoping, (c) the failure modes that make premature start
expensive. It is intentionally short on step-by-step instructions —
those will be written when the entry conditions clear.

A fresh agent picking this folder up should **not** start any of these
items unless explicitly directed.

## Why park instead of plan

Each Phase 2 item is a multi-week scope. Two have unresolved design
decisions (Items 49 / 50 — golem chain witness, on-chain semantics).
Two cross product boundaries (Items 53 / 54 — public-cloud auth,
plugin ABI). Two are aspirational TUI features (Item 52) that compete
with maintenance.

The followup catalogue already files these as P2. Promoting any of
them to active without first closing Phase 1 (plans `01`-`11` in this
folder) creates partial-work-in-flight that blocks the people working
on Phase 1.

---

## Entry conditions per item

These are the prerequisites a future agent must verify before starting
any Phase 2 item.

### Item 49 — Roko-golem chain-witness

> "Decide chain-witness semantics: what's being witnessed, where is
> the chain of evidence persisted, who consumes it."

**Entry conditions**:
- Plans `01` (mirage extraction) and `04` (chain discovery) merged.
- A written design doc at `docs/v2/CHAIN-WITNESS.md` answering:
  1. *What event* triggers a witness? (Gate verdict? Episode? Plan
     revision?)
  2. *What is persisted* on-chain? (Hash + URI? Full payload? Bloom
     filter?)
  3. *Who consumes* the witness? (A new contract? An existing
     ValidationRegistry caller? Off-chain analytics?)
  4. *What is the cost model*? (Gas per witness, frequency, batching
     strategy.)
- Resolution of `tmp/ux/06-open-questions.md` Q9 (auth model for
  cross-tenant chain-witness reads).

**Anti-pattern**: don't start by writing `crates/roko-golem/`. Start
by writing the design doc. The crate already exists as a skeleton
(see `bardo-backup/tmp/agent-chain/`); fleshing it out without a doc
will recreate the same orphan.

### Item 50 — Roko-chain primitives (write surface)

> "Implement once golem is specced."

**Entry conditions**:
- Item 49 design doc green-lit by both eng + product.
- A clear ABI for the chain-witness contract (`contracts/src/Witness.sol`).
- An impedance-matched read path (already shipped in `roko-chain`
  via plan `04`).

This is purely an implementation deliverable; once item 49 is specced,
this is ~2 weeks of focused work.

### Item 51 — Roko-dreams full cycle

> "Drain recent episodes, cluster via HDC, compress into playbooks."

**Entry conditions**:
- Plan `07` (feature gating) merged so `roko-dreams` can be promoted
  out of phase 2 cleanly when ready.
- Plan `02` (knowledge backends) merged so playbook sinks have a real
  destination.
- A heuristic budget: how many episodes per drain? How often? CPU
  ceiling?
- Decision: is consolidation an offline batch (cron) or an online
  trickle (background task)?

**Anti-pattern**: don't run dreams against an unbounded episode log.
Plan `05` item 73 caps the in-memory tail; dreams must read disk
chunks (file-system, not memory).

### Item 52 — Full-Mori TUI features (TOML editor, PRD editor)

> "Mori has additional editor modes (TOML live-edit, PRD in-terminal
> edit) not covered by any T-batch."

**Entry conditions**:
- Plan `05` (TUI event parity) merged. The polling story has to be
  closed before adding new editor surfaces.
- Decision: do we want in-TUI editors at all? Many users prefer
  `$EDITOR` plus auto-reload (already implicit via `fs_watch`).
- Spec doc covering: TOML schema validation in-editor (live), PRD
  multi-line edit with markdown preview, save-on-Enter vs explicit
  commit semantics.

**Anti-pattern**: don't reinvent `helix` or `nano`. If the value-add
is TOML schema validation, the TUI can shell out to `$EDITOR` then
validate on save.

### Item 53 — HTTP server Phase 2 (auth, multi-tenant, public cloud)

> "Add an `AuthLayer` on the axum router, JWT-based, tenant-aware.
> Document a public-cloud deployment mode."

**Entry conditions**:
- Security review of the existing single-operator surface
  (`roko-serve` has ~85 routes; we'd inherit them all into a
  multi-tenant model).
- Decision: federated identity (OAuth via GitHub, etc.) vs
  self-hosted (issue our own JWTs)?
- Operations story: how does an operator rotate a tenant key? How is
  tenant data partitioned in `.roko/`?
- Compliance story: what data does a multi-tenant deploy hold and
  where (US, EU, etc.)?

This is the highest-blast-radius item. Premature start risks
shipping insecure defaults. Don't begin without explicit product +
security sign-off.

### Item 54 — `roko-plugin` extensibility surface

> "Decide fate: delete, document as public ABI, or fold into another
> crate."

**Entry conditions** (pre-decision):
- Run plan `07` step 4 audit to determine current state.
- Decide fate.

If "document as public ABI", the entry conditions to *implement* the
ABI are:
- A public crate consumer outside our workspace willing to be the
  pilot.
- A semver guarantee story.
- A test suite that runs the ABI through its public surface, not
  internals.

---

## Cross-item open questions (no Phase 2 item starts before these resolve)

| Q | Question | Owner |
|---|----------|-------|
| 1 | Do we want a single chain (e.g. Roko's own L2) or multi-chain witness? | wp + product |
| 2 | Is `roko-serve` going to be public-cloud-deployable, ever? | wp + ops |
| 3 | What's the long-term split between roko-agent-server and roko-serve? Do they merge in Phase 2? | wp |
| 4 | Does `roko-dreams` produce *human-readable* playbooks or *opaque* embeddings? | learning team |
| 5 | Is the TUI's role to be the operator console *forever*, or do we replace with a web UI? | sdb + product |

Park these in `tmp/ux/06-open-questions.md` as new entries (or note
that they belong here).

---

## What this folder *does* commit to

Even though Phase 2 is parked, three small Phase-2-adjacent things
are still in scope of *other* plans:

- `roko-chain` is an active dependency for plans `02` and `04`. Treat
  it as Phase 1, not Phase 2.
- `roko-dreams` and `roko-daimon` are Phase 2 today, gated by plan `07`.
- `roko-golem` is Phase 2; do *not* gate it differently from the rest.

When the time comes to upgrade an item from this parking document,
the agent's first move should be to:

1. Re-read the entry conditions above.
2. Verify they're met (open Slack thread, link evidence).
3. Write a focused implementation plan in `tmp/ux/implementation-plans/`
   replacing this parking entry with a real document.
4. Update `00-INDEX.md` to point at it.

---

## Anti-patterns to avoid (general for Phase 2 promotion)

- **Don't start any Phase 2 item before Phase 1 is fully green.**
  Plans `01`-`11` in this folder cover Phase 1.
- **Don't promote a Phase 2 item by deleting its `Phase 2+` row in
  CLAUDE.md.** Status is downstream of work, not upstream. Update
  CLAUDE.md only when the work merges.
- **Don't write a Phase 2 plan as a single mega-doc.** When a Phase 2
  item activates, decompose into multiple ~1-week tracks per the
  pattern of plans `01`-`11`.
- **Don't conflate "phase 2 vision" with "phase 2 contracts".** Some
  items (e.g. `IdentityRegistry::updateAgentCard` in plan `04`) are
  tactical contract changes that *look* like Phase 2 but ship in
  Phase 1.
- **Don't park a Phase 2 item silently.** Anyone considering
  starting it should find this document first. Hence the explicit
  parking-document framing.

## Done when

This document does not have a "done" state. It is a parking document.
Items are removed from this list (and replaced with full plans) when
their entry conditions clear.

For now: confirm the file is linked from `tmp/ux/implementation-plans/00-INDEX.md`
and that `tmp/ux/ux-followup/08-phase-2-vision.md` cross-references
this parking lot at the top.
