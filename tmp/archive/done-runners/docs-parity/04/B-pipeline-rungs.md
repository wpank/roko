# B — Pipeline & Rungs (Docs 02, 03)

Post-audit refresh for the verification runtime path.

---

## Verdict

The verification runtime already exposes a **live 7-rung dispatch path** in the executor/plan flow.

The old parity note overstated the gap by centering the unused/secondary abstractions and understating the live production path.

---

## What Is Live

The current runtime path is:

`ExecutorAction::RunGate`
-> `run_gate_pipeline(plan_id, rung)` in `orchestrate.rs`
-> `run_gate_rung(...)`
-> `roko_gate::rung_dispatch::run_rung(...)`

Key anchors:

- `crates/roko-cli/src/orchestrate.rs:6181-6245`
- `crates/roko-cli/src/orchestrate.rs:12604-12732`
- `crates/roko-cli/src/orchestrate.rs:12913-12930`
- `crates/roko-gate/src/rung_dispatch.rs:76-120`

That means the pack must treat gate execution as **live runtime**, not as an activation backlog.

---

## What The Live 7-Rung Path Actually Means

Current runtime mapping in `rung_dispatch.rs`:

- `0`: compile
- `1`: test
- `2`: clippy
- `3`: symbol + generated-test
- `4`: property-test + verify-chain
- `5`: fact-check
- `6`: llm-judge + integration

Important qualification:

- some higher-rung steps fall back to explicit stub/pass verdicts when their richer inputs are not wired for a given task

That is still meaningfully different from “not wired.” It means the runtime path exists, while some advanced inputs remain conditional.

---

## What Is Real But Not The Primary Production Path

These abstractions are still real and worth documenting, but should not be described as the main shipped entrypoint:

- `rung_selector.rs`
- `PlanComplexity`
- `Rung` + `CANONICAL_ORDER`
- `select_rungs(...)`
- `GatePipeline`

Key anchors:

- `crates/roko-gate/src/rung_selector.rs:25-214`
- `crates/roko-gate/src/gate_pipeline.rs:36-224`

Post-audit wording should say:

- these are **reusable selector/pipeline abstractions**
- they are **not the clearest description of the current CLI runtime path**

---

## Rewrite Guidance

### Keep

- 7-rung verification is a real concept in code.
- Executor/plan gate execution is real.
- There is a canonical selector/pipeline layer in `roko-gate`.

### Rewrite

- Do not claim that `select_rungs(...)` is the production caller today.
- Do not claim that `GatePipeline` is the current dispatch entrypoint.
- Do not treat the section as a major implementation gap.

### Narrow

- The real documentation gap is alignment between the canonical library story and the actual runtime dispatch story.
- The real runtime caveat is conditional higher-rung input wiring, not missing core verification infrastructure.

---

## Replacement Summary

This section should now read as:

- **shipped core**: the 7 runtime rungs exist, and gate verdicts are produced on the execution path
- **secondary abstractions**: selector/pipeline helpers exist and are reusable, but they are not the best single-file explanation of today’s production dispatch
- **remaining seam**: higher-rung richness is conditional and should be documented honestly

---

## Do Not Turn This Into

- a selector-unification roadmap
- a large rewrite plan for `rung_selector.rs`
- a claim that every advanced rung has fully rich inputs on every task

The audit fix here is documentation accuracy, not a new refactor program.
