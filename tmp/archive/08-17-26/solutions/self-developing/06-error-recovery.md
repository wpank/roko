# 06: Error Recovery

## Problem

When roko fails, it prints an error and a "hint" and exits. The user is left to figure out what
to do. For a self-developing system, this is unacceptable — the system should recover or guide
recovery.

### Observed Failure Patterns

| Failure | Current behavior | What should happen |
|---------|-----------------|-------------------|
| Model can't produce valid TOML | Retry 3x with same model, give up | Escalate to stronger model |
| API key missing | Warning at startup, then opaque failure | Clear error + `roko setup` link |
| Agent exits with code 1 (93 bytes) | "agent failed with exit code 1" | Show the 93 bytes, diagnose the error type |
| Repo context timeout | Warning, continue degraded | Either wait longer or say "generating without context" |
| PRD has no content | "no actionable requirements" in model output | Tell user their PRD is empty, show how to fix |
| Plan already exists | Silent overwrite or unclear behavior | "Plan exists (3/6 done). Resume, overwrite, or view?" |

---

## Existing Error Infrastructure

### What already exists

**Gate failure classification** — `crates/roko-gate/src/compile_errors.rs:39`

The `FailureClass` enum classifies task execution failures:

```rust
pub enum FailureClass {
    SyntaxError,
    ImportError,
    TypeError,
    MissingDependencyOrFeature,
    BorrowOrLifetime,
    TestExpectationFailure,
    ExternalEnvironment,      // network, toolchain, timeout
    UnsafeStubOrPassBehavior, // agent wrote a no-op
    PromptContextInsufficiency, // agent didn't have enough context
    RoleToolPermission,       // wrong role or tool permissions
    ArchitecturalConflictRequiresReplan, // plan shape is wrong
    Unknown,
}
```

**Gate recovery actions** — `crates/roko-gate/src/compile_errors.rs:69`

```rust
pub enum GateFailureAction {
    Retry,        // retry or deterministic remediation is appropriate
    NeedsReplan,  // plan shape must be revised
    Blocked,      // external/environmental condition blocks progress
    NeedsHuman,   // human input required
}
```

**Gate failure kind** — `crates/roko-gate/src/compile_errors.rs:82`

```rust
pub enum GateFailureKind {
    Transient,   // retry immediately; 2s cooldown
    Permanent,   // code change needed before retry
    Resource,    // capacity issue, 0s cooldown (drop immediately)
    Structural,  // verification contract needs repair; 5s cooldown
}
```

**Orchestrator classification routing** — `crates/roko-cli/src/orchestrate.rs:5381`

```rust
fn gate_failure_next_action(
    attempts: u32,
    attempt_limit: u32,
    failing_verdicts: &[GateVerdictSummary],
) -> GateFailureAction {
    // String-based classification from verdict metadata:
    if has_class("role_tool_permission")         => NeedsHuman
    if has_class("external_environment")         => Blocked
    if has_class("unsafe_stub_or_pass_behavior")
    || has_class("prompt_context_insufficiency")
    || has_class("architectural_conflict_requires_replan")
    || attempts >= attempt_limit                 => NeedsReplan
    else                                         => Retry
}
```

**Gate failure replan trigger** — `crates/roko-cli/src/orchestrate.rs:5593`

When `NeedsReplan` is returned and `learning.replan_on_gate_failure = true`:
1. `build_gate_failure_plan_revision` builds a `RokoEvent::PlanRevision` with gate details and
   log tail
2. `handle_runtime_event` picks up the event and calls `replan_plan` with the architectural model
   from `[agent.tier_models.architectural]`
3. `replan_plan` finds the PRD via slug matching, calls
   `generate_plan_from_prd_with_failure_context` with a `failure_summary` string injected into
   the generation prompt

**TOML repair pipeline** — `crates/roko-cli/src/task_parser.rs:1129`

`repair_toml` runs deterministic fixes before TOML parsing:
- Strips trailing prose after last `]]`
- `split_merged_fields`: splits fields merged onto one quoted string
- `close_unclosed_strings`: closes lines with odd quote counts

**Post-generation structural fixing** — `crates/roko-cli/src/prd.rs:2071`

`validate_and_fix_generated_plan` auto-fixes:
- `meta.plan` slug mismatch (corrects to expected slug)
- Unknown field names with edit-distance ≤ 2 (renames to closest known field)
- Invalid `status` value (defaults to `"ready"`)
- Invalid `role` value (defaults to `"implementer"`)

---

## Error Classification Taxonomy

There are four distinct layers where failures occur. Each needs different recovery.

