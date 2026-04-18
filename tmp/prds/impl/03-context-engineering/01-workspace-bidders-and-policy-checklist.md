# Workspace, Bidders, And Policy Checklist

## Scope

Use this file for `CognitiveWorkspace`, bidder wiring, auction assembly, and learnable context-policy work.

## Implementation checklist

- [ ] Define the canonical workspace data model.
  - `ContextCategory`
  - `ContextSection`
  - `SectionSource`
  - `CognitiveWorkspace`
- [ ] Make the data model reflect current reality.
  - sections should map cleanly onto the prompt layers already produced by `roko-compose`;
  - sources should preserve whether content came from files, plans, episodes, knowledge, config, or future chain/worldgraph inputs.
- [ ] Introduce a bidder contract over the current context pipeline.
  - each bidder returns a score, token estimate, and provenance;
  - each bidder must explain why it bid;
  - each bidder must degrade cleanly when its subsystem is unavailable.
- [ ] Start with bidders that already have data.
  - file/repo context
  - plan/task state
  - recent episodes
  - knowledge/neuro context
  - learning-derived hints
  - affect/somatic bias where already available
- [ ] Only then add future-facing bidders.
  - chain-sourced context
  - worldgraph context
  - other experimental sources
- [ ] Wire auction output back into the existing prompt builder.
  - selected sections become ordered prompt sections;
  - token allocation and placement remain deterministic;
  - rejected sections should be inspectable for debugging.
- [ ] Add `ContextPolicy`.
  - policy adjusts section quotas, bidder weighting, or reserve budgets;
  - policy updates should be driven by measured outcomes from `roko-learn`, not hidden heuristics.

## Concrete file touchpoints

- `crates/roko-neuro/src/context.rs`
- `crates/roko-compose/src/context_assembler.rs` or its current re-export path
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-learn/src/section_effect.rs`
- `crates/roko-learn/src/prompt_experiment.rs`

## Verification checklist

- [ ] Auction produces deterministic output from the same inputs.
- [ ] Bidders can be turned on/off independently in tests.
- [ ] The winning sections can be logged with scores and token budgets.
- [ ] A policy change produces an observable difference in section allocation.

## Acceptance criteria

- A fresh engineer can explain why each prompt section was included.
- The workspace model is rich enough to support later chain/worldgraph injection.
- Policy learning uses measured outcomes, not hand-wavy future hooks.
- No second prompt assembly path is created.
