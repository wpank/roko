# 02: Plan Generation UX

## Problem

`roko prd plan` is the critical bridge between "I have an idea" and "roko executes it." It fails
constantly and gives no useful feedback.

### Failure Modes Observed

1. **TOML validation failures** — Model outputs TOML with missing fields, retries 3x with the same
   weak model, then gives up.
2. **"93 bytes of output"** — Agent fails instantly, no way to see why. Was it an auth error? Rate
   limit? Invalid request?
3. **"repo context build timed out"** — What does this mean? Is it blocking? Does it affect output
   quality?
4. **"repository context not verified for keywords"** — Warning printed but unclear what action
   user should take.
5. **Model too weak** — `gpt-5.4-mini` can't produce valid TOML, but roko doesn't suggest using a
   better model.

### Actual Transcript

```
  Plan TOML extraction failed (attempt 1/3), retrying with stricter prompt...
  Plan TOML extraction failed (attempt 2/3), retrying with stricter prompt...
error: Plan generation failed after retries: generated plan TOML has 2 validation error(s):
  - T4 verify[1]: missing required field 'phase'
  - T4 verify[1]: missing required field 'command'
hint: Try again, or create plans/cursor-composer-backend/tasks.toml manually.
hint: The model output TOML without proper fencing. Try a more capable model.
```

The user's only options: "try again" (same result) or "create it manually" (defeats the purpose).

---

## Exact Code Paths

### Entry point: `roko prd plan <slug>`

**File:** `crates/roko-cli/src/commands/prd.rs:744`

```rust
PrdCmd::Plan { slug, dry_run } => {
    let model_key = roko_cli::model_selection::resolve_effective_model_key(
        &workdir,
        cli.model.clone(),
        Some("strategist"),   // role used for plan generation
        "prd plan",
    )?;
    let _generated_plans_root = roko_cli::prd::generate_plan_from_prd_with_model(
        &slug,
        &prd_path,
        dry_run,
        Some(model_key.as_str()),
    )
    .await?;
}
```

The role `"strategist"` is used for model routing. If `[agent.roles.strategist.model]` is set in
`roko.toml`, that model is used. Otherwise falls through the precedence chain.

### Model selection precedence chain

**File:** `crates/roko-cli/src/model_selection.rs:214`

The precedence order (highest to lowest):

1. `--model` CLI flag (`SelectionSource::CliOverride`)
2. `--provider` CLI flag (`SelectionSource::ProviderOverride`)
3. Task `model_hint` field (never set by generator; `SelectionSource::TaskModel`)
4. `[agent.roles.strategist.model]` in `roko.toml` (`SelectionSource::RoleConfig`)
5. Cascade router (`SelectionSource::CascadeRouter`)
6. `[agent.default_model]` in `roko.toml` (`SelectionSource::ProjectDefault`)
7. Built-in default from `RokoConfig::default()` (`SelectionSource::BuiltInDefault`)

**There is no automatic escalation.** If step 7 selects Haiku and Haiku fails, roko retries with
Haiku again.

### The generation function

**File:** `crates/roko-cli/src/prd.rs:993` — `generate_plan_from_prd_with_outcome`

Key steps:

1. Read PRD content, parse frontmatter to get `plan_template` field (`prd.rs:1006`)
2. Resolve template kind (Default/Compact/Strict) from frontmatter (`prd.rs:1007-1008`)
3. Build repo context by scanning codebase for PRD keywords (`prd.rs:1051-1073`)
4. Truncate PRD content to 8000 chars if too long (`prd.rs:1082-1091`)
5. Build `task_prompt` with TOML quality checklist inline (`prd.rs:1093-1120`)
6. Call `run_agent_capture_silent` with `allowed_tools: Some("Read,Grep,Glob")` (`prd.rs:1130`)
7. Extract TOML from output, validate/fix, retry up to 2 more times (`prd.rs:1200-1290`)
8. Write `plans/<slug>/tasks.toml` atomically (`prd.rs:1292-1321`)
9. Validate the written plan with `validate_plans_dir_with_workdir` (`prd.rs:1422`)

