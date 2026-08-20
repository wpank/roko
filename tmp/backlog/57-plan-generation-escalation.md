# 57 — Plan Generation Crash Retry and Escalation

**Priority**: P2 — when the plan generation agent crashes on the first call, `roko prd plan` gives up immediately with an opaque byte-count error; the crash classifier and retry loop that exist in the rest of the codebase are not wired into the plan generation path
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli/` (`roko-cli`)
**Depends on**: None

---

## Background

`roko prd plan <slug>` runs an agent that reads a PRD document and produces a `tasks.toml` plan file. The generation uses a multi-call pattern: one initial call captures the agent's output, then a retry loop (up to 2 attempts) re-runs the agent with a stricter prompt if the initial output contains invalid TOML.

The retry loop for invalid TOML is well-built: it escalates to a higher-tier model on each attempt, passes the previous error and output back in the retry prompt, and respects `EscalationConfig` settings. However, this retry loop only runs when the initial agent call exits with code 0 but produces invalid TOML. When the agent exits with a non-zero code — which happens on rate limits, auth failures, network timeouts, or internal errors — the function returns immediately with a message like `"plan generation agent failed with exit code 1 (12345 bytes of output)"` and the actual cause (buried in the captured output) is discarded.

There is already a crash classifier in the codebase: `classify_agent_crash()` in `crates/roko-cli/src/agent_exec.rs` returns an `AgentCrashClass` (`AuthenticationError`, `RateLimited`, `ContextOverflow`, `ModelNotFound`, `NetworkError`, `Unknown`) with associated `is_retriable()` and `recovery_hint()` methods. This classifier is used in `event_loop.rs` and `do_cmd.rs` but is not imported or called in `prd.rs`.

The `roko plan generate` command (in `crates/roko-cli/src/commands/plan.rs`) has a completely separate gap: it calls `run_agent_logged()` once, and if the agent crashes or writes invalid TOML files, it prints a warning and returns the exit code with no retry.

---

## Current State

1. The initial agent call in `roko prd plan` is at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` lines 1318-1329. It calls `run_agent_capture_silent()` and returns `(exit_code, output)`.

2. The crash branch is at line 1343:
   ```rust
   if exit_code != 0 {
       let _ = persist_capture_episode(...).await;
       return Err(anyhow!(
           "plan generation agent failed with exit code {exit_code} \
            ({} bytes of output)",
           output.len()
       ));
   }
   ```
   This returns immediately with no classification and no retry.

3. The TOML-validation retry loop starts at line 1414 with `if validated_toml.is_err()`. It runs up to `max_retries = 2` attempts with model escalation. This loop is only entered when `exit_code == 0`.

4. The crash classifier `classify_agent_crash(stderr: &str) -> AgentCrashClass` is defined at line 365 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_exec.rs`. `AgentCrashClass::is_retriable()` returns `true` for `RateLimited` and `NetworkError`. `AgentCrashClass::recovery_hint()` returns a human-readable string for each class.

5. `prd.rs` imports from `agent_exec` at line 27-30 but does not import `AgentCrashClass` or `classify_agent_crash`.

6. The `EscalationConfig` struct is at line 360 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs`. Its `max_retries` field defaults to 3 (`default_max_retries()` at line 370), but the TOML-validation retry loop in `prd.rs` hardcodes `max_retries = 2` rather than using the config value.

7. The `next_tier_model()` function is at line 1142 of `prd.rs`. It reads from `tier_models` config and falls back to `DEFAULT_ESCALATION_CHAIN = ["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"]` (line 1130).

8. `roko plan generate` uses `run_agent_logged()` at line 1105 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`. If the exit code is non-zero or the written `tasks.toml` files are invalid, the function exits with a warning at lines 1145-1150. `classify_agent_crash()` is not imported or used in `commands/plan.rs`.

---

## Implementation Plan

### Change A: Classify and print crash reason in the initial crash branch

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`, at line 1343:

1. Add `classify_agent_crash` and `AgentCrashClass` to the import from `crate::agent_exec` at line 27.
2. In the `if exit_code != 0` branch, call `classify_agent_crash(&output)` to classify the failure.
3. Replace the generic error string with a classified one:

```rust
if exit_code != 0 {
    let crash_class = classify_agent_crash(&output);
    let _ = persist_capture_episode(...).await;
    // For auth errors, fail fast with an actionable message.
    if matches!(crash_class, AgentCrashClass::AuthenticationError) {
        return Err(anyhow!(
            "plan generation agent auth failure: {} (exit code {exit_code})\n\
             Hint: {}",
            crash_class.recovery_hint(),
            "Check ANTHROPIC_API_KEY or your provider config in roko.toml"
        ));
    }
    // For retriable errors, fall through to the retry loop (Change B).
    // For non-retriable unknown errors, still return an error but with the hint.
    if !crash_class.is_retriable() {
        return Err(anyhow!(
            "plan generation agent failed (exit code {exit_code}, {}): {}\n\
             Hint: {}",
            format!("{crash_class:?}"),
            &output[..output.len().min(500)],
            crash_class.recovery_hint()
        ));
    }
    // crash_class.is_retriable() == true; fall through to Change B retry loop.
}
```

