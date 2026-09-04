# P0 implementation record

> **Historical P0 checkpoint.** The “still deferred” list below is accurate for `a58bdbacb`, not
> for the expanded integration. Impact gates, run APIs/evidence hooks, bounded plan context,
> hard-deadline scheduling, timeout salvage, and terminal convergence are reconciled in
> [11-implementation-status.md](11-implementation-status.md). Final batched verification and real
> cold/warm benchmarks remain open.

Date: 2026-08-31

Implementation commit: `a58bdbacbf80d72583edd628354ddb2750a8b822`

Documentation reconciliation: `c28b2d618a142332022e1670764d8e5073053177`
Branches pushed: `main`, `feat/dev-speed-p0`

The tracked operator contract is [docs/v2/29-FAST-DEVELOPMENT.md](../../docs/v2/29-FAST-DEVELOPMENT.md).

## Run it

```bash
./dev.sh fast plans/<plan-directory>
```

FAST uses an existing `target/debug/roko`; it never invokes Cargo to build Roko itself. It is
headless, defaults to one task at a time, gives the patching agent 90 seconds, gives the run a
bounded execution deadline plus settlement headroom, skips hot-path warmup/GC/target cleaning,
and writes a private evidence bundle under `.roko/runs/`.

Each FAST task must provide exactly one authored `verify` command. Zero or multiple commands fail
closed. This makes verification ownership explicit and prevents a vacuous green gate.

## Implemented

- [x] Scheduler capacity and exact attempt ownership are reserved before worktree, prompt, routing,
  model, and hook preparation. Provider-call counters increment only after a runtime launches.
- [x] Agents receive a patch-only contract; Cargo/build/test/clippy/npm work belongs to the runner.
- [x] FAST preserves warm build artifacts and skips startup workspace warming and critical-path
  maintenance.
- [x] Canonical Cargo checks can narrow only when Git and Cargo metadata prove one unambiguous target;
  deletions, renames, mixed paths, unsafe names, and shared target paths fall back safely.
- [x] Duplicate verification removal is FAST-only and exact; broad, optional, and intentionally
  repeated authored commands remain intact.
- [x] Codex shared-target access is a separate explicit trusted opt-in and is confined to the
  canonical repository target directory. Default sandbox authority is unchanged.
- [x] Fresh worktrees compile with `SKIP_FRONTEND_BUILD=1` using a tracked fallback page instead of
  requiring ignored Vite output.
- [x] `run-evidence` records redacted command/environment metadata, status transitions, optional
  lifecycle events, timings, Git state, bounded diffs, and capped stdout/stderr. It kills the
  complete process group, including descendants left after a leader exits.
- [x] Timed-out FAST attempts durably settle first, then best-effort export bounded worktree evidence.

## Verified

- [x] Rebased cleanly over TUI-parity main (`53e275a22`) with no merge conflicts.
- [x] `cargo +nightly fmt --all -- --check` passed.
- [x] `cargo check -p roko-cli --lib` passed from a clean worktree with no generated frontend `dist/`
  and `SKIP_FRONTEND_BUILD=1`.
- [x] Shell/Python syntax and FAST help/forbidden-TUI checks passed.
- [x] Evidence success, timeout, resistant-descendant cleanup, secret redaction, permissions, noisy
  output truncation, and oversized Git-diff smokes passed.
- [ ] The heavyweight `roko-cli` unit-test target was deliberately stopped: it compiled the entire
  workspace-scale test harness for several minutes before running any test, reproducing the audit
  bottleneck. Broad tests/clippy remain RELEASE/CI work rather than interactive FAST work.

## Still deferred

- [ ] First-class timeout-diff salvage into the normal gate lifecycle (requires a typed durable state).
- [ ] Semantic deduplication across arbitrary shell wrappers.
- [ ] Changed-crate plus reverse-dependency test selection and asynchronous full CI.
- [ ] Endpoint discovery and browser/TUI screenshot collection in the evidence bundle.
- [ ] Benchmarks from [06-benchmark-scorecard.md](06-benchmark-scorecard.md); FAST must remain
  opt-in until those results are representative and escaped-regression rates do not increase.
