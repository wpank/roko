# 81 — Layer-Check False Positives and Empty Event Field Type

**Priority**: P2 — false positives in the architecture linter erode its value as a guardrail
**Size**: S (half day to 1 day)
**Crates**: `scripts/layer_check.rs`, `crates/roko-cli` (path: `src/auth_detect.rs`, `src/bootstrap.rs`, `src/doctor.rs`)
**Depends on**: None

---

## Background

`roko layer-check` is a custom architecture linter that scans the codebase for violations of two rules: (1) direct subprocess dispatch of LLM providers bypassing `ModelCallService`, and (2) empty placeholder strings in event fields that should carry real values. When developers see repeated false positives from a linter, they learn to ignore it — which means they also ignore real violations.

There are two categories of false positives and one genuine type issue to fix.

## Current State

1. **`check_direct_model_subprocess()` flags legitimate binary presence probes** — In `/Users/will/dev/nunchi/roko/roko/scripts/layer_check.rs` line 288, the check scans all `.rs` files under `crates/` for the string `Command::new("claude")`. It flags three legitimate uses:
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs` line 146: `Command::new("claude").arg("--version")` — probes for CLI presence before selecting an auth method
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bootstrap.rs` line 120: `std::process::Command::new("claude")` — probe at bootstrap
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` lines 807 and 978: two separate `std::process::Command::new("claude").arg("--version")` probes in `check_claude_cli()` and `check_available_providers()`

   All four are `--version` probes that detect binary presence. None dispatch LLM requests. The rule's intent is to prevent actual LLM inference from bypassing the provider abstraction layer; these probes are system-level checks, not inference calls.

2. **`check_empty_event_fields()` scans the wrong directory** — In `/Users/will/dev/nunchi/roko/roko/scripts/layer_check.rs` line 344, the check scans for `model: String::new()` and `agent_id: String::new()` — but only in files under `crates/roko-runtime/src/`. The violations the original bug report referenced were in `crates/roko-cli/src/tui/` and `crates/roko-core/src/dashboard_snapshot.rs`. If those files are not in the scan path, the check may produce zero findings or miss real issues. (Verify the current finding count by running `roko layer-check` before and after changes.)

3. **The `model` field on several structs is `String` when `Option<String>` is more accurate** — In `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` lines 376 and elsewhere, `model: String` is used with `String::new()` as a placeholder for "no model associated with this event." Using `Option<String>` makes the absence explicit, enables the compiler to enforce that consumers handle the None case, and eliminates the need for sentinel empty strings. The affected structs are `AgentStatusEntry` (line 376, `pub model: String`) and any parallel structs in `crates/roko-cli/src/tui/state.rs`. This is a broader change that touches serialization and display code; it may be scoped to a follow-up item if it proves large.

4. **The `claude --version` probe is copy-pasted across three files** — `auth_detect.rs` line 146, `bootstrap.rs` line 120, and `doctor.rs` lines 807 and 978 all call `std::process::Command::new("claude").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)` with identical logic. This is a DRY violation and each is an independent fix target for the layer-check.

## Implementation Plan

### Step 1 — Add `--version` probe exclusion to `check_direct_model_subprocess()`

File: `/Users/will/dev/nunchi/roko/roko/scripts/layer_check.rs`

In `check_direct_model_subprocess()` (line 284-312), after finding a line containing `Command::new("claude")`, check whether the line or the next line contains `--version`. If so, skip the violation:

```rust
fn check_direct_model_subprocess(
    root: &Path,
    findings: &mut Vec<ArchitectureFinding>,
) -> Result<()> {
    let needles = ["Command::new(\"claude\")", "Command::new(\"codex\")"];
    for path in rust_files_under(&root.join("crates"))? {
        let contents =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let gated = legacy_gated_lines(&contents);
        let lines: Vec<&str> = contents.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            let line_no = idx + 1;
            if gated.contains(&line_no) {
                continue;
            }
            for needle in needles {
                if !line.contains(needle) {
                    continue;
                }
                // Skip --version probes: these detect binary presence,
                // not LLM dispatch.
                let context_window = &lines[idx..usize::min(idx + 3, lines.len())];
                if context_window.iter().any(|l| l.contains("--version")) {
                    continue;
                }
                push_finding(
                    findings,
                    &path,
                    Some(line_no),
                    format!(
                        "direct model subprocess dispatch `{needle}` found in un-gated code; \
                         use ModelCallService or gate legacy CLI subprocess code behind \
                         `legacy-orchestrate`"
                    ),
                );
            }
        }
    }
    Ok(())
}
```

### Step 2 — Consolidate the `claude --version` probe into a shared helper

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs`

