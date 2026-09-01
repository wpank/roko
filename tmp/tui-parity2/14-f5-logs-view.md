# F5 Logs View Audit

**Files reviewed:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/logs_view.rs` (342 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/error_digest.rs` (633 lines)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` (LogEntry, LogSearchState, build_unified_log_cache, filter/scroll logic)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs` (LogFilterLevel, handle_logs_key, search keybindings)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` (action dispatch, scroll_logs_by, render_input_bar)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs` (ROSEDUST palette, semantic styles)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs` (SubView enum, for_tab, ViewState)

**Sub-tabs:** 1:Log (all sources) | 2:Signals (signal/episode only) | 3:Errors (error digest aggregation)

---

## 1. Log Entry Format

**Verdict: GOOD -- clear and well-structured**

Each log entry renders as: `[indicator] timestamp [LEVEL] source: message`

```
 timestamp  [level]  source:kind    message body
 12:34:56   [INF]    signal:gate    gate compile passed (2.3s)
```

- **Timestamp:** `HH:MM:SS` format from `format_log_timestamp_ms()`. UTC-only; no date component.
- **Level:** Three-letter labels (`DBG`, `INF`, `WRN`, `ERR`) in square brackets.
- **Source:** Prefixed by type (`signal:`, `episode:`, `gate:`, `efficiency:`, `event:`), with kind truncated to the last two colon-delimited segments via `truncate_log_kind()`.
- **Message:** Content-dependent per source type. Signal messages use `payload_preview` (truncated to 120 chars). Episode messages include kind, ok/FAIL, task ID, duration, gate summary, and model. Efficiency messages include role, model, token counts, cost, duration, and cache hit rate.
- **Selection indicator:** `>` glyph on the focused row, space otherwise.

**Issues:**
- Timestamps are UTC with no timezone indicator and no date. For runs spanning midnight or multi-day runs, two entries at "23:59:59" and "00:00:01" are ambiguous without a date column.
- No millisecond resolution despite the data source having millisecond precision -- two events in the same second are indistinguishable by timestamp.
- Signal message construction falls back to `signal.kind` when `payload_preview` is empty, which can produce confusing double-labeling (source shows `signal:gate:compile` and message also shows `gate:compile`).

**Recommendations:**
- Add a `YYYY-MM-DD` date prefix when entries span more than one calendar day, or always show it.
- Consider showing sub-second precision (`HH:MM:SS.mmm`) for high-frequency events.

---

## 2. Color Coding by Level

**Verdict: GOOD -- four distinct colors with semantic meaning**

The ROSEDUST palette maps levels as follows:

| Level | Color constant | RGB | Visual | Bold |
|-------|---------------|-----|--------|------|
| DEBUG | `theme.muted()` (TEXT_DIM) | `(145, 120, 138)` | Dim mauve | No |
| INFO  | `theme.text()` (TEXT) | `(165, 142, 158)` | Warm grey-rose | No |
| WARN  | `theme.warning()` (WARNING) | `(195, 155, 95)` | Amber/gold | **Yes** |
| ERROR | `theme.danger()` (EMBER) | `(195, 110, 85)` | Burnt orange-red | **Yes** |

Source labels get their own color coding:
- `signal:` -> `theme.info()` = DREAM (indigo, `120,115,165`)
- `episode:` -> `theme.accent()` = ROSE (pink, `185,120,148`)
- `gate:` -> `theme.warning()` = WARNING (amber)
- `efficiency:` -> `theme.muted()` = TEXT_DIM (dim)
- `event:` -> `theme.info()` = DREAM

**Issues:**
- DEBUG and INFO are close in the palette (both muted rose-grey tones, difference is 20 units in R, 22 in G, 20 in B). On some displays they will be hard to distinguish.
- Both `signal:` and `event:` sources share the same `theme.info()` color, making them visually indistinguishable by source tag alone. The text label differentiates them, but a quick visual scan cannot separate them.
- The message body uses `level_style` with `BOLD` removed. For WARN/ERROR messages, this means the message text is the same color as the level tag but without bold -- this is fine, but the level bracket and message can blur together since they are the same hue.

**Recommendations:**
- Consider giving DEBUG a more distinctive treatment -- perhaps italic modifier or a blue-ish tint -- to separate it from INFO.
- Give `event:` a distinct source color (e.g., `theme.text()` or a dedicated color).

---

## 3. Search Functionality

**Verdict: STRONG -- regex search with two modes, match navigation, and live feedback**

The search system (tagged `#217` in the codebase) is comprehensive:

