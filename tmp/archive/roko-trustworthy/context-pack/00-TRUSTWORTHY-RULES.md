# Trustworthy Runner Rules

Each batch exists to make Roko more trustworthy as a self-hosting executor. Do not optimize for a broad-looking diff. Optimize for behavior that can be verified, resumed, inspected, and improved over time.

## Required Working Style

- Assume no prior chat context. Read the shared context pack, then read the source docs and code named by the batch prompt.
- Read relevant files fully enough to understand the local architecture. Do not rely only on search hits.
- Prefer local patterns, crate boundaries, and existing naming unless the batch explicitly asks to generalize them.
- Keep edits scoped. Do not mix dashboard/product work into execution-kernel batches.
- Use structured types, schemas, manifests, ledgers, and explicit events instead of prompt-only conventions.
- Do not add fake pass gates, noop production paths, or tests that merely assert stubs.
- Preserve user or unrelated work in the checkout. The runner uses isolated worktrees, but the same rule still applies.
- If a batch uncovers a larger design issue, capture it in a focused TODO, plan packet, or ledger row instead of expanding the batch uncontrolled.

## Done Means

A batch is done only when:

- the implementation matches the batch scope;
- focused tests or compile gates cover the changed behavior;
- verification commands in the batch prompt pass, or the final message clearly explains why they could not be run;
- failure modes fail closed instead of silently succeeding;
- data needed for future self-hosting is persisted structurally;
- implemented doc requirements have parity ledger evidence where the repo already has a ledger surface.

## Anti-Patterns to Remove

- hardcoded agent roles that cannot be configured or composed;
- live Mori/Bardo prompt leakage in Roko execution paths;
- context injection embedded directly in orchestration logic;
- string-parsed review verdicts without a typed contract;
- unscoped prompt stuffing that hides source/provenance;
- retries that do not use failure classification;
- learning loops without action identifiers and reward observations;
- dashboard-first implementation that forces unstable backend contracts.

## Self-Hosting Safety

Roko should not be trusted to implement the rest of the architecture until it can:

- assemble context from declared policies;
- generate and validate plans;
- dispatch agents with role and prompt manifests;
- parse outputs structurally;
- run gates and classify failures;
- replan or retry from failure evidence;
- resume after interruption;
- record policy/outcome learning signals;
- produce a parity trail showing which doc requirements were actually implemented.
