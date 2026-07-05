# ⚠️ SUPERSEDED — See [MASTER-PLAN.md](../MASTER-PLAN.md) Tier 1H
>
> Content absorbed into MASTER-PLAN.md. This file retained for historical reference.

---

# 09 — TUI & Dashboard

> **Priority**: 🟡 P2 — UX improvement, not functional blocker
> **Parity sections**: §18 (Efficiency dashboard), §19 (TUI)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §18, §19

## Problem statement

Mori has a rich TUI with 26 widgets and 13 modals. Roko has a basic TUI
framework but the efficiency dashboard (6 pages) is not implemented.

## Checklist

### Efficiency dashboard (§18)

- [ ] **9.1** Health page (6 gauges: pass rate, cost/task, iterations, haiku use, prompt size, cache hit)
- [ ] **9.2** Trends page (time-series sparklines, learning velocity, regression detection)
- [ ] **9.3** Correlations page (learning-pack ↔ pass-rate, strategy comparison)
- [ ] **9.4** Parameters page (tunable knobs with impact ratings)
- [ ] **9.5** Experiments page (A/B test results, z-test, verdicts)
- [ ] **9.6** Optimizer page (learning loops with confidence bars)

### TUI improvements (§19)

- [ ] **9.7** Agent status view (per-agent live status, tokens, cost)
- [ ] **9.8** Plan view (DAG visualization, task progress)
- [ ] **9.9** Log view (filtered, searchable)
- [ ] **9.10** Config view (live config with overrides highlighted)

> Maps to checklist: §18.1-18.6, §19.1-19.26
