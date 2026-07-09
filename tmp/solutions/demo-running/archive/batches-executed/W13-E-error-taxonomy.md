# W13-E: Typed Error Taxonomy, Schema Validation, Scaffold Fixes

**Wave**: 13 -- Speed & Reliability
**IMPROVEMENTS ref**: 3.2 + 3.4 + 3.5 + 3.6
**Priority**: P2 -- correctness and safety improvements
**Effort**: 2-3 hours
**Files to modify**: 3 files
**Dependencies**: None

## Problem

Four related reliability issues:

1. **3.2 String-based error classification**: `classify_error_pattern()` in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs` line 582
   uses `.contains()` string matching to classify errors into 6 categories. This is
   fragile (e.g. "this test compiles fine" matches both `Test` and `Compile`) and
   the ordering is wrong -- `Compile` is checked before `Test`.

2. **3.4 No schema-driven validation**: Task definitions are validated with ad-hoc
   field checks in `TasksFile::validate()` but there is no schema object that maps
   roles to required fields or validates enum values (roles, tiers, statuses) against
   a canonical list.

3. **3.5 Scaffold uses string search for Cargo.toml**: `scaffold_missing_crates()` in
   `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs` lines
   201-207 finds the `]` closing `members` via `new_content[members_pos..].find(']')`.
   Comments inside the members array break this.

4. **3.6 No crate name validation in scaffold**: Lines 122-141 accept any string from
   task file paths as a crate name, including `..` (directory traversal) or names with
   `/` (path injection).

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs`

#### Change 1: Improve `classify_error_pattern` with priority-ordered matching

**Find this code** (lines 582-605):
```rust
fn classify_error_pattern(output: &Engram) -> ErrorPattern {
    let Ok(text) = output.body.as_text() else {
        return ErrorPattern::Unknown;
    };
    let text = text.to_ascii_lowercase();

    if text.contains("compile") || text.contains("borrow checker") || text.contains("rustc") {
        ErrorPattern::Compile
    } else if text.contains("test") || text.contains("assert") {
        ErrorPattern::Test
    } else if text.contains("tool") {
        ErrorPattern::ToolCall
    } else if text.contains("timeout") || text.contains("timed out") {
        ErrorPattern::Timeout
    } else if text.contains("io error")
        || text.contains("filesystem")
        || text.contains("permission denied")
        || text.contains("network")
    {
        ErrorPattern::Infrastructure
    } else {
        ErrorPattern::Unknown
    }
}
```

**Replace with:**
```rust
fn classify_error_pattern(output: &Engram) -> ErrorPattern {
    let Ok(text) = output.body.as_text() else {
        return ErrorPattern::Unknown;
    };
    let text = text.to_ascii_lowercase();

    // Priority-ordered matching: more specific patterns first to avoid
    // false positives (e.g. "this test compiles fine" should match Test,
    // not Compile).

    // Timeout is the most unambiguous signal.
    if text.contains("timed out")
        || text.contains("timeout")
        || text.contains("deadline exceeded")
    {
        return ErrorPattern::Timeout;
    }

    // Infrastructure: network/IO errors are unambiguous when they include
    // specific keywords.
    if text.contains("io error")
        || text.contains("permission denied")
        || text.contains("connection refused")
        || text.contains("dns resolution")
        || text.contains("network error")
        || text.contains("econnreset")
        || text.contains("broken pipe")
    {
        return ErrorPattern::Infrastructure;
    }

    // Tool call failures -- look for specific tool error patterns.
    if text.contains("tool call failed")
        || text.contains("tool execution error")
        || text.contains("tool_use_error")
    {
        return ErrorPattern::ToolCall;
    }

    // Compile: look for compiler-specific indicators, not just "compile".
    if text.contains("error[e") // rustc error codes like error[E0308]
        || text.contains("borrow checker")
        || text.contains("cannot find")
        || text.contains("mismatched types")
        || text.contains("unresolved import")
        || (text.contains("rustc") && text.contains("error"))
        || (text.contains("cargo build") && text.contains("failed"))
    {
        return ErrorPattern::Compile;
    }

    // Test: look for test runner output patterns.
    if text.contains("test result: failed")
        || text.contains("assertion failed")
        || text.contains("panicked at")
        || (text.contains("cargo test") && text.contains("failed"))
    {
        return ErrorPattern::Test;
    }

    // Fallback: broad filesystem/network patterns (lower priority to avoid
    // false positives).
    if text.contains("filesystem") || text.contains("no such file") {
        return ErrorPattern::Infrastructure;
    }

    ErrorPattern::Unknown
}
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs`

