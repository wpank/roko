# F7 Inspect View Audit

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/context_view.rs`
**Lines**: ~680 lines (render functions + helpers)
**Sub-views**: 6 (Overview, Signals, Episodes, Knowledge, Cost/Model, Inspect)

---

## 1. MCP Panel: What Does It Show?

The MCP panel lives in sub-view 6 (ThreePanelInspect), column 1 of a three-column layout.

**What it shows** (from `render_mcp_panel`, line ~530):
- Whether `roko.toml` config exists (green "roko.toml" / red "not found")
- Total registered tool count (from `.roko/state/mcp-stats.json`)
- Connected MCP server names (from `mcp-stats.json`), listing up to 5
- AST index file count
- AST index symbol count

**Data source**: `InspectData.mcp` (`McpRuntimeData` struct), loaded from disk every 5 seconds via `InspectData::load_from_workdir`.

**Verdict: Shallow and mostly static.** The panel shows counts and a boolean "config exists" check, but nothing about runtime health:
- No connection status per server (connected/disconnected/errored)
- No latency or call counts per MCP server
- No tool invocation success/failure rates
- No error messages or last-error timestamps
- No indication of which tools belong to which server
- The server list is capped at 5 with no scroll or "and N more" indicator
- The AST index stats are tangential to MCP runtime and feel like padding

**Compare to F1 Dashboard MCP sub-tab** (sub-tab index 5, `render_sub_mcp` in `dashboard_view.rs`): The F1 version is actually richer -- it loads the real `McpConfig` from `roko.toml`, shows the configured/resolved path, per-server command lines, and error messages. The F7 version is a stripped-down summary that loses all that operational detail.

---

## 2. Learning Panel: Cascade Router, Playbooks, Experiments

The Learning panel lives in sub-view 6 (ThreePanelInspect), column 2.

**What it shows** (from `render_learning_panel`, line ~570):
- Episode count with pass/fail breakdown
- Accuracy percentage (color-coded green/amber/red)
- Playbook rule count
- Routing coverage percentage with model count
- Gate thresholds per rung (if non-empty), up to 8 rows with color-coded values

**Data source**: `InspectData.learning` (`LearningData` struct), also loaded from disk every 5 seconds.

**Verdict: Adequate summary, but not actionable.**
- Shows the "what" (counts) but not the "when" (no timestamps, no trend)
- No experiment information whatsoever despite experiments being a major learning subsystem. The `TuiState.experiment_winners` field exists but is not used here.
- No playbook content preview -- just a count of rules
- Gate thresholds are shown as bare numbers without their EMA alpha, observation count, or trend direction
- The routing coverage metric is a single percentage with no breakdown of which models have coverage

---

## 3. Prompt Stats Panel: What's Shown?

The Prompt Stats panel lives in sub-view 6 (ThreePanelInspect), column 3.

**What it shows** (from `render_prompt_stats_panel`, line ~610):
- "Avg Tokens per Role" with per-role token count and context utilization percentage
- "Context Utilization" overall summary with avg tokens and total role count
- Utilization is color-coded: green (<50%), amber (50-80%), red (>80%)

**Data source**: `InspectData.prompt_stats` (`PromptStatsData` struct). Fields `tokens_per_role` and `context_utilization` are populated from efficiency events. The `top_sections_by_cost` field exists but is hardcoded to an empty `Vec::new()` with the comment "requires section effectiveness data" -- it is never populated.

**Verdict: Incomplete and underspecified.**
- The "Top Sections by Cost" feature is stubbed out and never filled
- No breakdown of input vs. output tokens per role
- No cache hit rate per role (even though the Overview sub-view shows cache% in its "Token Burn by Role" table)
- No system prompt vs. user content vs. tool output breakdown
- "Context Utilization" is computed from efficiency events but the denominator (max context window) is never shown, so the percentage is meaningless to the user without context

---

## 4. Three-Panel Inspect: Accessible? Useful?

**Accessibility**: The three-panel layout is sub-view 6 (the last one). Users reach it by pressing `6` while on the F7 tab. The sub-view bar shows `[1:Overview] 2:Signals 3:Episodes 4:Knowledge 5:Cost/Model 6:Inspect`.

**Problems**:
- It is the deepest-nested panel: F7 then press 6. Users are unlikely to discover it without reading docs.
- The sub-view label "Inspect" on the F7 tab called "Inspect" is confusingly recursive. The F7 tab header says "Inspect" and the sixth sub-tab also says "Inspect."
- Each column is 33% width. On an 80-column terminal that leaves ~25 usable characters per panel, which is very tight for table data. The MCP panel's "config: roko.toml" barely fits.
- No focus navigation between the three columns -- there is no way to expand one panel or scroll within them independently.
- The three topics (MCP / Learning / Prompt Stats) have no conceptual relationship. They are three orphaned panels glued together because they did not have a home elsewhere.

---

## 5. Sub-Tab Organization: Are the 6 Sub-Tabs Well-Organized?

The F7 Inspect tab has these sub-views:

| # | Label | Content |
|---|---|---|
| 1 | Overview | System health + token burn by role + cost by model + cascade route + alerts |
| 2 | Signals | Signal DAG tree browser |
| 3 | Episodes | Episode replay table (time, agent, result, wall, tokens) |
| 4 | Knowledge | Knowledge store browser with search filtering |
| 5 | Cost/Model | Dedicated per-model cost table (widget from `widgets/cost_by_model.rs`) |
| 6 | Inspect | Three-panel MCP / Learning / Prompt Stats |

**Problems**:

- **Overview is overloaded.** Sub-view 1 crams four panels into a 20/40/40 vertical split, each split horizontally. That is 6 logical panels in one sub-view. It is the most complex single render in the entire TUI.
- **Cost/Model duplicates Overview.** Sub-view 1 already has a "Cost by Model" panel in its mid-right quadrant. Sub-view 5 is a full-screen version of the same data with slightly richer columns (adds Provider and $/Task). Having both is confusing -- users see cost-by-model in two places and wonder if they are the same.
- **Signals and Episodes should arguably live under F5 Logs.** The Signal DAG and Episode Replay are log/event browsing views. F5 already has FilteredLog, SignalStream, and ErrorDigest sub-views. Having Signal DAG under F7 and Signal Stream under F5 with the same label "Signals" is actively confusing.
- **Knowledge is well-placed** but could be under a dedicated tab given its depth (search filtering, confidence bars, tag matching).
- **Sub-view 6 has no thematic coherence** (see finding 4).

---

## 6. Data Freshness

**Refresh cadence**: `InspectData` (used by sub-view 6) refreshes every 5 seconds, triggered by `inspect_needs_refresh()` which checks `Instant::now() - inspect_last_refresh >= 5s`. This is called in the main app tick loop.

**The rest of the sub-views** use `TuiState` fields directly:
- `efficiency_events` -- loaded at startup from `efficiency.jsonl`, appended by the runner during execution
- `cascade_router` -- loaded from `cascade-router.json`, refreshed by the snapshot loader
- `recent_signals` -- loaded from `engrams.jsonl`
- `episodes_cache` -- loaded from `episodes.jsonl`
- `knowledge_entries` -- loaded from the neuro store
- `conductor_alerts` -- pushed from the runner
- `gate_results_page` -- pushed from the runner

**Verdict: Mixed staleness.**
- Sub-views 1-4 use push/snapshot data that stays reasonably current during active execution.
- Sub-view 5 (Cost/Model widget) reads `efficiency_events` directly, which is live during a run.
- Sub-view 6 reads from disk every 5 seconds. The `mcp-stats.json` and `gate-thresholds.json` files are only written at specific lifecycle points (not continuously), so MCP data can be stale for the entire duration of a run.
- No "last updated" timestamp is shown on any panel, so the user has no way to know the age of the displayed data.

---

## 7. Purpose of This View vs Others

**Stated purpose** (from the module doc comment): "token burn, cost breakdown, routing, health."

**Actual purpose**: F7 is a grab-bag of six unrelated inspection modes:
1. A dense operational dashboard (Overview)
2. A signal graph browser
3. An episode replay viewer
4. A knowledge store browser
5. A cost table
6. A three-panel diagnostic dump

It tries to be "the view where you look at things closely" but the result is that it does not have a clear identity. Each sub-view has a different data model, different interaction pattern, and different user intent.

**The coherent core would be**: internals inspection -- things you cannot see from the plan/agent/log views. Signal DAG, knowledge store, and the overview dashboard fit this framing. Episodes and cost tables are better served elsewhere.

---

## 8. Overlap with F1 Dashboard and F10 Learning

### F1 Dashboard Overlap

| F7 Panel | F1 Equivalent | Duplication |
|---|---|---|
| Overview: token burn by role | F1 sub-tab 0 (Agents): per-agent token table | **Partial** -- same data, different grouping (by role vs by agent) |
| Overview: cost by model | F1 sub-tab 5 (MCP): efficiency section with model usage | **Full** -- same aggregation from efficiency_events |
| Overview: cascade route | F1 sub-tab 5 (MCP): routing overview section | **Full** -- identical model/trials/successes/rate table |
| Overview: conductor alerts | F1 sub-tab 5 (MCP): conductor diagnosis section | **Partial** -- F7 shows raw alerts, F1 shows deduplicated diagnosis rows |
| Overview: gate summary | F1 sub-tab 3 (Verify): gate results + trends | **Partial** -- F7 shows aggregate pass rates, F1 shows per-run verdicts |
| Cost/Model (sub-view 5) | F1 sub-tab 5 (MCP): model usage aggregation | **Full** -- both aggregate efficiency_events by model |
| MCP panel (sub-view 6) | F1 sub-tab 5 (MCP): MCP config section | **Full** -- both show MCP config/server info; F1 is richer |

### F10 Learning Overlap

| F7 Panel | F10 Equivalent | Duplication |
|---|---|---|
| Overview: cascade route | F10 sub-view 1 (Route): cascade stage + per-model stats | **Full** -- same model_slugs/confidence_stats table |
| Learning panel (sub-view 6) | F10 sub-view 1 (Route): stage indicator | **Partial** -- F7 shows routing coverage %, F10 shows stage/threshold with visual progress |
| Learning panel: gate thresholds | F10 sub-view 3 (Efficiency): model efficiency stats | **Partial** -- related but different focus |

### F6 Config Overlap

| F7 Panel | F6 Equivalent | Duplication |
|---|---|---|
| MCP panel (sub-view 6) | F6 sub-view 2 (ProviderHealth) | **Partial** -- both show provider/MCP operational status |

**Summary**: F7 Overview has heavy overlap with F1's MCP sub-tab. The cascade router table appears in three places (F7 Overview, F7 Learning panel, F10 Route). Cost-by-model data appears in at least three places (F7 Overview, F7 Cost/Model sub-view, F1 MCP sub-tab). The MCP config view appears in two places (F7 Inspect panel, F1 MCP sub-tab) with F1 being the better version.

---

## 9. What Data Should Be Here That Isn't?

**Missing from MCP panel**:
- Per-server connection state (connected/disconnected/error)
- Per-server last successful call timestamp
- Per-tool invocation count and success rate
- Error log / last error message per server
- Tool latency percentiles

**Missing from Learning panel**:
- Active prompt experiments (A/B test in-progress status, variant assignment counts)
- Concluded experiment winners (`tui_state.experiment_winners` exists but is unused here)
- Playbook rule preview (when/then patterns, not just a count)
- Knowledge tier distribution (how many entries at each tier: Transient/Working/Durable)
- HDC fingerprint / consolidation state

**Missing from Prompt Stats panel**:
- Top sections by cost (the field exists but is always empty)
- Input vs. output token split per role
- System prompt vs. user content vs. tool result breakdown
- Cache hit/miss ratio (data exists in efficiency events)
- Context window headroom (% of max model context used)

**Missing from Overview**:
- No provider health status / circuit breaker state
- No disk usage or resource status
- No active run elapsed time or ETA

**Missing globally from F7**:
- No timestamp or "last refreshed" indicator on any panel
- No drill-down from any summary metric to its underlying data
- No export or copy-to-clipboard for diagnostic sharing

---

## 10. Proposed Additions

### A. System Health Dashboard (replace or enhance sub-view 1)

The current Overview tries to be everything. Replace it with a focused system health dashboard:

- **Top strip**: workspace status (running/paused/idle), elapsed time, total cost, disk free
- **Provider grid** (left 60%): one row per configured provider showing:
  - Name, model, status (healthy/degraded/down)
  - Circuit breaker state (closed/half-open/open) with color coding
  - Request count, error count, current latency (p50/p99)
  - Cooldown remaining (if in cooldown from tool isolation)
- **Resource panel** (right 40%): disk usage, JSONL log sizes, worktree count, memory pressure
- **Alert ticker** (bottom 3 lines): most recent conductor alerts scrolling

This replaces the current 6-panel cramped layout with a genuinely useful operational view.

### B. Provider Circuit Breaker Visualization

Add as sub-view 2 or integrate into the health dashboard:

```
Provider         State      Reqs  Errs  p50    p99    Reset In
anthropic-api    [CLOSED]   234    2    1.2s   4.8s   -
openai-compat    [HALF]      89   12    0.8s   3.2s   45s
gemini-api       [OPEN]       0   15    -      -      2m30s
cerebras-api     [CLOSED]    56    0    0.3s   1.1s   -
```

Data sources: `provider_health` registry in the runner, per-provider metrics from efficiency events. The infrastructure exists -- `roko_agent::dispatcher` tracks provider health and cooldown -- but it is not surfaced in the TUI.

### C. Prompt Token Breakdown Chart

Add to the Prompt Stats panel or as its own sub-view:

Show a horizontal stacked bar per role showing the composition of each prompt:
- System prompt base (from `SystemPromptBuilder` 9 layers)
- Playbook injections
- Knowledge context
- Tool descriptions
- User/task content
- Tool results
- Cache portion (overlay or separate bar)

This requires wiring section-level token accounting from the `SystemPromptBuilder` through to efficiency events. The `top_sections_by_cost` field on `PromptStatsData` was designed for this but never populated. The `RoleSystemPromptSpec` already builds the 9-layer prompt with named sections; each section's token count could be recorded at composition time and passed through the efficiency event.

### Recommended Sub-View Reorganization

Given the overlap findings, the ideal F7 structure would be:

| # | Label | Content |
|---|---|---|
| 1 | Health | System health dashboard (proposal A) |
| 2 | Providers | Provider circuit breaker grid (proposal B) |
| 3 | Signals | Signal DAG browser (existing) |
| 4 | Knowledge | Knowledge store browser (existing) |
| 5 | Prompts | Prompt token breakdown (proposal C) + current prompt stats |

Move Episodes to F5 Logs (where SignalStream already lives). Remove Cost/Model as a standalone sub-view (it is already in the health dashboard and F1). Remove the three-panel Inspect (its useful content is absorbed into the new sub-views). The MCP panel content moves into Provider Health. The Learning panel content moves to F10 Learning where it belongs.

---

## Summary of Issues

| # | Issue | Severity |
|---|---|---|
| 1 | MCP panel shows counts but no runtime health | Medium |
| 2 | Learning panel ignores experiments entirely | Medium |
| 3 | Prompt stats `top_sections_by_cost` is permanently empty | Low |
| 4 | Three-panel inspect has no thematic coherence | Medium |
| 5 | "Inspect" sub-tab inside "Inspect" tab is confusingly named | Low |
| 6 | Cost-by-model duplicated across sub-views 1 and 5 | Medium |
| 7 | Cascade router table appears in 3 places (F7, F7, F10) | Medium |
| 8 | F1 MCP sub-tab is richer than F7 MCP panel | Medium |
| 9 | No data freshness indicator anywhere | Medium |
| 10 | Signal DAG and Episode Replay arguably belong in F5 | Low |
| 11 | Overview sub-view is overloaded (6 panels in one view) | Medium |
| 12 | No provider circuit breaker visualization anywhere in TUI | High |
| 13 | No prompt section-level token accounting | Medium |
| 14 | Three-panel columns too narrow on 80-col terminals | Low |
