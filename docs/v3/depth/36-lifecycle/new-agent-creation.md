# New Agent Creation (Successor Patterns)

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/new-agent-creation.md`
> Canonical source: v1 `docs/v1/17-lifecycle/07-new-agent-creation.md`
> Status: **Current** (lineage_id/generation fields in
> `AgentExtendedManifest`; SuccessorConfig with exploration boost in
> `roko-agent/src/lifecycle.rs`)

---

## 1. Purpose

Creating a new agent after deleting a previous one follows the same creation
flow documented in `agent-creation.md`. This document covers the specific
considerations that apply when the new agent is intended to receive knowledge
from a predecessor via backup/restore.

This is the third step of the four-step knowledge transfer process:

```
BACKUP --> DELETE --> CREATE --> RESTORE
```

---

## 2. Same Flow, Different Context

The creation flow is identical to first-time creation:

```bash
# Option A: Same config, fresh agent
roko init --config roko.toml

# Option B: New config from scratch
roko init --prompt "New strategy: focus on stablecoin yields"

# Option C: From template
roko init --template stablecoin-yield
```

The new agent receives:
- A new agent ID (`agent-{nanoid(12)}`).
- A fresh knowledge store (empty).
- A fresh affect state (neutral PAD vector).
- A fresh `PLAYBOOK.md` (empty, populated by Dream integration).
- Fresh coordination connections (if enabled).
- For chain domain: a new wallet, a new ERC-8004 identity.

The new agent is NOT the predecessor with a new name. It is a genuinely new
agent. This mirrors Arendt's concept of natality (Arendt 1958) -- every new
agent is a moment of beginning, carrying inherited context but not inherited
identity.

---

## 3. Three Successor Patterns

### Pattern A: Clean Start (No Inheritance)

Fresh agent with no knowledge from the predecessor. Appropriate when:
- The predecessor's strategy was fundamentally wrong.
- Conditions have changed so dramatically that old knowledge is harmful.
- The operator wants to test a completely different approach.

```bash
roko init --prompt "Completely new approach: passive index tracking"
```

The predecessor's backup exists on disk but is not restored.

### Pattern B: Same Strategy, Fresh Knowledge

Fresh agent with the same strategy but no inherited knowledge. Appropriate
when:
- The predecessor's knowledge has become stale (knowledge plateau).
- The operator wants the same goals but fresh learning.
- The predecessor had accumulated corrupt knowledge entries.

```bash
roko init --config roko.toml
# No restore step -- agent starts with empty knowledge store
```

### Pattern C: Lineage Continuation (Selective Restore)

Fresh agent with selected knowledge restored from the predecessor's backup.
This is the pattern that replaces legacy "succession":

```bash
roko init --config roko.toml
roko knowledge restore ./backups/agent-V1St-2026-04-12.neuro \
    --confidence-decay 0.85
```

See `selective-restore.md` for the full restore specification.

---

## 4. The Operator's Decision

The operator's choice among these patterns replaces the legacy system's
automatic succession. In the old architecture, succession was triggered by
death. In the new architecture, the same decision happens -- but it is
triggered by deliberate deletion, not an artificial death clock.

| Legacy option | New equivalent | When to use |
|--------------|----------------|-------------|
| No successor | Pattern A: Clean start | Strategy was wrong |
| New strategy | Pattern B: Same config, fresh knowledge | Knowledge was stale |
| Continue lineage | Pattern C: Selective restore | Knowledge is valuable |

The research that motivated the legacy design -- Rogers' Paradox (Rogers
1988), Enquist's critical social learners (Enquist et al. 2007), the Baldwin
Effect (Baldwin 1896, Hinton & Nowlan 1987) -- still applies. It just applies
to the operator's conscious decision rather than an automatic system event.

---

## 5. Identity and Continuity: Parfit's Relation R

The new agent receives a new ID, new wallet (chain domain), and new on-chain
identity (chain domain). There is no concept of "the same agent" across
deletion and recreation. This mirrors Parfit's argument in _Reasons and
Persons_ (1984): what matters in survival is not numerical identity but
psychological continuity and connectedness.

The new agent has psychological connectedness to its predecessor through
shared knowledge (if restored) -- Parfit's Relation R -- but it is not
numerically identical.

### Lineage Tracking

For operators tracking knowledge evolution across generations:

```rust
// crates/roko-agent/src/lifecycle.rs

pub struct AgentExtendedManifest {
    // ...
    pub lineage_id: Option<String>,  // Shared across successors
    pub generation: u32,             // Incremented on each creation
    pub successor: Option<SuccessorConfig>,
}
```

Lineage tracking is metadata only -- it does not affect agent behavior. It
provides history of:
- How many agents have been created in this lineage.
- What knowledge was backed up and restored at each generation.
- Whether later generations improved on earlier ones (via efficiency metrics).

---

## 6. Elevated Initial Exploration

When an agent is created with `generation > 0` (indicating a predecessor),
the affect engine defaults to slightly elevated exploration temperature for
the first 100 cognitive loop iterations:

```rust
pub struct SuccessorConfig {
    pub initial_exploration_boost: f64,  // Default: +0.2
    pub exploration_boost_duration: u64, // Default: 100 iterations
}
```

This ensures that a successor explores the current environment independently
before settling into inherited strategy patterns. It is the computational
equivalent of the Baldwin Effect -- the successor has capacity to learn
faster (thanks to inherited knowledge) but must still learn independently
(thanks to elevated exploration).

---

## 7. Anti-Proletarianization

Stiegler (2010, 2018) defined proletarianization as the process by which
knowledge, formalized by a technique, escapes the individual who thereby
loses it. A successor agent that merely executes inherited knowledge without
developing its own understanding is a proletarianized agent.

The architecture prevents proletarianization through four mechanisms:

1. **Confidence decay on restore** (0.85^N per generation): inherited
   knowledge is not trusted at face value.
2. **Elevated initial exploration**: the agent is architecturally biased
   toward independent learning.
3. **PLAYBOOK.md non-transfer**: the predecessor's machine-evolved heuristics
   are available for reference but not automatically loaded as active
   heuristics.
4. **Divergence tracking**: if lineage tracking is used, the system records
   how much the new agent's learned knowledge diverges from the restored
   knowledge. Low divergence is a warning sign of proletarianization.

---

## 8. Domain-Specific Successor Considerations

### Chain Domain

- New wallet via configured custody mode. Predecessor's wallet settled during
  deletion.
- New ERC-8004 identity with fresh capabilities and zero slash history.
- Reputation reset -- reputation is earned, not inherited. Prevents
  reputation laundering.
- Domain stakes returned during deletion; successor must re-stake.

### Coding Domain

- Codebase context not inherited via backup/restore (codebases change).
  Successor re-indexes independently.
- MCP/tool configurations carry forward if the same config file is used.
- Coding heuristics undergo standard 0.85^N confidence decay.

### Research Domain

- Citation networks are valuable and decay slowly (papers do not become stale
  as quickly as market data).
- Research methodology heuristics are relatively stable across generations.
- Topic expertise decays at the domain's natural rate.

---

## 9. Lifecycle Position

| Step | Command | Description |
|------|---------|-------------|
| 1. Backup | `roko knowledge backup` | Serialize knowledge state |
| 2. Delete | `roko agent delete --name X` | Clean shutdown |
| 3. **Create** | **`roko init`** | **New agent, fresh state** |
| 4. Restore | `roko knowledge restore` | Selective knowledge import |

All four steps are independently executable. An operator can back up without
deleting, delete without creating, or create without restoring.

---

## 10. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `lineage_id` / `generation` | `crates/roko-agent/src/lifecycle.rs` | Lineage tracking fields |
| `SuccessorConfig` | `crates/roko-agent/src/lifecycle.rs` | Exploration boost config |
| Agent creation | `crates/roko-cli/src/agent_serve.rs` | CLI creation path |

---

## Cross-References

- [agent-creation.md](agent-creation.md) -- Full creation flow
- [selective-restore.md](selective-restore.md) -- Restore knowledge into successor
- [knowledge-backup-export.md](knowledge-backup-export.md) -- How backup was created
- [agent-deletion-8-step.md](agent-deletion-8-step.md) -- How predecessor was deleted
- [academic-foundations.md](academic-foundations.md) -- Parfit, Baldwin, Stiegler citations
