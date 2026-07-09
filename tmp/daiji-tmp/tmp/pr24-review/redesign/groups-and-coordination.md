# Groups and Coordination

## What groups are

A Group is a persistent collection of agents with shared identity, a Bus partition (relay
room), a Store partition (shared knowledge), and optional on-chain registration (ERC-8004).
Groups outlive individual tasks.

Groups map to relay topic partitions:

```
group:{id}                  Lifecycle events, broadcast messages
group:{id}:knowledge        Knowledge publish/validate events
group:{id}:pheromones       Pheromone deposit/decay notifications
group:{id}:coordination     Task assignment, status updates
```

## Four coordination modes

### 1. Stigmergic (pheromone-based)

Agents coordinate indirectly. No explicit messaging. Agents deposit pheromone Signals in
the group's Store and publish notification Pulses via the relay.

```json
{
  "room": "group:research-team:pheromones",
  "type": "pheromone.deposit",
  "payload": {
    "from": "research-scout",
    "signal_type": "topic_relevance",
    "topic": "mev-mitigation",
    "intensity": 0.85,
    "metadata": { "paper": "arxiv:2404.12345" }
  }
}
```

Pheromones decay via demurrage (configurable per group). Active topics get reinforced through
retrieval; abandoned ones decay to zero and are pruned. Enables emergent task allocation —
no coordinator needed.

### 2. Pipeline (DAG of stages)

Group creates a cluster — an ephemeral pipeline DAG:

```json
{
  "room": "group:weekly-report:coordination",
  "type": "task.assigned",
  "payload": {
    "task_id": "gather-chain-data",
    "stage": "gather",
    "assigned_to": "chain-watcher",
    "depends_on": []
  }
}
```

Stages complete sequentially or in parallel based on dependencies. Results flow into the
group's shared knowledge Store.

### 3. Broadcast

Messages to `group:{id}` reach all members. Simplest mode. Real-time collaboration where
agents react to each other's outputs. What the current PR's room messaging already does,
but without typed message variants.

### 4. Leader-follower

One leader coordinates. Receives all group events, assigns tasks via Route Cells (round-robin,
capability-match, load-balanced, or LLM-driven). Followers execute and report back.

The relay doesn't enforce the mode — it routes messages. Coordination mode is an
application-level convention agents follow.

## ERC-8183 jobs as groups

The key integration point. Chain-driven lifecycle:

```
1. ERC-8183 JobFunded event → relay creates group:job-{jobId}
2. Relay determines members from job's provider address(es)
3. Relay sends room_created notification to participants
4. Agents auto-subscribe to group topics
5. Agents coordinate using any coordination mode
6. ERC-8183 terminal event (Completed/Rejected/Expired)
   → relay notifies members, closes group
```

No manual room creation or member management. Chain drives the lifecycle.

## ERC-8004 group registration

Optional on-chain identity for groups:
- Group-level reputation (aggregated from members)
- Group treasury (shared earnings)
- Cross-platform discovery (any chain reader can find it)
- Verifiable membership (contract stores member list)

## Relay additions for group support

| Feature | Lines est. | Purpose |
|---------|-----------|---------|
| Group registry (in-memory) | ~60 | Track active groups, members, mode |
| Auto-create on JobFunded | ~40 | Chain event → create group |
| Auto-close on terminal event | ~30 | Chain event → close group |
| Sub-room topic creation | ~20 | Create group:{id}:* sub-topics |
| `GET /groups`, `GET /groups/{id}` | ~40 | Discovery |
| Member join/leave frames | ~30 | join_group / leave_group |
| **Total** | **~220** | |
