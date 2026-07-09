# Milestone: Q4 — Scale, Safety, and Domains

> Roko becomes domain-shaped, auditable, and multi-tenant enough for serious team use.

**Target**: Q4 (full-team estimate)
**Status**: Planned
**Owner**: Domain lead (domain profiles); Platform engineer (safety, deployment hardening)
**Prerequisites**: [Q3 — Ecosystem and UX](milestone-q3-ecosystem-ux.md) complete
**Unlocks**: [Q5–Q6 — Phase 2 Optionality](milestone-q5q6-phase2-optionality.md) (optional)
**Roadmap quarter risk**: Multi-tenant auth and isolation
**Last reviewed**: 2026-04-19

---

## Headline

Roko becomes domain-shaped, auditable, and multi-tenant enough for serious team workflows.

---

## Quarter demo

A team selects a domain profile, runs an auditable plan with custody records, and watches c-factor, cost, and safety surfaces update in real time.

---

## Tracks

| Track | Scope | REFs |
|---|---|---|
| Domain profiles | First domain profiles with `TypedContext`, starter heuristics, and gates | REF25 |
| Safety spine | Custody, sandbox tiers, provenance, taint, and audit tooling coherent across surfaces | REF32 |
| Replication ledger expansion | Claim tracking and evidence export from starter kit into broader runtime use | REF16 |
| Deployment hardening | Multi-tenant shape, identity integration, Helm-grade packaging | REF24 |
| c-factor actuation | Policy reacts to degraded collective process, not only measures it | REF13 |

---

## Deliverables

- [ ] At least two domain profiles shipped with `TypedContext`, starter heuristics, and domain-specific gates
- [ ] Safety spine: custody records, taint tracking, provenance, and audit tooling coherent across CLI, TUI, web
- [ ] Replication ledger: claim tracking operational beyond the starter kit
- [ ] Multi-tenant deployment shape with identity integration
- [ ] Helm-grade deployment packaging
- [ ] c-factor actuation: policy changes observable when collective intelligence degrades

---

## Exit criteria

- [ ] Domain profile checkpoint: "Are domain profiles producing surprising replication-ledger findings?" → assessment
- [ ] Multi-tenant: at least two isolated tenants can run concurrently without cross-contamination
- [ ] Safety audit: `roko audit` produces a machine-readable custody record for any completed plan
- [ ] c-factor actuation: a degraded collective process observable signal triggers a measurable policy change

---

## Current status

Not started. Awaiting Q3 completion.

---

## Risk

**Multi-tenant auth and isolation**: identity integration and tenant isolation are the hardest engineering problems in Q4. Mitigation: isolate multi-tenancy behind a feature flag; ship domain profiles and safety spine first even if multi-tenancy slips.

---

## REF alignment

| REF | Scope |
|---|---|
| REF13 | Collective intelligence c-factor (actuation component) |
| REF16 | Research-to-runtime / replication ledger expansion |
| REF24 | Deployment UX / hardening |
| REF25 | Domain-specific agents |
| REF31 | Synergy integration framing |
| REF32 | Safety sandbox and provenance (safety spine) |

---

## See also

- [`strategy/roadmap/milestone-q3-ecosystem-ux.md`](milestone-q3-ecosystem-ux.md) — prerequisite
- [`strategy/roadmap/milestone-q5q6-phase2-optionality.md`](milestone-q5q6-phase2-optionality.md) — optional next step
- [`strategy/roadmap/dependencies.md`](dependencies.md)
