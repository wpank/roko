# TUI Stubs, Placeholders, Hardcoded Values & Silent Failures

Every TODO, stub, placeholder, hardcoded value, silent error swallow, and stringly-typed state in the TUI codebase. Found by grepping and reading every file.

**Audit date**: 2026-04-14
**Codebase**: 37,875 lines across 69 Rust files in `crates/roko-cli/src/tui/`

---

## 1. TODOs, FIXMEs, todo!(), unimplemented!()

**None found.** Zero `TODO`, `FIXME`, `HACK`, `XXX`, `STUB` comments. Zero `todo!()` or `unimplemented!()` macros. Issues are embedded in code structure, not flagged with comments.

---

## 2. Hardcoded context limit: 200,000 (5 occurrences)

Every occurrence means the TUI displays fake context window size.

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S1 | `state.rs` | 771 | `context_limit: 200_000, // sensible default` | Agent rows always get 200K regardless of model (Haiku 200K, GPT-4 128K, Gemini 1M) |
| S2 | `agents_view.rs` | 246 | `let ctx_limit = 200_000u64;` | F3 Agents tab context bar always against 200K |
| S3 | `dashboard_view.rs` | 218-222 | `row.map_or(200_000, |r| { if r.tokens_used > 0 { r.tokens_used * 2 } else { 200_000 } })` | Dashboard parallel pool uses 200K fallback OR fabricated "2x tokens_used" total |
| S4 | `config_meta.rs` | 219 | `max: Some(200_000)` | Config UI caps agent.context_limit_k at 200K |

---

## 3. Hardcoded terminal size: 80x24

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S5 | `app.rs` | 1093-1094 | `Rect::new(0, 0, 80, 24)` | **ACTIVE BUG**: Mouse click hit-test uses hardcoded size instead of actual terminal. Every click on terminals >80x24 maps to wrong zone. |

---

## 4. Hardcoded timing constants

| ID | File | Line | Value | Purpose |
|----|------|------|-------|---------|
| S6 | `app.rs` | 250 | `16ms` | Event poll interval (~60fps) |
| S7 | `app.rs` | 264 | `3s` | Sys metrics collection interval |
| S8 | `app.rs` | 283 | `500ms` | Data refresh thread interval (high disk I/O) |
| S9 | `app.rs` | 316 | `3s` | Git data refresh interval |
| S10 | `app.rs` | 354 | `3s` | Idle timeout before adaptive frame rate kicks in |
| S11 | `scroll.rs` | 23 | `300ms` | Scroll acceleration threshold |

---

## 5. Phase pipeline is fake/synthetic

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S12 | `state.rs` | 945-955 | `let midpoint = CANONICAL_PHASES.len() / 3;` + position-based done/active/pending | Phase progress bar divides 9 phases into thirds, marks first third "done", one "active", rest "pending". Zero connection to actual task execution. Progress bar lies. |
| S13 | `state.rs` | 963-967 | `PhaseStatus::Active => 50.0` | Active phases always show 50% regardless of real progress |
| S14 | `state.rs` | 962 | `elapsed_secs: 0.0` | All phases start at 0 elapsed, only populated if episodes match |

---

## 6. Plan tasks_done is binary (0 or 100%)

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S15 | `state.rs` | 739 | `let tasks_done = if completed { p.task_count } else { 0 };` | A plan with 9/10 tasks done shows 0/10. Progress jumps from 0% to 100% with no intermediate values. |
| S16 | `state.rs` | 748 | `tasks_failed: 0` | Failed task count always hardcoded to 0. Plan tree never shows failures. |

---

## 7. Modals open with empty data (Vec::new())

| ID | File | Line | Modal | Impact |
|----|------|------|-------|--------|
| S17 | `app.rs` | 586-589 | WaveOverview | `waves: Vec::new()` instead of `tui_state.execution_waves.clone()` |
| S18 | `app.rs` | 597-601 | QueueOverview | `milestones: Vec::new()` — no milestone source exists |
| S19 | `app.rs` | 608-612 | TaskPicker | `tasks: Vec::new()` — "No tasks found" always |

---

## 8. Agent rows constructed with empty/zero defaults

| ID | File | Line | Field | Impact |
|----|------|------|-------|--------|
| S20 | `state.rs` | 768 | `model: String::new()` | Blank until episode provides it |
| S21 | `state.rs` | 769-770 | `input_tokens: 0, output_tokens: 0` | Zero until episode |
| S22 | `state.rs` | 773 | `current_task: String::new()` | Empty until episode |
| S23 | `state.rs` | 774 | `last_output_line: String::new()` | Empty until episode |

Running agents show with no model, no tokens, no task, no output.

---

## 9. TogglePause / Approve / Confirm — actions that do nothing

