# Domain Runtime And Arenas Checklist

## Scope

Use this file for `DomainProfile` wiring, task/domain parsing, tool/gate filtering, and the arena framework.

## Implementation checklist

- [ ] Define the canonical `DomainProfile` surface.
  - domain name;
  - default model/routing hints;
  - gate profile;
  - tool allowlist;
  - context mix;
  - extension set.
- [ ] Load domain profile from existing config surfaces.
  - task TOML or task parser;
  - `roko.toml`;
  - agent role/profile config.
- [ ] Route observable behavior by domain.
  - gate selection;
  - tool selection;
  - context-budget weighting;
  - tier or escalation defaults.
- [ ] Implement an `Arena` trait only after the runtime profile boundary is stable.
  - task source;
  - execution harness;
  - scoring;
  - persistence of results.
- [ ] Build two initial arenas only.
  - self-hosting arena against the current repo;
  - one coding benchmark arena such as SWE-bench or a smaller internal subset.
- [ ] Add CLI only when the arena contract is real.
  - `roko bench arena`
  - clear output format for scores and artifacts.

## Concrete file touchpoints

- `crates/roko-cli/src/task_parser.rs`
- `crates/roko-cli/src/agent_config.rs`
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/tests/e2e_domain.rs`
- `crates/roko-orchestrator/src/`

## Verification checklist

- [ ] Domain config changes tool and gate selection in an integration test.
- [ ] Arena runs produce persisted scores, not only console output.
- [ ] A failed arena run records enough state to debug the failure.

## Acceptance criteria

- One runtime can run multiple domains via profiles.
- Arena evaluation is repeatable and produces structured results.
- CLI verbs for arenas reflect real behavior, not placeholders.
