# Screenshot harness status

**Audit date:** 2026-09-01
**Evidence standard:** a capture is useful only when its state source, renderer, dimensions,
event trigger, and output limitations are explicit.

## Result

Roko now has two complementary text-capture paths that use the production `App::draw` layout:

1. `roko screenshot` renders a controlled snapshot across selected or all tabs at an explicit
   terminal size.
2. `roko plan run ... --screenshots` starts a bounded background collector before cache warmup and
   records startup, lifecycle, periodic, error, completion, and shutdown frames from live
   `StateHub` snapshots.

The continuous path was previously a dead boolean scaffold. The implementation added during this
audit makes it operational without putting rendering or filesystem writes on the runner loop.

## Continuous capture contract

| Property | Implemented behavior |
|---|---|
| Enablement | `roko plan run <path> --screenshots` |
| Interval | `--screenshot-interval <1..86400 seconds>`, default 60 |
| Destination | Timestamped `.roko/screenshots/run-*` directory, or collision-safe `--screenshot-dir <path>` |
| Current pointer | `.roko/screenshots/latest` symlink; a pre-existing legacy directory is preserved before replacement |
| State source | A cloned `DashboardSnapshot` at the capture request boundary |
| Renderer | Materialized `App` using `App::draw`, not a second content-only renderer |
| Event triggers | startup/warmup, run/plan/task/agent/gate/merge/error terminal events, interval, shutdown |
| Backpressure | Bounded synchronous queue; ordinary events use non-blocking `try_send` and count drops |
| Durability | Schema-v2 manifest rewritten atomically after each attempt; frame files are atomic writes |
| Safety | Dimension validation, unique directories, capture-count limit, free-disk guard, bounded metadata, panic containment |
| Shutdown | Final all-tab snapshot followed by worker join |

Eight focused tests cover live snapshot materialization (including typed agent content), periodic
state, warmup visibility, low-disk skips, validation, output-directory collisions, legacy `latest`
preservation, and final shutdown capture. Two CLI tests cover the new flags.

## Static evidence matrix

The checked-in evidence set contains every top-level tab at 80x24, 120x40, and 200x60 under
`tmp/tui-parity2/evidence/`. A separate 120x40 Full-preset capture documents the opt-in rendering
path: **40 frames total**, each mechanically checked for the exact declared row count. Reduced-motion
captures are the usability baseline.

These frames caught two defects that ordinary widget tests did not:

- historical snapshot replay created a stack of stale toasts that covered most of an 80x24 frame;
- the narrow Agents layout eagerly indexed a third split that did not exist and panicked.

Both defects were corrected before the final capture matrix was regenerated.

## Evidence boundary

The files preserve terminal cell symbols and production layout, but not cell foreground/background
attributes. They therefore prove reachability, hierarchy, wrapping, clipping, and panic-free layout;
they do **not** prove palette fidelity or the appearance of atmospheric effects.

Still missing:

- ANSI/style-preserving capture;
- PNG/font rasterization;
- baseline comparison, tolerance, and visual diff output;
- automated reference assessment and regression gating;
- a paid/provider live-run fixture captured after these changes.

Backlog #111 and #112 are consequently **partial**. Backlog #151-153 remain **missing**.
