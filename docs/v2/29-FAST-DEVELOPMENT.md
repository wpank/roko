# 29 — FAST Development

> **Status**: P0 + IMPACT-AWARE P1 IMPLEMENTED; BUILD/LIBRARY CHECKPOINT VERIFIED; OPT-IN,
> EXPERIMENTAL
>
> **Landed**: 2026-08-31 in `a58bdbacbf80d72583edd628354ddb2750a8b822`
>
> **Purpose**: Shorten Roko self-development feedback loops while retaining one explicit,
> bounded, impact-aware verification and a private evidence record.

The measured baseline and expanded source-vs-final-verification ledger are tracked in
[`tmp/dev-audit`](../../tmp/dev-audit/README.md). Representative cold/warm promotion benchmarks
remain open even when an implementation checkbox is complete.

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
| Gate mode | `focused` |
| Compile concurrency | 1 |
| Evidence root | `.roko/runs/` |

`--max-tasks` and `--max-retries` can override their defaults, but doing so can reintroduce
compiler contention or repeated provider latency.

The wrapper exports `ROKO_GATE_MODE=focused` and `ROKO_COMPILE_CONCURRENCY=1`. The first selects the
impact-focused verification policy; the second serializes compiler ownership so parallel agent
work cannot turn a warm target into lock contention.

---

## Required Task Contract

Every task must author exactly one `[[task.verify]]` entry. Zero or multiple entries fail closed.
The authored command should prove behavior; the runner owns compilation and adds the narrowest
Cargo checks proven by the actual diff:

```toml
[[task.verify]]
phase = "test"
command = "cargo test -p roko-core --lib -- config::gates::tests::"
```

A focused test, syntax check, or deterministic assertion is also valid when it is the appropriate
proof. Avoid `cargo test --workspace`, workspace clippy, or another broad command in FAST plans;
those erase the latency benefit and belong in CI or release verification.

The patching agent is instructed not to run Cargo, tests, clippy, npm, builds, or servers. The
runner executes the authored proof and selects Cargo checks from changed package targets. Exact
library, binary, integration-test, example, and benchmark roots are recognized, including target
`required-features`. Shared modules widen to the package's targets. Public/re-export/trait/serde
contract edits add bounded transitive reverse dependents from `cargo metadata`; ambiguity,
metadata timeout, workspace build inputs, or cap overflow widens to the full gate.

Gate breadth is explicit under `[gates]`: `none`, `structural`, `focused`, or `full`. Normal mode
defaults to `full`; `./dev.sh fast` exports `ROKO_GATE_MODE=focused`. Invalid environment values
fail closed to `full`.

---

## Deadlines and Settlement

The wrapper separates execution from durable terminal settlement:

- For deadlines of 120 seconds or more, 30 seconds is reserved for settlement.
- For shorter deadlines, one quarter of the requested time is reserved, using integer seconds.
- FAST rejects a total deadline below 10 seconds.
- `ROKO_FAST_PLAN_DEADLINE_SECS` receives the requested deadline minus that headroom.
- `ROKO_FAST_SETTLEMENT_HEADROOM_SECS` carries the reserved headroom into the runner; process
  cleanup consumes only a bounded portion and leaves a deterministic tail for durable flushes.
- FAST scheduling is wake-driven. Capacity release, gate/agent completion, retry readiness, and
  deadline settlement wake admission directly instead of polling and repeating preparation.
- The non-resetting run deadline interposes every awaited dispatch-preparation step and both CLI
  and in-process bridge startup. Startup has its own 15-second default cap, configurable through
  `ROKO_FAST_STARTUP_DEADLINE_SECS`.
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
- the broad canonical gate pipeline when focused impact analysis proves a narrower scope;
- runner-owned Cargo auto-fix mutation/recompile passes;
- TUI approval, endpoint probing, browser/TUI screenshots, frontend builds, and agent-started
  servers.

