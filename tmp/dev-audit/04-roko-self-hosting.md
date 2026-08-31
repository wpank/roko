# Roko self-hosting bottlenecks

## P0: fix before tuning models or deleting tests

Implementation status in expanded integration snapshot `52d5f4df4` (final batched verification
pending):

- [x] Reserve capacity and exact-attempt ownership before dispatch preparation; count only actual
  launches.
- [x] Give FAST agents a patch-only contract and require one runner-owned authored verification.
- [x] Narrow canonical Cargo gates only for an unambiguous target and deduplicate exact required
  Cargo checks.
- [x] Skip critical-path warmup/cleanup and preserve warm artifacts in FAST.
- [x] Persist timeout terminal state before exporting bounded best-effort attempt evidence.
- [x] Provide headless, hard-deadline automation through `./dev.sh fast`.
- [x] Replace expensive/capacity polling with wake-driven FAST scheduling.
- [x] Redesign FAST plan policy to require cohesive bounded tasks and exact context. A general
  deterministic pre-edit/reflex broker remains separate work.
- [x] Re-enter the normal gate lifecycle with a safe, immutable-fingerprinted timed-out diff.
- [x] Converge terminal TUI/dashboard/status/ledger/PID projections and distinguish current from
  cumulative usage. Real final-tree fixtures remain part of the final batch.

### 1. Admission happens too late

Baseline behavior before P0:

    100ms executor tick
      -> prepare worktree/routing/context/prompt/hooks
      -> log “spawning”
      -> increment call counters
      -> try to acquire task capacity
      -> no permit, return and repeat

The pictured plan allowed one parallel task. While T1 held that permit, Roko prepared T2 about
1,108 redundant times. Relevant areas in the current dirty tree:

- Tick: event_loop.rs around 2532 and 4914–4995.
- Counter increment: around 9846–9847.
- Model event: around 10222.
- Pre-inference hook: around 10385.
- Capacity check: around 10458–10464.
- Capacity helper: around 8918–8938.

Required design:

    ready task
      -> atomically reserve attempt ID + capacity
      -> mark queued/running once
      -> prepare context/prompt once
      -> launch provider
      -> increment actual-launch metric

Use a ready queue awakened by permit release, not 100ms polling. Enforce an invariant:
one provider dispatch per run_id/plan_id/task_id/attempt_id.

Expanded result: admission, ownership, and actual-launch accounting are implemented. Capacity
release, agent/gate completion, retry readiness, and deadline settlement now wake FAST scheduling
directly, so the former 100 ms capacity poll no longer drives repeated preparation.

### 2. Agent and gate compilation ownership conflicts

Baseline behavior before P0:

- Runner gives Codex the parent repository target.
- Codex workspace sandbox cannot write it.
- Agent falls back to /tmp and cold-compiles.
- Runner gates compile again using the shared target.

Required design:

- FAST mode prompt explicitly forbids agent Cargo.
- Runner command broker owns exactly one selected check.
- If an agent genuinely needs Cargo, add only the shared target as a writable sandbox root and
  serialize compile ownership.
- Record cache mode, lock wait, command span, and cache hit/miss in the bundle.

P0 result: the patch-only contract, single authored verify owner, preserved cache, and narrowly
scoped trusted shared-target opt-in are implemented. Complete per-internal-command cache/lock spans
remain deferred.