Add a public helper at the bottom of `auth_detect.rs` (it already imports `std::process::Command`):

```rust
/// Probe whether the `claude` CLI binary is available on PATH.
///
/// Runs `claude --version` — a lightweight check that does not perform any
/// LLM inference.
pub fn claude_cli_available() -> bool {
    std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

Then replace the three copy-pasted probe patterns with calls to `roko_cli::auth_detect::claude_cli_available()`:

- `crates/roko-cli/src/bootstrap.rs` line 120: replace inline probe
- `crates/roko-cli/src/doctor.rs` line 807 (`check_claude_cli()`): replace inline probe
- `crates/roko-cli/src/doctor.rs` line 978 (`check_available_providers()`): replace inline probe

After this change, `check_direct_model_subprocess()` in the layer-check still needs the `--version` exclusion from Step 1, because the helper itself still uses `Command::new("claude")`.

### Step 3 — Evaluate and scope the `model: String` -> `Option<String>` change

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`

Run `grep -n "model: String::new()" crates/` to find all construction sites. If the count is under 20, proceed with the change. If it's larger, file a separate backlog item.

For `AgentStatusEntry.model` at line 376, change:
```rust
pub model: String,
```
to:
```rust
#[serde(default)]
pub model: Option<String>,
```

Update all construction sites, serde deserializers (add `#[serde(default)]` if needed), and display code. In the TUI, render `None` as `"---"` or omit the field.

### Step 4 — Verify scan path for `check_empty_event_fields()`

File: `/Users/will/dev/nunchi/roko/roko/scripts/layer_check.rs`

In `check_empty_event_fields()` (line 344), the scan path is hardcoded to `crates/roko-runtime/src/`. Verify this is intentional:

```rust
for path in rust_files_under(&root.join("crates/roko-runtime/src"))? {
```

If the intent is to catch empty fields in all event-producing crates (including `roko-core` and `roko-cli`), expand the scan path to cover all of `crates/`. If the intent is narrow (only `roko-runtime`), the current path is correct but the backlog item's description was misleading.

Run `roko layer-check` to see the current violation count and determine whether this check is producing false positives or simply not scanning broadly enough.

## Acceptance Criteria

1. `roko layer-check` reports 0 violations for `Command::new("claude")` calls that include `.arg("--version")` in the surrounding 3 lines.
2. `roko layer-check` still flags `Command::new("claude")` calls that do NOT include `--version` (i.e., actual dispatch attempts).
3. The `claude_cli_available()` helper exists in `auth_detect.rs` and is used by `bootstrap.rs` and `doctor.rs`.
4. `cargo test -p roko-cli` passes after the consolidation.
5. The `check_empty_event_fields()` scan path decision is documented in a comment in `layer_check.rs`.

## Verification Checklist

- [ ] Run `roko layer-check` before any changes — record baseline violation count
- [ ] Apply Step 1 (--version exclusion), run `roko layer-check` again — verify doctor.rs and auth_detect.rs violations are gone
- [ ] Add a test `Command::new("claude")` call without `--version`, verify layer-check still flags it
- [ ] Apply Step 2 (consolidate probe), run `cargo test -p roko-cli` — all tests pass
- [ ] Check that `check_claude_cli()` in `doctor.rs` still correctly returns `DoctorStatus::Ok` when the binary is present and `Warn` when absent
- [ ] If Step 3 is undertaken: `roko dashboard` displays correctly with `Option<String>` model field

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/scripts/layer_check.rs` | Add `--version` context exclusion to `check_direct_model_subprocess()` (lines 293-309); add comment documenting `check_empty_event_fields()` scan path rationale |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs` | Add `pub fn claude_cli_available() -> bool` helper |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bootstrap.rs` | Replace inline `Command::new("claude")` probe at line 120 with `claude_cli_available()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` | Replace two inline `Command::new("claude")` probes at lines 807 and 978 with `claude_cli_available()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` | (Optional/scoped) Change `model: String` to `model: Option<String>` at line 376 and update construction sites |
