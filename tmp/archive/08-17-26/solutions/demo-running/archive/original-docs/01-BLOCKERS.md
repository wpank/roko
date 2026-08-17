# Demo Pipeline Blockers

These are the specific bugs that prevent the demo-app from running the PRD pipeline end-to-end. Each has file paths, root cause analysis, and a concrete fix.

---

## B1: `plan run` Ignores `--repo` for plans_dir — FIXED

**Status**: FIXED in plan.rs:220-227. `resolved_plans_dir = wd.join(plans_dir)` before validate_before_run().

---

## B2: `prd plan` Silent Extraction Failure — FIXED

**Status**: FIXED. Tools stripped from plan-generation dispatch (W0-A). Post-gen TOML validation added (R2).

---

## B3: Schema Mismatch Between `plan validate` and `plan run` — PARTIALLY FIXED

**Status**: Both now use `TasksFile::parse()`. Some edge cases remain where validate is more lenient.

---

## B4: Tracing Logs Dominate CLI Output — OPEN

**Symptom**: Every command dumps tracing INFO/WARN lines to stderr.

**Fix needed**: Route tracing to `.roko/roko.log` by default. Only show on stderr with `--verbose` or `RUST_LOG`.

**File**: `crates/roko-cli/src/main.rs` — tracing subscriber initialization

---

## B5: Config Version Warning on Every Command — OPEN

**Symptom**: `WARN: roko.toml uses config version 1` on fresh configs.

**Fix needed**: Check `config_version` field value, not `[providers]` table presence.

---

## B6: Terminal Freeze After Error — OPEN

**Symptom**: `roko` chat mode → API error → terminal unresponsive.

Three fixes needed:
1. Add Ctrl+C to Phase::Error in `chat_inline.rs:1323-1338`
2. RAII terminal guard in `inline/terminal.rs`
3. Panic hook to restore terminal

---

## B7: Error Messages Duplicated — OPEN

**Symptom**: Errors print twice (eprintln + top-level error handler).

**Fix**: Remove eprintln from command handlers. Let main() handle all error display.

---

## B8: Negative Cost Display — OPEN

**Symptom**: `roko status` shows `$-0.0000`.

**Fix**: `cost.max(0.0)` before formatting.

---

## B9: Demo-Specific: Consistent `--repo` Usage — FIXED

**Status**: FIXED. `roko_cmd()` in dev.sh passes `--repo $PIPELINE_WORKSPACE` consistently.

---

## B10: OpenAI `max_tokens` Rejected by Newer Models — FIXED

**Status**: FIXED. `use_max_completion_tokens` field added to ModelProfile.

---

## B11: Demo showCmd Returns ok:true for Failed Commands — FIXED

**Status**: FIXED. `showCmd()` captures exit code via hidden `execCmd('(exit $?)')`.

---

## B12: Gate Deadlock for Read-Only Roles — FIXED (Session 4)

**Symptom**: `plan run` hangs when researcher task completes and gate auto-pass sends on gate_tx from inside the select loop.

**Root cause**: Auto-pass gate completion used `ctx.gate_tx.send(completion).await` inline in `dispatch_action()`, which runs inside the `tokio::select!` loop. Sending on gate_tx blocks the loop that reads gate_rx → deadlock.

**Fix**: Send via `tokio::spawn()` so the select loop continues draining channels.

**File**: `crates/roko-cli/src/runner/event_loop.rs:2408-2440`

---

## B13: Scaffold Doesn't Handle Glob/Empty File Paths — FIXED (Session 3)

**Symptom**: Plan generates `files = ["crates/"]` or `files = ["crates/*/src/*.rs"]`. Scaffold tries to create crate named "" or "*".

**Fix**: Skip empty names and glob patterns in scaffold_missing_crates().

**File**: `crates/roko-cli/src/runner/plan_loader.rs:134-137`

---

## B14: Researcher Tasks Fail Structural Verify Steps — FIXED (Session 4)

**Symptom**: Researcher task T1 has verify step `test -f crates/roko-cli/src/main.rs` which checks for files that later implementer tasks create. Fails because the file doesn't exist yet.

**Fix**: Skip ALL gates for read-only roles (researcher/strategist/quick-reviewer). They don't produce artifacts to verify.

**File**: `crates/roko-cli/src/runner/event_loop.rs:2404-2445`