#### Change 2: Add `TaskFieldSchema` and `validate_against_schema()`

The `quality_warnings()` method ends at line 783. The next function is
`validate_modern_fields` (line 785). Add the schema struct and validation
method between them (after line 783, before line 785).

**Find this code** (lines 781-786):
```rust
        warnings
    }

    /// Validate that the raw `tasks.toml` still carries the modern task fields.
    pub fn validate_modern_fields(path: &Path) -> Result<Vec<ModernFieldIssue>> {
```

**Replace with:**
```rust
        warnings
    }

    /// Validate task definitions against the field schema.
    ///
    /// Checks that role, tier, and status values are from the known set,
    /// and that role-specific required fields are present.
    /// Returns a list of issues (empty = valid).
    pub fn validate_against_schema(&self) -> Vec<String> {
        const VALID_ROLES: &[&str] = &[
            "implementer",
            "researcher",
            "strategist",
            "architect",
            "reviewer",
            "quick-reviewer",
            "scribe",
        ];
        const VALID_TIERS: &[&str] = &[
            "mechanical",
            "focused",
            "integrative",
            "architectural",
        ];
        const VALID_STATUSES: &[&str] = &[
            "pending",
            "ready",
            "active",
            "done",
            "blocked",
            "skipped",
        ];
        // Role -> required fields
        const IMPLEMENTER_REQUIRED: &[&str] = &["verify", "files"];

        let mut issues = Vec::new();

        for task in &self.tasks {
            let tid = &task.id;
            let role = task.role.as_deref().unwrap_or("implementer");

            // Check role is valid.
            if !VALID_ROLES.contains(&role) {
                issues.push(format!(
                    "{tid}: unknown role '{role}' (valid: {})",
                    VALID_ROLES.join(", ")
                ));
            }

            // Check required fields for implementer role.
            if role == "implementer" {
                for &field in IMPLEMENTER_REQUIRED {
                    let missing = match field {
                        "verify" => task.verify.is_empty(),
                        "files" => task.files.is_empty(),
                        _ => false,
                    };
                    if missing {
                        issues.push(format!(
                            "{tid}: missing '{field}' (required for role '{role}')"
                        ));
                    }
                }
            }

            // Check tier is valid.
            if !task.tier.is_empty()
                && task.tier != "unknown"
                && !VALID_TIERS.contains(&task.tier.as_str())
            {
                issues.push(format!(
                    "{tid}: unknown tier '{}' (valid: {})",
                    task.tier,
                    VALID_TIERS.join(", ")
                ));
            }

            // Check status is valid.
            if !task.status.is_empty()
                && !VALID_STATUSES.contains(&task.status.as_str())
            {
                issues.push(format!(
                    "{tid}: unknown status '{}' (valid: {})",
                    task.status,
                    VALID_STATUSES.join(", ")
                ));
            }

            // Check numeric bounds.
            if task.timeout_secs == 0 {
                issues.push(format!("{tid}: timeout_secs must be > 0"));
            }
            if task.max_loc.is_some_and(|m| m == 0) {
                issues.push(format!("{tid}: max_loc must be > 0"));
            }
        }

        issues
    }

    /// Validate that the raw `tasks.toml` still carries the modern task fields.
    pub fn validate_modern_fields(path: &Path) -> Result<Vec<ModernFieldIssue>> {
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs`

#### Change 3: Add `is_valid_crate_name` function before `scaffold_missing_crates`

**Find this code** (lines 90-99):
```rust
/// Scan all tasks across `plans` for file references to crates that don't
/// exist yet, then scaffold those crates (`Cargo.toml` + `src/lib.rs`) and
/// register them in the workspace `Cargo.toml` members list.
///
/// This handles the common case where a plan's first task is to *create* a new
/// crate — the gate would fail (`cargo check` can't find the crate) unless a
/// minimal scaffold is present.
///
/// Returns the names of any newly created crates.
pub fn scaffold_missing_crates(workdir: &Path, plans: &[Plan]) -> Result<Vec<String>> {
```

