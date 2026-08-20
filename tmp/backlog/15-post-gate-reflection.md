# 15 — Post-Gate Reflection Loop: LLM-Generated Lessons

**Priority**: P2 — improves retry quality and accelerates convergence on hard tasks by replacing pattern-code summaries with reasoned lessons
**Size**: M (2–3 days)
**Crates**: `crates/roko-cli/` (event loop + runner state), `crates/roko-learn/` (reflection store — read only, no changes)
**Depends on**: None

---

## Background

Roko executes agent tasks and validates them through a gate pipeline (compile, test, clippy, diff). When a gate fails, the runner retries by dispatching the agent again with the gate output appended to its prompt. This retry loop works well for simple errors but degrades on multi-step failures.

The problem: when a compile gate fails with 12 errors, the agent sees a wall of compiler output, picks the most salient error, fixes it, and often introduces a new one. Iteration count climbs; cost rises; the quality of each retry attempt does not systematically improve.

The workspace already has a complete infrastructure for recording structured retrospectives after gate failures — called "post-gate reflections" — in `crates/roko-learn/src/post_gate_reflection.rs`. The runner already calls `record_gate_failure_reflection()` to write these records and `lessons_from_post_gate_reflections()` to inject them into the retry prompt. However, the lessons in these records are currently synthesised deterministically from classified failure pattern IDs (e.g., "investigate compile gate failure; pattern: E0308:type_mismatch"). This is useful but shallow.

The missing piece is a brief LLM call that reads the gate output, the files changed, and the iteration number, and writes a short, actionable 2–4 sentence lesson. This replaces the deterministic string synthesis while reusing all existing deduplication, cost guard, and prompt injection machinery.

The `ModelCaller` trait (in `crates/roko-core/src/foundation.rs` line 456) provides the `call(ModelCallRequest) -> Result<ModelCallResponse>` interface used throughout the codebase. The runner already holds a model caller reference for agent dispatch — the same reference can be passed to the new function.

## Current State

1. **`PostGateReflectionStore`** — fully implemented at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/post_gate_reflection.rs`. Key types: `PostGateReflectionRecord` (line 52), `ReflectionInput` (line 88), `ReflectionAdmissionStatus` (line 36), `PostGateReflectionStore::observe()` (line 230), `PostGateReflectionStore::save()` (line 217), `PostGateReflectionStore::load()` (line 204).
2. **`record_gate_failure_reflection()`** — at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 12104. Accepts `learn_dir`, `gate_name`, `gate_output`, `plan_id`, `task_id`, `iteration`, and `cumulative_reflection_cost_usd`. The current implementation synthesises a deterministic `proposed_lesson` string (lines 12172–12190), then writes the record.
3. **`lessons_from_post_gate_reflections()`** — at `event_loop.rs` line 16099. Reads stored records for a gate, filters by `confidence > 0.3`, deduplicates, truncates to 3, and returns. Already injected into the retry prompt at line 4558–4576.
4. **Cost guard constants** — `REFLECTION_COST_PER_OBSERVATION_USD = 0.00025` (line 12083) and `REFLECTION_COST_GUARD_USD = 0.05` (line 12090). The guard is already enforced at line 12114.
5. **Deduplication** — already implemented in `record_gate_failure_reflection` at lines 12139–12162. Skips writing a new record if the error digest already has a matching record (prevents repeated LLM calls for the same error).
6. **`RunState::cumulative_reflection_cost_usd`** — defined at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs` line 243. Used to track cumulative spend against the guard.
7. **`ModelCaller` trait** — in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs` line 456. Single-shot call: `async fn call(&self, req: ModelCallRequest) -> Result<ModelCallResponse>`. `ModelCallRequest` and `ModelCallResponse` are defined in the same file.
8. **The runner currently makes `record_gate_failure_reflection` a non-async `fn`** — to wire an async `ModelCaller` call, the function signature must change to `async fn`, or the call must be wrapped with `tokio::task::spawn_blocking` or `Handle::current().block_on()`. The simplest approach is to change it to `async fn` and `.await` it at the call site (line 4598).

## Implementation Plan

### Step 1: Define `ReflectionError`

Add `ReflectionError` to `event_loop.rs` (near the top of the file or in a `reflection` module):

```rust
/// Error type for the reflection agent call.
#[derive(Debug)]
pub enum ReflectionError {
    ModelCallFailed(String),
    EmptyResponse,
}

