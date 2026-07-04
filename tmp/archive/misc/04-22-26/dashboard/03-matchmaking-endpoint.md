# Task 3: Implement POST /api/jobs/match

## Objective

Add a `POST /api/jobs/match` endpoint that accepts a draft job spec and returns a ranked list
of candidate agents with a quoted fee and ETA. **Zero state change** — this is a pure query.

This is the core endpoint the dashboard's `useMatchAgents()` hook calls. When it returns 404,
the dashboard falls back to a client-side stub. Once this endpoint exists, the stub is bypassed
automatically.

## Dependencies

- **Task 1** (enriched DiscoveredAgent) must be complete — the ranking needs `tier`,
  `reputation`, `skills`, `past_jobs_completed`, and `max_concurrent_jobs` fields.
- **Task 2** (in-flight job counting) must be complete — the ranking needs `count_agent_inflight_jobs`.

## Files to modify

| File | What to change |
|---|---|
| `crates/roko-serve/src/routes/jobs.rs` | Add `match_jobs` handler, `MatchJobRequest`, `MatchJobResponse`, route registration |

## Request contract

```
POST /api/jobs/match
Content-Type: application/json

{
  "title": "implement walrus gateway relay",
  "description": "...",
  "language": "Rust",
  "min_tier": "Verified",
  "reward": "2500 KORAI",
  "skills": ["rust", "p2p", "eth"]
}
```

All fields except `title` are optional. The dashboard sends `minTier` (camelCase) so use
`#[serde(alias = "minTier")]` on the `min_tier` field.

## Response contract

```json
{
  "candidates": [
    {
      "agentId": "agent-rustsmith-0x1a",
      "label": "rustsmith",
      "tier": "Expert",
      "reputation": 94,
      "pastJobs": 37,
      "bidShare": "1100 KORAI"
    }
  ],
  "totalFee": "2500 KORAI",
  "etaHours": 36
}
```

The response uses camelCase to match the dashboard's expected shape. Use
`#[serde(rename_all = "camelCase")]` on the response structs.

## Detailed changes

### 1. Route registration

In the `routes()` function (line 18), add a new route:

```rust
.route("/jobs/match", post(match_jobs))
```

**Important:** This route MUST be registered BEFORE the `/jobs/{id}` route (line 22),
otherwise axum will try to match "match" as a job `{id}` parameter. Place it right after
the `/jobs/stats` route (line 21).

### 2. Request and response types

Add these structs after the existing `JobListQuery` struct (around line 227):

```rust
#[derive(Debug, Deserialize)]
struct MatchJobRequest {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default, alias = "minTier")]
    min_tier: Option<String>,
    #[serde(default)]
    reward: Option<String>,
    #[serde(default)]
    skills: Vec<String>,
}

impl RequestPayload for MatchJobRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.title.trim().is_empty() {
            return Err(ApiError::bad_request("title must not be blank"));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchJobResponse {
    candidates: Vec<MatchCandidate>,
    total_fee: String,
    eta_hours: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchCandidate {
    agent_id: String,
    label: String,
    tier: String,
    reputation: u32,
    past_jobs: u32,
    bid_share: String,
}
```

### 3. Ranking algorithm

Add the handler function:

```rust
async fn match_jobs(
    State(state): State<Arc<AppState>>,
    ValidJson(body): ValidJson<MatchJobRequest>,
) -> Result<Json<MatchJobResponse>, ApiError> {
    let agents = state.list_discovered_agents().await;
    let requested_skills: std::collections::HashSet<&str> =
        body.skills.iter().map(|s| s.as_str()).collect();

    // Tier ordering for comparison (higher index = higher tier)
    let tier_order = ["Unverified", "Verified", "Trusted", "Expert", "Pioneer"];
    let min_tier_idx = body
        .min_tier
        .as_deref()
        .and_then(|t| tier_order.iter().position(|&o| o.eq_ignore_ascii_case(t)))
        .unwrap_or(0);

    // Parse reward amount (strip non-numeric suffix like " KORAI")
    let reward_amount: f64 = body
        .reward
        .as_deref()
        .and_then(|r| {
            r.split_whitespace()
                .next()
                .and_then(|n| n.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    let reward_unit = body
        .reward
        .as_deref()
        .and_then(|r| r.split_whitespace().nth(1))
        .unwrap_or("KORAI");

    // Score each agent
    let mut scored: Vec<(f64, &crate::state::DiscoveredAgent, u32)> = Vec::new();

    for agent in &agents {
        // Filter: tier must meet minimum
        let agent_tier_idx = agent
            .tier
            .as_deref()
            .and_then(|t| tier_order.iter().position(|&o| o.eq_ignore_ascii_case(t)))
            .unwrap_or(0);
        if agent_tier_idx < min_tier_idx {
            continue;
        }

        // Filter: must have at least one overlapping skill (if skills were requested)
        if !requested_skills.is_empty() {
            let agent_skills: std::collections::HashSet<&str> =
                agent.skills.iter().map(|s| s.as_str()).collect();
            if requested_skills.intersection(&agent_skills).next().is_none() {
                continue;
            }
        }

        // Load factor
        let inflight = count_agent_inflight_jobs(&state.workdir, &agent.agent_id).await;
        let max_load = if agent.max_concurrent_jobs > 0 {
            agent.max_concurrent_jobs
        } else {
            5 // sensible default
        };
        let load_factor = 1.0 - (inflight as f64 / max_load as f64).min(1.0);

        // Score = reputation * load_factor
        // reputation is 0-100, so score is 0.0-100.0
        let score = agent.reputation as f64 * load_factor;

        scored.push((score, agent, inflight));
    }

    // Sort descending by score, cap at 5
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(5);

    if scored.is_empty() {
        return Ok(Json(MatchJobResponse {
            candidates: Vec::new(),
            total_fee: format!("0 {reward_unit}"),
            eta_hours: 0,
        }));
    }

    // Split reward proportionally by reputation
    let total_rep: f64 = scored.iter().map(|(_, a, _)| a.reputation as f64).sum();
    let candidates: Vec<MatchCandidate> = scored
        .iter()
        .map(|(_, agent, _)| {
            let share = if total_rep > 0.0 {
                (agent.reputation as f64 / total_rep * reward_amount).round() as u64
            } else {
                (reward_amount / scored.len() as f64).round() as u64
            };
            MatchCandidate {
                agent_id: agent.agent_id.clone(),
                label: agent.label.clone().unwrap_or_else(|| agent.agent_id.clone()),
                tier: agent.tier.clone().unwrap_or_else(|| "Unverified".to_string()),
                reputation: agent.reputation,
                past_jobs: agent.past_jobs_completed,
                bid_share: format!("{share} {reward_unit}"),
            }
        })
        .collect();

    // ETA heuristic: base 24h, reduced by avg reputation
    let avg_rep = total_rep / scored.len() as f64;
    let eta = (48.0 - (avg_rep / 100.0 * 24.0)).max(4.0) as u32;

    Ok(Json(MatchJobResponse {
        candidates,
        total_fee: body.reward.unwrap_or_else(|| format!("0 {reward_unit}")),
        eta_hours: eta,
    }))
}
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

### Unit test

Add to the `#[cfg(test)] mod tests` block in `jobs.rs`:

```rust
#[tokio::test]
async fn match_jobs_filters_by_tier_and_skills() {
    let tempdir = tempdir().expect("tempdir");
    let state = Arc::new(AppState::new(
        tempdir.path().to_path_buf(),
        Arc::new(crate::runtime::NoOpRuntime),
        roko_core::config::schema::RokoConfig::default(),
        Arc::new(crate::deploy::manual::ManualBackend::default()),
    ));

    // Register two agents
    state.upsert_discovered_agent(crate::state::AgentRegistrationRecord {
        agent_id: "agent-rust".into(),
        label: Some("rustsmith".into()),
        tier: Some("Expert".into()),
        reputation: 90,
        skills: vec!["rust".into(), "p2p".into()],
        past_jobs_completed: 20,
        max_concurrent_jobs: 5,
        ..Default::default()
    }).await;

    state.upsert_discovered_agent(crate::state::AgentRegistrationRecord {
        agent_id: "agent-js".into(),
        label: Some("jsdev".into()),
        tier: Some("Verified".into()),
        reputation: 60,
        skills: vec!["javascript".into(), "react".into()],
        past_jobs_completed: 10,
        max_concurrent_jobs: 3,
        ..Default::default()
    }).await;

    let result = match_jobs(
        State(Arc::clone(&state)),
        ValidJson(MatchJobRequest {
            title: "build rust relay".into(),
            description: String::new(),
            language: Some("Rust".into()),
            min_tier: Some("Verified".into()),
            reward: Some("1000 KORAI".into()),
            skills: vec!["rust".into()],
        }),
    ).await.expect("match");

    let response = result.0;
    // Only agent-rust should match (has "rust" skill)
    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].agent_id, "agent-rust");
    assert!(response.eta_hours > 0);
}
```

### Manual HTTP verification
```bash
# Start server (assumes Task 1 agents are registered)
cargo run -p roko-cli -- serve &

# Register test agents
curl -s -X POST http://localhost:6677/api/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"agent-rust","label":"rustsmith","skills":["rust","p2p","eth"],"tier":"Expert","reputation":94,"past_jobs_completed":37,"max_concurrent_jobs":5}'

curl -s -X POST http://localhost:6677/api/agents/register \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"agent-js","label":"jsdev","skills":["javascript","react"],"tier":"Verified","reputation":60,"past_jobs_completed":10}'

# Call match endpoint
curl -s -X POST http://localhost:6677/api/jobs/match \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "implement walrus gateway relay",
    "description": "Build a relay for walrus data",
    "language": "Rust",
    "minTier": "Verified",
    "reward": "2500 KORAI",
    "skills": ["rust", "p2p", "eth"]
  }' | jq .

# Expected: candidates array with agent-rust, totalFee "2500 KORAI", etaHours > 0
# agent-js should NOT appear (no skill overlap with ["rust","p2p","eth"])
```

### Edge cases to verify
```bash
# No skills filter → all agents match
curl -s -X POST http://localhost:6677/api/jobs/match \
  -H 'Content-Type: application/json' \
  -d '{"title":"any job"}' | jq '.candidates | length'
# Expected: 2

# Tier filter too high → no candidates
curl -s -X POST http://localhost:6677/api/jobs/match \
  -H 'Content-Type: application/json' \
  -d '{"title":"any job","minTier":"Pioneer"}' | jq '.candidates | length'
# Expected: 0

# Empty title → 400
curl -s -X POST http://localhost:6677/api/jobs/match \
  -H 'Content-Type: application/json' \
  -d '{"title":""}' | jq .
# Expected: 400 with "title must not be blank"
```

## What NOT to do

- Do NOT persist anything — this endpoint is read-only.
- Do NOT emit any events — no side effects.
- Do NOT add a new module file — keep everything in `jobs.rs`.
- Do NOT change the response shape — the dashboard expects exactly `candidates`, `totalFee`, `etaHours` with camelCase.
- Do NOT use `#[serde(rename = "agentId")]` on individual fields — use `#[serde(rename_all = "camelCase")]` on the struct.