FAST does not implicitly add `--fresh`, `--force`, permission bypasses, endpoint probes, or other
mutating native options. Eligible worktree cleanup and terminal-state persistence still run.

---

## Evidence and Security Bounds

`./dev.sh fast` delegates capture to `./dev.sh run-evidence`. Each run gets a private directory with
mode `0700`; created files use mode `0600`. It records the redacted command, an allowlisted
environment snapshot, timings, terminal status, machine/cache metadata, structured events, Git
state, and before/after tracked diffs. Output remains live while it is captured. The full collector
also samples fresh runner status, filters newly appended JSONL by the observed runner ID, inventories
the process group, calculates metrics and a score, writes a deterministic debrief, and validates the
portable bundle. See [Evidence bundles](30-EVIDENCE-BUNDLES.md).

While `roko serve` is available, new runs are also queryable without opening the global JSONL:
`GET /api/runs/{run_id}` discovers bounded links for events, tasks/attempts, gates, scrubbed logs,
metrics, artifacts, screenshots, and the bundle manifest. Event pagination uses the returned opaque
byte cursor, and `/api/runs/{run_id}/events/stream` provides run-filtered SSE. These GET-only routes
require a loopback bind or enabled API authentication and never serve arbitrary artifact content.

Behavior probes stay opt-in:

```bash
./dev.sh fast \
  --endpoint-base http://127.0.0.1:6677 \
  --cli-smoke 'status=target/debug/roko status --json' \
  --text-snapshot 'dashboard=target/debug/roko dashboard --text' \
  plans/my-plan
```

Endpoint discovery issues only bounded GET requests, follows no redirects, and permits loopback
hosts by default. `--screenshots` enables Roko's text collector; `--png-hook` integrates an
operator-selected browser command using its `{output}` placeholder. None of these hooks is enabled
or required implicitly. FAST does always require its structured event log to contain exactly one
run start and exactly one run terminal.

FAST also performs resource admission before launching Roko. It records filesystem capacity,
swap/memory state, and a two-second-bounded Cargo target size measurement. The default lane rejects
less than 5 GiB or 3% free space. `--allow-low-disk` is the explicit, evidenced escape hatch; the
equivalent automation variable is `ROKO_EVIDENCE_ALLOW_LOW_DISK=1`.

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

After confirmed provider cleanup, a timed-out FAST attempt with a non-empty, structurally safe diff
runs the ordinary post-dispatch safety contract. A passing diff is durably fingerprinted from its
immutable base plus exact tracked/untracked bytes and modes, then handed to the normal gate
lifecycle under exact no-provider ownership. Restart restores that handoff and the gate producer
revalidates the content identity immediately before starting; empty, conflicted, unsafe, changed,
or unconfirmed diffs terminalize normally. Evidence export remains best effort after durable
settlement, so a slow or oversized Git operation cannot prevent the terminal fact.

If the settlement budget expires before process absence can be proved, Roko retains exact ownership
and PID metadata. The terminal event, dashboard snapshot, and ledger report degraded cleanup instead
of clearing the orphan-cleanup source of truth.

---

## Cache and Disk Lifecycle

Build cleanup is an operator-visible background lane, not part of task dispatch. `roko cache
status` reports target, evidence, context-pack, and immutable log-archive sizes without mutating
anything. `roko cache prune` produces the same deletion plan; only `--apply` permits mutation. The
dev wrapper exposes the same surface:

```bash
./dev.sh cache status
./dev.sh cache prune
./dev.sh cache prune --apply --target-budget-gb 64 --min-age-hours 1
```

Target pressure selects old Cargo incremental partitions first and preserves compiled dependencies,
the newest incremental partitions, every Git-authoritative checkout, and current-revision evidence.
The report labels cold-build risk and projected/reclaimed bytes. Nonterminal evidence and the newest
terminal evidence runs are never selected. JSONL cleanup only targets immutable rotated generations;
live logs are never deletion candidates.

