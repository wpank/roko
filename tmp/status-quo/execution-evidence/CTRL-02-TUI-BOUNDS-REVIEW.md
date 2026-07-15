# CTRL-02 TUI bounds precursor independent review

Verdict: **ACCEPTED**

## Reviewed object

- Candidate: `2b18ae8142119d0a6b372418c1d76e8f9c3f6763`
- Exact base and candidate parent: `bafcebb686d12bd83b0c9a76f0d937c8b53083dd`
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3`, relative to `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Review branch/worktree: `review/CTRL-02-tui-bounds-2b18ae814211` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-02-TUI-BOUNDS-2b18ae814211`
- Candidate range: one direct-child commit changing exactly
  `crates/roko-cli/src/tui/state.rs`,
  `crates/roko-cli/src/tui/views/dashboard_view.rs`, and
  `tmp/status-quo/execution-evidence/CTRL-02-TUI-BOUNDS.md`.

I independently read the complete master checklist, the sealed recovery inventory,
the full SH04 manifest, issues 09, 11, and 14, the July 14 self-heal audit, the
historical `3041d095d` TUI diff, the worker evidence, all candidate lines, and the
unchanged snapshot publication and TUI call paths.

## Requirement reconstruction and reproduction

`DashboardSnapshot` is documented and implemented as full materialized state. Its
`task_outputs` values are capped authoritative rings, not per-refresh deltas. The
pre-candidate base used a non-empty filter and then fell back to the previous agent
row for both `output_lines` and `last_output_line`. Source-level reproduction with
`git show bafcebb686d:crates/roko-cli/src/tui/state.rs` therefore proves that an empty
or absent authoritative ring retained stale output. The older
`1649c18b..3041d095d` dashboard diff likewise proves the original route defect:
after the border was already removed, the renderer subtracted two more rows, so an
inner height of two retained a header but zero data rows.

The candidate removes the stale-row fallback. A non-empty same-task ring still flows
through the single `bounded_output_lines` projection, while an empty or absent map
entry becomes empty by `unwrap_or_default`; `last_output_line` is derived solely from
that replaced vector. The complete `task_output_tails` map is rebuilt from the same
snapshot and projection, so stale task keys cannot survive. This is consistent with
the publisher contract and does not discard a valid non-empty same-task ring.

For route rendering, `Block::inner` has already removed the two border rows. The
table itself consumes one header row, so `inner_height.saturating_sub(1)` is the
correct capacity. At the minimum tested outer buffer of `48x4`, the inner rectangle
is `46x2`, leaving exactly one data row after the header. The candidate makes no
production layout change beyond the inherited capacity correction; its new direct
buffer test closes the exact off-by-one boundary.

## Independent verification

All commands ran at the immutable candidate. The shared integration target was
initially build-locked by another programme lane, so review used one isolated,
non-incremental target at `/private/tmp/roko-ctrl02-tui-review-real` and removed it
after validation.

- `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-tui-review-real CARGO_INCREMENTAL=0 cargo test -p roko-cli --lib connected_snapshot -- --nocapture`
  - PASS: 3 passed, 0 failed. This covers repeated authoritative replacement without
    duplication, valid non-empty same-task preservation, exact 50-line tail
    retention (`line-50` through `line-99`), stale task-key removal, and explicit
    empty-ring clearing of both output fields.
- `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-tui-review-real CARGO_INCREMENTAL=0 cargo test -p roko-cli --lib one_route_is_visible -- --nocapture`
  - PASS: 2 passed, 0 failed, including the exact `48x4` buffer.
- `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-tui-review-real CARGO_INCREMENTAL=0 cargo test -p roko-cli tui`
  - PASS: 245 TUI library tests passed, 0 failed; all other filtered targets passed
    with zero selected tests.
- `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-tui-review-real CARGO_INCREMENTAL=0 cargo check -p roko-cli --all-targets`
  - PASS. It emitted only the pre-existing missing-crate-documentation warning for
    `tests/plan_validation.rs`.
- `cargo fmt --all -- --check`
  - PASS.
- `git diff --check bafcebb686d12bd83b0c9a76f0d937c8b53083dd..2b18ae8142119d0a6b372418c1d76e8f9c3f6763`
  - PASS.
- Direct parent, merge-base, exact three-path census, and clean pre-review worktree
  checks all passed.

No candidate line adds an ignored failure, unsafe default, panic path, unbounded
collection, public API or serialization change, phase inference, widget behavior,
or hidden fallback. Test `unwrap` calls are conventional setup/assertion failures
and do not enter production.

## Scope limits

This acceptance is only for the bounded CTRL-02 precursor attribution. It proves the
authoritative connected-output replacement/bound and the one-route layout boundary.
It does **not** accept `SH04-T06`: that task depends on `SH04-T01` and still requires
active-agent/task/phase reconciliation. In particular, issue 11 remains open because
an active agent can still coexist with an all-complete inferred phase pipeline, and
`phase_compact.rs` was intentionally untouched.

Nor does this close issue 14 as a whole. The connected-output growth and stale
task-tail findings covered here have proof, but the background-thread terminal
cleanup, dual push/pull race, async-run drains, synchronous Git refresh,
notification expiry, recursive Git polling, empty-option cycling, and signal-output
findings remain outside this candidate. Structured identity, approval-mode event
connection, typed output severity, preflight diagnoses, liveness/tokens, Git refresh,
operational logs, manifests, and canonical task status also remain unclaimed.

## Decision

**ACCEPTED**, high confidence. No candidate correction is required. The coordinator
may merge this review branch, rerun the focused output and route tests on the
integrated head, and record only the bounded CTRL-02 precursor as integrated. It
must not mark `SH04-T06`, issue 11, SH04, or the broader issue-14 audit complete from
this evidence.
