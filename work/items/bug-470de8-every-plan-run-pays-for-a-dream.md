+++
id = "bug-470de8"
kind = "bug"
title = "Every plan run pays for a dream consolidation nobody reads: dream_on_completion defaults to true"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #6; §3 park dreams; §6 decision 12); tldr/research/B6-cognitive-subsystems.md"
anchors = ["crates/roko-core/src/config/learning.rs::LearningConfig", "crates/roko-core/src/config/presets.rs::minimal", "crates/roko-core/src/config/presets.rs::thorough", "roko.toml:373"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '^dream_on_completion = false' roko.toml && grep -rqw 'fn dream_on_completion_defaults_to_false' crates/roko-core/src/ && cargo test -p roko-core --lib dream_on_completion_defaults_to_false"
+++

## Problem

After every plan, `DreamConsolidationSink` (`runtime_feedback/plan_completion.rs`) starts a dream consolidation that
runs a `claude` CLI job. It fires when two flags are both true, and both default to true:

- `learning.dream_on_completion` (`config/learning.rs:214`);
- `learning.dreams.trigger_on_plan_complete` (`config/learning.rs:41`).

The `minimal` and `thorough` presets set the first flag to true explicitly (`config/presets.rs:111`, `:182`), and so
does this repository's own `roko.toml:373`. Research note B6 found that the immune boundary denied all 56 of the LLM
distillations these runs produced, so the output changes nothing.

## Why it matters

Every plan run, including every self-hosting run, pays for a model call nobody reads. The whitepaper's measure is cost
per verified task. This is tldr/05 P0 #6, and decision 12 there (default: park dreams). It is part of epic spec-9a3131.

## Where

- `crates/roko-core/src/config/learning.rs::LearningConfig`: the field's serde default, its doc comment and the
  `Default` impl.
- `crates/roko-core/src/config/presets.rs::minimal` and `::thorough`.
- `roko.toml:373`.

## Current state

Unchanged at `41c7ffbd6`. The sink already skips when either flag is false (`plan_completion.rs:85`), so the sink
needs no change. No doc states the default.

## Plan

1. Default `dream_on_completion` to `false`, in the serde default and in `Default`. Keep the field, so existing
   configs still parse.
2. Set it to `false` in both presets and in `roko.toml`.
3. Say in the field's doc comment that dreams run on demand, through `roko knowledge dream run`.

## Done when

- [ ] `LearningConfig::default()`, and a config that omits the key, both give `false`.
- [ ] `roko.toml` sets `false`.
- [ ] A plan run with the default config starts no dream job (check this once with a fake provider).
- [ ] Test `dream_on_completion_defaults_to_false` exists and passes, and the `[[verify]]` command passes.

## Notes

- A user config that says `true` explicitly keeps dreaming. That is intended.
- Parking the dream items themselves is a separate question: dec-e70592.
- Premise check at `a17d4dadd`: no production code emits `FeedbackEvent::PlanCompleted` (q-6b7cca), so Graph plan
  runs do not dream today and the cost is latent. The default change still stands: it keeps dreams opt-in once that
  event is wired.
- Implemented on `work/bug-470de8` at `1f312c78c`; cargo verification deferred to the batch check.
