# 85 — Plan Generation TOML First-Attempt Reliability

**Priority**: P2 — efficiency (saves 2-5 min and one LLM call per plan generation failure)
**Size**: S (2-3 hours)
**Crates**: `crates/roko-cli` (`src/plan_generate.rs`, `src/prd.rs`, `src/task_parser.rs`)
**Depends on**: None

---

## Background

`roko prd plan <slug>` generates an implementation plan by sending a system prompt to an LLM agent and expecting the output to be a valid TOML block. The system has a retry loop that attempts the request up to 3 times total if the TOML is malformed or missing.

The first attempt fails roughly half the time. Common failure modes: the LLM embeds Rust code or markdown commentary inside the TOML fence, omits the required `[meta]` section, or generates invalid TOML syntax. The retry succeeds on the second attempt because the retry prompt is simpler and the model gets another chance, not because the retry prompt itself is well-targeted.

A `strip_embedded_code()` / `repair_toml()` post-processing pipeline exists in `task_parser.rs` as a defense-in-depth measure. It handles some cases (stripping Rust keywords, closing unclosed strings), but it cannot fix fundamental structural issues like a missing `[meta]` section. The root problem is that the system prompt does not anchor the expected output format prominently enough.

## Current State

1. The plan generator system prompt is defined as a `const` string `PLAN_GENERATOR_SYSTEM_PROMPT` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_generate.rs` starting at line 154. It is 296 lines long (lines 154-450). The TOML output format example appears at line 175 (the `## Output format` section), but this section is nested inside a very long system prompt and the concrete minimum structure is not called out separately.

2. The function `build_generator_system_prompt` at line 455 of `plan_generate.rs` concatenates `PLAN_GENERATOR_SYSTEM_PROMPT` with optional naming glossary and CLAUDE.md content.

3. The function `build_generation_prompt` at line 465 appends PRD source content after the system prompt.

4. The retry loop is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` at lines 1386-1530. The retry prompt (lines 1476-1483) already includes the parse error and the truncated invalid output, plus the instruction to output only a `\`\`\`toml` fenced block. It is currently reasonable but lacks an explicit minimum structure template.

5. The `repair_toml` function is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs` at line 1145. The `strip_embedded_code` helper is at line 1229. Both are called from `parse_agent_output` at line 724.

6. The fallback extraction function `extract_toml_content_fallback` at `prd.rs:2050` scans for a `[meta]` section and trailing prose. It already requires both `[meta]` and `[[task]]` to be present before returning a result.

7. The `tasks.toml` meta field is named `plan` (not `name`) — the `TaskMeta` struct at `task_parser.rs:22` uses `pub plan: String`. This is important: any example TOML in the prompt must use `plan = "..."`, not `name = "..."`.

## Implementation Plan

### Step 1: Add a "minimum valid structure" preamble to `PLAN_GENERATOR_SYSTEM_PROMPT`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_generate.rs`, prepend a format-anchoring block to the beginning of `PLAN_GENERATOR_SYSTEM_PROMPT`. The block should:

- State that output must be a single ` ```toml ` fenced block
- Show the minimum valid structure using the correct field names (`plan`, not `name`)
- List explicit negative examples (Rust code inside TOML block)

The current start of `PLAN_GENERATOR_SYSTEM_PROMPT` (line 154) begins with the system role description. Add the format anchoring before the existing content:

```rust
pub const PLAN_GENERATOR_SYSTEM_PROMPT: &str = r#"## CRITICAL: Output format

Your entire response MUST be a single ```toml fenced code block containing ONLY valid TOML.
Do not include prose, explanations, Rust code, or markdown outside the TOML block.

MINIMUM VALID STRUCTURE (use this as your template):
```toml
[meta]
plan = "slug-matches-prd"
total = 2
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "First task"
description = "What this task accomplishes."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
allowed_tools = ["read_file", "grep"]
denied_tools = []
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-core/src/lib.rs", lines = "1-50", why = "Read existing types." },
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```

INVALID — do NOT produce output like this (Rust code inside TOML):
```toml
[[task]]
id = "T1"
title = "Add struct"
prompt = "Implement the following:

