# Plan Generation Escalation

**Priority**: P2
**Size**: S (1-2 days)

---

## Problem

The `roko prd plan` retry loop has model escalation wired for TOML
extraction/validation failures, but two real gaps remain:

1. **Initial agent crash has no retry.** When the first `run_agent_capture_silent`
   call returns a non-zero exit code (lines 1320-1338 in `prd.rs`), the function
   returns immediately with a raw byte-count error. No retry, no escalation. The
   user sees: `plan generation agent failed with exit code 1 (12345 bytes of
   output)`. The bytes of agent stderr (rate-limit message, auth rejection,
   upstream timeout) are silently discarded; the caller learns nothing actionable.

2. **`roko plan generate` has no retry at all.** The `commands/plan.rs` path
   calls `run_agent_logged` once, forwards its exit code, and prints a
   `"plan generate: one or more generated tasks.toml files failed TOML
   validation"` warning when the file is invalid. There is no retry loop,
   no model escalation, and no error classification.

### What already exists

| Component | Location | Status |
|---|---|---|
| Retry loop (TOML validation path) | `prd.rs:1391-1498` | EXISTS — up to 2 retries with `escalate_model` |
| `next_tier_model()` | `prd.rs:1135` | EXISTS — haiku → sonnet → opus, config-driven |
| `EscalationConfig` | `config.rs:358-385` | EXISTS — `max_retries: 3`, `escalate_model: true` by default |
| `DEFAULT_ESCALATION_CHAIN` | `prd.rs:1127` | EXISTS — `["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"]` |
| Error classification for stderr | `roko-agent/src/process/stderr.rs` | EXISTS — classifies benign subprocess noise, not propagated to callers |
| Initial crash error message | `prd.rs:1334-1338` | EXISTS — reports exit code + byte count only |
| `run_agent_capture_silent` | `agent_exec.rs:90` | EXISTS — returns `(exit_code, rendered)`, exit code is `0` or `1` based on `result.success` |

### What is missing

1. **Crash classification.** When `exit_code != 0`, `run_agent_capture_silent`
   returns the rendered text (which may contain diagnostic content from the
   agent or provider). The caller at `prd.rs:1320` discards it. There is no
   inspection of the output for rate-limit signals (`"rate limit"`,
   `"429"`, `"overloaded"`), auth failures (`"401"`, `"Invalid API key"`,
   `"authentication"`), or upstream timeouts (`"timeout"`, `"timed out"`).
   All crashes are treated identically.

2. **Retry on initial crash.** The retry loop at `prd.rs:1391` only runs when
   `validated_toml.is_err()`, which requires a successful agent call that
   returned invalid TOML. A crash before any output skips the retry loop
   entirely. The crash case should enter its own retry sequence with
   appropriate backoff and escalation gating.