**Activation:** `/` key in the logs tab opens the search input bar. Pattern is typed character-by-character with live regex compilation via `LogSearchState::recompile()`.

**Regex support:** Full regex via `regex::RegexBuilder` with `case_insensitive(true)` by default. Invalid patterns are caught and displayed as `[invalid regex]` in the input bar suffix. The compiled regex matches against both `entry.message` and `entry.source` fields.

**Two modes (toggled with `f` key):**
1. **Highlight mode** (default): All entries remain visible. Matching text segments are highlighted with `TEXT_STRONG` foreground on `DREAM` background via the `highlight_spans()` function, which splits text at regex match boundaries.
2. **Filter mode**: Only entries matching the pattern are shown. Non-matches are excluded from the rendered list entirely.

**Match navigation:**
- `n` jumps to next match (wraps around via modulo).
- `N` jumps to previous match (wraps around).
- Current match index is tracked. The input bar shows `[3/15 highlight]` style feedback.
- Navigating to a match scrolls the view and disables auto-tail.

**Input bar rendering:** `render_input_bar()` shows `[/] > pattern|` with a cursor indicator and suffix showing match count, current match index, and mode label.

**Match tracking:** `LogSearchState::update_matches()` rebuilds match indices on every keystroke by scanning all unified log entries. Indices are stored as `Vec<usize>` positions into the filtered entry list.

**Issues:**
- Match indices are recomputed on every keystroke against the full `cached_unified_log` vector. With 10,000 entries and a complex regex, this is O(n) per keystroke. Acceptable for 10K but could be noticeable with complex patterns.
- The `update_matches` method matches against `cached_unified_log` (all entries), but the rendered view may be level-filtered. This means match count/indices can reference entries that are currently hidden by level filters, leading to confusing jumps or incorrect `3/15` counts.
- Search is not retained across sub-tab switches. Switching from Log to Signals to Errors clears the search context.
- The `Enter` key accepts the search and exits input mode, but keeps the search pattern active for `n`/`N` navigation. The `Esc` key cancels and clears entirely. This is vim-like and intuitive.

**Recommendations:**
- Reconcile match indices with the currently active level filter to avoid phantom matches.
- Consider a debounce on regex recompilation for expensive patterns.

---

## 4. Filter by Level

**Verdict: GOOD -- intuitive toggle keys with visual feedback**

**Keys (active on F5 Logs tab):**
- `1` toggles INFO visibility
- `2` toggles WARN visibility
- `3` toggles ERROR visibility
- `4` toggles DEBUG visibility
- `a` shows all levels (resets to all visible)

**State:** `HashSet<LogFilterLevel>` in `TuiState`. Levels start all-enabled. Toggle inserts if absent, removes if present.

**Visual feedback:** The status bar renders each level label like `[INF]` `[WRN]` `[ERR]` `[DBG]` with its semantic color when active, and `theme.muted()` when disabled. The entry count shows `123/456 entries` when filters are active vs `456 entries` when all are shown.

**Filtering path:** `render_with_entries()` applies level filtering first (`log_level_visible`), then applies search filtering. This means search and level filters compose correctly.

**Issues:**
- The key mapping `1=INF, 2=WRN, 3=ERR, 4=DBG` does not follow severity order (DEBUG should logically be first, or the keys should match the visual order in the status bar). The `LogFilterLevel::all()` method returns `[Info, Warn, Error, Debug]`, and the status bar renders them in that order -- but the key mapping `1-4` does match this displayed order, so it is consistent with what the user sees.
- There is no indication in the keybinding help that `1-4` are available. Users must discover these by trial.
- No quick "errors only" or "warnings and above" shortcut -- users must toggle each level individually.

**Recommendations:**
- Add a severity-threshold shortcut (e.g., `!` for errors-only, `@` for warn+error).
- Show the `1/2/3/4/a` hint in the status bar or a help line.

---

## 5. Log vs Signals Sub-Tab

**Verdict: ADEQUATE -- clear intent, but filtering is basic**

The three sub-tabs are rendered in the sub-view bar as `[1:Log] 2:Signals 3:Errors`:

| Sub-tab | Index | What it shows |
|---------|-------|---------------|
| Log | 0 | All unified entries from all 5 sources |
| Signals | 1 | Only entries whose `source` starts with `signal:` or `episode:` |
| Errors | 2 | Full error digest aggregation panel (separate widget) |

