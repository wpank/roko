# Task 1: Enrich DiscoveredAgent with matchmaking fields

## Objective

Add `tier`, `reputation`, `skills`, `past_jobs_completed`, and `max_concurrent_jobs` fields
to the `DiscoveredAgent` struct and the `AgentRegistrationRecord` upsert payload so that the
matchmaking endpoint (Task 3) can rank candidates.

## Files to modify

| File | What to change |
|---|---|
| `crates/roko-serve/src/state.rs` | Add fields to `DiscoveredAgent` (line ~58) and `AgentRegistrationRecord` (line ~108) |
| `crates/roko-serve/src/routes/agents.rs` | Add fields to `RegisterAgentRequest` (line ~319) and wire them into the upsert call (line ~56) |

## Detailed changes

### 1. `crates/roko-serve/src/state.rs` — DiscoveredAgent struct (starts at line 58)

Add these fields after the existing `domain_tags` field (line 88):

```rust
/// Agent tier label (e.g. "Verified", "Expert", "Pioneer").
#[serde(default, skip_serializing_if = "Option::is_none")]
pub tier: Option<String>,

/// Reputation score (0–100). Higher is better.
#[serde(default)]
pub reputation: u32,

/// Skill tags distinct from capabilities (e.g. ["rust", "p2p", "eth"]).
/// Capabilities are route-level ("messaging", "tasks"); skills are domain-level.
#[serde(default)]
pub skills: Vec<String>,

/// Total number of past jobs completed successfully.
#[serde(default)]
pub past_jobs_completed: u32,

/// Maximum concurrent jobs this agent can handle. 0 means unlimited.
#[serde(default)]
pub max_concurrent_jobs: u32,
```

### 2. `crates/roko-serve/src/state.rs` — AgentRegistrationRecord struct (starts at line 108)

Add matching fields after `domain_tags` (line 132):

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub tier: Option<String>,

#[serde(default)]
pub reputation: u32,

#[serde(default)]
pub skills: Vec<String>,

#[serde(default)]
pub past_jobs_completed: u32,

#[serde(default)]
pub max_concurrent_jobs: u32,
```

### 3. `crates/roko-serve/src/state.rs` — upsert_discovered_agent method (starts at line 443)

In the `or_insert_with` closure (line 451), initialize the new fields from the registration:

```rust
tier: registration.tier.clone(),
reputation: registration.reputation,
skills: registration.skills.clone(),
past_jobs_completed: registration.past_jobs_completed,
max_concurrent_jobs: registration.max_concurrent_jobs,
```

After the existing update block for `domain_tags` (around line 493), add update logic:

```rust
if registration.tier.is_some() {
    entry.tier = registration.tier;
}
if registration.reputation > 0 {
    entry.reputation = registration.reputation;
}
if !registration.skills.is_empty() {
    entry.skills = registration.skills;
}
if registration.past_jobs_completed > 0 {
    entry.past_jobs_completed = registration.past_jobs_completed;
}
if registration.max_concurrent_jobs > 0 {
    entry.max_concurrent_jobs = registration.max_concurrent_jobs;
}
```

### 4. `crates/roko-serve/src/routes/agents.rs` — RegisterAgentRequest (starts at line 319)

Add fields to `RegisterAgentRequest` after `mcp_endpoint` (line 344):

```rust
#[serde(default)]
tier: Option<String>,
#[serde(default)]
reputation: u32,
#[serde(default)]
skills: Vec<String>,
#[serde(default)]
past_jobs_completed: u32,
#[serde(default)]
max_concurrent_jobs: u32,
```

### 5. `crates/roko-serve/src/routes/agents.rs` — register_agent handler (starts at line 51)

Update the `AgentRegistrationRecord` construction (line 56) to pass the new fields:

```rust
tier: req.tier,
reputation: req.reputation,
skills: req.skills,
past_jobs_completed: req.past_jobs_completed,
max_concurrent_jobs: req.max_concurrent_jobs,
```

## Verification

### Compile check
```bash
cargo build -p roko-serve
```

### Existing tests must pass
```bash
cargo test -p roko-serve
```

### Manual verification
```bash
# Start the server
cargo run -p roko-cli -- serve &

# Register an agent with new fields
curl -s -X POST http://localhost:6677/api/agents/register \
  -H 'Content-Type: application/json' \
  -d '{
    "agent_id": "agent-rustsmith",
    "label": "rustsmith",
    "capabilities": ["messaging", "tasks"],
    "skills": ["rust", "p2p", "eth"],
    "tier": "Expert",
    "reputation": 94,
    "past_jobs_completed": 37,
    "max_concurrent_jobs": 3
  }' | jq .

# Verify the fields are returned on GET
curl -s http://localhost:6677/api/agents/agent-rustsmith | jq '.tier, .reputation, .skills'
# Expected: "Expert", 94, ["rust", "p2p", "eth"]
```

### Snapshot persistence
```bash
# After registering, check that the snapshot includes new fields
cat .roko/state/server-state.json | jq '.discovered_agents["agent-rustsmith"].tier'
# Expected: "Expert"

# Kill and restart the server, verify fields survive
kill %1
cargo run -p roko-cli -- serve &
curl -s http://localhost:6677/api/agents/agent-rustsmith | jq '.tier'
# Expected: "Expert"
```

## What NOT to do

- Do NOT rename or remove existing fields. This is purely additive.
- Do NOT add validation that rejects registrations missing these fields — they're all optional
  with serde defaults so existing callers continue to work.
- Do NOT modify any files outside `crates/roko-serve/`. The `roko-core::AgentEndpoints` type
  is unrelated.
