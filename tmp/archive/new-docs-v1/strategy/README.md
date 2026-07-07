# Strategy

> Forward-looking plans, sequencing decisions, and open proposals.
> If you are looking for what Roko *is* today, see [`reference/`](../reference/README.md).
> If you are looking for *how we got here*, see [`analysis/`](../analysis/README.md).
> This folder is about *where we are going and in what order*.

**Last reviewed**: 2026-04-19

---

## Contents

| Subfolder | What it covers |
|---|---|
| [`refactor-phases/`](refactor-phases/README.md) | The four-phase plan (A → D) to migrate Roko from ad hoc transport to the two-medium, two-fabric kernel model |
| [`roadmap/`](roadmap/README.md) | Quarter-by-quarter delivery milestones from Q1 Foundation through Q5–Q6 Phase-2 optionality |
| [`refinements/`](refinements/README.md) | Index of all `tmp/refinements/*.md` proposals (REF02–REF34); status, owner, and migration state |

---

## How these folders relate

```
refactor-phases/   ←──── answers HOW the codebase transitions
       │
       │  Phase A-C land in Q1; Phase D is Q5-Q6
       ▼
roadmap/           ←──── answers WHEN each capability becomes real
       │
       │  Each milestone pulls on one or more refinements
       ▼
refinements/       ←──── answers WHAT each design proposal contains
```

The refactor phases describe the *mechanics of the kernel cutover*. The roadmap describes the *delivery sequence of capabilities visible to users*. The refinements are the *source proposals* that the roadmap and phase plan pull from.

---

## Suggested reading order

| Goal | Path |
|---|---|
| Understand the migration strategy | `refactor-phases/00-overview.md` → `refactor-phases/01-phase-docs-alignment.md` → `refactor-phases/README.md` |
| Understand the delivery plan | `roadmap/00-overview.md` → `roadmap/milestone-q1-foundation.md` → `roadmap/dependencies.md` |
| Find a specific refinement | `refinements/README.md` — search by REF number or topic |
| See current status | `refactor-phases/current-status.md`, `roadmap/current-quarter.md` |

---

## See also

- [`reference/`](../reference/README.md) — the specification (what exists today)
- [`analysis/readiness-audit/`](../analysis/readiness-audit/README.md) — the current-state scorecard this roadmap sequences
- [`analysis/synergy-map/`](../analysis/synergy-map/README.md) — why the dependency order in the roadmap compounds
- [`GLOSSARY.md`](../GLOSSARY.md) — canonical vocabulary used across all strategy docs
