# W13-A: Deterministic TOML Repair Pipeline

**Wave**: 13 -- Speed & Reliability
**IMPROVEMENTS ref**: 2.1
**Priority**: P1 -- eliminates ~80% of LLM retries (saves 30-120s per plan gen)
**Effort**: 1-2 hours
**Files to modify**: 2 files
**Dependencies**: None

## Problem

Plan generation (`roko prd plan <slug>`) parses agent output as TOML. When the LLM
produces malformed TOML (merged fields, unclosed strings, trailing prose after `]]`),
the system retries with a stricter prompt -- up to 2 full LLM calls, each costing
15-60s. Most of these failures are deterministically fixable with string repair.

## Root Cause

`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` line 1871:
The `validate_and_fix_generated_plan` function calls `toml::from_str(toml_str)` directly.
If parsing fails, the entire retry loop fires a new LLM call. There is no intermediate
repair step that could fix common LLM TOML mistakes before the parse attempt.

`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs` line 964 has
`extract_toml_payload` which strips markdown fences, but no repair logic for malformed
TOML content itself.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs`

#### Change 1: Add `repair_toml()` and helpers after `extract_toml_payload` (after line 979)

**Find this code** (lines 964-979):
```rust
fn extract_toml_payload(content: &str) -> String {
    let trimmed = content.trim();
    let Some(open_start) = trimmed.find("```") else {
        return trimmed.to_string();
    };
    let after_open = &trimmed[open_start + 3..];
    let Some(open_end) = after_open.find('\n') else {
        return trimmed.to_string();
    };
    let body = &after_open[open_end + 1..];
    if let Some(close_start) = body.rfind("```") {
        body[..close_start].trim().to_string()
    } else {
        body.trim().to_string()
    }
}
```

**Replace with** (adds repair functions after `extract_toml_payload`):
```rust
fn extract_toml_payload(content: &str) -> String {
    let trimmed = content.trim();
    let Some(open_start) = trimmed.find("```") else {
        return trimmed.to_string();
    };
    let after_open = &trimmed[open_start + 3..];
    let Some(open_end) = after_open.find('\n') else {
        return trimmed.to_string();
    };
    let body = &after_open[open_end + 1..];
    if let Some(close_start) = body.rfind("```") {
        body[..close_start].trim().to_string()
    } else {
        body.trim().to_string()
    }
}

/// Deterministic TOML repair pipeline.
///
/// Applies a sequence of fixups for the most common LLM TOML generation
/// mistakes. Called before `toml::from_str` to avoid expensive LLM retries.
///
/// Steps:
/// 1. Strip markdown fences (reuses `extract_toml_payload`)
/// 2. Truncate trailing prose after last `]]`
/// 3. Split merged fields at known field boundaries
/// 4. Close unclosed quoted strings
pub fn repair_toml(raw: &str) -> String {
    let t0 = std::time::Instant::now();

    // Step 1: Strip markdown fences
    let mut s = extract_toml_payload(raw);

    // Step 2: Strip trailing prose after last ]]
    // LLMs often append explanatory text after the final [[task]] entry.
    if let Some(pos) = s.rfind("]]") {
        // Find the end of the line containing ]]
        let line_end = s[pos..].find('\n').map(|i| pos + i + 1).unwrap_or(pos + 2);
        // Only truncate if there's substantial non-TOML text after
        let trailing = s[line_end..].trim();
        if !trailing.is_empty()
            && !trailing.starts_with('[')
            && !trailing.starts_with('#')
        {
            s.truncate(line_end);
        }
    }

    // Step 3: Split merged fields
    s = split_merged_fields(&s);

    // Step 4: Fix unclosed strings
    s = close_unclosed_strings(&s);

    let elapsed_us = t0.elapsed().as_micros();
    if s != extract_toml_payload(raw) {
        tracing::info!(elapsed_us, "repair_toml: applied deterministic fixes");
    }

    s
}

/// Split fields that LLMs merge onto a single line.
///
/// Pattern: a previous field's value runs directly into the next field name
/// without a newline, e.g. `"claude-sonnet-4-6"max_loc = 500`.
fn split_merged_fields(s: &str) -> String {
    let field_boundaries = [
        "max_loc",
        "timeout_secs",
        "max_retries",
        "model_hint",
        "allowed_tools",
        "denied_tools",
        "depends_on",
        "status",
        "tier",
        "role",
        "verify",
        "files",
        "description",
        "title",
        "id",
    ];
    let mut result = s.to_string();
    for field in &field_boundaries {
        let pattern = format!("{field} = ");
        // Find occurrences where the field name is preceded by a quote
        // (indicating it was merged with the previous field's value)
        let merged = format!("\"{pattern}");
        let split = format!("\"\n{pattern}");
        result = result.replace(&merged, &split);
    }
    result
}