### Layer 1: Provider / network errors

These occur before any agent output is received.

| Class | Symptoms | Recovery |
|-------|----------|---------|
| `auth_missing` | No API key env var, agent exits code 1 with JSON error body | Show the JSON, tell user which env var to set |
| `auth_invalid` | 401 from provider | Show error, link to `roko config set-secret` |
| `rate_limited` | 429 from provider | Wait + retry with exponential backoff |
| `model_not_found` | 404 / `model 'x' not found` | Show error, suggest nearest configured model |
| `network_error` | Connection refused, timeout | Retry after cooldown |
| `context_overflow` | 413 / "maximum context length" | Truncate input further, escalate to model with larger context |

**Where to intercept:** `crates/roko-cli/src/agent_exec.rs` — `run_agent_capture_impl`.
Currently the exit code and raw output are passed through without classification.

### Layer 2: Agent output format errors

These occur when the agent exits 0 but the output cannot be parsed into the expected format.

| Class | Symptoms | Recovery |
|-------|----------|---------|
| `no_toml_block` | Agent output has no `` ```toml `` fence | Try fallback extraction, then stricter retry |
| `invalid_toml_syntax` | `toml::from_str` fails | `repair_toml` pass, then retry with error in context |
| `missing_required_fields` | `validate_and_fix_generated_plan` finds errors after repair | Self-heal fixable fields, escalate model for unfixable |
| `empty_output` | Agent returns 0 bytes or whitespace only | Different error: likely prompt issue or context overflow |
| `placeholder_paths` | `files = ["<crate>/src/lib.rs"]` | Structural validator exists; could auto-reject and retry |

**Where to intercept:** `crates/roko-cli/src/prd.rs:1200` — `try_extract_and_validate` closure.

### Layer 3: Semantic / correctness errors

These occur when the plan is syntactically valid but semantically wrong. Currently undetected.

| Class | Symptoms | Recovery |
|-------|----------|---------|
| `nonexistent_paths` | `files = ["crates/nonexistent/src/lib.rs"]` | Validate file paths exist (partial: `validate_file_references` in `plan_validate.rs`) |
| `duplicate_crate_creation` | Plan creates a crate that already exists | `validate_no_greenfield_duplicates` in `plan_validate.rs` catches this |
| `circular_dependencies` | `depends_on` forms a cycle | `detect_cycle_nodes` from `roko-orchestrator` is already wired in plan validation |
| `scope_too_large` | Task with `tier = "mechanical"` has `max_loc = 500` | Warn only; not enforced |
| `wrong_verify_commands` | `cargo check -p nonexistent-crate` | Not validated before execution |

**Where to intercept:** `crates/roko-cli/src/plan_validate.rs` — `validate_plans_dir_with_workdir`.
This already runs after generation (`prd.rs:1422`). Errors are logged but don't currently trigger
model escalation.

### Layer 4: Execution gate failures

These occur during `roko plan run`. The classification and recovery infrastructure already
exists (see above). Gaps:

| Class | Gap |
|-------|-----|
| `external_environment` | Classifies as `Blocked` but doesn't retry after a delay |
| `prompt_context_insufficiency` | Triggers replan but doesn't add more context to the new plan |
| `unsafe_stub_or_pass_behavior` | Triggers replan but the replan may repeat the same mistake |

---

## Auto-Recovery Flows

### Flow 1: TOML extraction failure → model escalation

**Current path:** same model × 3 attempts → hard error
**Proposed path:**

```
attempt 1: configured model (e.g. haiku)
  → TOML extraction fails
  → classify error: format_error / missing_fields
  → escalate to tier-up model (e.g. sonnet)
attempt 2: sonnet
  → TOML extraction fails
  → escalate to strongest available (e.g. opus)
attempt 3: opus
  → success OR hard error with full diagnostic
```

**Hook:** `prd.rs:1224` — replace the retry loop.

```rust
fn build_escalation_chain(base_model: &str, config: &RokoConfig) -> Vec<String> {
    // Start from base_model, look for tier-up models in [agent.tier_models]
    let tier_order = ["mechanical", "focused", "integrative", "architectural"];
    let base_tier = infer_tier_for_model(base_model, config);
    let mut chain = vec![base_model.to_string()];
    for tier in tier_order.iter().skip_while(|&&t| t != base_tier).skip(1) {
        if let Some(model) = config.agent.tier_models.get(*tier) {
            if !chain.contains(model) {
                chain.push(model.clone());
            }
        }
    }
    chain
}
```

### Flow 2: Agent crash (non-zero exit) → diagnose from output

**Current path:** `return Err(anyhow!("... ({} bytes of output)", output.len()))`
**File:** `prd.rs:1169`

**Proposed path:**

```rust
if exit_code != 0 {
    let classified = classify_agent_crash(&output);
    let output_preview: String = output.chars().take(2000).collect();
    return Err(anyhow!(
        "Agent failed (exit {exit_code}): {}\n\
         Output:\n---\n{output_preview}\n---\n\
         {}",
        classified.short_description(),
        classified.recovery_hint()
    ));
}