| ID | File | Line | Action | Problem |
|----|------|------|--------|---------|
| S24 | `app.rs` | 636-642 | TogglePause | Toggles string "paused"/"running" — no widget reads it, no signal to orchestrator |
| S25 | `app.rs` | 657-658 | ApproveCommand | Sets `pending_approval = None` — but it was already None. No signal written. |
| S26 | `app.rs` | 660-661 | ApproveAll | Same no-op |
| S27 | `app.rs` | 663-664 | RejectCommand | Same no-op |
| S28 | `app.rs` | 748-778 | ConfirmYes | Writes `roko.tui.confirm` signal to signals.jsonl — orchestrator never reads this file for commands. Logging-only. |

---

## 10. Silent error swallowing — `.ok()` on critical paths

Zero logging/tracing in entire TUI. Every error is invisible.

### Thread spawn failures

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S29 | `app.rs` | 267 | `tx.send(metrics).ok();` + thread panic | If sys-metrics thread fails, TUI shows CPU 0% MEM 0 forever with no error |
| S30 | `app.rs` | 286 | `.ok();` | If data-refresh thread fails, TUI never updates. Shows initial snapshot forever. |
| S31 | `app.rs` | 319 | `.ok();` | If git-refresh thread fails, git info never updates. |

### File/JSON operations

| ID | File | Line | Pattern | Impact |
|----|------|------|---------|--------|
| S32 | `dashboard.rs` | 64 | `std::fs::metadata(path).ok()?` | File metadata failure → silent None |
| S33 | `dashboard.rs` | 2309 | `std::fs::read_to_string(path).ok()?` | Efficiency file read failure → silent None |
| S34 | `dashboard.rs` | 2313 | `.and_then(|line| serde_json::from_str(line).ok())` | JSON parse failure → silently dropped |
| S35 | `dashboard.rs` | 2051 | `.filter_map(|entry| serde_json::from_value::<Episode>(entry).ok())` | Corrupted episodes silently disappear |
| S36 | `dashboard.rs` | 2452-2463 | `.filter_map(|line| serde_json::from_str(line).ok())` | Corrupted signals silently vanish |
| S37 | `dashboard.rs` | 3672-3673 | `std::fs::read_to_string(&state_path).ok()? / serde_json::from_str(&state_text).ok()?` | Executor state parse failure → silent None |
| S38 | `git_view.rs` | 432 | `Command::new("git").args(args).output().ok()?` | Git command failure → silent None |
| S39 | `config_meta.rs` | 668-675 | `.ok().and_then(|s| s.parse().ok())` | Config parse failure → silent None |

### Signal/inject write failures

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S40 | `app.rs` | 687-694 | `if let Ok(mut f) = ... { let _ = writeln!(f, ...); }` | If signals.jsonl can't be opened OR write fails, inject silently fails BUT success notification still shows |
| S41 | `app.rs` | 764-770 | Same pattern | Confirm signal write silently fails with false success notification |

---

## 11. No logging or tracing anywhere

**Zero occurrences** of `eprintln!`, `tracing::warn!`, `tracing::error!`, `tracing::debug!`, `log::warn!`, `log::error!` in the entire TUI codebase. If anything fails, the user sees empty/zero data with absolutely no clue why.

---

## 12. Stringly-typed status (90+ occurrences)

Status comparisons use raw string literals across 15+ files with no enum. Inconsistencies:

| Pattern | Meaning | Problem |
|---------|---------|---------|
| `"running"` / `"active"` | Agent is executing | Two strings for same state |
| `"failed"` / `"error"` | Agent/task failed | Two strings for same state |
| `"done"` / `"completed"` / `"passed"` | Task succeeded | Three strings for same state |
| `"running"` / `"active"` / `"executing"` | Task in progress | Three strings for same state |
| `"queued"` / `"pending"` | Waiting to start | Two strings for same state |

Any typo in a status string silently falls through to default/catch-all with no compile-time check.

---

## 13. `"unknown"` fallback labels (10+ occurrences)

| File | Line | Code |
|------|------|------|
| `dashboard.rs` | 1508 | `.unwrap_or("unknown")` |
| `dashboard.rs` | 1637 | `current_phase_label(...).unwrap_or_else(|| "unknown".to_string())` |
| `dashboard.rs` | 1690, 1733 | `.unwrap_or("unknown")` |
| `dashboard.rs` | 2293, 2436 | Same |
| `dashboard.rs` | 3707, 4597, 4625 | Same |
| `context_view.rs` | 310, 408 | Same |

User sees "unknown" for status/phase/model when data is missing or malformed.

---

## 14. `"--"` placeholder for missing timing/cost

| File | Line | Context |
|------|------|---------|
| `dashboard.rs` | 1892 | Plan cost → "--" |
| `dashboard.rs` | 2138 | Duration format → "--" |
| `state.rs` | 1107 | Duration parse → 0.0 when "--" |

---