pub struct Foo {
    bar: String,
}
"
```

The above is INVALID because it embeds Rust code inside the TOML block.

---

You are a task decomposition engine for software projects. ...
```

Note: the rest of `PLAN_GENERATOR_SYSTEM_PROMPT` (the existing content) follows the `---` separator. Do not remove existing content.

### Step 2: Add a quick structural pre-check before TOML parse

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`, the `try_extract_and_validate` closure at line 1388 calls `extract_fenced_block` followed by `validate_and_fix_generated_plan`. Add a fast structural pre-check between extraction and full parse. If the extracted string lacks `[meta]` or `[[task]]`, return the specific missing element in the error message rather than waiting for the TOML parser to fail with a cryptic error.

Insert after line 1398 (where `toml_content` is confirmed present):

```rust
// Fast structural pre-check before full parse.
if !toml_content.contains("[meta]") {
    return Err("TOML block is missing the required [meta] section".to_string());
}
if !toml_content.contains("[[task]]") {
    return Err("TOML block is missing required [[task]] entries".to_string());
}
```

### Step 3: Improve the retry prompt with a concrete minimum structure

The retry prompt at `prd.rs:1476-1483` already includes the parse error and truncated output. Extend it to also include the minimum valid TOML template, using the correct `plan` field name:

```rust
let retry_prompt = format!(
    "Previous attempt produced invalid TOML. Error: {error}\n\n\
     Invalid output (truncated):\n```\n{truncated_output}\n```\n\n\
     Please regenerate a valid tasks.toml. Your ENTIRE response must be \
     a single ```toml fenced block with no other text.\n\n\
     MINIMUM REQUIRED STRUCTURE:\n\
     ```toml\n\
     [meta]\n\
     plan = \"{slug}\"\n\
     total = 1\n\
     done = 0\n\
     status = \"ready\"\n\
     max_parallel = 1\n\n\
     [[task]]\n\
     id = \"T1\"\n\
     title = \"Task title\"\n\
     description = \"What this task does.\"\n\
     status = \"ready\"\n\
     tier = \"focused\"\n\
     max_loc = 50\n\
     files = [\"crates/roko-core/src/lib.rs\"]\n\
     allowed_tools = [\"read_file\", \"grep\"]\n\
     denied_tools = []\n\
     depends_on = []\n\
     role = \"implementer\"\n\
     ```\n\n\
     Do NOT include Rust code, markdown prose, or explanations outside the TOML block.\n\
     Note: the meta field is `plan`, not `name`.",
    error = validated_toml.as_ref().unwrap_err(),
    slug = slug,
    truncated_output = truncated_output,
);
```

## Acceptance Criteria

1. Running `roko prd plan <slug>` on 3 different PRDs: at least 2 of 3 succeed on the first attempt without hitting the retry path.
2. When a retry does occur, the retry prompt in the `tracing::warn!` log at `prd.rs:1458-1463` contains the specific missing structural element (e.g., "missing [meta] section") when applicable.
3. The fast structural pre-check at step 2 correctly identifies a TOML block missing `[meta]` and returns an error message that includes "missing the required [meta] section".
4. Existing `strip_embedded_code` and `repair_toml` tests at `task_parser.rs:2343-2412` continue to pass.
5. The minimum structure example in `PLAN_GENERATOR_SYSTEM_PROMPT` uses `plan = "..."` (not `name = "..."`), matching the actual `TaskMeta.plan` field.
6. `cargo test -p roko-cli` passes.

## Verification Checklist

- [ ] `cargo clippy -p roko-cli -- -D warnings` passes
- [ ] `cargo test -p roko-cli` passes
- [ ] Run `cargo run -p roko-cli -- prd plan <existing-slug>` and observe the first attempt succeeds without retry messages in the terminal
- [ ] Deliberately test failure: create a dummy PRD with `roko prd idea "test"`, run `roko prd plan test`, verify retry prompt includes the structural template
- [ ] Verify the TOML example in the updated prompt parses correctly with `toml::from_str` in a quick test

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_generate.rs` | Prepend format-anchoring block (minimum structure + negative example) to `PLAN_GENERATOR_SYSTEM_PROMPT` at line 154 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` | Add fast structural pre-check after line 1398; extend retry prompt at lines 1476-1483 to include minimum structure template |