### Fenced block extraction

**File:** `crates/roko-cli/src/prd.rs:1749` — `extract_fenced_block`

Tries in order:
1. `` ```toml `` fenced block
2. `` ```tasks.toml `` fenced block
3. `extract_toml_content_fallback` — scans for `[meta]` header and grabs to end of file

### Deterministic TOML repair

**File:** `crates/roko-cli/src/task_parser.rs:1129` — `repair_toml`

Runs before TOML parsing on every attempt:
- Strips trailing prose after last `]]`
- Splits merged fields (e.g. `"status = "ready"max_loc = 20"` → two lines)
- Closes unclosed string literals (odd quote count on a line → append `"`)

### Structural validation + field fixing

**File:** `crates/roko-cli/src/prd.rs:2071` — `validate_and_fix_generated_plan`

Required `[meta]` fields: `plan`, `total`, `status`
Required `[[task]]` fields: `id`, `title`, `status`, `role`, `tier`

Auto-fixes applied silently:
- `meta.plan` mismatched slug → corrected to expected slug
- Unknown field name with edit-distance ≤ 2 → renamed to closest known field
- Invalid `status` value → defaulted to `"ready"`
- Invalid `role` value → defaulted to `"implementer"`
- Missing `model_hint` → not added (it's forbidden)

**Not auto-fixed (becomes hard error):**
- Missing `[meta]` section
- Missing required fields (`id`, `title`, `status`, `role`, `tier`)
- Empty required field values
- TOML syntax errors (caught before this stage)

### Retry loop

**File:** `crates/roko-cli/src/prd.rs:1224`

```
attempt 0: original agent call with full prompt
attempt 1 (retry): same model, stricter prompt ("output ONLY the ```toml block")
attempt 2 (retry): same model, stricter prompt (same)
→ all failed → hard error
```

The retry prompt (`prd.rs:1240-1245`) asks the model to re-output only the TOML block with no
preamble. It does NOT pass the previous invalid output back to the model. It does NOT escalate to
a stronger model.

### Plan template system

**File:** `crates/roko-cli/src/plan_generate.rs:26`

PRD frontmatter field `plan_template` selects:

| Template | Max tasks | Default tier | Gate strictness |
|----------|-----------|--------------|-----------------|
| `default` | 20 | `focused` | standard |
| `compact` | 12 | `mechanical` | standard |
| `strict` | 8 | `integrative` | strict |

The generator system prompt (`PLAN_GENERATOR_SYSTEM_PROMPT`, `plan_generate.rs:154`) is ~3KB of
structured instructions. It explicitly says `NEVER set model_hint`. The `tier` field drives model
selection at execution time, not `model_hint`.

### Gate failure replan path

**File:** `crates/roko-cli/src/orchestrate.rs:5479` — `build_gate_failure_plan_revision`

When a task hits gate failures during execution (not during generation):
1. `gate_failure_next_action` classifies the failure (`orchestrate.rs:5381`)
2. If `NeedsReplan` → emit `RokoEvent::PlanRevision`
3. `handle_runtime_event` calls `replan_plan` with the architectural model
   (`orchestrate.rs:5562-5569`)
4. `replan_plan` finds the PRD, calls `generate_plan_from_prd_with_failure_context` with a failure
   summary injected into the prompt (`prd.rs:980-990`)

`replan_on_gate_failure` must be `true` in `[learning]` config for this to trigger.

---

## Root Causes

### Why weak models fail at plan generation

TOML generation is structurally harder than prose. The model must:
- Produce syntactically valid TOML (no trailing commas, correct quoting, correct array syntax)
- Use `[[task]]` array-of-tables syntax (not `[task]` or `tasks = [{...}]`)
- Include multiple nested tables: `[task.context]`, `[[task.verify]]`
- Keep `meta.plan` exactly matching the slug argument
- Never use placeholder paths like `<crate>` or glob patterns in `files = [...]`
- Respect the `role` and `tier` vocabulary exactly

Smaller models (Haiku-class, mini variants) exhibit these specific failure modes:
- Outputting `tasks.toml` content without fences (caught by `extract_toml_content_fallback`)
- Using `[task]` instead of `[[task]]` (TOML parse error)
- Merging multiple fields onto one line (caught by `split_merged_fields`)
- Unclosed string literals when paths contain colons (caught by `close_unclosed_strings`)
- Missing `[[task.verify]]` entries or providing them without `phase`/`command`
- Setting `model_hint` despite being explicitly told not to (auto-removed if close match found)
- Hallucinating crate paths that don't exist (not caught by current validation)

Token budget is also an issue. A 20-task plan with full context is ~3000-4000 tokens of output.
Mini models at 8k context have barely enough room when the system prompt + PRD content + repo
context is factored in. The 8000-char PRD truncation (`prd.rs:1082`) was added to mitigate this.

### Auto-escalation design gap

The retry loop (`prd.rs:1224`) uses `effective_model` for all retries — the same model that
already failed. There is no escalation tier. The `PlanTemplateKind::default_model_tier()` returns
`"focused"` but this is advisory text in the prompt, not a model selection constraint.

The cascade router (`model_selection.rs:262`) is not consulted during retries; it's only consulted
once at the start of the command. The cascade router has no feedback from plan-generation failures.

---

## Proposed Solutions

### S1: Auto-escalate on TOML extraction failure

The retry loop at `prd.rs:1224` should escalate the model on each retry rather than repeating
the same call.

**Design:**

```rust
// In generate_plan_from_prd_with_outcome, replace the retry loop:
let escalation_models = build_escalation_chain(effective_model, &resolved.config);
// e.g. ["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"]
// or whatever is configured in [agent.tier_models]

for (attempt, escalated_model) in escalation_models.iter().enumerate().skip(1) {
    if validated_toml.is_ok() { break; }
    eprintln!(
        "  TOML extraction failed ({err}). Escalating to {escalated_model} (attempt {}/{})...",
        attempt + 1, escalation_models.len()
    );
    let retry_result = run_agent_capture_silent(AgentExecOpts {
        model: Some(escalated_model),
        ..retry_opts
    }).await;
    // ...
}
```

**Config hook:** `[agent.tier_models]` already exists in `orchestrate.rs`. Add a
`plan_generation_escalation_models = ["sonnet", "opus"]` key to `[agent]` config, or derive from
`tier_models` order.

**Show what's happening:**
```
  claude-haiku-4-5: TOML extraction failed (missing [[task.verify]] entries)
  Escalating to claude-sonnet-4-6 (attempt 2/3)...
  claude-sonnet-4-6: plan generated (12 tasks, focused complexity)
```

### S2: Self-healing TOML for known fixable errors

The `validate_and_fix_generated_plan` function (`prd.rs:2071`) already auto-fixes some issues.
Extend it to fix `[[task.verify]]` entries with missing `phase` or `command`:

**Concrete Rust sketch:**

```rust
// In validate_and_fix_generated_plan, after per-task field validation:
if let Some(verify_arr) = task.get_mut("verify") {
    if let Some(verifies) = verify_arr.as_array_mut() {
        for (vi, verify_val) in verifies.iter_mut().enumerate() {
            if let Some(verify) = verify_val.as_table_mut() {
                // Fix missing 'phase': infer from position
                if !verify.contains_key("phase") {
                    let inferred_phase = if vi == 0 { "structural" }
                                         else if vi == 1 { "compile" }
                                         else { "test" };
                    eprintln!(
                        "warning: {task_id_label} verify[{vi}]: missing 'phase'; \
                         defaulting to '{inferred_phase}'"
                    );
                    verify.insert(
                        "phase".to_string(),
                        toml::Value::String(inferred_phase.to_string()),
                    );
                }
                // Fix missing 'command': infer from phase
                if !verify.contains_key("command") {
                    let phase = verify.get("phase")
                        .and_then(|v| v.as_str())
                        .unwrap_or("compile");
                    let inferred_command = match phase {
                        "compile" => "cargo check --workspace",
                        "test" => "cargo test --workspace",
                        _ => "true",  // structural: user must fix
                    };
                    eprintln!(
                        "warning: {task_id_label} verify[{vi}]: missing 'command'; \
                         defaulting to '{inferred_command}'"
                    );
                    verify.insert(
                        "command".to_string(),
                        toml::Value::String(inferred_command.to_string()),
                    );
                }
            }
        }
    }
}
```

This turns the most common failure mode (missing `phase`/`command` in verify entries) from a hard
error into a warning with a sensible default.

### S3: Pass previous invalid output back on retry

The stricter retry prompt (`prd.rs:1240`) currently says only:
> "Your previous output could not be parsed as valid TOML. Output ONLY the ```toml block."

It does not show the model what it got wrong. The model has no context about why it failed.

**Fix:** Include the validation error and the bad output in the retry prompt:

```rust
let retry_prompt = format!(
    "Your previous output could not be parsed as valid TOML plan.\n\n\
     Error: {prev_err}\n\n\
     Previous output (first 1000 chars):\n```\n{}\n```\n\n\
     Output ONLY a ```toml fenced block.\n\
     The plan must start with [meta] then [[task]] entries.\n\
     Plan slug: {slug}",
    output.chars().take(1000).collect::<String>(),
    prev_err = validated_toml.as_ref().unwrap_err(),
);
```

### S4: `--show-raw` flag for debugging

When `exit_code != 0` and the output is short ("93 bytes of output"), show it:

**Current behavior** (`prd.rs:1169`):
```rust
return Err(anyhow!(
    "plan generation agent failed with exit code {exit_code} \
     ({} bytes of output)",
    output.len()
));
```

**Fix:** Always show the output when it fails, capped at 2000 chars:
```rust
let output_preview: String = output.chars().take(2000).collect();
return Err(anyhow!(
    "plan generation agent failed with exit code {exit_code}.\n\
     Agent output ({} bytes):\n---\n{output_preview}\n---",
    output.len()
));
```

For large outputs, add `roko prd plan <slug> --show-raw` to dump the full agent output before
validation.

### S5: Actionable error message with next steps

Replace the current terminal error with a structured one:

```
✗ Plan generation failed for 'cursor-composer-backend'

  Model: claude-haiku-4-5 (3 attempts)
  Last error: T4 verify[1]: missing required field 'phase'

  What to do:
    1. Use a stronger model:
         roko prd plan cursor-composer-backend --model claude-sonnet-4-6
    2. Add the missing field manually:
         $EDITOR plans/cursor-composer-backend/tasks.toml
       Then validate: roko plan validate plans/
    3. Check raw agent output (add to roko.toml):
         [agent]
         log_raw_output = true

  Schema reference: run 'roko plan validate --help'
