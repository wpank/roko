+++
id = "bug-8fe462"
kind = "bug"
title = "Plan writing turns a spec's wildcard test check (tests.test_osc.NoiseTest.*, SVFTest.test_lowpass_*) into invented test names, even after its cannot-run retry, so G1's plan kept 12 of 14 checks and two tasks carry checks that cannot pass"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-18
updated = 2026-10-18
source = "round-v6-dogfood-desktop#L4-g1"
discovered_from = "agent:dogfood-desktop (round v6 showcase take L4, 2026-10-18)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::repair", "crates/roko-cli/src/plan_generate/runnable.rs::named_tests", "crates/roko-cli/src/plan_generate/source_checks.rs"]
links = { depends_on = [], blocks = [], related = ["gap-ecafde"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn wildcard_spec_checks_become_runnable_steps' crates/roko-cli/src && cargo test -p roko-cli --lib -- wildcard_spec_checks_become_runnable_steps wildcard_checks_expand_to_their_tests"
+++

## Problem

When a spec states a check as a wildcard over real tests, plan writing replaces the wildcard with test names it
invents, and they don't exist. This happened even after roko's cannot-run report asked the planner again. roko then
reports those checks as CANNOT RUN, but it hands over the plan with them as the tasks' verify steps.

Seen in round v6's showcase take L4 (G1, 2026-10-18, roko cc69556c2). In a Hermes chat, "Plan this spec in roko
first", with pocket-synth's spec pasted word for word. The planner was gpt-oss-120b (`[authoring] planner_model`),
the cap was `max_generation_usd = 0.10`, and the plan writing cost $0.0737.

| Spec check | As the spec states it | What the plan's verify runs | The real tests |
|---|---|---|---|
| INV-4 (SC4) | `tests.test_osc.NoiseTest.*` | `tests.test_osc.NoiseTest.test_white_noise && …test_pink_noise` | `test_white_noise_is_centred`, `test_pink_is_darker_than_white`, `test_seeded_noise_repeats` |
| INV-5 (SC5) | `tests.test_svf.SVFTest.test_lowpass_*` | `tests.test_svf.SVFTest.test_lowpass_400hz` | `test_lowpass_attenuates_highs`, `test_lowpass_passes_lows` |

roko's report on the plan (`state/operations.jsonl`, `source_checks`) read `total 14, kept 12, cannot_run 2,
regenerated true`. The card said "12 of 14 kept · 2 cannot run · asked again", and each of the two read "it names
`test_white_noise`, which `tests/test_osc.py` does not define and no task writes".

## Why it matters

G1's claim is "a prose PRD keeps its checks", and roko-planner's re-run kept 14/14. Here two of the spec's checks
became commands that fail on every try, so the tasks that carry them can't pass. The report is honest, but the
plan isn't runnable as written, and the planner didn't fix it when asked again. A wildcard is easy to resolve
without a model: the test files are right there.

## Where

- Plan writing's source-check pass and its regeneration (gap-ecafde's checks, gap-dde99e and gap-b95913's retries):
  `crates/roko-cli/src/plan_generate/`.

## Current state

The cannot-run detection works, and the retry happens. The planner's second draft still invents the names.

## Plan

- When a spec's check is a test id pattern (`module.Class.*`, `module.Class.test_prefix_*`), roko expands it itself
  from the project's test files: each matching test id, or the pattern as a runner selection (`-k`, or a
  `unittest` id list). It puts that in the verify step, instead of leaving the names to the model.
- Failing that, a plan that still has a check that cannot run asks before it runs (or is refused, as `plan validate
  --strict` would).

## Done when

Writing a plan from pocket-synth's spec keeps INV-4 and INV-5 as runnable checks over the real tests (14 of 14 kept),
with any planner model.

## Notes

- Evidence: `tmp/hermes-roko-v6/findings/dogfood-desktop.md`.
  - The take is `tmp/hermes-roko-v6/media/pipeline/work/takes/l4-g1-prd-0043`, whose `plan-card.png` is the plan
    card with the CANNOT RUN chips.
  - The plan is `/Users/Shared/pocket-synth-prd/plans/plan-this-spec-in-roko-first-and-give-it-to-the/tasks.toml`.
  - The spec is `/Users/Shared/pocket-synth-prd/specs/pocket-synth.md`, lines 77-78.
- The showcase's guard (`media/pipeline/tools/check_plan_tests.py`) stopped the take before the run, so no run
  spend.
