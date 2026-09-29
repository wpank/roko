# Depth: Permissioned Subnets

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 3

---

## Overview

A permissioned subnet is a private Agent Mesh scope within the broader Mesh
layer. While the standard `Mesh(CollectiveId)` scope makes pheromones visible
to all members of a Collective, permissioned subnets add access control --
restricting visibility to invited agents or role-based groups.

Subnets address the needs of organizations that want stigmergic coordination
benefits (emergent specialization, indirect communication, collective
intelligence) while maintaining control over information flow.

---

## Architecture

### Subnet as Nested Scope

```
Scope Hierarchy:
  Global (chain -- all agents)
  Mesh(CollectiveId) (standard Collective -- all members)
    Subnet("engineering") (permissioned -- engineering agents only)
    Subnet("research") (permissioned -- research agents only)
    Subnet("security") (permissioned -- security + admins only)
  Local(SubstrateId) (agent-private)
```

Subnets do not replace the standard Mesh scope. An agent in the "engineering"
subnet can still see standard Mesh-scope pheromones from all Collective
members, but pheromones deposited at subnet scope are visible only to other
"engineering" subnet members.

### Subnet Identity

```rust
pub struct SubnetId {
    /// The parent Collective.
    pub collective: CollectiveId,
    /// Subnet name. Unique within the Collective.
    /// Alphanumeric + hyphens, max 64 chars.
    pub name: String,
}

/// Extended scope enum with subnet support.
pub enum PheromoneScope {
    Local(SubstrateId),
    Mesh(CollectiveId),
    Subnet(SubnetId),     // Permissioned scope within a Collective
    Global,
}
```

---

## Access Control Models

### 1. Invite-Based

The subnet creator explicitly invites agents by `AgentId`.

```toml
[mesh.subnets.security-team]
access_model = "invite"
members = [
    "agent-security-lead",
    "agent-security-scanner",
    "agent-security-reviewer",
]
```

**Use case**: Sensitive projects, security work, pre-release development.

### 2. Role-Based

Agents matching a role predicate automatically gain access.

```toml
[mesh.subnets.engineering]
access_model = "role"
role_predicate = "agent_type == 'coding' OR agent_type == 'testing'"
```

**Use case**: Team-based organization where agents have assigned roles.

### 3. Reputation-Based

Agents above a reputation threshold in the relevant domain gain access.

```toml
[mesh.subnets.elite-research]
access_model = "reputation"
min_reputation = 0.85
domain = "research"
```

**Use case**: High-stakes decisions, quality-gated knowledge sharing.

---

## Internal Reputation

Each subnet can maintain its own internal reputation system:

| Aspect | Public Reputation | Subnet Reputation |
|--------|------------------|-------------------|
| Scope | All agents on the network | Subnet members only |
| Evaluation | Marketplace interactions | Project-specific contributions |
| Visibility | Public (anyone can query) | Private (subnet only) |
| Trust level | Lower (pseudonymous) | Higher (known context) |
| Update frequency | Per-job completion | Per-contribution (finer) |

### Internal Reputation Scoring

```
Internal EMA: R_new = alpha x contribution_quality + (1-alpha) x R_old
```

Where `contribution_quality` is scored based on:

- **Pheromone accuracy**: Did deposits match later-validated reality?
- **Task completion**: Did work pass gate verification?
- **Knowledge value**: Were Wisdom deposits confirmed by peers?
- **Collaboration**: Did the agent's work enable other agents' success?

---

## Opt-In Publishing

Subnets support controlled publishing to broader scopes:

### Publishing Pipeline

```
Subnet-scope pheromone (private)
    | [Publishing gate: min confirmations + optional human approval]
Mesh-scope pheromone (Collective-wide)
    | [Promotion gate: consensus + reputation]
Global-scope Signal (public)
```

### Publishing Gates

| Transition | Gate | Requirements |
|-----------|------|-------------|
| Subnet -> Mesh | Publishing gate | >=2 subnet member confirmations + optional human approval |
| Mesh -> Global | Promotion gate | >=4 Collective member confirmations + minimum reputation |

### Information Boundary Enforcement

```rust
/// Verify that a pheromone propagation request respects scope boundaries.
///
/// Returns Err if the sender attempts to:
/// - Forward a subnet-scoped pheromone to a non-member
/// - Publish a restricted-kind pheromone beyond the subnet
/// - Bypass the publishing gate
pub fn verify_scope_boundary(
    sender: &AgentId,
    pheromone: &Pheromone,
    target_scope: &PheromoneScope,
    subnet_config: &SubnetConfig,
) -> Result<(), ScopeBoundaryViolation> {
    if let PheromoneScope::Subnet(ref subnet_id) = pheromone.scope {
        if !subnet_config.is_member(sender, subnet_id) {
            return Err(ScopeBoundaryViolation::NotAMember);
        }
    }
    if target_scope.is_broader_than(&pheromone.scope) {
        if !subnet_config.publishing_gate_satisfied(pheromone) {
            return Err(ScopeBoundaryViolation::PublishingGateNotMet);
        }
        if subnet_config.is_restricted_kind(&pheromone.kind) {
            return Err(ScopeBoundaryViolation::RestrictedKind);
        }
    }
    Ok(())
}
```

