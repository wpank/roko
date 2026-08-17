# Product Surfaces, Deployment, Onboarding, Security, And Observability

## Scope

Use this file for the PRD-08 material that spans user-facing products and operator infrastructure: AI Studio, Agent Studio, OpenClaw, deployment targets, gateway, onboarding, security model, coordination, and observability.

## Implementation checklist

- [ ] Break the three product surfaces into explicit build tracks.
  - AI Studio read-only collective-intelligence surfaces;
  - Agent Studio operator control plane;
  - OpenClaw end-user hedging flow.
- [ ] For AI Studio, capture concrete backlog for:
  - InsightStore corpus browser;
  - reputation explorer;
  - predictive-analysis views;
  - stigmergy visualization;
  - auto-research flow;
  - revenue/entitlement gates if relevant.
- [ ] For Agent Studio, capture concrete backlog for:
  - deployment/lifecycle management;
  - cognitive-frequency monitoring;
  - retrieval-to-action audit trail;
  - cost analytics;
  - staking tier management;
  - domain module management.
- [ ] For OpenClaw, capture concrete backlog for:
  - wallet connect;
  - position scan across supported protocols;
  - rate-exposure analysis;
  - recommendation and one-action approval flow;
  - watch-only mode;
  - trust-building surfaces.
- [ ] Add deployment and gateway tasks explicitly.
  - local install;
  - binary install path;
  - container deployment;
  - inference gateway with routing, auth, caching, and failover;
  - environment validation and `roko doctor` behavior.
- [ ] Add onboarding tasks by persona.
  - developer in 5 minutes;
  - operator in 15 minutes;
  - end user in 30 seconds.
- [ ] Add security-model tasks.
  - reasoning traces;
  - track record surfaces;
  - hard limits/delegation caveats;
  - observation-only mode;
  - pre/post execution safety checks.
- [ ] Add monitoring and observability tasks.
  - metrics;
  - structured tracing;
  - event log for crash recovery;
  - efficiency events;
  - health probes;
  - realtime streaming;
  - HTTP API scope and docs.
- [ ] Add coordination/discovery tasks where PRD-08 expects them.
  - agent discovery;
  - four coordination mechanisms;
  - interaction with passport/reputation infrastructure.

## Additional gap-closure tasks

- [ ] Add a task for AI Studio entitlement-aware degradation.
  - free vs pro vs enterprise limits;
  - clear paywall/limit messaging;
  - no silent partial results.
- [ ] Add a task for OpenClaw trust-mode progression.
  - observe-only mode;
  - simulated recommendation mode;
  - capped-autonomy mode;
  - full delegated execution with explicit caveats.
- [ ] Add a task for deployment artifact parity.
  - source install;
  - binary install;
  - container install;
  - all produce compatible config/state layout.
- [ ] Add a task for `roko doctor` breadth.
  - env vars;
  - file layout;
  - provider connectivity;
  - optional chain connectivity;
  - dashboard/nexus reachability where configured.
- [ ] Add a task for observability retention policy.
  - metrics TTL;
  - log/event compaction;
  - crash-recovery event-log size bounds;
  - export path for postmortems.

## Agent-ready task sequence

1. `UX-GAP-01` AI Studio entitlement degradation
   - Scope: define exact behavior at quota/plan boundaries for AI Studio queries and views.
   - Touches: dashboard/frontend entitlement layer, backend limit signaling, product docs.
   - Deliverable: explicit free/pro/enterprise degradation behavior.
   - Done when: quota exhaustion produces deterministic UI/backend behavior instead of silent truncation.

2. `UX-GAP-02` OpenClaw trust-mode ladder
   - Scope: turn observe-only, simulated, capped-autonomy, and full delegation into explicit product modes.
   - Touches: OpenClaw UX spec, wallet/approval flow, backend mode flags.
   - Deliverable: one mode model with upgrade path between trust levels.
   - Depends on: `UX-GAP-01`.
   - Done when: a user can be placed into one mode and every action respects its limits.

3. `UX-GAP-03` Deployment artifact parity check
   - Scope: ensure source, binary, and container installs produce compatible config/state layout.
   - Touches: install docs/scripts, Docker configs, CLI init paths.
   - Deliverable: parity checklist and verification script or test.
   - Depends on: none.
   - Done when: the same project can be initialized and inspected consistently across install modes.

4. `UX-GAP-04` `roko doctor` expansion
   - Scope: expand doctor coverage to providers, file layout, optional chain reachability, and dashboard/nexus reachability.
   - Touches: CLI doctor/status code path, docs, tests.
   - Deliverable: richer environment validation with actionable remediation text.
   - Depends on: `UX-GAP-03`.
   - Done when: doctor catches and explains at least one missing dependency in each major category.

5. `UX-GAP-05` Observability retention policy
   - Scope: define TTL, compaction, postmortem export, and event-log bounds.
   - Touches: logs/metrics retention code and docs.
   - Deliverable: one retention policy wired into runtime or maintenance tasks.
   - Depends on: `UX-GAP-04`.
   - Done when: long-running installations do not accumulate unbounded observability state.

## Relevant current files

- `crates/roko-cli/src/status.rs`
- `crates/roko-cli/src/config_cmd.rs`
- `crates/roko-cli/src/agent_serve.rs`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/subscriptions.rs`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/`

## Verification checklist

- [ ] Each product surface has a distinct backlog with no hidden scope creep.
- [ ] Onboarding flows are reproducible from clean-machine assumptions.
- [ ] Security/trust claims have corresponding UX and backend tasks.
- [ ] Observability work spans CLI, serve, and web/TUI consumers.

## Acceptance criteria

- PRD-08’s product and operations surface is represented comprehensively in implementation tasks.
- OpenClaw, AI Studio, and Agent Studio are all scoped, not collapsed into generic “web UI” work.
- Deployment, onboarding, security, and observability are no longer under-specified.
