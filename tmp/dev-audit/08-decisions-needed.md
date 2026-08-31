# Decision record and remaining choices

The audit used conservative defaults so work could proceed without blocking. The expanded
integration implements the opt-in lane, resource admission, impact gates, run evidence/API, bounded
context, and hard-deadline settlement without changing default or auto-merge authority. Decisions
that would change defaults or auto-merge policy remain open until representative benchmarks exist.

## 1. Default lane

Recommendation: FAST is opt-in during benchmarking, then becomes the local default for T0/T1 after
promotion criteria pass. RELEASE remains explicit and blocks merges for T2/T3.

P0 decision: **implemented as opt-in** through `./dev.sh fast`; default promotion is deferred.

Decision: should FAST eventually become the default for all local roko plan runs, or only for a
new command/flag?

## 2. Merge authority

Recommendation:

- T0/T1 may auto-commit after valid evidence.
- Auto-merge stays off initially.
- T2/T3 require human/release approval.

P0 decision: **explicit merge only**. The P0 branch was merged after review; no auto-merge default
was enabled.

Decision: after the benchmark proves reliability, may T0/T1 auto-merge to main, or should every
merge remain explicit?

## 3. Tests in the local lane

Recommendation:

- T0: none by default.
- T1: one exact test only when logic changed.
- T2: impacted tests plus real smoke.
- T3: focused invariant test is mandatory.

P0 decision: every FAST task must author exactly one runner-owned verification command. The broader
tier policy and any zero-test prototype lane remain deferred.

Decision: do you want an even more aggressive prototype lane with zero tests for T0–T2 and only
runtime evidence, clearly prohibited from auto-merge?

## 4. Model policy

Recommendation: deterministic transforms first; fast/low-reasoning model for clear patches;
frontier model only for high-risk/open-ended work.

P0 status: model-policy changes were not included.

Decision: should the benchmark optimize primarily for wall time, cost, or lowest regression rate
when model choices differ?

## 5. Machine/cache policy

Recommendation: reserve disk, serialize Cargo, and choose one cache strategy by benchmark.

Current status: FAST preserves warm artifacts, suppresses critical-path cleanup, keeps agent access
to the shared Cargo target behind an explicit trusted opt-in, and refuses severe disk pressure
unless an override is recorded. Final cache-strategy selection remains benchmark-dependent.

Decision: is it acceptable for Roko to refuse starting a cold self-host run when disk/swap pressure
exceeds a threshold, with an explicit force override?

## 6. P0 implementation order — resolved

The approved batch was scheduler admission + one verification owner + a minimum evidence bundle.
It is merged on `main` at `a58bdbacb`:

- [x] Reserve scheduler capacity and exact-attempt ownership before preparation.
- [x] Give the patching agent a no-build contract and the runner one authored verify command.
- [x] Add the `run-evidence` wrapper and FAST evidence bundle in the same batch.
- [x] Add focused impact/reverse-dependent selection, bounded plan context, run-scoped APIs, and
  safe optional CLI/API/text/PNG evidence.
- [x] Add wake-driven hard-deadline/startup interposition, bounded conductor settlement, safe
  timeout-diff gate salvage, and convergent terminal projections.
- [ ] Run the representative cold/warm benchmark scorecard.
- [ ] Run the final rebased integration verification/RELEASE batch.
- [ ] Decide whether asynchronous RELEASE verification should become a mandatory merge policy.
- [ ] Add an operation-level Codex broker and optional offline repair for pre-index historical runs.

The next implementation batch should be chosen from those unchecked items after the scorecard
identifies the largest remaining measured bottleneck.