fn classify_agent_crash(output: &str) -> AgentCrashClass {
    if output.contains("\"type\": \"authentication_error\"") || output.contains("401") {
        AgentCrashClass::AuthFailed
    } else if output.contains("\"type\": \"invalid_request_error\"")
        && output.contains("not found")
    {
        AgentCrashClass::ModelNotFound
    } else if output.contains("429") || output.contains("rate_limit") {
        AgentCrashClass::RateLimited
    } else if output.contains("context_length_exceeded")
        || output.contains("maximum context length")
    {
        AgentCrashClass::ContextOverflow
    } else {
        AgentCrashClass::Unknown
    }
}
```

### Flow 3: Repo context timeout → graceful degradation

**Current path:** `eprintln!("warning: repository context unavailable...")` then continues
**File:** `prd.rs:1063`

This is already handled correctly — generation continues without repo context. The only gap is
that the warning doesn't tell the user what was lost or how to fix it.

**Proposed fix** (minimal):
```rust
eprintln!(
    "  warning: skipping repo context ({}). \
     The generated plan may contain incorrect file paths.\n\
     To fix: ensure the workspace has a 'crates/' or 'src/' directory \
     and the keywords [{kws}] match real code.",
    err,
    kws = prd_feature_keywords.join(", ")
);
```

### Flow 4: Gate failure → classify → route to correct recovery

This flow is largely wired. The classification taxonomy in `compile_errors.rs` maps directly to
recovery actions. The gaps are:

**Gap 1:** `ExternalEnvironment` → `Blocked` but no automatic retry after delay.

```rust
// In orchestrate.rs, handle_runtime_event:
GateFailureAction::Blocked => {
    // Currently: mark task as blocked, move on.
    // Proposed: schedule retry after cooldown if the block is transient.
    if is_transient_external_block(&failing_verdicts) {
        schedule_retry_after(task_id, Duration::from_secs(30));
    }
}
```

**Gap 2:** Replan doesn't pass the failure classification to `generate_plan_from_prd_with_failure_context`.

**File:** `orchestrate.rs:5569` calls `replan_plan`, which calls `generate_plan_from_prd_with_failure_context` with a `failure_summary` string. The summary includes gate details and log tail. This is correct. The gap is that the generated plan doesn't adjust its `context.anti_patterns` to prevent the same mistake.

---

## Concrete Rust Code Sketches

### AgentCrashClass enum

```rust
// New type in crates/roko-cli/src/agent_exec.rs (or plan generation module)
#[derive(Debug)]
enum AgentCrashClass {
    AuthFailed,
    ModelNotFound { model: String },
    RateLimited,
    ContextOverflow,
    NetworkError,
    Unknown { raw_preview: String },
}

impl AgentCrashClass {
    fn short_description(&self) -> &'static str {
        match self {
            Self::AuthFailed => "authentication failed",
            Self::ModelNotFound { .. } => "model not found",
            Self::RateLimited => "rate limited",
            Self::ContextOverflow => "context length exceeded",
            Self::NetworkError => "network error",
            Self::Unknown { .. } => "unknown agent error",
        }
    }

    fn recovery_hint(&self) -> String {
        match self {
            Self::AuthFailed => "Check your API key: roko config check-secrets".to_string(),
            Self::ModelNotFound { model } => format!(
                "Model '{model}' is not available. \
                 Check configured models: roko config models list"
            ),
            Self::RateLimited => "Rate limited. Wait a moment and retry.".to_string(),
            Self::ContextOverflow => {
                "The PRD or repo context is too large for this model. \
                 Try: roko prd plan <slug> --model <larger-context-model>".to_string()
            }
            Self::NetworkError => "Check your network connection.".to_string(),
            Self::Unknown { raw_preview } => format!(
                "Inspect the raw output for clues.\n\
                 Raw output preview:\n{raw_preview}"
            ),
        }
    }
}
```

### Plan generation error taxonomy

```rust
// New enum for plan generation failures
#[derive(Debug)]
enum PlanGenError {
    AgentCrashed { exit_code: i32, class: AgentCrashClass },
    EmptyOutput,
    NoTomlBlock { output_preview: String },
    InvalidTomlSyntax { err: toml::de::Error, raw: String },
    MissingRequiredFields { errors: Vec<String>, raw: String },
    AllRetriesFailed { attempts: u32, last_err: String },
}