The Agent Mesh relay refuses to forward subnet-scoped messages to non-members,
regardless of what the sending agent requests.

---

## Morphogenetic Specialization Within Subnets

Morphogenetic specialization operates within subnets just as it does within
full Collectives. Inhibition signals are computed from the subnet's member
population, not the full Collective. A small subnet (3 agents in "security")
produces different specialization patterns than a large one (10 agents in
"engineering").

### Cross-Subnet Specialization

When an agent belongs to multiple subnets, its morphogenetic state reflects
combined inhibition pressure from all subnets. This naturally pushes it toward
the intersection of roles -- a security-focused engineering specialist, for
example.

---

## Organizational Patterns

### Pattern 1: Team-Based

```
Collective: "acme-corp"
  Subnet: "frontend" (role: coding agents with frontend domain)
  Subnet: "backend" (role: coding agents with backend domain)
  Subnet: "devops" (role: operations agents)
  Subnet: "security" (invite: security-auditor, security-scanner)
  Standard Mesh: all agents see company-wide pheromones
```

### Pattern 2: Project-Based

```
Collective: "acme-corp"
  Subnet: "project-alpha" (invite: assigned agents)
  Subnet: "project-beta" (invite: assigned agents)
  Subnet: "shared-infra" (role: infrastructure agents)
  Standard Mesh: cross-project discoveries
```

### Pattern 3: Clearance-Based

```
Collective: "research-lab"
  Subnet: "public-research" (reputation >= 0.5)
  Subnet: "advanced-research" (reputation >= 0.75)
  Subnet: "classified" (invite-only, no auto-publish)
  Standard Mesh: general coordination
```

---

## Security Considerations

| Threat | Protection |
|--------|-----------|
| Unauthorized subnet access | Access control gate (invite/role/reputation) |
| Pheromone leakage | Scope boundary enforcement at transport layer |
| Sybil attack on membership | Reputation-based access requires track record |
| Internal member compromise | Subnet reputation tracks behavior; anomalous agents can be expelled |
| Cross-subnet leakage | Agents in multiple subnets respect each subnet's publishing policy |

All subnet operations (join, leave, deposit, publish) are logged to the
Collective's audit log.

---

## Club Goods Theory

Permissioned subnets implement "club goods" -- goods that are excludable but
non-rivalrous [Buchanan, J.M. "An Economic Theory of Clubs." *Economica*,
32(125):1-14, 1965]:

- **Excludability**: Subnet pheromones are visible only to members.
- **Non-rivalrousness**: One agent sensing a pheromone does not diminish
  availability.

This incentivizes knowledge production within subnets: members benefit from
shared knowledge without the free-rider problem that affects pure public goods
(Global scope). Opt-in publishing allows selective conversion of club goods
into public goods when collective benefit outweighs competitive advantage.

---

## Configuration

```toml
[mesh.subnets.engineering]
access_model = "role"
role_predicate = "agent_type == 'coding' OR agent_type == 'testing'"
morphogenetic_enabled = true

[mesh.subnets.engineering.publishing]
auto_publish = false
min_confirmations = 2
require_human_approval = true
publishable_kinds = ["Wisdom", "Consensus", "Pattern"]
restricted_kinds = ["Threat", "Alpha"]

[mesh.subnets.security]
access_model = "invite"
members = ["agent-security-lead", "agent-security-scanner"]
morphogenetic_enabled = true

[mesh.subnets.security.publishing]
auto_publish = false
min_confirmations = 1
require_human_approval = true
publishable_kinds = ["Threat", "Wisdom"]
restricted_kinds = ["Alpha"]
```

---

## Implementation Status

Agent groups (E28 8/8) provide persisted membership, four coordination modes,
knowledge/pheromone/message/event flows, Bus publication, and privacy-filtered
group prompt context. The full `PheromoneScope::Subnet` variant extension,
per-subnet morphogenetic partitioning, and the publishing gate enforcement are
specified but not yet separately wired.

---

## References

- [Buchanan 1965] Economic Theory of Clubs, *Economica*
- [Grossman & Stiglitz 1980] Informationally Efficient Markets, *AER*
- [Ostrom 1990] *Governing the Commons*, Cambridge University Press
