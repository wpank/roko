# Agent Runbook — Batch 05

Use this when executing any batch from `tmp/docs-parity/05`.

## Mission

Keep the learning parity materials honest about what already ships. Prefer small bridge changes and clearer runtime contracts over new theory or framework inflation.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and verify the relevant code paths before planning changes.
3. Classify every claim as `shipping`, `ship soon`, or `deferred`.
4. Narrow the batch to one realistic seam that a single agent can finish and verify.
5. If the work drifts into demurrage, worldviews, replication-ledger, or theory-first framework work, record the handoff and stop.
6. Run the verify command and leave concrete evidence.

## Default Decision Rules

- Treat shipped modules as the default: `active_inference`, `prediction`, `cascade_router`, `prompt_experiment`, `drift`, `pattern_discovery`, `runtime_feedback`, `efficiency`, and `regression` already exist.
- Treat `roko-neuro/src/tier_progression.rs` as the canonical proof that tier progression is real.
- Treat the `Engram` HDC fingerprint field as the highest-value nearby bridge, but keep it as a carry-forward unless the batch explicitly owns it.
- For memory freshness, prefer `last_used` and `access_count` extensions over a full demurrage model.
- Academic framing is never acceptance evidence; runtime behavior and file references are.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether verification passed,
- what was intentionally deferred,
- and why the remaining work belongs to a later batch.

## Failure Modes To Avoid

- describing demurrage, worldviews, or replication-ledger work as shipped,
- treating existing learning modules as if they still need to be invented,
- using FEP or VSM language as a substitute for code evidence,
- widening a parity batch into a multi-crate architecture rewrite,
- giving one batch more than a single agent can realistically finish and verify.