Codex workspace-write can be configured with additional writable roots:
[Codex sandbox and approvals](https://learn.chatgpt.com/docs/agent-approvals-security#sandbox-and-approvals).

### 3. Gates are duplicated and target-blind

gate_dispatch.rs around 539–570 runs canonical gates and then every task verify. The default Rust
payload adds --lib around roko-gate/src/payload.rs 156–173. That missed T1's main.rs target and
then paid an additional authored compile.

Required design:

- Compute changed Cargo target.
- Select --lib, --bin, --test, or example/bench correctly.
- Normalize commands into semantic requirements.
- Execute each requirement once.
- Make gate mode explicit: none, structural, focused, full.
- Make startup warm explicit: on/off/auto; do not hardcode it in plan.rs.

Expanded result: gate mode is explicit (`none`, `structural`, `focused`, `full`); normal mode stays
full and FAST selects focused. Actual diffs plus Cargo metadata select target kinds/features and
bounded reverse dependents, while ambiguity/cap overflow widens safely. General equivalence hidden
behind arbitrary shell wrappers remains deferred.

### 4. Plans amplify overhead

doctor-network-v2 used seven serial microtasks. Every task can repeat model startup, worktree,
prompt, compilation, gates, commit, learning, and cleanup.

Required design:

- Bundle changes sharing context/target/evidence into one coherent task.
- Use deterministic transforms for mechanical pre-edits.
- Parallelize truly independent tasks only after admission is correct.
- Maintain one stable plan worktree where safe so absolute paths and incremental artifacts remain
  reusable.
- Validate referenced symbols, visibility, and source snippets before provider dispatch.

Expanded result: generated/direct/regenerated FAST plans share one fail-closed policy with exact
PRD/task artifacts, bounded files/ranges/reads/verification, cohesive task limits, and rejection of
same-file fragmentation. Deterministic mechanical transforms remain a separate optimization.

### 5. Timeout discards a useful patch

T2 had already edited doctor.rs before its cold build hit the 600-second timeout. The runner marked
it terminal and blocked five dependent tasks.

Required design:

- Distinguish editing from optional verification subprocesses.
- At the edit deadline, request structured handoff.
- If a diff exists, terminate agent-owned compilation and run the runner-owned selected gate.
- Preserve exact worktree/diff on failure.
- Retry only from an explicit attempt ID with bounded context.

Expanded result: timeout and cancellation settlement is bounded. After confirmed provider cleanup,
a non-empty mutable/safe diff is fingerprinted from its base plus exact content/modes, receives the
ordinary safety contract, and enters normal gate ownership with no second provider. Resume
revalidates identity and suppresses duplicate producers. Empty, conflicted, read-only, unsafe,
changed, or cleanup-unconfirmed diffs terminalize without certification.

### 6. Terminal TUI state is misleading

The runner failed at 08:17:23, but the TUI intentionally remained operator-owned and continued
showing a stale active agent and increasing elapsed/ETA. The displayed 408k/200k context was
cumulative input usage, mostly cached, not current prompt occupancy.

Required design:

- On terminal event, freeze elapsed time and ETA.
- Clear live PIDs/agents and mark tasks terminal.
- Display FAILED 18:09 while allowing postmortem navigation.
- Default automation to --no-tui or --exit-on-complete.
- Separate current context occupancy from cumulative input, cached input, and output usage.

Expanded result: FAST remains headless by default, while terminal projection now freezes elapsed
and ETA, clears only confirmed process ownership, preserves degraded cleanup truth, converges task
totals/outcome across durable views, and separates current/cumulative usage presentation. Runtime
fixtures on the final integrated tree are still pending.

## P1: shorten the build graph and critical path

### Feature-bound heavy dependencies

The expanded integration makes the ordinary CLI/serve development graph lean and moves full
provider, Alloy/chain, ACP, embedded frontend, Docker, and release behavior behind explicit
features/jobs. Chain-disabled routes return typed `501` diagnostics. Final lean/full compile proof
and representative before/after timing remain open.

### Remove frontend builds from normal Rust checks

roko-serve/build.rs can run npm install when node_modules is absent and npm run build during
ordinary Cargo work. Fresh worktrees omit ignored frontend artifacts.

- [x] Set `SKIP_FRONTEND_BUILD=1` in FAST and runner-owned Cargo.
- [x] Move embedded frontend production to explicit release/prebuild jobs with a tracked fallback.
- [x] Never perform a network package installation from a normal Cargo check.

### Choose one cache strategy

Benchmark, do not guess:

1. Shared persistent incremental target, stable worktree, serialized Cargo, no sccache expectation.
2. Per-worktree/revision target with CARGO_INCREMENTAL=0, sccache, normalized base paths, and
   bounded cache.

The current hybrid has an inaccessible shared cache for agents and zero useful sccache hits.
sccache notes that incremental crates and several crate types are not cacheable:
[sccache documentation](https://github.com/mozilla/sccache).

### Build profile

Benchmark a dev-fast profile:

- opt-level 0 for local crates and dependencies initially.
- debug info 0 or line-tables-only for fast checks.
- lld if compatible.
- pinned Rust toolchain.
- separated check/test artifacts if that reduces fingerprint explosion.

Do not apply profile changes globally without cold/warm benchmarks and debugging tradeoff review.

## P2: runtime hot paths and observability

- [x] Move prompt source-tree scans off async runtime threads and bound/cache selected context.
- [x] Reuse the in-memory cascade router rather than loading from disk on each global outcome.
- [x] Replace deep per-call config clones with `Arc`.
- [x] Avoid unconditional Git subprocesses for non-Git paths.
- [x] Buffer per-run nonterminal JSONL records and flush at lifecycle boundaries.
- [x] Index new event storage by run ID and opaque cursor; do not scan global logs on each query.
- [ ] Run size/revision-aware target and log GC off the plan critical path. This separate lane is
  not part of snapshot `52d5f4df4` and is not claimed complete here.
- [x] Add resource admission: refuse a cold self-host run under severe disk pressure unless the
  operator explicitly records an override; swap/memory and target sizing remain evidence.

tmp/backlog/58-perf-hot-path-fixes.md already specifies several of these local code changes.

## Baseline post-task cargo clean

resources.target_cleanup_enabled triggers clean_task_target_after_gate after a pass. The helper
runs cargo clean in the attempt worktree without explicitly carrying the shared target environment.
It therefore may erase a local fallback cache or do little in shared-target mode; it should not be
treated as reliable shared-cache reclamation.

FAST now skips this cleanup on its critical path. The normal lane and long-term background-GC
design remain unchanged.

Replace it with:

- No per-task clean.
- Revision/size-aware cache retention.
- Background GC at a disk high-water mark.
- Protected artifacts for active runs.
- Metrics for bytes reclaimed and the next-run cold-build penalty.

## Expected result for the pictured feature

After P0 only:

| Phase | Target |
|---|---:|
| Setup | under 2–5s |
| Plan validation/context | under 10s |
| Cohesive agent patch | 60–150s |
| One warm target-aware check | 8–30s |
| Direct CLI smoke/evidence | 10–30s |
| Commit/cleanup | under 10s |
| Total | about 2–4m warm |

A guaranteed cold sub-five-minute build is not realistic until the graph is leaner or compilation
is deferred/backgrounded.
