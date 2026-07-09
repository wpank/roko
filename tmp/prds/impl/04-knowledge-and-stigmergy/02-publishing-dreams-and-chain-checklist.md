# Publishing, Dreams, And Chain Checklist

## Scope

Use this file for the publishing pipeline, seven-layer defense, dream-trigger wiring, and future InsightStore publication/query integration.

## Implementation checklist

- [ ] Define `KnowledgePublisher` against the current crates.
  - input from `roko-neuro`;
  - optional publish targets: local export, future mesh, future chain;
  - explicit policy result for allow, redact, embargo, reject.
- [ ] Implement the seven-layer publishing defense as discrete steps.
  - content classifier;
  - distillation/abstraction;
  - IFC labels;
  - quality gate;
  - temporal embargo;
  - PP-HDC transform;
  - selective sharing / novelty check.
- [ ] Wire dream triggers using existing infrastructure first.
  - leverage `roko-dreams` runner scheduling;
  - trigger on idle time and minimum episode count before inventing more modes;
  - send dream outputs back into neuro and daimon only through explicit interfaces.
  - include hypnagogia-stage backlog and counterfactual-hypothesis generation as named dream outputs if not yet implemented.
- [ ] Add local-first InsightStore integration.
  - start with a client boundary or mirage-backed stub;
  - cache responses locally;
  - keep query provenance and freshness visible.
- [ ] Do not treat mesh/pheromone behavior as already implemented.
  - if knowledge sharing depends on future coordination work, say so in code and docs.
  - include deferred coordination backlog for permissioned subnets, morphogenetic specialization, and collective contagion where those PRD concepts touch shared knowledge flow.

## Relevant current files

- `crates/roko-dreams/src/runner.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-neuro/src/context.rs`
- `apps/mirage-rs/src/chain/knowledge.rs`
- `apps/mirage-rs/src/chain/insight.rs`
- `docs/10-dreams/16-implementation-status.md`
- `docs/13-coordination/12-current-status-and-gaps.md`

## Verification checklist

- [ ] Each publishing-defense layer can fail independently with a visible reason.
- [ ] Dream scheduling can be triggered in test without human timing.
- [ ] Dream outputs that are accepted become queryable knowledge entries.
- [ ] Chain or mirage query failures degrade cleanly to local-only behavior.

## Acceptance criteria

- Publishing is governed by an explicit, testable policy pipeline.
- Dreams contribute real knowledge or somatic updates through the same stores other subsystems use.
- Chain integration is staged as a client boundary with local fallback.
- Nothing in this path assumes mesh stigmergy is already shipping when it is still mostly specified.