## 15. Verify column always placeholder dot

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S42 | `plan_tree.rs` | 431-435 | `let verify_cell = (format!("{:>width$}", "\u{00b7}", ...), MoriTheme::TEXT_PHANTOM)` | Plan tree "vfy" column always shows `·` in phantom color. Never shows actual verification status. |

---

## 16. Fallback task titles use raw IDs

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S43 | `state.rs` | 1096-1097 | `title: t.task_id.clone()` | In fallback path (no PlanExecutionSnapshot), task title = task ID. User sees "task-0042" instead of "Implement auth middleware". |

---

## 17. macOS-only system metrics

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S44 | `app.rs` | 1633 | `#[cfg(target_os = "macos")]` | `collect_sys_metrics_bg()` only works on macOS (uses `top -l 2`). Linux returns `SysMetrics::default()` — all zeros forever with no indication. |

---

## 18. Network/disk metric names lie

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S45 | `app.rs` | 1682 | `metrics.net_down_bytes_sec = net_in_bytes;` | Field named `_bytes_sec` (rate) but stores cumulative total since boot |
| S46 | `app.rs` | 1683 | `metrics.disk_read_bytes_sec = disk_read_bytes;` | Same: named as rate, stores total |

---

## 19. Git age never refreshes from background thread

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S47 | `app.rs` | 305 | `let age = String::new();` | Background git thread always sends empty age. `tui_state.git_age` set once at startup, never updated. Goes stale during sessions. |

---

## 20. Gate verdicts have timestamp 0

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S48 | `dashboard_view.rs` | 414 | `ts_millis: 0` | Gate verdicts in error digest have epoch timestamp (1970-01-01). Age display shows "~20000d" or similar absurdity. |

---

## 21. `#[allow(dead_code)]` annotations

| File | Line | Code |
|------|------|------|
| `app.rs` | 1274 | `#[allow(dead_code)] fn select_page_by_slot(...)` — never called |
| `app.rs` | 1432 | `#[allow(dead_code)] fn scroll_for(...)` — never called |
| `dashboard.rs` | 2766 | `#[allow(dead_code)] provider_id: String` — deserialized, never used |

---

## 22. Entire pages system is placeholder scaffold

| ID | File | Lines | Impact |
|----|------|-------|--------|
| S49 | `pages/mod.rs` | 1-420 | Explicitly labeled "placeholder page and widget models" |
| S50 | `pages/efficiency.rs` | all | 8 efficiency pages (Health, Trends, etc.) as placeholder scaffolds |
| S51 | `pages/operations.rs` | all | 7 operational pages as placeholder scaffolds |

These scaffolds exist for the text-mode legacy rendering path. The F1-F7 tabs use real views, but the PageId-based pages behind the legacy path render only text summaries.

---

## 23. ExpandCollapse / CollapseExpand identical implementations

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S52 | `app.rs` | 618-635 | Both do `plan.expanded = !plan.expanded` | Two TuiAction variants with identical behavior. `CollapseExpand` is unreachable (no key binding). |

---

## 24. `secondary_selected: 0` always hardcoded

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S53 | `app.rs` | 415 | `secondary_selected: 0` | ViewState sub-selection always starts at 0. Any view using this for within-list selection can't maintain position. |

---

## 25. `extract_episode_output` last resort returns empty string

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S54 | `state.rs` | 1044 | `episode.failure_reason.as_deref().unwrap_or("").to_string()` | Episodes with no output in 8 searched keys AND no failure_reason return empty. Agent appears to have produced nothing. |

---

## 26. `episode_to_phase_name` returns empty for unrecognized episodes

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S55 | `state.rs` | 1197 | `String::new()` | Unrecognized episode kinds contribute no elapsed time to any phase. Invisible in phase timeline. |

---

## 27. Diff panel only shows unified-diff-format lines

| ID | File | Line | Code | Impact |
|----|------|------|------|--------|
| S56 | `dashboard_view.rs` | 370-398 | `gather_diff_text()` filters for lines starting with `+`, `-`, `@@`, `diff ` | If agent output contains code changes not in unified diff format, diff panel is empty. |

---

## Summary

| Category | Count |
|----------|-------|
| Hardcoded values (context limit, terminal size, timing) | 11 |
| Fake/synthetic data (phase pipeline, binary progress) | 5 |
| Modals with empty Vec data | 3 |
| Agent row empty defaults | 4 |
| No-op actions (pause, approve, confirm) | 5 |
| Silent error swallowing (.ok()) | 13 |
| Zero logging/tracing | 1 (systemic) |
| Stringly-typed status | 1 (systemic, 90+ sites) |
| Placeholder/fallback values | 12 |
| Platform-specific gaps | 2 |
| Dead code with allow(dead_code) | 3 |
| Scaffold/placeholder systems | 3 |
| Duplicate/identical actions | 1 |
| **Total discrete gaps** | **64** |
