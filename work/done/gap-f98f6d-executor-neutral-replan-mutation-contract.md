+++
id = "gap-f98f6d"
kind = "gap"
title = "Executor-Neutral Replan Mutation Contract"
status = "done"
triage = "verified"
severity = "p0"
subsystem = ["roko-core"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/273-replan-mutation-contract.md#273 — Executor-Neutral Replan Mutation Contract"
discovered_from = "audit:tmp/backlog/archive/273-replan-mutation-contract.md#273 — Executor-Neutral Replan Mutation Contract"
anchors = ["crates/roko-core/src/plan_mutation.rs::apply_mutation", "crates/roko-core/src/plan_mutation.rs::PlanMutationErrorV1", "crates/roko-execution/src/replan_controller.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core plan_mutation"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Implemented (source status 2026-09-04) and committed (91b4745f8). crates/roko-core/src/plan_mutation.rs defines MutationAuthorV1, MutablePlanV1, PlanMutationOpV1, PlanMutationV1, PlanMutationResultV1 and PlanMutationErrorV1 (:199), plus canonical_fingerprint (:304) and apply_mutation (:399). apply_mutation checks the base fingerprint and mutates a clone of the plan. It is consumed by roko-graph and crates/roko-execution/src/replan_controller.rs."
+++
Executor-Neutral Replan Mutation Contract

Imported without verification from:
- `tmp/backlog/archive/273-replan-mutation-contract.md#273 — Executor-Neutral Replan Mutation Contract`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#Phase A: Foundation (Wave 0-1, ~3-5 days #273`

Some cited files are gone: `tmp/engine-audit/08-runner-extractable.md`.

How to verify: Check: Create the exact file, types, variants, and application rules above.; Apply mutations atomically to a cloned plan and commit only after all validation succeeds.; Validate IDs, references, acyclicity, schema compatibility, task limits, and… [evidence: own status: Implemented (2026-09-04); 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): -- | engine DAG | Done (Wave 0; 2026-09-04); (newer evidence overrides own status…]

Verified 2026-09-28: closed as done; the V1 mutation contract exists and is consumed by the replan controller.
