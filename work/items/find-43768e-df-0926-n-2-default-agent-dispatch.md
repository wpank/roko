+++
id = "find-43768e"
kind = "finding"
title = "Default agent_dispatch_secs=600 too low for build-heavy tasks"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-core/config"]
created = 2026-09-26
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds"
anchors = ["crates/roko-core/src/config/timeouts.rs::default_agent_dispatch_secs", "roko.toml:393", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_task_dispatch.rs:3629", "crates/roko-graph/src/engine.rs::max_retries"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -A1 'const fn default_agent_dispatch_secs' crates/roko-core/src/config/timeouts.rs | grep -qE '^\\s*600\\s*$' && ! grep -qE '^agent_dispatch_secs = 600' roko.toml"
+++
Tasks that build and verify exceed 600s, fail with error_class=Timeout and burn the full retry budget (3 x 600s) learning nothing.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds`

How to verify: Review default and timeout-retry policy.

Verified 2026-09-28 (static check against 3d0ee4d02): The default is still 600s (roko-core/src/config/timeouts.rs:139-141; test :346) and the workspace roko.toml:393 sets agent_dispatch_secs = 600. Graph dispatch uses the task's timeout_secs when set, else agent_dispatch_secs (graph_task_dispatch.rs:3395-3399). A per-task override exists (task_parser.rs:170), but nothing treats a Timeout failure differently: the engine retries by FailureStrategy max_retries alone (roko-graph/src/engine.rs:2761), and no timeout-specific retry or escalation handling was found on the Graph path.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. The default and roko.toml are still 600 s, and the Graph path still has no timeout-specific retry or escalation (provider failover reacts to refusals and usage exhaustion only). The timeout selection is now at graph_task_dispatch.rs:3629 (dispatch) and :4202 (dispatch_streaming). Related: bug-562c22 (a timed-out Claude CLI attempt records zero cost).
