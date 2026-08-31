# Baseline evidence

> **Historical baseline, not current behavior.** These measurements are intentionally preserved
> as the before-state. The implementation mapped to each bottleneck is reconciled in
> [11-implementation-status.md](11-implementation-status.md); representative after-state benchmark
> repetitions have not yet been run.

## Snapshot

Collected on 2026-08-31 in /Users/will/dev/nunchi/roko/roko.

| Measurement | Value |
|---|---:|
| Final pictured runner duration | 1,089s |
| Completed tasks | 1 / 7 |
| Reported agent calls | 1,109 |
| Actual agent action dispatches | 2 |
| T1 agent phase | 368,607ms |
| T1 gate phase | 107,859ms |
| Startup cargo warm | 35,623ms |
| T1 default compile | 99,212ms |
| T1 authored compile | 8,106ms |
| T2 timeout | 600s |
| Repository target directory | 137GB |
| target/debug/incremental | 96GB |
| target/debug/deps | 39GB |
| Free disk | 39GiB, filesystem 98% used |
| Swap in use | about 54.8GiB |
| sccache Rust hits | 0 / 14 |
| roko-cli test attributes | 2,775 in 233 files |

The current shell had no RUSTC_WRAPPER, CARGO_TARGET_DIR, or CARGO_BUILD_JOBS override. sccache is
installed, but 35 calls were non-cacheable because Rust incremental compilation was enabled and
no base directories were configured.

## Exact timeline

Evidence sources:

- .roko/roko.log.2026-08-31 around lines 1289–4655.
- .roko/events.jsonl around lines 282393–282580.
- plans/doctor-network-v2/tasks.toml.

Timeline:

1. 07:58:28: plan setup completed.
2. 07:58:37: policy-triggered GC began.
3. 07:59:13: unconditional cargo cache warm completed after 35.6s.
4. 07:59:16: T1 was actually dispatched.
5. T1 made its two-line enum edit early, could not write the parent shared target from the Codex
   workspace sandbox, and switched to a cold target under /tmp.
6. 08:05:25: T1 agent completed with 408,156 cumulative input tokens, 358,656 cache-read tokens,
   and 2,270 output tokens.
7. 08:05:25–08:07:13: runner gate took 107.65s.
8. 08:07:14: T2 was actually dispatched.
9. T2 made a useful edit, hit the same cache denial, started another cold build, and was killed at
   its 600-second task deadline.
10. 08:17:23: the run ended Failed, with one completed and one failed task.
11. The TUI stayed open and displayed stale active state, explaining the later 26-minute screen.

## The 1,109-call anomaly

Within the final run's log range:

    spawning agent records:          1,109
    model-selected records:          1,109
    daimon marker records:           1,109
    agent action dispatched records: 2

The runner ticks every 100ms at event_loop.rs around lines 2532 and 4914. Dispatch preparation
logs “spawning agent,” updates routing/prompt/learning state, and increments counters before the
capacity check around lines 10458–10464. While T1 held the single plan permit, T2 was repeatedly
re-prepared and rejected.

Consequences:

- Bogus agent-call metrics.
- CPU and disk work on every retry.
- Thousands of redundant log lines/events.
- Repeated pre-inference hooks and learning side effects.
- Corrupted dispatch timing and exhausted daimon energy.
- A TUI that looks busy even when no provider launch happened.

This should be treated as a correctness bug, not a logging optimization.

## Cache mismatch

The runner exports CARGO_TARGET_DIR pointing at the repository target for Codex agents in
event_loop.rs around lines 10689–10699. The Codex invocation uses workspace-write rooted in the
attempt worktree and does not add the parent target as a writable directory in dispatch_v2.rs
around lines 514–555.

Therefore:

- The runner believes the agent has a hot shared cache.
- The sandbox rejects the write.
- The agent improvises a new /tmp target.
- The task pays a cold build and the subsequent runner gate compiles again.

Immediate choices:

1. Preferred FAST path: agent never runs Cargo; runner owns one targeted command.
2. Safe fallback: add only the shared target as an extra writable root and serialize Cargo owners.
3. Alternative experiment: isolated targets with CARGO_INCREMENTAL=0, sccache, and normalized base
   paths. Do not combine this blindly with the shared-incremental mode.

Cargo documents incremental compilation and target-directory behavior in its
[profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and
[build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html) references.

## Verification duplication

gate_dispatch.rs runs the canonical gate pipeline before every authored task verify. T1 incurred:

- Agent-owned cargo check in a cold /tmp target.
- Canonical cargo check scoped to roko-cli --lib, taking 99.2s.
- A structural grep, taking 14ms.
- Authored cargo check -p roko-cli, taking 8.1s.

The canonical check did not compile the edited main.rs binary. The cheap structural assertion and
the authored compile were the relevant evidence.

Escape hatches found during the baseline audit:

- ROKO_TASK_VERIFY_ONLY=1 bypasses the canonical pipeline.
- ROKO_SKIP_PREFLIGHT=1 bypasses preflight.
- SKIP_FRONTEND_BUILD=1 avoids the roko-serve frontend build path.

P0 update: `./dev.sh fast` now owns these settings as an explicit supported opt-in experiment,
requires one authored verification command, and adds deadlines/evidence capture. They no longer
need to be assembled manually, although FAST must remain opt-in until the scorecard passes.

## Plan amplification

doctor-network-v2 is seven tasks and sets max_parallel=1. Each task can pay:

- Agent startup and context assembly.
- A worktree and branch lifecycle.
- An agent-owned verification attempt.
- Canonical runner gates.
- Task-authored cargo checks.
- Commit, learning, cleanup, and transition overhead.

T1 and T2 had no dependency but could not run concurrently. More importantly, adjacent changes in
the same CLI feature should usually be one coherent task or two slices, not seven isolated model
sessions.

T2's plan context referenced a missing external contract and an invalid/private import. The agent
spent time searching rather than editing. A pre-dispatch plan compiler must resolve symbols and
embed the exact signatures; missing context should fail in milliseconds before provider launch.

## Machine and build-graph pressure

- The workspace has 35 packages and roko-cli has a very broad dependency graph.
- roko-cli pulls in about 20 internal crates and heavy chain/backend dependencies.
- roko-serve/build.rs may run frontend install/build work in fresh worktrees unless explicitly
  skipped.
- The dev profile uses debuginfo and optimization, generating very large incremental artifacts.
- A floating stable toolchain can invalidate caches after upgrades.
- Disk and swap pressure make every cold build and linker process slower.

Do not respond with recurring cargo clean in the interactive loop. The current post-gate helper
runs cargo clean in the task worktree without the shared-target environment, so it may clean a
local cache or be largely a no-op in shared-target mode; either way it is not useful critical-path
work. Use size/revision-aware cache GC when idle.

## Reproduction commands

These commands are read-only:

    rg -n "cargo cache warmed|task timing|gate completed|run complete" \
      .roko/roko.log.2026-08-31

    sed -n '1295,4647p' .roko/roko.log.2026-08-31 | rg -c "spawning agent"
    sed -n '1295,4647p' .roko/roko.log.2026-08-31 | rg -c "agent action dispatched"

    du -sh target target/debug/incremental target/debug/deps
    df -h .
    sysctl vm.swapusage
    sccache --show-stats