**Signals filtering:** The Signals sub-tab performs a simple `.starts_with()` filter on source strings. This means efficiency events, gate failure rows, and generic event log entries are excluded. The filtering happens by cloning matching entries into a new `Vec<LogEntry>`, which allocates.

**Issues:**
- The "Signals" label may confuse users who expect raw Signal data (engram JSONL contents). What this actually shows is signal-sourced and episode-sourced log entries formatted as log rows -- not a raw signal inspector.
- There is a naming collision: the F7 Inspect tab also has a "Signals" sub-view (`SubView::SignalDag`). Two different tabs with "Signals" sub-views showing different content is potentially confusing.
- The Signals sub-tab does not carry over the search state from the Log sub-tab. Search and level filters apply independently per sub-tab switch since they use the same `tui_state.log_search` but the entry set changes.
- The Errors sub-tab is an entirely different rendering path (`error_digest::render_error_aggregation_panel`) with its own scroll state but no search support.

**Recommendations:**
- Rename "Signals" to "Events" or "Signal Events" to distinguish from the raw signal DAG.
- Carry search state across sub-tab switches where applicable.

---

## 6. Error Digest

**Verdict: STRONG -- well-organized aggregation with multiple sources**

**Data sources collected by `collect_all_errors()`:**
1. Gate recent failures (`tui_state.gate_recent_failures`)
2. Efficiency events with `gate_passed == Some(false)` (individual gate errors extracted, with a fallback generic entry)
3. Gate results page failure rows
4. Failed agents (status `is_failed()`)

**Categorization:** `ErrorCategory::from_message()` classifies by keyword matching into 5 categories:
- Gate (keywords: gate, verify, rung)
- Compile (keywords: compil, cargo, rustc)
- Agent (keywords: agent, stall, timeout)
- Preflight (keywords: preflight, pre-flight)
- Runtime (fallback)

Each category has a Unicode icon and a dedicated color from the theme palette.

**Layout:**
- Category summary bar: icon + label + count per category, total count, latest timestamp.
- Scrollable error list: most-recent-first, each row shows `[HH:MM:SS] icon source: message`.
- Messages are truncated to fit terminal width.
- Panel border turns red (`theme.danger()`) when errors are present.
- Centered "no errors recorded" message with helpful description when empty.

**Deduplication:** Errors are deduplicated by `(category, message, source)` tuple, keeping the entry with the latest timestamp (via `dedup_by` after sort).

**Compact mode:** `render_error_digest()` provides an inline compact version with a gate pass/fail ratio header and a simple error list, used by other views.

**Issues:**
- The keyword-based categorization is fragile. A message like "agent failed to compile cargo manifest" would match "agent" first (due to match ordering) and be categorized as Agent rather than Compile. The `from_message()` method checks categories in a fixed order: Gate -> Compile -> Agent -> Preflight -> Runtime.
- The `fmt_ts()` function computes timestamp from epoch milliseconds as `(ts / 1000) % 86400` which is UTC mod-24h. This loses date information entirely.
- The compact `render_error_list` uses `.take(area.height as usize)` which clips to viewport height but does not support scrolling within the compact panel.
- No error detail expansion -- clicking or selecting an error does not show the full message, stack trace, or related context.

**Recommendations:**
- Use structured error categories from the source data rather than keyword-matching on message text.
- Add an enter/drill-in action to see full error details.

---

## 7. Auto-Scroll Behavior

**Verdict: GOOD -- correct auto-tail with intuitive break/resume**

**State:** `tui_state.log_auto_tail: bool`, defaults to `true`.

**Behavior:**
- When `auto_tail == true`: scroll offset is set to `max_scroll` (bottom of the log), and the focused row is the last entry. New entries arriving via `refresh_cached_unified_log()` keep the view pinned to the bottom.
- When user scrolls up (`k`/Up): `scroll_logs_by()` sets `log_auto_tail = false` and tracks a manual scroll offset.
- When user scrolls down past the bottom (`j`/Down past max): `auto_tail` is re-enabled automatically.
- `End`/`G` key explicitly sets `auto_tail = true` and resets scroll to 0.
- `Home` key scrolls to top (auto-tail disabled implicitly via `scroll_logs_by` with negative delta).
- Search match navigation (`n`/`N`) disables auto-tail to keep the matched entry in view.

**Status bar feedback:** Shows `[TAIL]` when auto-tailing, `[SCROLL]` when manually scrolled. Styled with `theme.accent()`.

**Scroll acceleration:** `scroll_accel.tick()` provides progressive scroll speed for held keys.

