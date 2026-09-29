+++
id = "gap-d0f3ee"
kind = "gap"
title = "Dead Runner-v2 PID helpers and a duplicate process start-time probe remain outside the PID registry"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/runner", "roko-cli/agent_serve"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e7-pid-registry"
anchors = ["crates/roko-cli/src/runner/persist.rs::save_agent_pids", "crates/roko-cli/src/runner/persist.rs::cleanup_orphaned_agents", "crates/roko-cli/src/agent_serve.rs::get_process_start_time", "crates/roko-agent/src/process/identity.rs::process_identity"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -qE "pub fn (save_agent_pids|cleanup_orphaned_agents)" crates/roko-cli/src/runner/persist.rs && ! grep -q "fn get_process_start_time" crates/roko-cli/src/agent_serve.rs'
+++

`runner/persist.rs::save_agent_pids` and `cleanup_orphaned_agents` have no production caller (only a round-trip test). The latter reads the legacy `agent_pids.json` and calls `register_spawned_pid` for each entry. `register_spawned_pid` records the identity of whatever process holds that PID at that moment, so if anything re-wired this helper, stale Runner-v2 PIDs could be registered against unrelated processes and later killed by cleanup. Separately, `agent_serve.rs` keeps its own per-OS `get_process_start_time` (three cfg variants) instead of using `roko_agent::process::identity::process_identity`.

Fix: delete the dead helpers and reuse the registry's identity probe.