### Change B: Add a retry loop for the initial crash case

After the fast-fail for auth errors in Change A, add a retry loop for retriable crashes (rate limit, network error). Mirror the structure of the TOML-validation retry loop at line 1414, reusing the same `max_retries` variable and the same `EscalationConfig`:

```rust
let max_retries = resolved.config.agent.escalation.max_retries.min(3);
let mut last_exit_code = exit_code;
let mut last_output = output.clone();

for attempt in 1..=max_retries {
    let crash_class = classify_agent_crash(&last_output);
    eprintln!(
        "  plan generation agent {} (attempt {}/{}) — retrying…",
        crash_class.recovery_hint(), attempt, max_retries + 1,
    );
    // No model escalation for rate-limit/network — same model, just retry.
    let retry_result = run_agent_capture_silent(AgentExecOpts {
        // same options as the initial call
    }).await?;
    last_exit_code = retry_result.0;
    last_output = retry_result.1;
    if last_exit_code == 0 {
        break; // Proceed to TOML extraction below.
    }
}
if last_exit_code != 0 {
    return Err(anyhow!(
        "plan generation agent failed after {} attempts (last exit code {last_exit_code}): {}",
        max_retries + 1,
        classify_agent_crash(&last_output).recovery_hint()
    ));
}
// Replace `output` with `last_output` for the TOML extraction step.
let output = last_output;
```

### Change C: Improve crash error message in `roko plan generate`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`, at line 1105, after the `run_agent_logged()` call:

1. Import `classify_agent_crash` from `roko_cli::agent_exec`.
2. If `exit_code != 0`, classify the failure using the `AgentExecEpisode` output that `run_agent_logged()` prints. Because `run_agent_logged` streams output to the terminal, the captured text is not directly available; use the episode output from the most recent episode file as a proxy.

A simpler approach: add a `last_output: Option<String>` return value to `run_agent_logged()` (currently it returns `Result<i32>`), or use a shared buffer. If that is invasive, just improve the error message to include the exit code and a hint to check the episode log:

```rust
if exit_code != EXIT_SUCCESS {
    eprintln!(
        "plan generate: agent exited with code {exit_code}. \
         Check the latest episode in .roko/episodes.jsonl for details."
    );
    // Retry loop is deferred — see tmp/backlog/57 for scope.
}
```

This is the minimum for this ticket. A full retry loop for `plan generate` requires either a different agent invocation strategy (to capture output) or a separate design pass; scope it as a follow-up rather than blocking this ticket.

---

## Acceptance Criteria

1. When `roko prd plan <slug>` fails on the first attempt with a non-zero exit code:
   - The error message includes the classified crash reason and a recovery hint, not just the byte count.
   - For auth errors, the command fails immediately with an actionable message pointing to the API key.
   - For rate-limit and network errors, the command retries up to `max_retries` times and prints a classified message on each attempt.
2. The retry prompt for crash retries uses the same options as the initial call (same model unless escalated, same system prompt).
3. The existing TOML-validation retry loop (line 1414 of `prd.rs`) is not changed or broken by this work.
4. `roko plan generate` prints a more actionable error message when the agent exits non-zero (exit code + hint to check episodes log).
5. `cargo test -p roko-cli` passes with zero failures.
6. `cargo clippy -p roko-cli -- -D warnings` is clean.

---

## Verification Checklist

- [ ] Set an invalid `ANTHROPIC_API_KEY`. Run `roko prd plan demo`. Confirm the error message says "auth failure" and mentions checking the API key — not "12345 bytes of output".
- [ ] Simulate a rate limit by temporarily setting a model name that returns a `429` response. Run `roko prd plan demo`. Confirm the command retries and prints "rate limited — retrying…" on each attempt.
- [ ] Run a normal `roko prd plan demo` with valid credentials. Confirm the happy path still works.
- [ ] Run `cargo test -p roko-cli` — all tests pass.
- [ ] Run `cargo clippy -p roko-cli -- -D warnings` — clean.

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` | Import `classify_agent_crash` and `AgentCrashClass` at line 27; replace the immediate return at line 1343 with classified error + retry loop for retriable crashes |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` | Improve non-zero exit message after `run_agent_logged()` at line 1105 to include exit code and hint to check episodes log |