3. **Error-informed retry prompt.** The retry prompt at `prd.rs:1439-1445`
   includes the previous error string and up to 2000 chars of the previous
   invalid output. This is good. But when the error is an agent crash, the
   retry prompt still uses the full original generation prompt, not a prompt
   that communicates what went wrong (e.g., "previous attempt timed out —
   generate a shorter, simpler plan").

4. **`roko plan generate` retry gap.** `commands/plan.rs:1050-1099` calls
   `run_agent_logged` once. If that agent crashes or writes invalid TOML, the
   user is told via a warning, and the command exits with the agent's exit code.
   No retry, no escalation. This path has a different shape (the agent writes
   files directly, not via stdout TOML capture), so the fix is different: detect
   invalid output files after the run and offer a retry prompt that includes the
   validation errors.

---

## Proposed changes

### Change A: crash classification helper

Add a small function `classify_agent_crash(output: &str, exit_code: i32) -> CrashKind`
to `agent_exec.rs` or a new `prd_helpers.rs`. `CrashKind` is an enum:

```rust
pub enum CrashKind {
    RateLimit,       // 429 / "rate limit" / "overloaded"
    Auth,            // 401 / "invalid api key" / "authentication"
    Timeout,         // "timed out" / "timeout" / duration >= 5m
    InvalidOutput,   // agent exited 0 but output is malformed
    Unknown,         // none of the above
}
```

Classification uses case-insensitive substring matching on the rendered output.
This is the same approach as `classify_benign_stderr` in `roko-agent/src/process/stderr.rs`,
extended to the caller side.

The escalation decision then becomes: escalate only on `InvalidOutput`; on
`RateLimit` and `Timeout`, retry same model with a short wait; on `Auth`, fail
fast with an actionable message.

### Change B: retry on initial crash in `prd plan`

In `generate_plan_from_prd_with_outcome`, instead of returning immediately on
non-zero exit, enter a retry loop bounded by `EscalationConfig::max_retries`
(currently defaults to 3, but `prd plan` currently uses a hardcoded `2`).
The structure mirrors the existing TOML-validation retry loop at `prd.rs:1391`.

```
initial call (any model)
  → exit 0, valid TOML    → done
  → exit 0, invalid TOML  → existing retry loop (up to 2 retries + escalation)
  → exit != 0             → NEW: classify crash, retry:
      RateLimit/Timeout   → retry same model (no escalation)
      InvalidOutput       → retry, escalate model
      Auth                → fail fast, print actionable message
      Unknown             → retry same model, log warning
```

The crash-retry prompt should be compact: reproduce the original PRD slug and
the nature of the failure, and instruct the agent to generate a shorter plan if
the failure looks like a context-length or timeout issue.

### Change C: user-facing error messages

Replace the current:

```
plan generation agent failed with exit code 1 (12345 bytes of output)
```

With classified output:

```
plan generation agent hit a rate limit (attempt 1/3) — retrying with same model…
plan generation agent auth failure: invalid API key — check ANTHROPIC_API_KEY
plan generation agent timed out after 5m (attempt 1/3) — retrying…
plan generation agent returned non-zero exit (code 1, unknown cause, attempt 1/3) — retrying…
```

### Change D: `roko plan generate` retry (optional for this ticket)

`commands/plan.rs` uses `run_agent_logged` which lets the agent write files
directly. Retrofitting a retry loop here is more invasive because the agent
may have partially written files. Scope this as a follow-up; for this ticket,
improve the error message on non-zero exit to include the classified crash kind.

---

## Where to wire it

| File | Change |
|---|---|
| `crates/roko-cli/src/prd.rs` | Add crash-classification logic at the `exit_code != 0` branch (line 1320). Add retry loop for initial crash. Improve error messages. |
| `crates/roko-cli/src/agent_exec.rs` | Add `classify_agent_crash(output, exit_code) -> CrashKind` helper. |
| `crates/roko-cli/src/commands/plan.rs` | Improve non-zero exit error message with crash classification. Retry loop deferred to follow-up. |

No new crates. No config schema changes. The existing `EscalationConfig` fields
(`max_retries`, `escalate_model`) govern the new crash-retry loop too.

---

## Acceptance criteria

1. When `roko prd plan <slug>` crashes on the first attempt (simulated by setting
   an invalid model name), the command retries up to `max_retries` times and
   prints a classified error message on each attempt rather than `"N bytes of output"`.
2. When the output contains `"rate limit"`, the retry uses the same model (no
   escalation). When the output contains `"invalid api key"`, the command fails
   immediately with a message pointing to the relevant env var.
3. When escalation is triggered (invalid output, escalate_model = true), the retry
   prompt includes the previous error and the previous output snippet, matching
   the existing behavior for TOML-validation retries.
4. `cargo test -p roko-cli` passes with zero failures.
5. `cargo clippy -p roko-cli -- -D warnings` is clean.

---

## References

- `crates/roko-cli/src/prd.rs` — retry loop at lines 1391-1498; initial crash at 1320-1338
- `crates/roko-cli/src/agent_exec.rs` — `run_agent_capture_silent`, exit code mapping at line 236
- `crates/roko-cli/src/config.rs` — `EscalationConfig` at line 358
- `crates/roko-cli/src/commands/plan.rs` — no-retry `plan generate` path at lines 1050-1099
- `crates/roko-agent/src/process/stderr.rs` — `classify_benign_stderr` as the pattern to follow
