# W0-C: Speed Optimizations for Demo Pipeline

**Priority**: P0 — demo pipeline too slow (4+ minutes for each LLM step)
**Effort**: 1-2 hours
**Files to modify**: 3-4 files
**Dependencies**: W0-A (strip tools)

## Problem

The demo pipeline takes 5-10 minutes total for `prd draft new` + `prd plan`. Even after stripping tools (W0-A), each LLM call can take 30-120 seconds because:

1. **Prompts are enormous**: The system prompt includes repo context, naming glossary, CLAUDE.md, PRD quality standards — often 10-30k tokens for context alone. The `prd plan` prompt logged `prompt_len=28777` (28K characters).

2. **Repo context scanning is slow**: `build_repo_context()` scans the codebase on every call, even when the workspace is empty (demo starts from `roko init`).

3. **The model (glm51) is mediocre at structured output**: Zhipu GLM-5.1 doesn't always produce properly fenced TOML blocks, causing extraction failures.

4. **No max_iterations cap for agent_exec**: The agent can do up to 50 tool iterations (default). Even with restricted tools, a confused model could loop.

## Exact Code to Change

### Fix 1: Skip repo context for empty workspaces

**File**: `crates/roko-cli/src/commands/prd.rs` — around line 379-406

The `build_repo_context()` call scans the codebase for keywords. But in the demo, the workspace was just created with `roko init` — there IS no codebase to scan. This wastes time and produces unhelpful warnings.

**Add a fast check before scanning:**
```rust
                // Skip repo context for workspaces without source code
                let has_source_code = workdir.join("src").is_dir()
                    || workdir.join("crates").is_dir()
                    || workdir.join("lib").is_dir()
                    || workdir.join("Cargo.toml").is_file()
                    || workdir.join("package.json").is_file();

                let repo_context_pack: Option<roko_cli::repo_context::RepoContextPack> =
                    if has_source_code {
                        match roko_cli::repo_context::build_repo_context(/* ... */).await {
                            // ... existing code
                        }
                    } else {
                        None  // Empty workspace — skip context scanning
                    };
```

Apply the same pattern in `crates/roko-cli/src/prd.rs` lines 960-992 (plan generation).

### Fix 2: Trim system prompt size

**File**: `crates/roko-cli/src/prd.rs` — `generate_plan_from_prd_with_outcome()`

The plan generation builds a massive system prompt (line 953-954) with:
- Generator system prompt (300+ lines from `plan_generate.rs`)
- Naming glossary (160 lines if exists)
- CLAUDE.md (120 lines if exists)
- Template guidance
- Full PRD content
- Repo context

**Cap the PRD content to essential sections only:**
```rust
        // Trim PRD content to keep prompt size manageable
        let max_prd_chars = 8000;
        let trimmed_content = if content.len() > max_prd_chars {
            let truncated = &content[..content.ceil_char_boundary(max_prd_chars)];
            format!("{truncated}\n\n[PRD content truncated at {max_prd_chars} chars]")
        } else {
            content.clone()
        };
```

Use `trimmed_content` in the task prompt instead of the full `content`.

### Fix 3: Cap tool iterations for agent_exec

**File**: `crates/roko-cli/src/agent_exec.rs` — line 134-152

The `SpawnAgentSpec` should have a sensible max_turns for direct CLI flows:

```rust
        let agent = spawn_agent_scoped(
            &routing_config,
            SpawnAgentSpec {
                // ... existing fields
                timeout_ms: Some(300_000), // 5 min (not 10) for CLI flows
                // ... existing fields
            },
            format!("create agent for model {model}"),
        )?;
```

Also, if the backend supports max_turns, pass a low value (5 iterations max for PRD, 10 for plan generation).

### Fix 4: Add `max_output` to the model config for faster responses

**File**: `docker/railway.roko.toml` — glm51 model config

The current `max_output = 16384` is fine for the model, but the API may default to generating the full 16K even when 2-4K would suffice. Check if the OpenAI-compat backend passes `max_tokens` or `max_completion_tokens`:

```bash
grep -rn 'max_tokens\|max_output\|max_completion_tokens' crates/roko-agent/src/openai_compat_backend.rs
```

If the backend respects `max_output` from model config and passes it as `max_tokens`, no change needed. But if it doesn't, the model may generate excessively long responses.

### Fix 5: Better error for glm51 extraction failures

**File**: `crates/roko-cli/src/prd.rs` — around line 1093-1102

When the fenced block extraction fails, the current error shows a 500-char preview. Add more diagnostic info:

```rust
        } else {
            let preview: String = output.chars().take(500).collect();
            let has_toml_like = output.contains("[meta]") || output.contains("[[task]]");
            eprintln!(
                "warning: agent output ({} bytes) did not contain a fenced ```toml block.\n\
                 Contains TOML-like content: {has_toml_like}\n\
                 Plan files not extracted. Output preview:\n---\n{preview}\n---",
                output.len()
            );
            if has_toml_like {
                eprintln!(
                    "hint: The model output TOML without proper fencing. \
                     Try a more capable model or check the plan_generate system prompt."
                );
            }
        }
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-C-speed-optimizations.md and implement all changes described in it. Focus on: (1) skip repo context for empty workspaces in commands/prd.rs and prd.rs, (2) cap PRD content size in plan generation prompt, (3) reduce timeout from 10min to 5min in agent_exec.rs, (4) better diagnostics for extraction failures. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical speed fixes). Do not commit individually.

## Checklist

- [x] Skip `build_repo_context()` for empty workspaces in `commands/prd.rs`
- [x] Skip `build_repo_context()` for empty workspaces in `prd.rs` plan generation
- [x] Cap PRD content size in plan generation prompt (8000 chars)
- [x] Reduce agent timeout from 600s to 300s in agent_exec.rs
- [x] Add TOML-like content detection in extraction failure diagnostics
- [ ] Verify: `prd draft new` completes in <60s for empty workspace
- [ ] Verify: `prd plan` completes in <120s for simple PRDs
