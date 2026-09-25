# Agent Deletion: 8-Step Ordered Shutdown

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/agent-deletion-8-step.md`
> Canonical source: v1 `docs/v1/17-lifecycle/06-agent-deletion.md`
> Status: **Current** (`roko agent delete` with ordered 8-step shutdown wired
> in `crates/roko-cli/src/agent_serve.rs`; DELETED marker written on
> completion)

---

## 1. Principle

Agent deletion in roko is **always user-initiated**. There is no natural
death, no stochastic termination, no vitality-driven shutdown. The user
decides when an agent should stop running. The deletion process follows a
clean, predictable shutdown sequence that reverses the provisioning pipeline.

Deletion is the second step of the four-step knowledge transfer process:

```
BACKUP --> DELETE --> CREATE --> RESTORE
```

The operator should back up the agent's knowledge before deleting. Deletion
without backup is allowed but produces a confirmation warning.

---

## 2. Deletion Commands

```bash
# Clean shutdown with confirmation prompt
roko agent delete --name my-agent

# Force kill (no graceful shutdown, skips pending work)
roko agent delete --name my-agent --force

# Delete with automatic pre-deletion backup
roko agent delete --name my-agent --backup
```

---

## 3. The 8-Step Shutdown Sequence

The clean shutdown reverses the provisioning pipeline. Each step undoes the
corresponding provisioning step, in reverse order:

```
Step 1: Signal shutdown to cognitive loop               ~instant
  - Current turn completes (if in progress)
  - No new turns started
  - Pending tool invocations complete or timeout (10s max)

Step 2: Flush knowledge store                           ~3-5s
  - All in-memory Signals written to persistent storage
  - Decay state snapshot persisted
  - Tier assignments persisted
  - If --backup flag: automatic backup created

Step 3: Deregister from coordination                    ~1-2s
  - Send deregistration message to relay
  - Close outbound WebSocket connection
  - Peers notified of departure

Step 4: Release tool handles                            ~instant
  - Close open file handles
  - Disconnect from external services
  - Cancel pending HTTP requests

Step 5: Shut down model routing                         ~instant
  - Close inference provider connections
  - Flush routing metrics to disk

Step 6: Release compute resources                       ~instant (self-hosted)
  - Stop health server                                  ~2-5s (hosted)
  - Release allocated memory
  - For hosted: control plane destroys VM

Step 7: Archive episode log                             ~1-2s
  - Flush .roko/episodes.jsonl to disk
  - Flush .roko/learn/efficiency.jsonl to disk
  - If --archive flag: copy all data to archive location

Step 8: Mark agent as deleted in state                  ~instant
  - Write DELETED marker with timestamp
  - Write deletion record to agent directory
```

**Total shutdown time**: 5-15 seconds for clean shutdown.
**Force shutdown time**: ~instant (SIGKILL, no graceful steps).

---

## 4. Shutdown Budget

The clean shutdown has a 30-second budget for all steps. If any step exceeds
its allocated time, it is skipped and the next step proceeds. This prevents
a hung connection or stuck tool invocation from blocking the entire shutdown:

| Step | Budget | Fallback if exceeded |
|------|--------|---------------------|
| Complete current turn | 10s | Abort turn, discard partial results |
| Flush knowledge store | 5s | Write what is flushed, log warning |
| Deregister from coordination | 3s | Close socket without clean deregistration |
| Release tools | 2s | Force close all handles |
| Shut down routing | 1s | Drop connections |
| Release compute | 5s (hosted) | Force destroy VM |
| Archive episodes | 3s | Write what is archived |
| Mark deleted | 1s | Write to stderr and exit |

---

## 5. Deletion Confirmation

Without `--force`, deletion prompts for confirmation:

```
$ roko agent delete --name my-monitoring-agent

WARNING: About to delete agent agent-V1StGXR8_Z5j

  Agent name:    my-monitoring-agent
  Neuro entries: 12,847 Signals
  Last backup:   2026-04-10T08:00:00Z (2 days ago)

  ! No backup exists from today. Consider running:
    roko knowledge backup

  Type 'delete' to confirm:
```

If no backup exists at all, the warning is more prominent, noting that
deletion will permanently lose all knowledge entries.

---

## 6. What Deletion Destroys

Deletion destroys the **agent process and its runtime state**. It does NOT
destroy:

- **Knowledge backups**: All backups in `.roko/backups/` are preserved.
- **Episode logs**: `.roko/episodes.jsonl` remains on disk.
- **Efficiency logs**: `.roko/learn/efficiency.jsonl` remains on disk.
- **Configuration**: `roko.toml`, `STRATEGY.md` remain on disk.
- **Learning data**: `cascade-router.json`, `gate-thresholds.json` preserved.

The philosophy: deletion removes the running agent, not its history. An
operator can always create a new agent and restore knowledge from a backup.

---

## 7. Post-Deletion State

After deletion, the agent directory contains:

```
.roko/
+-- backups/
|   +-- agent-V1StGXR8_Z5j/
|       +-- 2026-04-12T14-30-00Z.neuro     # Preserved backup
+-- episodes.jsonl                           # Preserved episode log
+-- learn/
|   +-- efficiency.jsonl                     # Preserved efficiency log
|   +-- cascade-router.json                  # Preserved routing data
|   +-- gate-thresholds.json                 # Preserved gate data
+-- agents/{name}/DELETED                    # Deletion marker
roko.toml                                    # Preserved config
STRATEGY.md                                  # Preserved strategy
PLAYBOOK.md                                  # Preserved heuristics
```

This directory is a complete record of the agent's existence. A new agent
can be created in the same workspace and coexist with the historical data.

---

## 8. Chain Domain: Wallet Settlement

For chain-domain agents, deletion includes wallet settlement:

### Delegation Mode (Recommended)

No settlement needed. The delegation grant expires. The operator retains full
control of funds in their own wallet. The session key is zeroized from memory
at shutdown.

### Embedded Mode

Remaining funds swept to the operator's wallet:
1. Query balance of agent's server wallet.
2. Initiate sweep transaction to operator's address.
3. Wait for confirmation (timeout: 60s).
4. If sweep fails: log failure, schedule retry in reconciliation job.

### LocalKey Mode

Delegation grant expires. Key material zeroized from memory. Encrypted
keystore file remains on disk for potential future use.

---

## 9. On-Chain Deregistration

For chain-domain agents with ERC-8004 identity:

1. Call `AgentRegistry.deregister(agent_id)` on Korai chain.
2. Wait for transaction confirmation.
3. ERC-8004 identity marked as deregistered (not burned -- historical record
   preserved).

Deregistration is optional but recommended. An underegistered identity
accumulates KORAI demurrage on staked tokens.

---

## 10. Graceful vs. Force Deletion

| Aspect | Graceful | Force (`--force`) |
|--------|----------|-------------------|
| Current turn | Completes | Aborted |
| Knowledge flush | Yes | No |
| Coordination deregistration | Yes | No |
| Wallet settlement | Yes | No |
| Episode archival | Yes | No |
| Data on disk | Preserved | Preserved |
| Time | 5-15s | ~instant |
| When to use | Normal operation | Hung process, emergency |

Force deletion should be used only when the agent process is unresponsive.
All data on disk (backups, logs, config) is preserved regardless.

---

## 11. Irreversibility

Deletion of the agent **process** is irreversible. Once deleted, you create
a new agent -- you do not "undelete." This prevents treating deletion as a
pause mechanism (use `roko plan pause` for that) and ensures that knowledge
transfer through backup/restore is explicit.

Deletion of the agent's **data** requires separate action (`rm -rf .roko/`).
This two-step design prevents accidental loss of valuable knowledge.

---

## 12. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `roko agent delete` | `crates/roko-cli/src/agent_serve.rs` | 8-step ordered shutdown |
| DELETED marker | `crates/roko-cli/src/agent_serve.rs` | Written on completion |
| `ProcessSupervisor` | `crates/roko-runtime/src/` | Process lifecycle |

---

## Cross-References

- [knowledge-backup-export.md](knowledge-backup-export.md) -- Back up before deleting
- [new-agent-creation.md](new-agent-creation.md) -- Create successor after deletion
- [selective-restore.md](selective-restore.md) -- Restore knowledge into successor
- [provisioning.md](provisioning.md) -- Deletion reverses provisioning