**Issues:**
- The `reset_scroll_state()` method sets `log_auto_tail = true`, which means any global scroll reset (tab switches, etc.) will snap the log back to the tail. This is probably desired but could be surprising if the user was reading a specific entry and switches tabs then returns.
- The scroll offset is stored as `usize` (`log_scroll`) but the `ViewState.scroll` field is `u16`, meaning max addressable is 65535 entries. The `max_scroll` calculation does `.min(u16::MAX as usize) as u16`. With 10,000 max unified log entries, wrapping is not possible for entry count, but with `Wrap { trim: false }` enabled, a long message could produce more visual lines than entries, potentially exceeding u16 in extreme cases.

**Recommendations:**
- Consider preserving scroll position per-tab so returning to F5 Logs does not always snap to tail.

---

## 8. Performance with Thousands of Log Entries

**Verdict: ADEQUATE -- bounded but with allocation concerns**

**Hard cap:** `MAX_UNIFIED_LOG_ENTRIES = 10_000`. The `build_unified_log_cache()` function keeps only the most recent 10K entries by skipping older ones.

**Cache strategy:** The unified log is rebuilt (`refresh_cached_unified_log`) on each data refresh, not on each frame render. The result is stored in `cached_unified_log: Vec<LogEntry>`.

**Sorting:** Uses `BTreeMap<(i64, usize), LogEntry>` during construction, which provides O(n log n) insertion and ordered iteration. The `(timestamp_ms, sequence_number)` composite key ensures stable ordering even for simultaneous events.

**Rendering:** Every frame iterates all filtered entries to build `Vec<Line<'_>>`, even though only `inner.height` lines are visible. The `Paragraph` widget with `.scroll((scroll, 0))` handles viewport clipping internally, but all lines are constructed first.

**Per-frame allocations:**
- `level_filtered: Vec<&LogEntry>` -- full scan of all entries.
- `filtered_entries: Vec<&LogEntry>` -- second allocation if search filter is active.
- `lines: Vec<Line<'_>>` -- one Line per filtered entry, each containing 8+ Spans with owned String data from `entry.message.clone()`, `entry.source.clone()`, `entry.timestamp.clone()`.
- Highlight mode calls `highlight_spans()` which allocates a `Vec<Span>` with `text.to_owned()` for each segment.

**Estimated per-frame cost at 10K entries:** ~10K String clones (message + source + timestamp = ~30K allocations) plus Line/Span construction. At 60fps this is ~1.8M allocations/sec. In practice, TUI tick rates are typically 4-30fps, making this ~120K-900K allocations/sec.

**Issues:**
- The Signals sub-tab clones all matching entries: `filtered.iter().cloned().collect()`. This copies every `LogEntry` struct including owned Strings. A `Vec<&LogEntry>` would suffice since the entries are borrowed for the duration of the render call.
- Search `update_matches()` is called on every keystroke and scans all 10K entries. Regex compilation also happens per-keystroke.
- The `BTreeMap` in `build_unified_log_cache()` allocates tree nodes for every entry, then converts to `Vec` via `into_values()`. A simpler approach would be to collect into a `Vec` and sort once.

**Recommendations:**
- Replace the per-entry `String::clone()` in line construction with borrowed spans where possible (entries live for the duration of the render call).
- Replace the Signals sub-tab `.cloned().collect()` with a reference-based filter.
- Consider a virtual list approach: only construct Lines for visible rows (`scroll..scroll+height`) rather than all entries.

---

## 9. Log Entry Selection / Detail View

**Verdict: GAP -- selection exists visually but has no detail action**

**Current state:**
- A `row_focus_idx` tracks the visually selected row. When not auto-tailing, it follows `view_state.scroll`. When auto-tailing, it is the last entry.
- The selected row gets a `>` indicator, bold styling, and a `selection_background` highlight.
- Search match entries also get the selection background treatment.

**Missing:**
- There is no `Enter` / drill-in action on the selected log entry. The `handle_logs_key` function maps `Enter` to nothing (falls through to `_ => TuiAction::None`).
- No detail modal or side panel showing the full untruncated message, original JSON source, associated plan/task, or related entries.
- No copy-to-clipboard for a selected entry.
- No way to jump from a log entry to the related plan/task in the F2 Plans tab.

**Recommendations:**
- Add `Enter` to open a detail modal showing: full message (untruncated), original source data (signal hash, episode ID, etc.), plan/task cross-reference, and timestamp with full precision.
- Add `y` to copy the selected entry's message to clipboard.
- Add cross-tab navigation: `Enter` on a gate failure entry could jump to the corresponding task in F2.

