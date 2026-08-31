# TUI parity post-merge live audit

**Date:** 2026-08-31

**Merged baseline:** `53e275a22` (PR #74), audited at `c28b2d618`

**Live command:** `cargo run -p roko-cli -- plan run plans/doctor-network-v2 --engine runner-v2 --tui --force-backend codex --fresh`

## Verdict

The statement “38/38 P0-P7 items complete” is not supported by an end-to-end
audit. At the merged baseline:

| Result | Count | Meaning |
|---|---:|---|
| Verified | 19 | A producer, state bridge, renderer, and/or interaction path exists and is operational for the stated item. |
| Partial / scaffolded | 11 | Some types, state, widgets, or handlers exist, but the claimed behavior is incomplete or disconnected. |
| Not operational | 8 | The required producer, state mutation, or usable input path is absent. |

The follow-up fixes made during this audit move P0.2 and P0.4 from partial to
verified, making the current working tree **21 verified, 9 partial, 8 not
operational** at the source level. This is still not 38/38, and the 21 count is
not a claim that the final rebased integration has completed a new live run.

## Reconciliation after the development-speed integration

The later dev-audit branch adds source-level fixes adjacent to this audit:

- `88c724744` honors configured redraw cadence, skips connected idle redraws, omits the broad
  filesystem watcher when StateHub is authoritative, and excludes worktrees/targets/caches from
  fallback recursion.
- `52d5f4df4` freezes terminal duration/ETA, converges active plan/agent state, preserves degraded
  PID ownership, and makes replayed plan/task/agent counters idempotent.
- The active handoff retains the P0.2/P0.4 denominator/history/EMA fixes and the panic-hook/post-FX
  overflow fixes described below.

These are implementation facts from the source/diffs. The coordinator intentionally deferred all
compilation and interactive checks to one final batch, so this audit does not promote any of them
to final-tree live-verified status yet. The original matrix remains the canonical description of
the still-partial and non-operational P0–P7 paths.

## What caused the TUI to exit

The screenshots captured a real process abort, not a normal TUI shutdown.
macOS wrote five `roko-*.ips` crash reports between 11:51 and 12:28; the latest
reports `SIGABRT` on thread `roko-plan-approval-tui` with Rust
`panic_in_cleanup` in the stack.

After preventing the cleanup double panic, the original diagnostic became
visible in `.roko/runner-stderr.log`:

```text
thread 'roko-plan-approval-tui' panicked at crates/roko-cli/src/tui/postfx.rs:236:32:
attempt to multiply with overflow
```

The failure sequence was:

1. An active agent enabled post-processing guide lines.
2. The third guide-line seed evaluated `2 * 0x9E37_79B9_7F4A_7C15` with normal
   debug arithmetic and overflowed.
3. Unwinding dropped `PanicHookRestoreGuard`, whose `Drop` implementation called
   `std::panic::set_hook` from an already-panicking thread.
4. That second panic forced `std::process::abort`, hiding the first diagnostic
   and producing the shell's `Abort trap: 6`.

The effects code predates PR #74, but the new effects defaults and the local
`[tui.effects] preset = "full"` setting exposed the dormant bug in normal plan
runs.

## Fixes applied by this audit

- Use wrapping multiplication for the post-processing seed and cover full
  guide-line intensity with a regression test.
- Do not restore the process panic hook from `Drop` while the TUI thread is
  already unwinding. This preserves the original error instead of aborting the
  process during cleanup.
- Preserve the authoritative `PlanStarted.tasks_total` denominator. The merged
  code incremented it again on every `TaskStarted`, which is why a seven-task
  plan displayed `0/8` in the screenshot.
- Convert the connected snapshot's bounded token-delta ring into cumulative
  TUI history and update token/cost rates on connected snapshots.

### Historical working-tree live recheck after the fix

A fresh rerun (`run-1788172543820`) crossed the former crash point: T1 spawned
an active Codex agent, emitted token usage, completed, and entered gate
dispatch. After nearly three minutes the TUI/runner process was still alive,
`.roko/runner-stderr.log` was empty, and macOS had produced no crash report
newer than the 12:28 baseline. This validates the active-agent effects abort
fix in that working tree; it predates the final rebased dev-audit tree. The run
was still in progress and is not evidence that every plan-run or gate behavior
is correct.

## P0-P7 claim matrix

Legend: **V** verified at merge, **P** partial/scaffolded, **N** not operational.
“Fixed here” refers to the follow-up working-tree fixes listed above.

| Item | Merge | Evidence and limitation |
|---|:---:|---|
| P0.1 token sparkline fallback | V | Connected widgets fall back to cumulative values in `TuiState`. |
| P0.2 plan task denominator | P | `PlanStarted` carried the total, but `TaskStarted` incremented it again (`0/8` for seven tasks). **Source-fixed and strengthened for idempotent replay; final-tree live check pending.** |
| P0.3 cost ordering race | V | Agent lookup includes recently inactive agents, covering the completion/cost ordering window. |
| P0.4 connected token rate/history | P | The ring existed, but `update_from_dashboard_snapshot()` neither copied it into `token_history` nor called the rate updater. **Source-fixed with cumulative history/zero-delta EMA handling; final-tree live check pending.** |
| P0.5 connected learning/efficiency bridge | P | Some JSON and aggregate fields are copied and cost-by-model has a fallback, but typed live efficiency/learning event data remains empty or incomplete. |
| P1.1 working pause/retry/skip channel | P | The enum/channel exists. `p` only flips local UI state; runner pause is not consulted before dispatch, and soft-retry/repair/reverify/skip handlers mostly only log “next tick.” Cancel is process-wide. |
| P1.2 log search | V | Search state is compiled and used for filtering/highlighting in `logs_view.rs`. |
| P1.3 plan filter | V | The plan tree applies the parsed filter and renders filtered counts. |
| P1.4 role tabs switch output | V | Selecting a role tab updates the selected matching agent. |
| P1.5 critical-path ETA | N | The field and header branch exist, but no production code assigns a value. |
| P1.6 three-panel Inspect reachable | V | Registered as the sixth Inspect sub-view. |
| P2.1 forward live gate output | N | `TuiBridge::gate_output_line()` exists but has no production caller; `gate_result()` always sends `output_text: None`. |
| P2.2 gate output widget | P | The colorized widget and snapshot ring exist, but no gate producer feeds them, so it cannot stream a real run. |
| P2.3 live gate-rung indicator | N | A start event is logged, but `TuiState.current_gate_rung` is never assigned or cleared. |
| P3.1 cache MCP config | P | The MCP panel still calls `Config::from_file` and `McpConfig::load` inside render on every frame. |
| P3.2 cache config editor parse | V | F6 renders a cached item list with TTL/invalidation refresh. |
| P3.3 cache Inspect file reads | V | Inspect panels consume cached data refreshed outside render. |
| P4.1 four-row bottom ribbon | V | Compact layout is implemented. |
| P4.2 contextual empty states | V | Targeted panels use contextual messages. |
| P4.3 NET/DSK metrics | V | Background sampling and header/system rendering are present. |
| P4.4 effects default | V | Default changed to Minimal, but active-agent effects exposed the fatal overflow described above. |
| P4.5 PAUSED badge styling | V | The visual badge exists; it does not prove runner pause semantics. |
| P4.6 warning bar | V | Persistent warning region is rendered. |
| P4.7 header MCP/NET/DSK/FPS | V | Header metrics are rendered from state. |
| P5.1 task dependencies | N | `TaskEntry` still has only id/name/status/agent_id; no `depends_on` field reaches the modal. |
| P5.2 acceptance/verify | N | Those fields are also absent from `TaskEntry` and the modal. |
| P5.3 diff statistics | P | Modal rows and `PlanEntry` fields exist, but all production constructors set them to `None`. |
| P5.4 branch/worktree/commit | P | Same issue: display scaffold exists, data is never populated. |
| P5.5 live elapsed timer | P | The renderer can calculate elapsed time, but snapshot refresh recreates active plans with `Instant::now()`, repeatedly resetting the timer. |
| P6.1 number-key shadowing | V | Agents and Logs keep their local digit handlers; other tabs use digit tab switching. |
| P6.2 `v` means verify | V | Global `v` maps to reverify rather than cycling effects. |
| P6.3 focus zones on remaining tabs | P | New enum variants cycle, but most focused scrolling still falls through to the shared `diff_scroll`; several panels do not consume the new zones. |
| P6.4 correct, scrollable help | P | Scrolling works, but help still says `v` cycles effects and advertises pause/recovery behavior that is not operational. |
| P6.5 independent Diff/Procs scroll | N | `procs_scroll` renders separately but is never changed by input; input still mutates `diff_scroll`. |
| P7.1 background `git_diff` refresh | N | The git watcher refreshes branch/commit/worktree summaries, not `TuiState.git_diff`; connected mode therefore keeps the Diff sub-tab empty/stale. |
| P7.2 Log/Signals split | V | Signals sub-view filters to signal/episode sources. |
| P7.3 Procs uses its own scroll | N | Same missing mutation path as P6.5. |
| P7.4 attempt in output title | V | Agent output title includes a nonzero attempt count. |

## Other live-run findings

### Repeated-run worktree collision

One retry failed before agent execution because branch
`roko/attempt/attempt-ff3253e7b62d82c2902d` was already checked out at
`.roko/worktrees/attempt-ff3253e7b62d82c2902d`. The runner then emitted a
failed run with orphaned/blocked tasks while a summary counter still reported
zero failed tasks. This is separate from the TUI abort, but repeated aborts and
`--fresh` retries make it easier to hit.

The worktree is still registered and was deliberately not deleted by this
audit; cleanup must first establish that no useful attempt state is needed.

### Logs are live, but not a complete agent transcript

The screenshots confirm that connected `DashboardEvent`s reach F5 (the view
went from zero to ten entries). The unified Logs cache combines event, signal,
episode, efficiency, and gate summaries; it is not the same as the agent
message/tool-output ring shown in F3. “Live data” should not be interpreted as
all panels receiving all run data.

### Large event archive warning

The runner also logs degraded prompt-experiment reconciliation while parsing a
large archived event file. This did not cause the TUI abort, but it is a
separate data-hygiene/performance issue worth tracking.

## Recommended completion order

1. Add an end-to-end debug-profile TUI smoke test with an active agent and
   effects enabled.
2. Make pause/resume gate dispatch and implement real retry/repair/reverify/skip
   state transitions before advertising the bindings.
3. Feed gate output from the executing process into `GateOutputLine`, and map
   gate-rung start/end into TUI state.
4. Finish the connected learning/efficiency bridge.
5. Populate plan/task enrichment data and preserve start instants across
   snapshots.
6. Remove the remaining MCP render-path I/O and finish independent focus/scroll
   routing.

Completion should require a live acceptance run for every item whose claim is
end-to-end, not only the presence of an enum, field, widget, or log statement.
