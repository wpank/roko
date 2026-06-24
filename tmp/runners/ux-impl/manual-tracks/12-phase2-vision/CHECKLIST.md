# PH — Phase 2 vision (parked)

> Source plan: `tmp/ux/implementation-plans/12-phase2-vision.md`.
> Tracker rows: ISSUE-TRACKER.md Wave PH (no rows tick — parked).

This document is **not** an active checklist. It captures the entry
conditions each Phase 2 item requires before promotion to a real
implementation plan.

---

## Why parked

Each Phase 2 item is multi-week scope with wide blast radius
(chain protocol changes, new contracts, public-cloud security
boundaries). Premature start blocks the people working on Phase 1.

The followup catalogue files these as P2 by design.

---

## PH01 — Roko-golem chain-witness

**Entry conditions before promotion**:

- [ ] Wave M (mirage extraction) merged.
- [ ] Wave CH (chain discovery) merged.
- [ ] Design doc at `docs/v2/CHAIN-WITNESS.md` covering:
  - [ ] What event triggers a witness (gate verdict / episode / plan revision)?
  - [ ] What is persisted on-chain (hash + URI / full payload / bloom filter)?
  - [ ] Who consumes the witness (new contract / existing ValidationRegistry / off-chain)?
  - [ ] Cost model (gas per witness, frequency, batching)?
- [ ] Resolution of `tmp/ux/06-open-questions.md` Q9 (auth model for
  cross-tenant witness reads).

**Anti-pattern**: do not start by writing `crates/roko-golem/`. Start by
writing the design doc.

---

## PH02 — Roko-chain primitives (write surface)

**Entry conditions**:

- [ ] PH01 design doc green-lit.
- [ ] Witness contract ABI specified (`contracts/src/Witness.sol`).
- [ ] Read path (already shipped in Wave CH) confirmed working in production.

---

## PH03 — Roko-dreams full cycle

**Entry conditions**:

- [ ] Wave FG (feature gating) merged so promotion mechanism exists.
- [ ] Wave AG (knowledge backend) merged so playbook sinks have a destination.
- [ ] Heuristic budget defined: episodes per drain, frequency, CPU cap.
- [ ] Decision: offline batch (cron) or online trickle?

---

## PH04 — Full-Mori TUI features (TOML editor, PRD editor)

**Entry conditions**:

- [ ] Wave TU (event parity) merged.
- [ ] Decision: in-TUI editors or `$EDITOR` + auto-reload?
- [ ] Spec doc covering: TOML schema validation, PRD multi-line edit,
  save-on-Enter vs explicit commit.

---

## PH05 — HTTP server Phase 2 (auth, multi-tenant, public cloud)

**Entry conditions**:

- [ ] Security review of the existing single-operator surface.
- [ ] Decision: federated identity (OAuth via GitHub etc) or self-hosted JWTs?
- [ ] Operations story: tenant key rotation, data partitioning in `.roko/`.
- [ ] Compliance story: data residency.

**Highest blast radius** of the Phase 2 set. Don't begin without
explicit product + security sign-off.

---

## PH06 — `roko-plugin` extensibility

**Entry conditions** (post-FG03 audit):

- [ ] FG03 audited and decided: keep / delete / promote.
- [ ] If promoted to public ABI: a pilot consumer outside the workspace.
- [ ] Semver guarantee story.

---

## Cross-item open questions

| Q | Question | Owner |
|---|----------|-------|
| 1 | Single chain (Roko's L2) or multi-chain witness? | wp + product |
| 2 | Public-cloud `roko-serve`, ever? | wp + ops |
| 3 | Long-term split between roko-agent-server and roko-serve? | wp |
| 4 | `roko-dreams` produces human-readable playbooks or opaque embeddings? | learning team |
| 5 | TUI as operator console forever, or replaced by web UI? | sdb + product |

Answer these before activating any Phase 2 item.

---

## Promotion procedure

When activating a parked item:

1. Re-read the entry conditions for that item.
2. Verify they're met (open Slack thread, link evidence).
3. Write a focused implementation plan in
   `tmp/ux/implementation-plans/`, replacing the parking entry.
4. Add corresponding batches to `tmp/runners/ux-impl/batches.toml` and
   prompts to `tmp/runners/ux-impl/prompts/`.
5. Update `tmp/runners/ux-impl/ISSUE-TRACKER.md`: add Wave PH rows
   for the active item, with `[ ]` status.
6. Update `tmp/runners/ux-impl/manual-tracks/12-phase2-vision/CHECKLIST.md`
   to remove the promoted item.