Mutation fails closed for symlinks and canonical-path escapes. It uses nonblocking advisory leases:
one global cache-GC lease, each affected workspace lock, each Cargo profile lock, and the live-log
writer lock. An active owner causes the entry to be skipped instead of making cleanup wait behind a
build or run. This keeps cleanup off the five-minute task critical path.

The compatibility `cargo_clean` hook preserves warm task targets unless the operator explicitly sets
`ROKO_EXPLICIT_CARGO_CLEAN=1`. Age-based automatic cleanup is limited to stale targets under
Git-unregistered runner worktrees; it does not evict the current or another linked worktree's cache.
Automatic post-terminal scheduling is intentionally deferred until it can share the runner's final
settlement ownership; the explicit command is the safe post-terminal/background integration point.

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

## Representative Benchmark Runner

The opt-in lane now has a deterministic scorecard orchestrator. It reuses the evidence bundle
collector and existing plan fixtures; it does not edit the operator's main worktree:

```bash
# No execution, provider call, build, or cache mutation.
python3 scripts/dev_benchmark.py list
python3 scripts/dev_benchmark.py run --dry-run --base <commit>

# Begin with a deliberately narrow paid trial.
python3 scripts/dev_benchmark.py run \
  --base <commit> \
  --roko-bin /path/to/roko-built-from-that-commit \
  --binary-base <commit> \
  --fixture enum-config \
  --repetitions 1 \
  --allow-network \
  --max-cost-usd 3
```

Every measured repetition is a detached linked worktree at the exact same resolved commit. The
runner also refuses to guess that a primary-worktree binary matches that revision: stock lanes need
`--roko-bin` plus a matching, recorded `--binary-base`, unless the operator deliberately marks the
session unverified. Cold samples receive unique, initially absent benchmark-owned Cargo targets;
after evidence finalization and process-group settlement they are safely disposed by default. Warm
samples reuse one bounded, separately seeded target per lane. The runner never calls `cargo clean`,
never removes a shared target, and forces Cargo offline. `--keep-targets` and `--keep-worktrees` are
explicit diagnostic retention controls.

Provider execution is locked until the operator supplies both `--allow-network` and an explicit
worst-case cost ceiling. Run count, per-run deadline, total deadline envelope, free disk, session
size, and individual cache size are bounded too. Execution is serial to avoid contaminating Cargo
lock and provider latency measurements.

Each private `.roko/benchmarks/<session>/` contains raw rows, every evidence bundle, admission and
worktree decisions, `scorecard.json`, and `SCORECARD.md`. Nearest-rank p50/p95 retains failures and
timeouts at observed duration, reports missing fields, excludes labeled warmups, and compares
current Roko/manual references with FAST without inventing absent manual samples. See
[`benchmarks/dev-audit/README.md`](../../benchmarks/dev-audit/README.md) for the complete contract.

The automation is implemented; the representative five-cold/five-warm matrix, imported manual
Codex/Claude samples, escaped-regression audit, and full-CI baseline are still required before FAST
promotion.

The final integration checkpoint verified that the default CLI dependency tree excludes the Alloy
provider/network/RPC-client graph, the no-default serve graph checks, the explicit Alloy/ACP CLI
graph checks, the current CLI binary builds, and the `roko-cli` library harness completes with
2,301 passed, zero failed, and one ignored test. Strict workspace no-dependency clippy passed for
the integrated implementation, and the final CLI/server/runtime delta passed a second strict,
targeted no-dependency clippy run. Loopback run
detail/events/tasks/gates/metrics,
opaque cursor pagination, bounded SSE replay, and a strict eight-endpoint/CLI evidence bundle were
then exercised successfully. The final production-path CLI check also passed after the immutable
listener-authorization boundary was added.

The workspace all-target test command is deliberately not marked green: the interactive attempt
was stopped while compiling its large integration-binary matrix after many earlier suites passed.
That work, escaped-regression measurement, and full CI stay in the release/promotion lane rather
than silently expanding the five-minute feedback loop.

---

## When Not to Use FAST

Use the normal runner plus release/CI verification when:

