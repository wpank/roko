# 32-deployment/11 -- Multi-Repo Coordination

> Single daemon managing N repositories: isolation model, shared
> scheduler, priority queuing, agent limits, and cross-repo knowledge.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-cli/src/commands/daemon.rs`,
`crates/roko-runtime/src/supervisor.rs`

---

## 1. Overview

The daemon runs as a single process managing N repository subscriptions.
Each subscription is isolated -- a plan run in repo A does not affect
repo B -- but they share system resources (CPU, memory, network) and can
optionally share knowledge through Agent Groups.

```
           Roko Daemon Process
  +----------+  +----------+
  | Repo A   |  | Repo B   |  ...
  | cron 30m |  | watch    |
  | 4 agents |  | 2 agents |
  | .roko/   |  | .roko/   |
  +----------+  +----------+
       |              |
       v              v
  +------------------------+
  |   Shared Scheduler     |
  |   (max 8 total agents) |
  +------------------------+
```

---

## 2. Isolation Model

### Filesystem Isolation

- Each repo has its own `.roko/` directory for state, signals, episodes
- Plan files are read from the repo's own plan directory
- Executor snapshots are per-repo in `.roko/state/`
- Signal logs are per-repo in `.roko/engrams.jsonl`
- Episode logs are per-repo in `.roko/episodes.jsonl`

### Process Isolation

- Agent processes for repo A cannot access repo B's files
- Each agent's working directory is set to its repo's root
- ProcessSupervisor tracks agents per repo; stopping a subscription
  kills only agents belonging to that subscription

### Configuration Isolation

- Each repo can override model, agent count, gates via `.roko/config.toml`
- API keys are daemon-level (shared across all subscriptions)

### What Is NOT Isolated

- CPU and memory: shared system resources, scheduler enforces limits
- API keys: daemon-level, shared across subscriptions
- Network: shared interface and provider rate limits
- The daemon process itself: a crash stops all subscriptions

---

## 3. Multi-Repo Loading

On startup, the daemon loads subscriptions from global config and
resolves per-repo overrides:

```rust
fn load_subscriptions(global: &GlobalConfig) -> Vec<ResolvedSubscription> {
    global.subscriptions.iter().map(|sub| {
        let repo_root = PathBuf::from(&sub.repo);
        let local = load_local_config(&repo_root);

        // Merge: defaults -> global sub -> local overrides
        ResolvedSubscription {
            repo: repo_root,
            model: local.model.or(sub.model.clone())
                .unwrap_or_else(|| defaults.model.clone()),
            max_agents: local.max_agents.or(sub.max_agents)
                .unwrap_or(defaults.max_agents),
            plan_dirs: local.plan_dirs.or(sub.plan_dirs.clone())
                .unwrap_or_else(|| vec!["plans/".into()]),
            triggers: merge_triggers(sub, &local),
            gates: merge_gates(&defaults.gates, &sub.gates, &local.gates),
        }
    }).collect()
}
```

Merge order:
1. Daemon defaults provide sensible baselines
2. Global subscription entries customize per-repo
3. Per-repo `.roko/config.toml` has final say

---

## 4. Shared Scheduler

The daemon enforces a global limit on concurrent agents:

```toml
[daemon]
max_concurrent_runs = 4
max_total_agents = 8
```

### Priority Queue

When multiple subscriptions trigger simultaneously and the agent pool is
full, the scheduler uses priority ordering:

| Priority | Trigger Type | Rationale |
|----------|-------------|-----------|
| 0 (highest) | Webhook | External event, time-sensitive |
| 1 | Watch | File change, user actively editing |
| 2 (lowest) | Cron | Scheduled, can wait |

Within the same priority level, runs are FIFO. The scheduler dequeues
as agent slots become available.

### Per-Repo Agent Limits

Each subscription can set `max_agents` as a ceiling within the global
limit:

```toml
[daemon]
max_total_agents = 8

[[subscriptions]]
repo = "/path/to/repo-a"
max_agents = 4          # Up to 4 of the 8

[[subscriptions]]
repo = "/path/to/repo-b"
max_agents = 2          # Up to 2 of the 8
```

---

## 5. Cross-Repo Knowledge Sharing

### Isolated by Default

Each repository's knowledge (signals, episodes, learned patterns) is
strictly isolated by default.

### Shared via Agent Groups

When Agent Groups are enabled, repositories can share knowledge:

```toml
[[subscriptions]]
repo = "/path/to/repo-a"

[subscriptions.groups]
enabled = true
share_kinds = ["Insight", "Heuristic", "Warning"]
min_confidence = 0.7
group = "nunchi-projects"
```

When sharing is enabled:
1. After a successful run, export qualifying Signals to the group
2. Before starting a run, query for relevant Signals from other repos
3. Imported Signals carry provenance indicating their source repo

This enables cross-domain insight resonance -- HDC structural analogy
detection (threshold 0.526) can identify patterns that transfer between
repositories. A heuristic from a web API project can surface when working
on a CLI project.

---

## 6. Error Handling

### Per-Repo Failure Isolation

A failure in one repo does not affect others:

- Failed plan runs are logged and recorded in subscription state
- The failed repo's next run proceeds normally
- Persistent failures (3+ consecutive) trigger a warning in
  `roko daemon status`

### Resource Exhaustion Protection

```toml
[[subscriptions]]
repo = "/path/to/repo-a"

[subscriptions.limits]
max_run_duration = "1h"
max_agent_memory_mb = 2048
max_roko_dir_size_mb = 1024
```

- **Memory**: ProcessSupervisor kills agents exceeding the limit
- **Time**: Plan runs have a configurable timeout (default: 1h)
- **Disk**: Old signals and episodes are garbage-collected when the
  `.roko/` directory exceeds the size limit

---

## 7. Git Operations

When the daemon triggers a plan run:

1. Verify the repo exists and is a git repository
2. Stash uncommitted changes (safety net)
3. Pull latest if remote is configured
4. Optionally create a worktree for isolated execution

The worktree option (`use_worktree = true`) runs plan execution in a
separate git worktree so the main working tree is not affected by
agent-generated changes.

---

## 8. Multi-Repo Status Display

```bash
$ roko daemon status

Roko Daemon
  PID:        12345
  Uptime:     3d 14h
  Agents:     5/8 (3 available)
  Queue:      1 pending

Subscriptions:
  /Users/will/dev/project-a
    Trigger:  cron (*/30 * * * *)
    Status:   running (3/5 tasks, 2 agents)
    Last:     2h ago (success)

  /Users/will/dev/project-b
    Trigger:  watch (plans/)
    Status:   idle
    Last:     15m ago (success)

  /Users/will/dev/project-c
    Trigger:  webhook (/hook/project-c)
    Status:   queued (waiting for agent slot)
```

---

## 9. Implementation Status

> **Implementation status:** Multi-repo coordination is at Tier 3H
> priority, dependent on daemon mode. The subscription format is designed.
> ProcessSupervisor already supports tracking multiple agent groups,
> mapping directly to per-repo isolation. The Graph executor supports
> concurrent plan runs via WorkflowGraphController. The shared scheduler,
> cross-repo knowledge sharing, and per-repo resource limits require the
> daemon event loop.