**Replace with:**
```rust
/// Validate that a crate name is safe and follows Rust naming conventions.
///
/// Rejects:
/// - Empty names
/// - Names starting with `-` or `.`
/// - Names containing `..` (directory traversal)
/// - Names containing `/` or `\` (path separators)
/// - Names with characters outside `[a-zA-Z0-9_-]`
fn is_valid_crate_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.starts_with('.')
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Scan all tasks across `plans` for file references to crates that don't
/// exist yet, then scaffold those crates (`Cargo.toml` + `src/lib.rs`) and
/// register them in the workspace `Cargo.toml` members list.
///
/// This handles the common case where a plan's first task is to *create* a new
/// crate — the gate would fail (`cargo check` can't find the crate) unless a
/// minimal scaffold is present.
///
/// Returns the names of any newly created crates.
pub fn scaffold_missing_crates(workdir: &Path, plans: &[Plan]) -> Result<Vec<String>> {
```

#### Change 4: Call `is_valid_crate_name` in the scaffold loop

**Find this code** (lines 124-128):
```rust
                if parts.len() >= 2 && parts[0] == "crates" {
                    let crate_name = parts[1].to_string();
                    if crate_name.is_empty() || crate_name.contains('*') {
                        continue;
                    }
```

**Replace with:**
```rust
                if parts.len() >= 2 && parts[0] == "crates" {
                    let crate_name = parts[1].to_string();
                    if crate_name.is_empty() || crate_name.contains('*') {
                        continue;
                    }
                    // Validate crate name to prevent directory traversal and
                    // invalid Rust crate names.
                    if !is_valid_crate_name(&crate_name) {
                        info!(
                            crate_name = %crate_name,
                            "scaffold: skipping invalid crate name"
                        );
                        continue;
                    }
```

#### Change 5: Improve Cargo.toml member insertion to skip comments

The current string-search approach finds `]` via `new_content[members_pos..].find(']')`,
which breaks when there are comments in the members array. Rather than adding a new
`toml_edit` dependency (not in workspace), improve the search to skip past comment lines.

**Find this code** (lines 192-218):
```rust
        let ws_content = std::fs::read_to_string(&ws_cargo_path)
            .context("scaffold: read workspace Cargo.toml")?;

        let mut new_content = ws_content.clone();
        for name in &scaffolded {
            let member_entry = format!("\"crates/{name}\"");
            if new_content.contains(&member_entry) {
                continue;
            }
            if let Some(members_pos) = new_content.find("members") {
                if let Some(bracket_offset) = new_content[members_pos..].find(']') {
                    let insert_at = members_pos + bracket_offset;
                    let insertion = format!("    {member_entry},\n");
                    new_content.insert_str(insert_at, &insertion);
                }
            }
        }

        if new_content != ws_content {
            std::fs::write(&ws_cargo_path, &new_content)
                .context("scaffold: write workspace Cargo.toml")?;
            info!(
                "added {} new crate(s) to workspace members: {:?}",
                scaffolded.len(),
                scaffolded
            );
        }
```

**Replace with:**
```rust
        let ws_content = std::fs::read_to_string(&ws_cargo_path)
            .context("scaffold: read workspace Cargo.toml")?;

        let mut new_content = ws_content.clone();
        for name in &scaffolded {
            let member_entry = format!("\"crates/{name}\"");
            if new_content.contains(&member_entry) {
                continue;
            }
            // Find the `members = [` line, then locate the matching `]`.
            // Skip comment lines and nested brackets to handle:
            //   members = [
            //     # core crates
            //     "crates/roko-core",
            //   ]
            if let Some(members_pos) = new_content.find("members") {
                if let Some(open_bracket) = new_content[members_pos..].find('[') {
                    let search_start = members_pos + open_bracket + 1;
                    let mut depth = 1i32;
                    let mut close_pos = None;
                    for (i, ch) in new_content[search_start..].char_indices() {
                        match ch {
                            '[' => depth += 1,
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    close_pos = Some(search_start + i);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(insert_at) = close_pos {
                        let insertion = format!("    {member_entry},\n");
                        new_content.insert_str(insert_at, &insertion);
                    }
                }
            }
        }

        if new_content != ws_content {
            std::fs::write(&ws_cargo_path, &new_content)
                .context("scaffold: write workspace Cargo.toml")?;
            info!(
                "added {} new crate(s) to workspace members: {:?}",
                scaffolded.len(),
                scaffolded
            );
        }
```