- preparing a release, security sign-off, migration, or broad refactor;
- correctness requires multiple independent verification commands;
- a high-impact change exceeds the configured reverse-dependent/target caps;
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
- [x] Evidence includes run-scoped status/log collection, safe GET discovery, process inventory,
  optional CLI/text/PNG hooks, metrics, scoring, deterministic debrief, and strict validation.
- [x] FAST records disk/swap/target resource admission and fails closed under severe disk pressure.
- [x] Gate breadth is explicit and normal mode remains full by default.
- [x] Focused gates detect changed lib/bin/test/example/bench targets with required features.
- [x] Public/high-impact diffs compile bounded transitive reverse-dependent packages.
- [x] One per-repository compile owner records lock wait, cache mode, command, and duration spans.
- [x] Runner-owned Cargo gates explicitly select `--profile dev-fast` when that profile exists and
  set `SKIP_FRONTEND_BUILD=1` in their subprocess environment.
- [x] Single-module and integration-target Cargo test commands are scoped conservatively.
- [x] Unchanged pre-existing failures are filtered only by stable structured evidence identity.
- [x] Planned-vs-actual file misses and impact decisions are emitted as structured logs.
- [x] Nextest `fast`, `slow`, and `live` profiles are defined; credentialed provider tests remain
  behind the existing `roko-agent/integration` feature.
- [x] FAST admission is wake-driven and exact ownership suppresses duplicate preparation.
- [x] Hard deadlines interpose awaited preparation plus CLI/bridge startup.
- [x] Preparation, startup, agent, gate, and cleanup durations remain separately attributable.
- [x] Safe timeout diffs enter the ordinary safety/gate lifecycle and resume by durable fingerprint.
- [x] Terminal dashboard/status/PID projections converge without losing unconfirmed cleanup truth.
- [x] Cache status/pruning is size-, age-, and revision-aware, dry-run first, and protected by
  nonblocking workspace/Cargo/log leases; target pressure retains compiled dependencies.
- [x] Fixed-SHA, worktree-isolated cold/warm benchmark automation emits raw evidence plus p50/p95
  current-vs-FAST/manual scorecards without deleting shared caches.
- [x] Verify the lean dependency tree, no-default serve and explicit Alloy/ACP graphs, current CLI
  build, complete CLI library harness, and strict no-dependency clippy checkpoint.
- [x] Verify the loopback run API, cursor/SSE behavior, and a strict explicit-endpoint evidence
  bundle with passing CLI hook, validation, feedback, and score.
- [x] Complete the final incremental format/diff check and strict CLI/server/runtime clippy over
  the small post-checkpoint delta.
- [ ] Complete the workspace all-target/full-CI release lane; the stopped interactive compilation
  is not a passing test result.
- [ ] Five cold and five warm representative samples, manual baselines, escaped regressions, and
  full-CI comparison have been collected and approved.

Verification completed for the landed P0 change:

- [x] Nightly formatting check.
- [x] `cargo check -p roko-cli --lib` from a clean worktree without generated frontend `dist/`.
- [x] Shell and Python syntax checks plus FAST help and forbidden-TUI checks.
- [x] Evidence success, timeout, resistant-descendant cleanup, redaction, permission, noisy-output,
  and oversized-diff smoke checks.
- [ ] Full `roko-cli` unit-test target: deliberately stopped after workspace-scale compilation took
  several minutes without reaching a test.
- [ ] Workspace all-target tests: deferred to release/CI; do not infer a pass from the green
  library harness or targeted final lint.

Deferred improvements:

- [ ] Benchmark representative FAST plans and track escaped-regression rates.
- [x] Select changed Cargo targets plus bounded reverse dependencies.
- [x] Add typed timeout-diff salvage to the normal gate lifecycle.
- [ ] Deduplicate semantically equivalent verification hidden behind arbitrary shell wrappers.
- [x] Discover safe GET endpoints and collect optional browser/TUI evidence without making probes
  part of the default FAST path.