impl std::fmt::Display for ReflectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelCallFailed(msg) => write!(f, "model call failed: {msg}"),
            Self::EmptyResponse => write!(f, "model returned empty response"),
        }
    }
}
```

### Step 2: Add `spawn_reflection_agent` async function

Add this function in `event_loop.rs`, near `record_gate_failure_reflection` (around line 12220):

```rust
/// Call a lightweight model to generate a short actionable lesson from a gate failure.
///
/// Uses the cheapest available model via `caller`. Bounded to `max_tokens` (hardcode
/// 500 at the call site). Returns a 2–4 sentence lesson string or `ReflectionError`.
///
/// The prompt is kept minimal so the response is fast and cheap (< $0.001 at
/// Haiku-class pricing for 500 tokens).
async fn spawn_reflection_agent(
    gate_name: &str,
    error_digest: &str,
    files_changed: &[String],
    iteration: u32,
    caller: &dyn roko_core::ModelCaller,
    max_tokens: u32,
) -> Result<String, ReflectionError> {
    use roko_core::{ModelCallRequest, ChatMessage, MessageRole, ModelInputMessage, MessageContent};

    let files_display = if files_changed.is_empty() {
        "(no files listed)".to_string()
    } else {
        files_changed.join("\n")
    };

    let prompt = format!(
        "Analyze this gate failure. What went wrong? What should the next attempt do differently?\n\n\
         Gate: {gate_name}. Attempt: {iteration}.\n\
         Error digest: {error_digest}\n\
         Files changed:\n{files_display}\n\n\
         Reply in 2-4 sentences. Be specific and actionable. Do not repeat the error message verbatim."
    );

    let req = ModelCallRequest {
        messages: vec![ModelInputMessage {
            role: MessageRole::User,
            content: MessageContent::Text(prompt),
            ..Default::default()
        }],
        max_tokens: Some(max_tokens),
        // Use default system prompt (none); this is a brief utility call.
        system: None,
        ..Default::default()
    };

    match caller.call(req).await {
        Ok(response) => {
            let text = response
                .content
                .iter()
                .filter_map(|block| match block {
                    roko_core::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            if text.is_empty() {
                Err(ReflectionError::EmptyResponse)
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(ReflectionError::ModelCallFailed(e.to_string())),
    }
}
```

### Step 3: Modify `record_gate_failure_reflection` to call `spawn_reflection_agent`

Change the function signature from `fn` to `async fn` and add `caller` and `files_changed` parameters:

```rust
// Old signature:
fn record_gate_failure_reflection(
    learn_dir: &std::path::Path,
    gate_name: &str,
    gate_output: &str,
    plan_id: &str,
    task_id: &str,
    iteration: u32,
    cumulative_reflection_cost_usd: &mut f64,
)

// New signature:
async fn record_gate_failure_reflection(
    learn_dir: &std::path::Path,
    gate_name: &str,
    gate_output: &str,
    plan_id: &str,
    task_id: &str,
    iteration: u32,
    cumulative_reflection_cost_usd: &mut f64,
    caller: &dyn roko_core::ModelCaller,
    files_changed: &[String],
)
```

Replace the `proposed_lesson` synthesis block (lines 12172–12190) with:

```rust
// Attempt an LLM-generated lesson when under the cost guard.
// Fall back to the deterministic synthesis if the call fails or is too expensive.
let proposed_lesson = if *cumulative_reflection_cost_usd < REFLECTION_COST_GUARD_USD {
    match spawn_reflection_agent(
        gate_name,
        &error_digest,
        files_changed,
        iteration,
        caller,
        500, // max output tokens
    ).await {
        Ok(lesson) => {
            // Accumulate actual cost: caller returned token usage, or use the nominal estimate.
            *cumulative_reflection_cost_usd += REFLECTION_COST_PER_OBSERVATION_USD;
            lesson
        }
        Err(e) => {
            warn!(error = %e, "reflection agent call failed — using deterministic lesson");
            // Fall back to deterministic synthesis (the code that was previously here).
            deterministic_lesson_from_patterns(&failure_pattern_ids, gate_name, iteration, gate_output)
        }
    }
} else {
    deterministic_lesson_from_patterns(&failure_pattern_ids, gate_name, iteration, gate_output)
};
```

Extract the deterministic lesson synthesis into a helper function (to keep the code clean):

```rust
fn deterministic_lesson_from_patterns(
    failure_pattern_ids: &[String],
    gate_name: &str,
    iteration: u32,
    gate_output: &str,
) -> String {
    if failure_pattern_ids.is_empty() {
        format!(
            "Investigate {gate_name} failure on attempt {iteration} before retrying; \
             error: {}",
            gate_output
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("unknown error")
                .chars()
                .take(120)
                .collect::<String>()
        )
    } else {
        format!(
            "On attempt {iteration} the {gate_name} gate failed with pattern(s): {}. \
             Address the root cause before retrying.",
            failure_pattern_ids.join(", ")
        )
    }
}
```

### Step 4: Update the call site in the runner event loop

At `event_loop.rs` line 4598, change the call from:

```rust
// Old (sync):
record_gate_failure_reflection(
    &config.layout.learn_dir(),
    gate_name,
    &completion.output,
    &completion.plan_id,
    &completion.task_id,
    failed_attempt,
    &mut state.cumulative_reflection_cost_usd,
);
```

To:

```rust
// New (async):
record_gate_failure_reflection(
    &config.layout.learn_dir(),
    gate_name,
    &completion.output,
    &completion.plan_id,
    &completion.task_id,
    failed_attempt,
    &mut state.cumulative_reflection_cost_usd,
    model_caller.as_ref(),          // whatever holds the ModelCaller in runner context
    &completion.files_changed,      // list of files touched in this attempt
).await;
```

To find the `ModelCaller` reference in the runner context, search for where agents are dispatched with a caller — it will be in a `dispatch_agent_with(ctx, caller, ...)` pattern or similar. The `files_changed` field either exists on the completion struct or needs to be tracked as files written/modified during the agent's turn (grep for `files_changed` in the runner to determine the right field name).

If `files_changed` is not already tracked: in the agent event handler branch (around line 2658), collect the list of file paths from tool call results (look for `Write`, `Edit`, `multi_edit` tool call results and extract their `path` arguments). Store this as a `Vec<String>` on the task attempt state.

### Step 5: Add unit tests

Add tests to `event_loop.rs`'s existing test module (around line 22496 where reflection tests are):

```rust
#[tokio::test]
async fn reflection_agent_lesson_is_natural_language() {
    // Use a mock ModelCaller that returns a fixed lesson string.
    struct MockCaller;
    impl roko_core::ModelCaller for MockCaller {
        async fn call(&self, _req: roko_core::ModelCallRequest) -> roko_core::Result<roko_core::ModelCallResponse> {
            Ok(roko_core::ModelCallResponse {
                content: vec![roko_core::ContentBlock::Text {
                    text: "The type mismatch on line 42 indicates a missing impl. \
                           Add `impl From<u32> for MyType` before retrying.".to_string(),
                }],
                ..Default::default()
            })
        }
    }

    let lesson = spawn_reflection_agent(
        "compile",
        "e0308 type mismatch",
        &["src/lib.rs".to_string()],
        2,
        &MockCaller,
        500,
    ).await.expect("lesson");

    // The lesson should be a natural language sentence, not a pattern-code summary.
    assert!(lesson.contains("type mismatch") || lesson.contains("impl"));
    assert!(!lesson.starts_with("On attempt")); // not the deterministic fallback format
}

#[tokio::test]
async fn reflection_dedup_skips_model_call_for_same_digest() {
    // Write a reflection record with a specific pattern ID.
    // Then call record_gate_failure_reflection with the same gate output.
    // The deduplication check should prevent the model call.
    // Verified by using a MockCaller that panics if called.
    // (This test verifies the guard logic, not the LLM output.)
}
```

## Acceptance Criteria

1. After a gate failure, a reflection record in `.roko/learn/post-gate-reflections.json` contains a `proposed_lesson` that is a full natural-language sentence (at least 20 words), written by a model call, not in the deterministic format "On attempt N the X gate failed with pattern(s): Y".
2. The lesson from the most recent reflection for a given task appears in the retry agent's system prompt under a "### Lessons from previous attempt on this gate" header — already injected by the existing `lessons_from_post_gate_reflections` path at line 4558.
3. Deduplication: a second gate failure with the same normalised error digest on the same gate does not spawn a second model call. Verified by a unit test with a mock caller that panics if called twice.
4. The cumulative cost guard (`REFLECTION_COST_GUARD_USD = 0.05`) prevents model calls once exceeded; subsequent gate failures use the deterministic fallback lesson.
5. `spawn_reflection_agent` returns `Err(ReflectionError::ModelCallFailed)` when the caller returns an error; the runner logs at `warn!` level and uses the deterministic fallback — no panic.
6. `cargo test -p roko-learn --lib post_gate_reflection` continues to pass without modification; the `PostGateReflectionStore` and `PostGateReflectionRecord` types are unchanged.
7. `cargo test --workspace` passes with no regressions.
8. `cargo run -p roko-cli -- plan run <plan-dir> --engine runner-v2` on a plan that causes a gate failure shows "post-gate reflection lessons added to retry prompt" in debug logs.

## Verification Checklist

- [ ] Run `cargo build -p roko-cli` — should compile with the modified `record_gate_failure_reflection` signature
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` — no new warnings
- [ ] Run `cargo test -p roko-learn` — `post_gate_reflection` tests pass unchanged
- [ ] Add a mock-caller unit test for `spawn_reflection_agent` and run it: `cargo test -p roko-cli --lib reflection_agent_lesson_is_natural_language`
- [ ] On a plan with a known compile failure (e.g. a syntax error), run `RUST_LOG=debug cargo run -p roko-cli -- plan run <dir> --engine runner-v2`
- [ ] Check `.roko/learn/post-gate-reflections.json` — `proposed_lesson` field should be a paragraph, not "On attempt N..."
- [ ] Check that the retry agent's prompt (visible at `RUST_LOG=roko_cli=debug`) includes "Lessons from previous attempt on this gate"
- [ ] Confirm the second attempt at the same gate re-uses the existing lesson (dedup path)
- [ ] Run `cargo test --workspace` to confirm no regressions

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Add `ReflectionError`, `spawn_reflection_agent`, `deterministic_lesson_from_patterns`; change `record_gate_failure_reflection` to `async fn` with `caller` and `files_changed` params; update call site at line 4598 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/post_gate_reflection.rs` | No changes — existing types and store are used as-is |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs` | No structural changes — `cumulative_reflection_cost_usd` field already exists at line 243 |
