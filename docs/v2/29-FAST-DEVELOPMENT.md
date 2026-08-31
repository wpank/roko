# 29 — FAST Development

> **Status**: P0 IMPLEMENTED, OPT-IN, EXPERIMENTAL
>
> **Landed**: 2026-08-31 in `a58bdbacbf80d72583edd628354ddb2750a8b822`
>
> **Purpose**: Shorten Roko self-development feedback loops while retaining one explicit,
> bounded verification command and a private evidence record.

FAST is an operator lane for small, well-scoped implementation tasks. It is not the release lane
and does not establish that the workspace is globally green.

---

## Run FAST

FAST requires a current executable at `target/debug/roko`. It deliberately does not build that
executable.

```bash
./dev.sh fast plans/<plan-directory>
```

Common forms:

```bash
# Resume within the default five-minute command deadline.
./dev.sh fast plans/my-plan -- --resume-plan

# Give runner execution 390 seconds and preserve 30 seconds for settlement.
./dev.sh fast --deadline 420 plans/my-plan

# Store the evidence bundle somewhere else.
./dev.sh fast --bundle-root /path/to/private/runs plans/my-plan
```

Wrapper options must precede the plan directory. Native `roko plan run` options follow `--`.
Run `./dev.sh fast --help` for the current option list. The wrapper rejects `--tui` and
`--approval` because automation must remain headless and non-interactive.

Defaults are:

| Setting | Default |
|---|---:|
| Total runner-command deadline | 300 seconds |
| Agent dispatch, task-attempt, and agent-silence cap | 90 seconds |
| Retries | 0 |
| Concurrent tasks | 1 |
| Evidence root | `.roko/runs/` |

`--max-tasks` and `--max-retries` can override their defaults, but doing so can reintroduce
compiler contention or repeated provider latency.

---

## Required Task Contract

Every task must author exactly one `[[task.verify]]` entry. Zero or multiple entries fail closed.
Use the narrowest command that proves the changed behavior:

```toml
[[task.verify]]
phase = "structural"
command = "cargo check -p roko-core --lib"
```

A focused test, syntax check, or deterministic assertion is also valid when it is the appropriate
proof. Avoid `cargo test --workspace`, workspace clippy, or another broad command in FAST plans;
those erase the latency benefit and belong in CI or release verification.

The patching agent is instructed not to run Cargo, tests, clippy, npm, builds, or servers. The
runner executes the one authored verification command. With `ROKO_TASK_VERIFY_ONLY=1`, the normal
canonical gate pipeline is skipped; FAST therefore depends on the plan author choosing a meaningful
command.

---

## Deadlines and Settlement

The wrapper separates execution from durable terminal settlement:

- For deadlines of 120 seconds or more, 30 seconds is reserved for settlement.
- For shorter deadlines, one quarter of the requested time is reserved, using integer seconds.
- FAST rejects a total deadline below 10 seconds.
- `ROKO_FAST_PLAN_DEADLINE_SECS` receives the requested deadline minus that headroom.
- Gate effects retain their configured gate deadline; FAST does not silently weaken a required
  verification or cleanup deadline.
- The outer evidence wrapper terminates the complete process group at its deadline, waits a
  three-second grace period, and then sends a kill signal if necessary.

For the default `--deadline 300`, Roko receives 270 seconds for runner execution and 30 seconds to
settle before the outer command deadline. Final bounded Git and artifact capture occurs afterward
and may add a few seconds.

---

## Work FAST Skips

The wrapper uses the existing binary and enables headless runner-v2 with preflight disabled. The
FAST runtime also skips or defers:

- startup workspace Cargo cache warming;
- pre-plan log rotation, stale-target cleanup, and filesystem GC;
- post-plan log rotation, stale-target cleanup, and filesystem GC;
- per-task target cleanup, preserving warm incremental artifacts;
- the canonical gate pipeline when task-owned verification is enabled;
- TUI approval, endpoint probing, browser/TUI screenshots, frontend builds, and agent-started
  servers.

FAST does not implicitly add `--fresh`, `--force`, permission bypasses, endpoint probes, or other
mutating native options. Eligible worktree cleanup and terminal-state persistence still run.

---

## Evidence and Security Bounds