Note: `tracing::info` is already imported at line 12 of plan_loader.rs.
No new dependencies needed.

## Verification

```bash
# Compile check
cargo check -p roko-cli 2>&1 | head -20
cargo check -p roko-agent 2>&1 | head -20

# Run existing scaffold tests
cargo test -p roko-cli scaffold

# Run task_parser validation tests
cargo test -p roko-cli validate
```

## Agent Prompt

```
You are implementing W13-E: Typed Error Taxonomy, Schema Validation, Scaffold Fixes.
These are correctness and safety improvements across 3 files.

## Changes to make

### 1. task_runner.rs -- improve error classification

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs`, find the
`classify_error_pattern` function (line 582). Replace it with a priority-ordered version
that uses more specific patterns. Key changes:

- Check `Timeout` first (most unambiguous)
- Check `Infrastructure` with specific keywords (not just "network")
- Check `ToolCall` with specific patterns ("tool call failed", "tool_use_error")
- Check `Compile` with compiler-specific indicators ("error[e", "mismatched types")
- Check `Test` with test-runner patterns ("test result: failed", "assertion failed")
- Remove the overly broad `text.contains("test")` and `text.contains("tool")` checks

Find this code:
```rust
fn classify_error_pattern(output: &Engram) -> ErrorPattern {
    let Ok(text) = output.body.as_text() else {
        return ErrorPattern::Unknown;
    };
    let text = text.to_ascii_lowercase();

    if text.contains("compile") || text.contains("borrow checker") || text.contains("rustc") {
        ErrorPattern::Compile
    } else if text.contains("test") || text.contains("assert") {
```

Replace the entire function body with the priority-ordered version from the batch doc.

### 2. task_parser.rs -- add validate_against_schema()

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs`, find the
`quality_warnings()` method which ends at line 783 (returns `warnings`). After it,
before `validate_modern_fields` (line 785), add a new method
`validate_against_schema()` to `impl TasksFile`.

This method uses const arrays for valid roles, tiers, and statuses. It checks:
- Role is in the known set
- Implementer role has verify + files
- Tier is valid
- Status is valid
- timeout_secs > 0
- max_loc > 0 if set

### 3. plan_loader.rs -- crate name validation + improved member insertion

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs`:

a) Add `is_valid_crate_name()` function before `scaffold_missing_crates` (before line 99).
   Rejects names with `..`, `/`, `\`, leading `-`/`.`, or non-alphanumeric chars.

b) Call `is_valid_crate_name()` in the scaffold loop (after the `contains('*')` check
   at line 127). Log skipped names with `tracing::info!`.

c) Replace the `find(']')` string search for Cargo.toml member insertion (lines 201-207)
   with a bracket-depth-tracking approach that correctly handles comments and nested
   brackets. Do NOT add `toml_edit` as a dependency.

No new dependencies needed. `tracing::info` is already imported in plan_loader.rs.

Do NOT run cargo build/test/clippy/fmt -- compilation is deferred.
```

## Commit

This batch is committed with all Wave 13 batches together. Do not commit individually.

## Checklist

- [ ] `classify_error_pattern` rewritten with priority-ordered matching in task_runner.rs
- [ ] `validate_against_schema()` method added to `TasksFile` in task_parser.rs
- [ ] `is_valid_crate_name()` added to plan_loader.rs
- [ ] Crate name validation called in scaffold loop with `tracing::info!` on skip
- [ ] Cargo.toml member insertion uses bracket-depth tracking instead of `find(']')`
- [ ] No new dependencies added
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. 1 issue fixed: em dash mismatch in plan_loader.rs "Find this code" and "Replace with" blocks (batch used `--` but source uses `\u{2014}`)
