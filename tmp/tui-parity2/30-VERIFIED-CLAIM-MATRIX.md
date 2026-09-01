# Verified P0-P7 claim matrix

**Audit date:** 2026-09-01
**Source baseline:** `fced716b6` (`main`, pushed)
**Method:** production call-site tracing, focused tests, persisted-run inspection, and
full-frame `TestBackend` captures. A field, widget, handler, or passing unit test is not
treated as end-to-end completion unless the production producer and consumer are both wired.

The earlier statement that all 38 P0-P7 items were complete was incorrect. At this
baseline the defensible result is **21 verified, 11 partial, and 6 not operational**.
Several partial items are materially better than they were at PR #74, but their original
acceptance criteria are still not fully satisfied.

Legend: **V** verified, **P** partial, **N** not operational.

| Item | Status | Verified behavior and remaining limitation |
|---|:---:|---|
| P0.1 token sparkline fallback | V | Connected snapshots populate cumulative token history used by the widgets. |
| P0.2 task denominator | V | `PlanStarted.tasks_total` remains authoritative and replayed task starts are idempotent. |
| P0.3 cost ordering race | V | Cost attribution includes recently inactive agents. |
| P0.4 connected token rate/history | V | Snapshot deltas become cumulative history and update rate state. |
| P0.5 learning/efficiency bridge | P | Aggregates and selected event fields cross the bridge; typed learning history and cost-by-model remain incomplete. |
| P1.1 pause/retry/skip channel | P | `p` now sends Pause/Resume and scheduler dispatch stops while paused. Retry, repair, reverify, and skip are still log-only; cancel is run-wide. |
| P1.2 log search | V | Search/filter renders and match indices now use the level/subview-visible list. |
| P1.3 plan filter | V | Task text participates in filtering and selection/actions retain the real plan identity. |
| P1.4 role tabs | V | Role selection changes the visible agent transcript. |
| P1.5 critical-path ETA | N | The algorithm/display scaffold exists, but production never assigns the value. |
| P1.6 Inspect panel reachability | V | The declared Inspect subviews are addressable through the subview model. |
| P2.1 live gate output | P | Captured gate output is replayed to the TUI at completion; stdout/stderr is not streamed during execution. |
| P2.2 gate output widget | P | The widget receives bounded real completion output, but has no live process-line producer. |
| P2.3 gate-rung indicator | P | Selected rung state is shown and cleared, but the runner publishes the selected set rather than exact per-rung transitions. |
| P3.1 MCP config caching | V | The render path consumes a five-second cached config view; it no longer parses files each frame. This is configured state, not live connectivity. |
| P3.2 config editor cache | V | Config items are cached with explicit invalidation. |
| P3.3 Inspect caches | V | Inspect panels consume cached data refreshed outside rendering. |
| P4.1 compact bottom ribbon | V | The four functional regions render in a compact bottom band. |
| P4.2 contextual empty states | V | The originally targeted core panels have contextual empty messages; the broader TUI still contains generic states tracked in Phase 2. |
| P4.3 NET/DSK metrics | P | Network and capacity values render, but network deltas use incorrect rate units and disk I/O rate remains zero. |
| P4.4 effects default/safety | V | Minimal is restrained, Off is a true master off switch, effects avoid operational glyphs, and overflow is covered. An explicit local `preset = "full"` still opts into Full. |
| P4.5 PAUSED state | V | Badge styling is present and pause now blocks new scheduler actions after in-flight work settles. |
| P4.6 warning bar | V | Persistent warnings render below the header. |
| P4.7 header enrichment | V | MCP/NET/DSK/FPS fields are visible with responsive compaction; some underlying metrics remain approximate under P4.3. |
| P5.1 task dependencies | N | Connected `TaskEntry` does not carry dependencies into plan detail. |
| P5.2 acceptance/verify fields | N | These fields are absent from the connected plan-detail model. |
| P5.3 diff statistics | P | Rows and fields exist, but connected constructors generally do not populate them. |
| P5.4 branch/worktree/commit | P | Display exists; connected runtime metadata is incomplete and diff collection targets the root rather than the accepted attempt worktree. |
| P5.5 per-plan elapsed | P | Rendering exists, but connected refresh does not preserve an authoritative start timestamp. |
| P6.1 number-key shadowing | V | Agent/Logs local number behavior is protected. |
| P6.2 `v` means verify | V | The global mapping is reverify, not effects. |
| P6.3 focus zones | P | Zones exist, but several tabs still fall through to shared scroll state. |
| P6.4 help accuracy/scroll | P | Help scrolls; several advertised recovery controls remain stronger than runner behavior. |
| P6.5 independent Diff/Procs scroll | N | `procs_scroll` exists but normal input still mutates `diff_scroll`. |
| P7.1 live git diff refresh | N | Background git refresh does not publish the active attempt-worktree diff into connected state. |
| P7.2 Logs/Signals split | V | Logs subviews have distinct source filtering and visible navigation. |
| P7.3 Procs scroll | N | The dedicated field is not driven by input. |
| P7.4 attempt title | V | Agent output titles include the attempt when known. |

## Changes since the PR #74 audit

`88996a418` removed the crash-prone, content-obscuring visual treatment and established
the Mori hierarchy. `fced716b6` then made Off honest, reduced Minimal, cached MCP config,
fixed plan/log filtered identity, exposed subviews, surfaced startup/tool activity, wired
pause into scheduling, projected gate-completion output, and unified static screenshots on
the complete `App::draw` path.

This matrix intentionally does not promote genuine gate streaming, connected execution
metadata, or facade recovery commands merely because adjacent UI scaffolding now exists.