impl PlanGenError {
    fn is_escalatable(&self) -> bool {
        // These errors benefit from a stronger model
        matches!(
            self,
            Self::NoTomlBlock { .. }
                | Self::InvalidTomlSyntax { .. }
                | Self::MissingRequiredFields { .. }
                | Self::EmptyOutput
        )
    }

    fn actionable_message(&self, slug: &str, model: &str) -> String {
        match self {
            Self::AgentCrashed { exit_code, class } => format!(
                "Agent failed (exit {exit_code}): {}\n{}",
                class.short_description(),
                class.recovery_hint()
            ),
            Self::MissingRequiredFields { errors, .. } => format!(
                "Generated plan has schema errors (model: {model}):\n{}\n\n\
                 What to do:\n  \
                   1. Use a stronger model: roko prd plan {slug} --model claude-sonnet-4-6\n  \
                   2. Fix manually: $EDITOR plans/{slug}/tasks.toml\n  \
                   3. Validate: roko plan validate plans/",
                errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"),
            ),
            // ...
        }
    }
}
```

### Escalation chain construction

```rust
// In prd.rs, replace the retry loop:
async fn generate_with_escalation(
    opts: &GenerationOpts<'_>,
    slug: &str,
    escalation_chain: &[String],
) -> Result<String> {
    let mut last_err = String::new();

    for (attempt, model) in escalation_chain.iter().enumerate() {
        let is_retry = attempt > 0;
        if is_retry {
            eprintln!(
                "  Escalating to {} (attempt {}/{})...",
                model,
                attempt + 1,
                escalation_chain.len()
            );
        }

        let retry_opts = if is_retry {
            // Include previous error in retry prompt
            build_retry_prompt(slug, &last_err)
        } else {
            opts.task_prompt.to_string()
        };

        match run_agent_capture_silent(AgentExecOpts {
            model: Some(model),
            prompt: &retry_opts,
            ..*opts.agent_opts
        })
        .await
        {
            Ok((0, output)) if !output.trim().is_empty() => {
                match try_extract_and_validate(&output, slug, &opts.models, opts.default_model) {
                    Ok(toml) => return Ok(toml),
                    Err(err) => {
                        eprintln!("  {model}: TOML extraction failed: {err}");
                        last_err = err;
                    }
                }
            }
            Ok((code, output)) => {
                let class = classify_agent_crash(&output);
                last_err = class.short_description().to_string();
                eprintln!("  {model}: failed (exit {code}): {}", class.short_description());
                // Don't escalate for auth/network errors — escalating model won't help
                if matches!(class, AgentCrashClass::AuthFailed | AgentCrashClass::NetworkError) {
                    return Err(anyhow!("{}", class.actionable_message()));
                }
            }
            Err(err) => {
                last_err = err.to_string();
                eprintln!("  {model}: agent error: {err}");
            }
        }
    }

    Err(anyhow!(
        "Plan generation failed after {} attempts.\nLast error: {last_err}\n\
         Tried models: {}\n\
         What to do:\n  \
           1. roko prd plan {slug} --model claude-opus-4-6\n  \
           2. $EDITOR plans/{slug}/tasks.toml",
        escalation_chain.len(),
        escalation_chain.join(" → "),
    ))
}
```

---

## Implementation Priority

| # | Fix | Impact | Effort | File | Line |
|---|-----|--------|--------|------|------|
| 1 | Show raw output when agent crashes | High | Trivial | `prd.rs` | 1169 |
| 2 | Classify agent crash from output | High | Small | `agent_exec.rs` | new fn |
| 3 | Add prev error to retry prompt | High | Trivial | `prd.rs` | 1240 |
| 4 | Model escalation in retry loop | High | Medium | `prd.rs` | 1224 |
| 5 | Self-heal missing verify fields | Medium | Small | `prd.rs` | 2071 |
| 6 | Improve repo context missing warning | Low | Trivial | `prd.rs` | 1063 |
| 7 | `ExternalEnvironment` retry-after-delay | Low | Medium | `orchestrate.rs` | 5619 |

Items 1 and 3 are single-line fixes that can ship immediately. Item 4 (escalation) is the
structural gap that makes roko actually recover rather than just give better errors.
