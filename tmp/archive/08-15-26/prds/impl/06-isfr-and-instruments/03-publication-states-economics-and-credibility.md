# Publication States, Solver Economics, And Credibility Path

## Scope

Use this file for the PRD-07 sections that are easy to omit during pure implementation work: publication-state semantics, source liveness, path-to-credibility, solver challenge economics, EventFabric integration, and multi-chain source operations.

## Implementation checklist

- [ ] Define publication states explicitly.
  - live;
  - degraded;
  - halted;
  - fallback or previous-live state as required by the PRD.
- [ ] Implement source-liveness tracking and stale-source policy.
  - heartbeat/last-update;
  - missing-source thresholds;
  - fallback to cached last-good value;
  - operator-visible state transitions.
- [ ] Add solver economics and anti-gaming tasks.
  - solver fees;
  - solver bond requirements;
  - challenge mechanism flow;
  - slashing conditions;
  - anti-collusion guardrails.
- [ ] Represent the credibility path as staged deliverables.
  - publication and transparency phase;
  - perp launch phase;
  - external integration phase;
  - evidence required to move between phases.
  - explicitly preserve the PRD phrase "path to credibility" in docs and rollout milestones.
- [ ] Wire ISFR into EventFabric or equivalent runtime eventing.
  - benchmark updates;
  - large-move amplification;
  - halted-state circuit-breaker events;
  - clearing insights flowing into world-model consumers.
- [ ] Add cross-domain usage tasks.
  - research-agent consumers;
  - coding-agent consumers;
  - security-agent consumers;
  - generalized benchmark-index consumers.
- [ ] Add multi-chain source-operations tasks.
  - per-chain adapter ownership;
  - cross-chain aggregation timing;
  - reconciliation and freshness policy.

## Additional gap-closure tasks

- [ ] Add a task for publication-state operator UX.
  - what dashboards/TUI show during Live, Degraded, Halted, and fallback states;
  - recommended operator action per state.
- [ ] Add a task for source-disagreement forensics.
  - retain per-source raw values;
  - disagreement snapshot for each published rate;
  - replay tooling for anomalous updates.
- [ ] Add a task for challenge-mechanism latency bounds.
  - how long a solver challenge can remain unresolved;
  - interim behavior for affected settlements;
  - visibility to downstream consumers.
- [ ] Add a task for benchmark migration strategy.
  - how ISFR transitions from simulator-only to public benchmark service without changing consumer contracts abruptly.
- [ ] Add a task for external credibility evidence capture.
  - transparency reports;
  - public methodology artifacts;
  - benchmark-quality dashboards.

## Agent-ready task sequence

1. `ISFR-GAP-01` Publication-state UX contract
   - Scope: define what operators and downstream consumers see for each publication state.
   - Touches: serve/status payloads, dashboard/TUI state labels, docs.
   - Deliverable: a stable state-contract used by both backend and surfaces.
   - Done when: Live, Degraded, Halted, and fallback states render distinctly and consistently.

2. `ISFR-GAP-02` Source-disagreement forensic bundle
   - Scope: retain per-source raw values and disagreement snapshots for anomalous updates.
   - Touches: source adapter outputs, aggregation logs, replay tooling.
   - Deliverable: one forensic record per published rate update.
   - Depends on: `ISFR-GAP-01`.
   - Done when: an anomalous publication can be replayed from captured raw source inputs.

3. `ISFR-GAP-03` Solver challenge timing and interim-state rules
   - Scope: bound challenge latency and define settlement behavior while challenges are open.
   - Touches: clearing engine state machine, challenge records, UI statuses.
   - Deliverable: explicit timing and state rules for open challenges.
   - Depends on: `ISFR-GAP-01`.
   - Done when: a simulated challenged batch follows the documented interim path under test.

4. `ISFR-GAP-04` Benchmark migration plan
   - Scope: document and implement the transition from simulator-only service to public benchmark service.
   - Touches: API contracts, versioning docs, deployment/config path.
   - Deliverable: one migration plan with compatibility constraints.
   - Depends on: `ISFR-GAP-02`.
   - Done when: consumers can be shown how they move from test to live benchmark without contract breakage.

5. `ISFR-GAP-05` Credibility evidence capture
   - Scope: produce the artifacts needed for the PRD’s credibility path.
   - Touches: transparency reports, methodology outputs, benchmark dashboards.
   - Deliverable: a reproducible evidence set for publication quality and benchmark trust.
   - Depends on: `ISFR-GAP-04`.
   - Done when: methodology and operational evidence can be published without manual reconstruction.

## Verification checklist

- [ ] State transitions between live/degraded/halted are tested.
- [ ] Solver challenge and fallback paths are deterministic in fixtures.
- [ ] Event emission for large moves and halt conditions is observable.
- [ ] Multi-chain aggregation handles one-chain degradation without undefined output.

## Acceptance criteria

- ISFR is specified operationally, not only mathematically.
- Credibility and solver-economics claims are reflected in executable backlog items.
- Runtime eventing and cross-domain consumers are part of the plan, not omitted.
