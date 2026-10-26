+++
id = "bug-02e0d3"
kind = "bug"
title = "A roko killed alone leaves its bash-tool and check processes running in the attempt tree, and a resume re-attaches that tree beside them: tools and gate steps never enter the agent PID registry that startup cleans"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-agent/process", "roko-std/tools", "roko-gate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/findings/roko-recovery.md, tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, 4d22b94e8)"
anchors = ["crates/roko-agent/src/process/registry.rs::cleanup_orphaned_agents", "crates/roko-std/src/tool/builtin/bash.rs", "crates/roko-gate/src/verify_shell.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::repair_worktree_state"]
links = { depends_on = [], blocks = [], related = ["gap-36f3fb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resume_stops_a_dead_runs_leftover_processes' crates/roko-cli/src && cargo test -p roko-cli --lib a_resume_stops_a_dead_runs_leftover_processes"
+++

## Problem

A roko can die alone while its children live on. That happens with a macOS jetsam or OOM kill, a panic with
`panic = "abort"`, or `kill -9` of just its pid. The bash tool's command and the gate's check commands then keep
running in the attempt tree, and the next roko neither knows about them nor stops them.

What the $0 kill matrix's orphan variants showed (round v7: `tmp/hermes-roko-v7/roko-recovery/`, `matrix.py
--orphans`, kill points b and c; 4d22b94e8):
- **b:** the agent's `bash` tool (`echo started > progress.txt; sleep 120; …`) and its `sleep` survived the kill,
  with their cwd in `.roko/worktrees/attempt-…`.
- **c:** the gate check's `bash` and `sleep` survived. `roko run plans/<p> --resume-plan` then:
  - re-attached that same tree and ran the next try in it, beside the dead run's still-running check;
  - succeeded, and "removed the delivered plan's attempt checkouts", deleting the tree under the live orphans.
  - The orphans were still alive afterwards.

For Undertow, the orphan is a cmake build or ctest run writing into the build tree the resumed try uses.

## Why it matters

- Goal core. The orphans can corrupt the resumed try's tree and build directory, or race its checks.
- They hold CPU and memory on a machine that was just killed for memory pressure: this round's reboot came after
  jetsam pressure.
- The same machinery already cleans up orphaned CLI agents. Tools and checks are the gap.

## Where

- `crates/roko-agent/src/process/registry.rs`: the per-process PID registry
  (`.roko/runtime/agent-pids/<owner>-<start>.json`) and `cleanup_orphaned_agents`. The cleanup runs at every roko
  start (`crates/roko-cli/src/main.rs::run_process_lifecycle_hooks`) and signals the children of owners that are
  provably gone.
- Only agent processes register: `exec.rs`, `claude_cli_agent.rs`, `cursor_cli_agent.rs`,
  `harness/child_process_runner.rs`, `acp_client.rs`. These don't:
  - the bash tool, `crates/roko-std/src/tool/builtin/bash.rs`, which runs in its own process group;
  - the gate's shell steps: `crates/roko-gate/src/verify_shell.rs`, `cancel_safe_command.rs` and `gate_env.rs`.
- The kill matrix's crashed states for b and c have no `agent-pids` record at all.

## Plan

1. Register the bash tool's child and each gate step's process group in the registry, with the owner's fingerprint
   as for agents, and unregister on exit.
2. Make `cleanup_orphaned_agents` signal the recorded process group (SIGTERM, then SIGKILL), not only the pid.
3. On a resume, run the cleanup before re-attaching the dead attempt's tree (`plan_runner.rs::repair_worktree_state`).
   Log what it stopped.

## Done when

- A test kills a roko alone mid-tool and mid-check, then starts a resume. The resume stops both groups before it
  re-attaches the tree (test `a_resume_stops_a_dead_runs_leftover_processes`).
- The kill matrix's orphan variants show no process left in the scenario after the resume.

## Notes

- Signal only processes whose recorded fingerprint still matches, as the registry already does for agents.
- A power cut needs none of this: everything dies together.
- Related: the roko-recovery p1 (resume after a kill), and gap-36f3fb (the re-attached tree).