```

This replaces the existing error path at `prd.rs:1358`.

### S6: Pre-flight model capability warning

Before generation, warn if the selected model has a known low success rate for plan generation.

**Hook point:** `prd.rs:1125` — after `effective_model` is resolved, before `run_agent_capture_silent`.

```rust
// After model resolution, before agent call:
if is_known_weak_plan_generator(effective_model) {
    eprintln!(
        "  warning: model '{}' has a low success rate for TOML plan generation.\n\
         Consider: roko prd plan {} --model claude-sonnet-4-6",
        effective_model, slug
    );
}

fn is_known_weak_plan_generator(model: &str) -> bool {
    // Mini/haiku-class models
    model.contains("haiku") || model.contains("mini")
        || model.contains("flash") || model.contains("small")
}
```

---

## Priority

| # | Solution | Impact | Effort | Ship when |
|---|----------|--------|--------|-----------|
| S1 | Auto-escalate on failure | High | Medium | v0.next |
| S4 | Show raw output on failure | High | Trivial | immediately |
| S5 | Actionable error message | High | Small | immediately |
| S2 | Self-heal verify entries | Medium | Small | v0.next |
| S3 | Pass prev error to retry | Medium | Trivial | v0.next |
| S6 | Pre-flight model warning | Low | Small | later |

S4 and S5 are one-line fixes. S1 is the structural gap: retrying with the same failing model is
always wrong and should be fixed first.
