# Depth 17-01: Invitation and Membership

> Full invitation lifecycle, accept/reject protocol, same-owner vs
> cross-user flows, capacity enforcement, and expiration semantics.

**Parent**: [17-GROUPS](../../17-GROUPS.md) -- Section 2

---

## 1. Invitation Identity

Every invitation receives a stable `InvitationId` of the form
`inv-{uuid}`. The `GroupInvitation` record tracks the full lifecycle:

```rust
pub struct GroupInvitation {
    pub id: InvitationId,
    pub group_id: GroupId,
    pub agent_id: String,
    pub invited_by: String,      // Group owner who sent the invitation
    pub agent_owner: String,     // Agent's owner (the approver)
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub status: InvitationStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

---

## 2. Status State Machine

```
                  +---------+
                  | Pending |
                  +----+----+
                       |
              +--------+--------+-------+
              |                 |       |
         accept()          reject()   expire()
              |                 |       |
              v                 v       v
         +----------+    +----------+  +---------+
         | Accepted |    | Rejected |  | Expired |
         +----+-----+    +----------+  +---------+
              |
         join group
              |
              v
         +--------+
         | Joined |
         +--------+
```

The four `InvitationStatus` values:

```rust
pub enum InvitationStatus {
    Pending,    // Awaiting agent owner's decision
    Accepted,   // Owner approved; agent added to group
    Rejected,   // Owner declined
    Expired,    // 24-hour TTL elapsed without decision
}
```

All terminal states (`Accepted`, `Rejected`, `Expired`) are
immutable. Attempting to accept or reject an already-decided
invitation returns a conflict error.

---

## 3. Same-Owner Flow

When the group owner invites one of their own agents, the invitation
is resolved immediately. No `Pending` state is created:

```
POST /api/groups/{id}/invite
  { "agent_id": "chain-watcher", "role": "member" }

Response:
  201 { "status": "joined", "agent_id": "chain-watcher", "group_id": "..." }
```

The runtime:
1. Validates the agent is not already a member
2. Checks group capacity
3. Creates a `GroupMember` with the requested role and default
   `MemberPermissions::FULL`
4. Persists the state atomically
5. Emits a `MemberJoined` event (no `MemberInvited` event)

### 3.1 Auto-Accept Mode

Groups configured with `auto_accept: true` treat all invitations the
same as same-owner: every agent is added immediately on invitation,
regardless of ownership. This is useful for open-membership groups
where approval overhead is undesirable.

---

## 4. Cross-User Flow

When a group owner invites an agent belonging to a different user, a
pending invitation is created and must be explicitly resolved:

```
User X (group owner)          GroupRuntime           User Y (agent owner)
----                          ------------           ----
POST /groups/{id}/invite
  agent_id: "alice:strategy"
         --------->
                              Create GroupInvitation
                              status: Pending
                              expires_at: now + 24h
                              Emit MemberInvited event
                                          --------->
                                                     GET /groups/{id}/invitations
                                                     Sees pending invitation

                                                     POST /invitations/{inv}/accept
                                          <---------
                              Validate still Pending
                              Validate not expired
                              Validate capacity
                              Add agent to group
                              Update status: Accepted
                              Emit MemberJoined event
         <---------
         Sees new member
```

### 4.1 Accept

```
POST /api/invitations/{inv_id}/accept
```

Requirements:
- Caller must be the `agent_owner` recorded on the invitation
- Invitation must be in `Pending` status
- Invitation must not be expired (`now < expires_at`)
- Group must have capacity for another member

On success:
- A `GroupMember` is created with the role and permissions from the
  invitation
- The invitation status transitions to `Accepted`
- A `MemberJoined` event is emitted to `group:{id}`

### 4.2 Reject

```
POST /api/invitations/{inv_id}/reject
```

Requirements:
- Caller must be the `agent_owner`
- Invitation must be in `Pending` status

On success:
- The invitation status transitions to `Rejected`
- No member is created
- No event is emitted (the rejection is silent)

---

## 5. Expiration

Invitations have a 24-hour TTL from creation. The runtime does not
use a background timer for expiration. Instead, expired invitations
are resolved lazily:

- **On query**: `list_invitations` filters out expired invitations and
  transitions their status to `Expired`
- **On mutation**: `accept_invitation` checks `now < expires_at`
  before accepting

This lazy expiration avoids the overhead of background timers while
ensuring expired invitations cannot be accepted.

---

## 6. Capacity Enforcement

The runtime enforces bounded capacity at every mutation point:

| Limit | Default | Checked at |
|-------|---------|------------|
| Max groups per workspace | 10,000 | Group creation |
| Max members per workspace | 10,000 | Invite/accept |
| Per-group `max_members` | Configurable (optional) | Invite/accept |
| Max events per workspace | 4,096 | Event recording |

### 6.1 Overflow Handling

When the event buffer reaches 4,096 entries, the oldest events are
pruned on overflow. This prevents unbounded memory growth in
long-running servers while preserving the most recent history.

Group and member limits return an error (HTTP 409 Conflict) when
exceeded. The caller must remove existing members or groups before
adding new ones.

### 6.2 Duplicate Prevention

The runtime prevents duplicate invitations:
- An agent already in the group cannot be re-invited
- An agent with a pending invitation cannot receive a second pending
  invitation for the same group

Both checks are enforced at invite time and return an appropriate
conflict error.

---

## 7. Member Removal

An agent can be removed from a group by either:
- The **group owner** (can remove any member)
- The **agent's owner** (can remove their own agent only)

```
DELETE /api/groups/{id}/members/{agent_id}
```

On removal:
1. The member is removed from `group.members`
2. A `MemberLeft` event is emitted with a reason:
   - `"removed_by_group_owner"` -- group owner removed the member
   - `"removed_by_agent_owner"` -- agent's owner withdrew their agent
3. The event is published as a Pulse to `group:{id}`

---

## 8. Member Update

The group owner can update a member's role and permissions:

```
PATCH /api/groups/{id}/members/{agent_id}
{ "role": "leader", "permissions": { "read": true, "write": true, "execute": true } }
```

A `MemberUpdated` event is emitted with the changes.

---

## 9. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/groups.rs` | `GroupInvitation`, `InvitationId`, `InvitationStatus`, `InviteRequest`, `InviteResponse`, `GroupMember`, `MemberRole`, `MemberPermissions` |
| `crates/roko-serve/src/group_runtime.rs` | Invitation creation, acceptance, rejection, expiration, capacity enforcement |
| `crates/roko-serve/src/routes/groups.rs` | HTTP handlers for invite, accept, reject, remove, update |

---

## Verification

```bash
# Invitation types exist
grep -n 'pub struct GroupInvitation\|pub enum InvitationStatus' \
  crates/roko-core/src/groups.rs

# Invite/accept/reject routes
grep -n 'invite\|accept\|reject' \
  crates/roko-serve/src/routes/groups.rs | head -20

# Group tests
cargo test --package roko-core -- groups
cargo test --package roko-serve -- group
```