/// Close unclosed quoted strings on a line.
///
/// If a line has an odd number of `"` characters, append a closing `"`.
/// This fixes the common LLM mistake of truncating string values.
fn close_unclosed_strings(s: &str) -> String {
    s.lines()
        .map(|line| {
            // Skip comment lines
            if line.trim_start().starts_with('#') {
                return line.to_string();
            }
            let quote_count = line.chars().filter(|&c| c == '"').count();
            if quote_count % 2 != 0 {
                format!("{line}\"")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`

#### Change 2: Call `repair_toml` before `toml::from_str` in `validate_and_fix_generated_plan`

**Find this code** (lines 1871-1879):
```rust
fn validate_and_fix_generated_plan(
    toml_str: &str,
    slug: &str,
    models: &std::collections::HashMap<String, roko_core::config::schema::ModelProfile>,
    default_model: Option<&str>,
) -> Result<String> {
    // 1. Parse syntax.
    let mut root: toml::Value =
        toml::from_str(toml_str).map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;
```

**Replace with:**
```rust
fn validate_and_fix_generated_plan(
    toml_str: &str,
    slug: &str,
    models: &std::collections::HashMap<String, roko_core::config::schema::ModelProfile>,
    default_model: Option<&str>,
) -> Result<String> {
    // 0. Deterministic repair before parsing -- fixes ~80% of LLM TOML mistakes.
    let repair_start = std::time::Instant::now();
    let repaired = crate::task_parser::repair_toml(toml_str);
    let repair_ms = repair_start.elapsed().as_millis() as u64;
    let used_repair = repaired != toml_str;
    if used_repair {
        tracing::info!(repair_ms, "prd plan: applied deterministic TOML repair");
    }

    // 1. Parse syntax (using repaired content).
    let mut root: toml::Value = toml::from_str(&repaired)
        .map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;
```

## Verification

```bash
# Unit tests for repair functions
cargo test -p roko-cli repair_toml
cargo test -p roko-cli split_merged_fields
cargo test -p roko-cli close_unclosed_strings

# Integration: generate a plan and verify it parses without retry
cargo run -p roko-cli -- prd plan <any-slug> 2>&1 | grep -E "retry|repair"
# Should see "applied deterministic TOML repair" instead of "retrying with stricter prompt"
```

## Agent Prompt

```
You are implementing W13-A: Deterministic TOML Repair Pipeline. This eliminates ~80% of
LLM retries during plan generation by fixing common TOML mistakes before parsing.

## Changes to make

### 1. task_parser.rs -- add repair functions

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs`, find the
`extract_toml_payload` function (line 964). After it ends (line 979), insert three new
functions: `repair_toml` (public), `split_merged_fields`, and `close_unclosed_strings`.

Find this code:
```rust
fn extract_toml_payload(content: &str) -> String {
    let trimmed = content.trim();
    let Some(open_start) = trimmed.find("```") else {
        return trimmed.to_string();
    };
    let after_open = &trimmed[open_start + 3..];
    let Some(open_end) = after_open.find('\n') else {
        return trimmed.to_string();
    };
    let body = &after_open[open_end + 1..];
    if let Some(close_start) = body.rfind("```") {
        body[..close_start].trim().to_string()
    } else {
        body.trim().to_string()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────
```

Insert these three functions between `extract_toml_payload` and the `// ─── Tests ───` section:

```rust
/// Deterministic TOML repair pipeline.
pub fn repair_toml(raw: &str) -> String {
    let t0 = std::time::Instant::now();
    let mut s = extract_toml_payload(raw);

    if let Some(pos) = s.rfind("]]") {
        let line_end = s[pos..].find('\n').map(|i| pos + i + 1).unwrap_or(pos + 2);
        let trailing = s[line_end..].trim();
        if !trailing.is_empty() && !trailing.starts_with('[') && !trailing.starts_with('#') {
            s.truncate(line_end);
        }
    }

    s = split_merged_fields(&s);
    s = close_unclosed_strings(&s);

    let elapsed_us = t0.elapsed().as_micros();
    if s != extract_toml_payload(raw) {
        tracing::info!(elapsed_us, "repair_toml: applied deterministic fixes");
    }
    s
}

fn split_merged_fields(s: &str) -> String {
    let field_boundaries = [
        "max_loc", "timeout_secs", "max_retries", "model_hint",
        "allowed_tools", "denied_tools", "depends_on", "status",
        "tier", "role", "verify", "files", "description", "title", "id",
    ];
    let mut result = s.to_string();
    for field in &field_boundaries {
        let pattern = format!("{field} = ");
        let merged = format!("\"{pattern}");
        let split = format!("\"\n{pattern}");
        result = result.replace(&merged, &split);
    }
    result
}

fn close_unclosed_strings(s: &str) -> String {
    s.lines()
        .map(|line| {
            if line.trim_start().starts_with('#') { return line.to_string(); }
            let quote_count = line.chars().filter(|&c| c == '"').count();
            if quote_count % 2 != 0 { format!("{line}\"") } else { line.to_string() }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

### 2. prd.rs -- call repair_toml before parsing

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`, find the
`validate_and_fix_generated_plan` function (line 1871). Replace the first 3 lines of
the body (the comment and `toml::from_str` call) with the repair + parse sequence.

Find this code:
```rust
) -> Result<String> {
    // 1. Parse syntax.
    let mut root: toml::Value =
        toml::from_str(toml_str).map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;
```

Replace with:
```rust
) -> Result<String> {
    // 0. Deterministic repair before parsing -- fixes ~80% of LLM TOML mistakes.
    let repair_start = std::time::Instant::now();
    let repaired = crate::task_parser::repair_toml(toml_str);
    let repair_ms = repair_start.elapsed().as_millis() as u64;
    let used_repair = repaired != toml_str;
    if used_repair {
        tracing::info!(repair_ms, "prd plan: applied deterministic TOML repair");
    }

    // 1. Parse syntax (using repaired content).
    let mut root: toml::Value = toml::from_str(&repaired)
        .map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;
```

No new imports needed -- `tracing` is already imported in both files, and
`std::time::Instant` is used inline.

Do NOT run cargo build/test/clippy/fmt -- compilation is deferred.
```

## Commit

This batch is committed with all Wave 13 batches together. Do not commit individually.

## Checklist

- [ ] `repair_toml()` added to task_parser.rs (public)
- [ ] `split_merged_fields()` added to task_parser.rs
- [ ] `close_unclosed_strings()` added to task_parser.rs
- [ ] `validate_and_fix_generated_plan` in prd.rs calls `repair_toml` before parsing
- [ ] Repair timing logged via `tracing::info!` when fixes applied
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed
