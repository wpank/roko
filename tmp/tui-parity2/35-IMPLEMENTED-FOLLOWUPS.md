# Implemented follow-ups from the parity audit

**Date:** 2026-09-01
**Scope:** changes made while verifying the previous P0-P7 completion claim. This is not a claim
that every item in the broader Phase 2 audit is complete.

**Released commits:** `88996a418`, `fced716b6`, `d4530a047`, `827181c3f`, `a8625a9cc`.

## Visual hierarchy and effects

- Adopted Mori's quiet black-canvas hierarchy and made operational content dominant.
- Made Minimal restrained and glyph-free; Full remains explicit opt-in; Off is a true master switch.
- Prevented effects from overwriting content and fixed overflow/panic behavior.
- Preserved TOML comments when changing the effects preset.
- Made notification placement responsive, reserved the footer, capped visible density, ordered the
  newest notice at the bottom, and made truncation Unicode-safe.
- Suppressed replay-created historical toasts in headless evidence captures.

## Responsive Agents and Logs views

- Added a stacked roster/transcript layout below 104 columns.
- Adapted roster columns to available width and kept the selected row in view.
- Suppressed token chrome in short panes and render the secondary Live Stream panel only when a
  real sidecar stream exists; runner output keeps one dominant transcript.
- Based transcript and log tails on wrapped display rows rather than source-line counts.
- Mapped log selection/search to the corresponding wrapped display offset.
- Added 80x24, 120x40, and 200x60 regression coverage.
- Fixed the 80x24 Agents split-index panic found by full-frame capture.

## Connected feedback and terminal truth

- Established the dashboard bridge before cache warmup and published explicit startup states.
- Projected bounded tool call/output text into connected agent output.
- Replayed real bounded gate output after completion and exposed active gate state. This is not true
  subprocess-line streaming.
- Made pause enqueue state truthful and made scheduler dispatch obey the pause barrier.
- Projected terminal `completed`/`failed`/`cancelled` status immediately so debounce cannot leave a
  stale `gate` status.
- Settled synthetic `plan-verify` attempts and populated plan completion cost/task totals from
  `RunState` instead of emitting zeros.
- Replaced unsupported retry/repair/reverify/skip “next tick” messages with explicit rejection and
  no state mutation; help/footer text no longer presents those controls as operational.
- Fit narrow footer hints by whole tokens, always retaining `?:help`, instead of allowing terminal
  clipping to create incomplete key labels.

## Evidence tooling

- Unified static screenshots on the complete `App::draw` path.
- Added dimension/tab validation, timestamped output, manifests, and `latest` management.
- Implemented the bounded continuous collector described in
  `32-SCREENSHOT-HARNESS-STATUS.md`, including startup, interval, lifecycle, and shutdown captures.
- Captured all ten tabs at three terminal sizes and used the 80x24 frames as an actual defect-finding
  gate rather than treating file generation as success.
- Preserve every virtual terminal row in the decoded frame so height can be checked mechanically
  even when the last row is blank.

## Earlier claim corrections retained

- Plan filtering now includes task text and preserves the real selected plan identity.
- Log search/filter uses the visible level/subview list.
- Dashboard and subview state no longer share unrelated scroll/selection state in the corrected
  paths.
- F10 is represented in the global header; top-level and local sub-tab behavior are distinct.
- Render-path MCP/config reads are cached with explicit refresh behavior.

## Verification boundary

The test and capture evidence verifies these bounded changes. It does not promote the remaining
partial/missing items in `30-VERIFIED-CLAIM-MATRIX.md`; in particular, gate streaming, acknowledged
recovery commands, connected plan metadata, critical-path ETA, PNG/ANSI evidence, and automated
visual comparison remain open.
