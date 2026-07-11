# Stale lock and dirty worktree block recovery

- Severity: high
- Area: crash cleanup / resume

Agent PIDs were cleaned, but `.roko/runtime/roko.lock` still contains dead PID 38532. The E01 Git worktree remains registered, dirty, and checked out on `roko/plan/E01-execution-engine`.

Earlier resume attempts already oscillated between "worktree missing" and "branch already checked out." The post-timeout artifacts recreate that recovery hazard.

Shutdown must release/replace the singleton lock atomically, persist dirty-worktree ownership, and make resume reuse or repair the registered plan worktree deterministically.

