# Phase Map

The 735 batches are partitioned into 5 phases that mirror
`tmp/solutions/roko/impl/00-MASTER-PLAN.md`.

| Phase | Goal | Prefixes | Estimated batches |
|---|---|---|---|
| 0 | Critical fixes — nothing crashes, nothing leaks, config is respected | STAB, CONF, LERN (Track B subset), UX__ (Track D subset) | ~120 |
| 1 | Architecture convergence — one dispatch path, one gate pipeline, decompose orchestrate.rs | ORCH, DISP, GATE, DEBT, XCUT, CONF (remainder) | ~190 |
| 2 | UX workflow + features — Context Packs, prompt model-aware, parallel execution | UX__, PROM, EVAL (Track A), RNNR | ~150 |
| 3 | Innovations + learning loops — agent memory, multi-agent, performance | INNO, LERN (Track A), PERF, OBS_ | ~190 |
| 4 | GTM + ecosystem — integrations, benchmarks, marketplace, safety | GTM_, ACPM, SAFE, TEST | ~110 |

## Wave structure

Each phase has internal waves that respect inter-batch dependencies. The
generator emits `deps = [...]` in `batches.toml`; the parallel-template
runner schedules waves automatically.

### Phase 0 quick wins (Wave 0, 46 batches)

These have effort ≤ 2h and zero dependencies. They can all dispatch in the
first wave. Listed in `ISSUE-TRACKER.md` under "Quick Wins".

### Phase 1 critical path

```
ORCH_01 (extract OrchestrateCtx)        # blocks ORCH_02..07
  └─> ORCH_02 (build runner v2 ctx)
        └─> ORCH_03 (FeatureExtractor)
              └─> ORCH_04, ORCH_05, ORCH_06
DEBT_* (12.01..12.10)                    # parallel with ORCH
DISP_01..06 (CascadeRouter wiring)       # blocked only by CONF_01
GATE_03..07 (GateService wiring)         # blocked only by CONF_02
```

### Phase 2 critical path

```
UX_01..10 (Context Pack data model)      # depends on Phase 1 dispatch unification
  └─> UX_11..20 (synthesis, architecture)
        └─> UX_21..30 (decomposition)
PROM_01..10 (ContextTier wiring)         # depends on Phase 1 dispatch
EVAL_01..12 (roko-eval foundation)       # depends on Phase 1 gate convergence
RNNR_01..15 (worktree, wave, cumulative) # depends on ORCH_*
```

### Phase 3+ details

See `tmp/solutions/roko/impl/00-MASTER-PLAN.md` §3 for full per-phase
goals and `tmp/solutions/roko/tasks/00-MASTER-INDEX.md` Track tables for
parallelism boundaries. Don't duplicate that here — link out.
