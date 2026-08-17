# Post-Gate Reflection Loop

**Origin**: `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 4 —
"Post-gate reflection loop" (lines 130–154)
**Status**: Backlog
**Priority**: P2 — improves retry quality and accelerates convergence on hard tasks
**Size**: M (2–3 days)

---

## Problem statement

When a gate fails (compile, test, clippy), the runner currently re-dispatches
the implementer agent with the raw gate output appended to its prompt. The agent
reads the error, attempts a fix, and tries again. This works for simple errors
but degrades on multi-step failures: the agent sees a wall of compiler output,
picks the most salient error, fixes it, and introduces a new one. Iteration
count climbs; cost rises; quality of each attempt does not improve over time.

The root cause is that there is no structured retrospective between the gate
failure and the retry dispatch. The agent enters each attempt without a
synthesised analysis of what went wrong on the previous one.

The workspace already has significant scaffolding for this:

- `crates/roko-learn/src/post_gate_reflection.rs` — `PostGateReflectionRecord`,
  `PostGateReflectionStore`, admission status, deduplication logic, and playbook
  candidate extraction are fully implemented.
- `crates/roko-cli/src/runner/event_loop.rs` `record_gate_failure_reflection`
  (line 12103) — calls into that store to write a reflection record after every
  gate failure, with deduplication and a cumulative cost guard.
- `crates/roko-cli/src/runner/event_loop.rs` `lessons_from_post_gate_reflections`
  (line 4558) — reads stored lessons and injects them into the retry agent's
  system prompt as "Lessons from previous attempt."
- `crates/roko-cli/src/runner/state.rs` `cumulative_reflection_cost_usd` (line
  237) — tracks spend against a per-run cap.

What is **not** implemented is the lightweight LLM agent call that generates the
`proposed_lesson` in the first place. `record_gate_failure_reflection` currently
synthesises the lesson from the classified failure pattern IDs (deterministic,
no LLM). This is useful but shallow: it cannot reason about the specific files
changed, the iteration count, or the interaction between multiple error classes.

The missing piece is a haiku-class agent call (max 500 output tokens, bounded to
`<$0.02` per invocation) that reads the gate output and the file-change list and
writes a short, actionable lesson — the analysis specified in the original gap.

---

## Proposed solution

Add a `spawn_reflection_agent` async function in
`crates/roko-cli/src/runner/event_loop.rs` (or a co-located `reflection.rs`
helper) with this contract:

```rust
async fn spawn_reflection_agent(
    gate_name: &str,
    error_digest: &str,        // first 200 chars of gate output, normalised
    files_changed: &[String],  // paths touched by the agent in this attempt
    iteration: u32,
    provider: &dyn ModelCaller,
    max_tokens: u32,           // hardcoded to 500 at call site
) -> Result<String, ReflectionError>
```

The prompt sent to the model is:

> Analyze this gate failure. What went wrong? What should the next attempt do
> differently?
>
> Gate: `{gate_name}`. Attempt: `{iteration}`.
> Error digest: `{error_digest}`.
> Files changed: `{files_changed joined by newline}`.
>
> Reply in 2–4 sentences. Be specific and actionable.

`spawn_reflection_agent` is called from `record_gate_failure_reflection` when:
- The cumulative reflection cost guard has not tripped
  (`cumulative_reflection_cost_usd < REFLECTION_COST_GUARD_USD`, currently
  `$0.10` across the run).
- The error digest does not already have a matching reflection record
  (deduplication already implemented).

The returned lesson replaces the deterministic `proposed_lesson` string that
`record_gate_failure_reflection` currently synthesises from pattern IDs. The
cost of the call (derived from token usage) is added to
`cumulative_reflection_cost_usd` in the runner state.

Model selection: always use the cheapest available model that supports text
generation. The caller passes a `&dyn ModelCaller` so the runner can supply the
cascade-router-selected haiku-class provider; the function does not hardcode a
provider name. The cost guard (500 tokens × Haiku pricing ≈ $0.0001) makes the
`$0.02` cap from the original spec extremely conservative.

No new Episode struct fields are required — `PostGateReflectionRecord` already
stores the lesson and the admission status. The existing
`lessons_from_post_gate_reflections` injection in the retry path already reads
these records. Wiring the LLM call in is the only outstanding work.

---

## Implementation location

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Add `spawn_reflection_agent`; call it from `record_gate_failure_reflection`; plumb `files_changed` from the attempt context |
| `crates/roko-learn/src/post_gate_reflection.rs` | No structural changes; `proposed_lesson` field already exists on `PostGateReflectionRecord` |
| `crates/roko-cli/src/runner/state.rs` | `cumulative_reflection_cost_usd` already exists (line 237, 319, 243); add update call after successful agent response |

---

## Acceptance criteria

1. After a gate failure, a reflection record in
   `.roko/learn/post-gate-reflections.json` contains a `proposed_lesson` that is
   a full natural-language sentence (not a deterministic pattern ID summary),
   written by a model call, visible in the JSONL.
2. The lesson from the most recent reflection for a given task is injected into
   the retry agent's system prompt under a "Lessons from previous attempt:"
   header — verifiable by inspecting the prompt log or adding a debug assertion
   in `lessons_from_post_gate_reflections`.
3. Deduplication: a second gate failure with the same normalised error digest on
   the same gate does not spawn a second model call; the existing record's lesson
   is reused.
4. The cumulative cost guard prevents more than `REFLECTION_COST_GUARD_USD` of
   reflection spend per run; once tripped, subsequent gate failures skip the
   model call and fall back to the deterministic lesson synthesis.
5. `spawn_reflection_agent` returns `Err(ReflectionError::ModelCallFailed)` on
   provider error without panicking; the runner logs the error at `warn!` level
   and falls through to the deterministic lesson.
6. `cargo test -p roko-learn --lib post_gate_reflection` continues to pass
   without modification; no existing reflection record types are changed.

---

## References

- `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 4 (lines 130–154) —
  original specification: trigger, model, prompt, deduplication, cost guard
- `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 4 spec clarification
  (lines 527–537) — token-based cost guard rationale; actual cost ~$0.0001 at
  Haiku pricing
- `tmp/architecture-archive/20-orchestrator-gaps.md` Gap 11 (lines 351–376) —
  reflection-derived playbook rules; depends on this gap being resolved first
- `crates/roko-learn/src/post_gate_reflection.rs` — `PostGateReflectionRecord`,
  `PostGateReflectionStore`, deduplication, admission status (fully implemented)
- `crates/roko-cli/src/runner/event_loop.rs` line 12103 —
  `record_gate_failure_reflection` (existing deterministic path to be extended
  with LLM call)
- `crates/roko-cli/src/runner/event_loop.rs` line 4558 —
  `lessons_from_post_gate_reflections` (injection into retry prompt; already wired)
- `crates/roko-cli/src/runner/state.rs` line 237 —
  `cumulative_reflection_cost_usd` field and cost cap constant