`./dev.sh fast` delegates capture to `./dev.sh run-evidence`. Each run gets a private directory with
mode `0700`; created files use mode `0600`. It records the redacted command, an allowlisted
environment snapshot, timings, terminal status, machine/cache metadata, structured events, Git
state, and before/after tracked diffs. Output remains live while it is captured.

The main bounds are:

| Artifact | Bound |
|---|---:|
| Standard output | 16 MiB |
| Standard error | 16 MiB |
| Each tracked Git diff | 16 MiB |
| Git metadata capture | 4 MiB |
| Timed-out attempt status | 1 MiB |
| Timed-out attempt patch | 16 MiB |
| Timed-out attempt Git helper | 5 seconds |

Untracked paths are listed, but their contents are not copied. Secret-like command arguments and
environment values are redacted from metadata, and the full environment is not recorded. Output
and diffs are captured verbatim, however, so they can still contain sensitive data. Treat the whole
bundle as sensitive and do not publish it without inspection.

Timed-out FAST attempts persist their terminal state before best-effort worktree evidence export,
so a slow or oversized Git operation cannot prevent durable settlement.

---

## Advanced Shared-Target Opt-In

By default, a Codex task agent cannot write the repository's shared Cargo target outside its task
worktree. A trusted operator can explicitly grant that narrow path:

```bash
ROKO_AGENT_SHARED_TARGET=1 ./dev.sh fast plans/my-plan
```

This adds the canonical repository `target/` directory to the Codex writable sandbox and enables
incremental Cargo output for that invocation. The grant fails closed unless the directory exists,
is the target belonging to Git's canonical common repository, and contains no symlink escape. It
does not authorize another arbitrary directory.

This is an advanced trust decision and is not enabled by `./dev.sh fast`. Normally the agent should
remain patch-only and the runner should own all compilation.

---

## When Not to Use FAST

Use the normal runner plus release/CI verification when:

- preparing a release, security sign-off, migration, or broad refactor;
- correctness requires multiple independent verification commands;
- a change spans ambiguous Cargo targets or needs reverse-dependency coverage;
- endpoint behavior, browser output, or TUI screenshots are acceptance evidence;
- the agent must compile or run a server interactively to discover the implementation;
- `target/debug/roko` is missing or does not contain the implementation being exercised;
- a missed regression would cost more than the saved feedback time.

FAST optimizes iteration. It does not replace clean-worktree, workspace-wide release validation.

---

## Implementation Status

Completed for P0:

- [x] Capacity and exact-attempt ownership are reserved before expensive dispatch preparation.
- [x] FAST agents receive a patch-only contract and a 90-second dispatch cap.
- [x] One authored, target-aware verification command is required and run by the runner.
- [x] Warm build artifacts are preserved and hot-path maintenance is deferred.
- [x] Ambiguous target narrowing fails back to the safe broad behavior outside task-owned mode.
- [x] Shared-target access is a separate, path-confined, explicit trusted opt-in.
- [x] Fresh worktrees support `SKIP_FRONTEND_BUILD=1` through a tracked fallback page.
- [x] Evidence capture is private, redacted, process-group bounded, and size bounded.
- [x] Timed-out attempt state settles before bounded evidence export.

Verification completed for the landed P0 change:

- [x] Nightly formatting check.
- [x] `cargo check -p roko-cli --lib` from a clean worktree without generated frontend `dist/`.
- [x] Shell and Python syntax checks plus FAST help and forbidden-TUI checks.
- [x] Evidence success, timeout, resistant-descendant cleanup, redaction, permission, noisy-output,
  and oversized-diff smoke checks.
- [ ] Full `roko-cli` unit-test target: deliberately stopped after workspace-scale compilation took
  several minutes without reaching a test.
- [ ] Workspace tests and strict workspace clippy: deferred to release/CI; do not infer a pass.

Deferred improvements:

- [ ] Benchmark representative FAST plans and track escaped-regression rates.
- [ ] Select changed crates plus reverse dependencies and run broad checks asynchronously.
- [ ] Add typed timeout-diff salvage to the normal gate lifecycle.
- [ ] Deduplicate semantically equivalent verification hidden behind arbitrary shell wrappers.
- [ ] Discover endpoints and collect browser/TUI screenshots in evidence bundles.