---

## 10. Proposed Enhancements

### 10a. Collapsible Log Groups

**Problem:** During a plan run, a single task generates a cluster of related entries: dispatch, efficiency metrics, gate results, success/failure. These are interleaved with entries from other tasks, making it hard to follow a single task's lifecycle.

**Proposal:**
- Group entries by `plan_id/task_id` (extractable from source and message fields).
- Show the group header as a single summary line with expand/collapse toggle.
- Collapsed: `[+] task-id: 5 entries (2 gates, 1 error) -- 12:34:56-12:35:12`
- Expanded: all entries indented under the group header.
- Toggle with `Space` or `Enter` on the group header.

**Complexity:** Medium. Requires grouping logic in `build_unified_log_cache()` or a post-processing step, plus tracking expanded/collapsed state per group.

### 10b. Syntax Highlighting for Structured Logs

**Problem:** Many log messages contain structured data (model names, token counts, durations, cost values, file paths, task IDs) that is rendered as plain text. Scanning for specific values requires reading each field.

**Proposal:**
- Detect and highlight common patterns within message text:
  - Numeric values (token counts, durations, costs) in `theme.accent()`.
  - Model names (e.g., `claude-opus-4-6`) in `theme.info()`.
  - File paths in `theme.muted()` with underline.
  - Status indicators (`ok`/`FAIL`/`passed`/`failed`) in `theme.success()`/`theme.danger()`.
  - Cost values (`$0.0042`) in `theme.warning()`.
- Apply as a post-processing step in line construction, similar to how `highlight_spans()` works for search matches.

**Complexity:** Medium. Pattern detection via regex, applied per-line during render. Should be cached alongside the unified log to avoid per-frame regex execution.

### 10c. Timeline View

**Problem:** The current log is a flat chronological list. There is no way to see temporal density, identify bursts of activity, or visualize gaps between events.

**Proposal:**
- A horizontal timeline bar at the top of the log panel (replacing or augmenting the status bar).
- Time axis spanning the range of visible entries.
- Colored dots or density bars per time bucket, colored by dominant severity.
- Clickable: selecting a time region scrolls the log to that timestamp.
- Optional sparkline showing event rate over time.

**Complexity:** High. Requires time-bucketing logic, a new widget, and interaction handling. The `ratatui` Canvas or Sparkline widgets could be used.

### Additional proposals:

### 10d. Log Export

**Problem:** No way to extract log data for external analysis or bug reports.

**Proposal:** `Ctrl-E` exports the current (filtered) log view to a file, respecting active level and search filters.

### 10e. Relative Timestamps

**Problem:** Absolute timestamps (`HH:MM:SS`) do not convey how long ago an event occurred or the interval between events.

**Proposal:** Toggle between absolute timestamps and relative timestamps (`+0.0s`, `+1.2s`, `+45.3s` from the first visible entry, or `3m ago`, `1h ago` from now). Toggle with `t` key.

---

## Summary

| # | Area | Verdict | Key Finding |
|---|------|---------|-------------|
| 1 | Log entry format | Good | Clear 4-field layout; lacks date and sub-second precision |
| 2 | Color coding | Good | Four distinct levels; DEBUG/INFO could be more distinct |
| 3 | Search | Strong | Full regex, two modes, match navigation; match indices ignore level filter |
| 4 | Level filter | Good | Toggle keys with visual feedback; no severity-threshold shortcut |
| 5 | Log vs Signals | Adequate | Clear sub-tabs; naming collision with F7 "Signals" |
| 6 | Error digest | Strong | Multi-source aggregation with dedup; keyword categorization is fragile |
| 7 | Auto-scroll | Good | Correct tail/scroll toggle; scroll state not preserved across tab switches |
| 8 | Performance | Adequate | 10K cap is safe; per-frame String cloning and full-list Line construction are wasteful |
| 9 | Selection/detail | Gap | Visual selection exists but no drill-in, no detail view, no cross-tab navigation |
| 10 | Proposals | -- | Collapsible groups, structured highlighting, timeline, export, relative timestamps |

### Priority fixes:
1. **Entry detail view** (item 9) -- the selected row is inert; `Enter` should show full details.
2. **Search-filter match index reconciliation** (item 3) -- match counts are wrong when level filters are active.
3. **Performance: eliminate per-frame clones** (item 8) -- use borrowed spans instead of `entry.message.clone()`.
