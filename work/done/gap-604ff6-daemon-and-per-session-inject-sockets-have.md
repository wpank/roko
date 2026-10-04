+++
id = "gap-604ff6"
kind = "gap"
title = "Daemon and per-session inject sockets have no sun_path length protection, unlike the hub socket"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/daemon", "roko-cli/inject"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "7789cfee6"
source = "wave-6 follow-up reports 2026-10-03 (PK06 gap-4b890c)"
discovered_from = "gap-4b890c"
anchors = ["crates/roko-cli/src/daemon.rs::daemon_socket_path", "crates/roko-cli/src/state_hub_ipc.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn daemon_socket_binds_under_a_deep_workdir' crates/roko-cli/ && cargo test -p roko-cli daemon_socket_binds_under_a_deep_workdir"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T11:30:35Z"
commit = "7789cfee6"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T09:32:36Z"
forced = false
evidence = "Gate 17a (merged 7789cfee6): verify daemon_socket_binds_under_a_deep_workdir passes. The daemon binds through state_hub_ipc::bind_socket, so a long path goes to /tmp/roko-<uid>/<hash>/ with a daemon.sock.path pointer that stop, status, reload and secrets follow; the socket is 0600. The inject socket already had the guard."
+++

## Problem

`crates/roko-cli/src/daemon.rs::daemon_socket_path` builds the daemon's control socket as a plain join with no
length protection:

```rust
fn daemon_root_dir(workdir: &Path) -> PathBuf { workdir.join(".roko") }
fn daemon_socket_path(workdir: &Path) -> PathBuf { daemon_root_dir(workdir).join("daemon.sock") }
```

So `<workdir>/.roko/daemon.sock` grows with `workdir`'s own length, with nothing capping it. `.roko/run/roko-<session>.sock`
(the per-session inject socket, per `crates/roko-cli/src/inject/transport.rs`) is built the same direct way. Unix
domain sockets have a `sun_path` limit of 104 bytes on macOS (108 on Linux) — `crates/roko-cli/src/state_hub_ipc.rs`'s
own module doc (lines 49-54) names this limit explicitly and backlog task 1224 worked around it for the hub socket
only: the real socket goes under a short path, and a pointer file (`hub.sock.path`) that clients read first carries
the actual location, so the visible path can stay long. Neither `daemon_socket_path` nor the per-session inject
socket got the same treatment.

## Why it matters

A sufficiently deep project checkout (nested worktrees, long CI runner paths, deep monorepo layouts) will silently
fail to bind the daemon or inject socket on macOS once `<workdir>/.roko/...sock` crosses 104 bytes — the same
class of failure 1224 fixed for the hub socket, left open here. `work/README.md`'s own batch rules already route
pause/control-ack test harnesses to root under `/tmp` specifically to dodge this (see `pause_canary`/`plan_control_ack`,
an earlier batch's research) — evidence this is a real, previously-hit failure mode, not a hypothetical one.

## Where

- `crates/roko-cli/src/daemon.rs::daemon_root_dir`, `::daemon_socket_path` (lines 844-846, 860-862).
- `crates/roko-cli/src/inject/transport.rs` — the per-session `.roko/run/roko-<session>.sock` path.
- The existing fix pattern to reuse: `crates/roko-cli/src/state_hub_ipc.rs` (pointer-file indirection, `hub.sock.path`, backlog 1224).

## Current state

Only the hub socket (`state_hub_ipc.rs`) has the pointer-file workaround. `daemon_socket_path` and the inject
transport's session socket still build a plain, unbounded path.

## Plan

1. Factor 1224's pointer-file pattern into a shared helper (`roko-fs` or `roko-cli`'s own ipc module) so a future
   socket doesn't have to rediscover it.
2. Apply it to `daemon_socket_path` and the per-session inject socket.
3. Add a regression test with a deliberately long workdir path asserting both sockets still bind.

## Done when

- The daemon and per-session inject sockets bind successfully under a workdir path deep enough to exceed 104 bytes
  with the naive join.
- The `[[verify]]` command passes.

## Notes

- Two other facets of this PK06 report were checked and not filed: `GraphExecutionControlAdapter` having no
  production caller is noted on `spec-9a3131` instead (see its Notes). The "30-CONDUCTOR design body still says
  twelve watchers" claim is already self-documented: `docs/v3/30-CONDUCTOR.md` line 41-42 already says "The design
  sections below speak of twelve watchers; the thirteenth, `retrieval-precision`, came later" — not a silent gap,
  so not filed.

## Progress

- 2026-10-04 (w4-length): implemented on `work/bug-045773` at c3450d285; cargo verification deferred to the batch
  gate. The shared helper already existed: `state_hub_ipc::{bind_socket, bound_socket_path, socket_pointer_path}`
  (1224), and the inject transport already uses it. The daemon now binds through `bind_socket` (via
  `bind_daemon_socket`), so a home too long for `sun_path` binds in `/tmp/roko-<uid>/<workspace hash>/` and
  `daemon.sock.path` names it. The socket is also owner-only now. The stop, status and reload clients and
  secrets' reload signal follow the pointer, and the cleanups remove the socket wherever it is bound, with its
  pointer. Test `daemon_socket_binds_under_a_deep_workdir`.
- Premise corrections: the per-session inject socket the item names already had 1224's treatment
  (`inject_socket_binds_under_a_long_workspace_path`). The per-session `roko-<session>.sock` that `DaemonConfig`
  names is never bound, only reported in `DaemonStatus`.
