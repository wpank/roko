<!-- Master Plan: Tier 5 (some items) + Tier 3 (PRD workflow items) -->
<!-- Status: Not started -->
<!-- Depends on: Tier 2-3 for webhook learning, Tier 1 for base learning -->

# Phase 7: Learning & Autonomous Improvement

> **Master Plan Reference**: Tier 5 (some items) + Tier 3 (PRD workflow items)
> **Status**: Not started
> **Depends on**: Tier 2-3 for webhook learning, Tier 1 for base learning
> **Blocks**: Tier 5 cognitive features build on this
>
> ### What Already Exists in Codebase
> - `crates/roko-learn/src/` — 20 modules (episodes, playbooks, bandits, cascade router, experiments, efficiency)
> - `crates/roko-learn/src/cascade_router.rs` — model routing (wired)
> - `crates/roko-learn/src/experiments.rs` — A/B testing (wired)
> - `crates/roko-learn/src/efficiency.rs` — efficiency events (wired)
> - `crates/roko-learn/src/adaptive_gates.rs` — adaptive thresholds (wired)
> - `.roko/learn/` — persisted learning data
> - `.roko/episodes.jsonl` — episode log
>
> ### Reference Material
> - PRD learning: `/Users/will/dev/nunchi/roko/bardo-backup/prd/09-learning/`
> - Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learning.md`
> - Mori learning: `/Users/will/dev/uniswap/bardo/crates/bardo-learn/`

---

## 7.1 Episode logging for webhook-triggered agents

### Additional episode metadata

Webhook-triggered episodes need extra fields beyond plan-run episodes:

```rust
/// Extended episode metadata for event-driven agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEpisodeMetadata {
    /// The signal kind that triggered this agent.
    pub trigger_kind: String,
    /// Hash of the trigger signal.
    pub trigger_signal_hash: String,
    /// Source service ("github", "slack", "scheduler", "fswatcher").
    pub trigger_source: String,
    /// Template name used.
    pub agent_template: String,
    /// Experiment variant (if A/B testing).
    pub experiment_variant: Option<String>,
    /// External actions performed during this episode.
    pub external_actions: Vec<ExternalAction>,
}
```

### Wiring into dispatch.rs

In `run_agent_for_subscription()`:

```rust
async fn run_agent_for_subscription(
    state: &AppState,
    sub: &Subscription,
    signal_hash: &str,
    kind: &str,
) -> anyhow::Result<()> {
    // ... load template, build prompt ...

    // Start episode
    let episode_id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now();

    // Track external actions during execution
    let action_tracker = Arc::new(RwLock::new(Vec::<ExternalAction>::new()));

    // ... spawn agent, wait for completion ...

    // Log episode
    let episode = serde_json::json!({
        "episode_id": episode_id,
        "agent_template": sub.agent_template,
        "trigger_kind": kind,
        "trigger_signal_hash": signal_hash,
        "trigger_source": source,
        "experiment_variant": variant,
        "started_at": started_at.to_rfc3339(),
        "completed_at": chrono::Utc::now().to_rfc3339(),
        "duration_secs": started_at.elapsed().as_secs(),
        "success": result.success,
        "external_actions": action_tracker.read().await.clone(),
        "model": template.model,
        "turns": result.turns,
        "tokens_used": result.tokens_used,
    });

    let episodes_path = state.workdir.join(".roko/episodes.jsonl");
    let mut file = tokio::fs::OpenOptions::new()
        .create(true).append(true).open(&episodes_path).await?;
    file.write_all(format!("{}\n", serde_json::to_string(&episode)?).as_bytes()).await?;

    // Feed into cascade router
    state.cascade_router.record_outcome(&template.model, result.success).await;

    // Feed into efficiency tracker
    state.efficiency.record_event(
        &sub.agent_template,
        result.turns,
        result.tokens_used,
        result.success,
    ).await;

    Ok(())
}
```

### Checklist — 7.1

- [ ] Add `WebhookEpisodeMetadata` struct
- [ ] Wrap agent dispatch with episode logging
- [ ] Record trigger signal as first entry
- [ ] Track external actions during execution
- [ ] Feed episode into cascade router
- [ ] Feed episode into efficiency tracker
- [ ] **Verify:** Send webhook → agent runs → episode in `.roko/episodes.jsonl`
- [ ] **Verify:** Episode contains trigger_kind, external_actions, duration
- [ ] **Verify:** Cascade router updated (check `.roko/learn/cascade-router.json`)

---

## 7.2 Prompt experiments for templates

### How it works

1. Template declares an experiment:
```toml
[experiment]
name = "review-depth"
variants = ["concise", "thorough"]
metric = "review_resolution_rate"
```

2. On dispatch, query `ExperimentStore` for variant:
```rust
let variant = state.experiment_store
    .assign_variant(&template.experiment.as_ref().unwrap().name)
    .await;
```

3. Modify system prompt based on variant:
```rust
let prompt = match variant.as_str() {
    "concise" => format!("{}\n\n[STYLE: Be concise. Focus only on critical issues. Max 5 inline comments.]", template.system_prompt),
    "thorough" => format!("{}\n\n[STYLE: Be thorough. Review every file. Include suggestions for improvement, not just bugs.]", template.system_prompt),
    _ => template.system_prompt.clone(),
};
```

4. After execution, record the outcome metric:
```rust
state.experiment_store
    .record_metric(&experiment_name, &variant, metric_value)
    .await;
```

### Concrete experiments

| Template | Experiment | Variants | Metric | How measured |
|---|---|---|---|---|
| `pr-review-agent` | `review-depth` | concise, thorough | `review_resolution_rate` | % of review comments resolved (from feedback) |
| `doc-lifecycle-agent` | `doc-review-strictness` | strict, permissive | `issues_resolved_rate` | % of created issues that get closed |
| `enrich-agent` | `enrichment-depth` | light, deep | `pr_merge_rate` | % of enrichment PRs that get merged |
| `triage-agent` | `triage-style` | conservative, aggressive | `label_retention_rate` | % of assigned labels not changed by humans |
| `slack-notify-agent` | `notify-style` | brief, detailed | `reaction_rate` | Slack reactions on messages |
| `digest-agent` | `digest-format` | bullets, narrative | `thread_engagement` | Thread replies on Slack digest |

### Checklist — 7.2

- [ ] Wire `ExperimentStore` into dispatch loop
- [ ] Implement variant-based prompt modification
- [ ] Record experiment assignment in episode
- [ ] Wire feedback metrics into experiment outcomes
- [ ] **Verify:** Template with experiment → variants are assigned
- [ ] **Verify:** System prompt differs between variants
- [ ] **Verify:** Metrics recorded in `.roko/learn/experiments.json`
- [ ] **Verify:** Over time, better variant gets selected more often (Thompson sampling)

---

## 7.3 Autonomous feedback collection

**File:** `crates/roko-serve/src/feedback.rs`

### Implementation

```rust
//! Feedback collection — polls external systems for outcomes of past agent actions.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::{info, warn};

use roko_plugin::{ExternalAction, Engagement, FeedbackSignal};
use crate::state::AppState;

/// Start the feedback collection loop as a background task.
pub fn start_feedback_loop(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(900)); // 15 minutes

        loop {
            interval.tick().await;

            if let Err(e) = collect_all_feedback(&state).await {
                warn!(error = %e, "feedback collection failed");
            }
        }
    })
}

async fn collect_all_feedback(state: &AppState) -> anyhow::Result<()> {
    // 1. Load recent episodes with external actions
    let episodes = load_recent_episodes(&state.workdir, Duration::from_secs(86400)).await?;

    let mut feedback_count = 0;

    for episode in &episodes {
        for action in &episode.external_actions {
            let feedback = match action.service.as_str() {
                "github" => collect_github_feedback(state, action).await,
                "slack" => collect_slack_feedback(state, action).await,
                _ => continue,
            };

            if let Ok(Some(fb)) = feedback {
                // Record feedback signal
                record_feedback_signal(state, &fb).await?;

                // Feed into experiment metrics (if episode had an experiment)
                if let Some(ref exp) = episode.experiment_variant {
                    let metric = engagement_to_metric(&fb.engagement, &action.action_type);
                    state.experiment_store
                        .record_metric(&episode.experiment_name.as_deref().unwrap_or(""), exp, metric)
                        .await;
                }

                feedback_count += 1;
            }
        }
    }

    if feedback_count > 0 {
        info!(count = feedback_count, "collected feedback signals");
    }

    Ok(())
}

async fn collect_github_feedback(
    state: &AppState,
    action: &ExternalAction,
) -> anyhow::Result<Option<FeedbackSignal>> {
    let gh = &state.github_client; // octocrab::Octocrab instance
    let pr_info = parse_pr_resource(&action.resource_id)?;
    let (owner, repo, number) = (&pr_info.owner, &pr_info.repo, pr_info.number);

    match action.action_type.as_str() {
        "review_pr" => {
            // Fetch current PR state via octocrab
            let pr = gh.pulls(owner, repo).get(number).await?;

            let pr_merged = pr.merged_at.is_some();
            let pr_state = pr.state.as_ref().map(|s| format!("{s:?}")).unwrap_or_default();

            // Fetch reviews to check if our review was resolved or dismissed
            let reviews: Vec<octocrab::models::pulls::Review> = gh
                .get(format!("/repos/{owner}/{repo}/pulls/{number}/reviews"), None::<&()>)
                .await?;

            // Find the review our agent submitted (match by submitted_at closest to action time)
            let our_review = reviews.iter().find(|r| {
                r.user.as_ref().map(|u| u.login.as_str()) == Some(&state.config.github_bot_login)
            });
            let review_state = our_review
                .map(|r| format!("{:?}", r.state))
                .unwrap_or_else(|| "unknown".into());
            let review_dismissed = our_review
                .map(|r| r.state == Some(octocrab::models::pulls::ReviewState::Dismissed))
                .unwrap_or(false);

            // If PR is still open and review is still pending, skip — check again later
            if !pr_merged && !review_dismissed && pr.state == Some(octocrab::models::IssueState::Open) {
                return Ok(None);
            }

            // Time to merge (if merged)
            let hours = pr.merged_at
                .and_then(|merged| pr.created_at.map(|created| (merged - created).num_hours()))
                .unwrap_or(0);

            Ok(Some(FeedbackSignal {
                action: action.clone(),
                engagement: Engagement {
                    sentiment: if pr_merged { 1.0 } else if review_dismissed { -0.5 } else { 0.0 },
                    acknowledged: true,
                    outcome_achieved: pr_merged,
                    details: serde_json::json!({
                        "pr_state": pr_state,
                        "review_state": review_state,
                        "time_to_merge_hours": hours,
                    }),
                },
                collected_at: Utc::now(),
            }))
        }
        "comment_issue" | "comment_pr" => {
            // Fetch reactions on the comment
            let comment_id = action.metadata["comment_id"].as_u64()
                .ok_or_else(|| anyhow::anyhow!("missing comment_id in action metadata"))?;

            let reactions: Vec<octocrab::models::Reaction> = gh
                .get(
                    format!("/repos/{owner}/{repo}/issues/comments/{comment_id}/reactions"),
                    None::<&()>,
                )
                .await?;

            let reaction_names: Vec<String> = reactions.iter()
                .map(|r| r.content.clone())
                .collect();
            let has_reactions = !reactions.is_empty();

            // Check for replies by listing comments after ours
            let all_comments: Vec<octocrab::models::issues::Comment> = gh
                .get(format!("/repos/{owner}/{repo}/issues/{number}/comments"), None::<&()>)
                .await?;
            let our_idx = all_comments.iter().position(|c| c.id.into_inner() == comment_id);
            let reply_count = our_idx
                .map(|idx| all_comments.len().saturating_sub(idx + 1))
                .unwrap_or(0);
            let has_replies = reply_count > 0;

            // Check if the issue/PR is now closed
            let issue = gh.issues(owner, repo).get(number).await?;
            let issue_closed = issue.state == octocrab::models::IssueState::Closed;

            let sentiment = calculate_comment_sentiment(&reaction_names);

            Ok(Some(FeedbackSignal {
                action: action.clone(),
                engagement: Engagement {
                    sentiment,
                    acknowledged: has_reactions || has_replies,
                    outcome_achieved: issue_closed,
                    details: serde_json::json!({
                        "reactions": reaction_names,
                        "replies": reply_count,
                        "issue_state": format!("{:?}", issue.state),
                    }),
                },
                collected_at: Utc::now(),
            }))
        }
        "create_issue" => {
            // Fetch current issue state
            let issue = gh.issues(owner, repo).get(number).await?;
            let issue_closed = issue.state == octocrab::models::IssueState::Closed;
            let assigned = !issue.assignees.is_empty();

            // Current labels
            let current_labels: Vec<String> = issue.labels.iter()
                .map(|l| l.name.clone())
                .collect();

            // Original labels from when the agent created the issue
            let original_labels: Vec<String> = action.metadata["labels"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let labels_changed = current_labels != original_labels;

            Ok(Some(FeedbackSignal {
                action: action.clone(),
                engagement: Engagement {
                    sentiment: if labels_changed { 0.5 } else { 0.0 },
                    acknowledged: labels_changed || assigned,
                    outcome_achieved: issue_closed,
                    details: serde_json::json!({
                        "original_labels": original_labels,
                        "current_labels": current_labels,
                        "assigned": assigned,
                        "closed": issue_closed,
                    }),
                },
                collected_at: Utc::now(),
            }))
        }
        _ => Ok(None),
    }
}

async fn collect_slack_feedback(
    state: &AppState,
    action: &ExternalAction,
) -> anyhow::Result<Option<FeedbackSignal>> {
    let client = &state.slack_client; // reqwest::Client with Bearer token
    let token = &state.config.slack_bot_token;

    match action.action_type.as_str() {
        "post_message" | "reply_thread" => {
            // Parse channel and message timestamp from resource_id ("C01234:1712345678.123456")
            let (channel, ts) = parse_slack_resource(&action.resource_id)?;

            // Fetch reactions on the message via reactions.get
            let reactions_resp: serde_json::Value = client
                .get("https://slack.com/api/reactions.get")
                .bearer_auth(token)
                .query(&[("channel", channel.as_str()), ("timestamp", ts.as_str()), ("full", "true")])
                .send()
                .await?
                .json()
                .await?;

            let reactions: Vec<(String, u32)> = reactions_resp
                .pointer("/message/reactions")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| {
                            let name = r.get("name")?.as_str()?.to_string();
                            let count = r.get("count")?.as_u64()? as u32;
                            Some((name, count))
                        })
                        .collect()
                })
                .unwrap_or_default();

            let has_reactions = !reactions.is_empty();

            // Fetch thread replies via conversations.replies
            let replies_resp: serde_json::Value = client
                .get("https://slack.com/api/conversations.replies")
                .bearer_auth(token)
                .query(&[("channel", channel.as_str()), ("ts", ts.as_str()), ("limit", "100")])
                .send()
                .await?
                .json()
                .await?;

            let messages = replies_resp
                .get("messages")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();

            // First message is the parent; the rest are replies
            let thread_replies = messages.len().saturating_sub(1) as u32;

            // Count unique repliers (excluding the bot itself)
            let unique_repliers: usize = messages.iter()
                .skip(1) // skip parent message
                .filter_map(|m| m.get("user").and_then(|u| u.as_str()))
                .filter(|user| *user != state.config.slack_bot_user_id)
                .collect::<std::collections::HashSet<_>>()
                .len();

            // Compute positive/negative reaction counts for details
            let positive_names = ["+1", "thumbsup", "white_check_mark", "tada", "heart", "fire", "rocket"];
            let negative_names = ["-1", "thumbsdown", "x", "no_entry"];
            let positive_count: u32 = reactions.iter()
                .filter(|(name, _)| positive_names.contains(&name.as_str()))
                .map(|(_, count)| count)
                .sum();
            let negative_count: u32 = reactions.iter()
                .filter(|(name, _)| negative_names.contains(&name.as_str()))
                .map(|(_, count)| count)
                .sum();

            Ok(Some(FeedbackSignal {
                action: action.clone(),
                engagement: Engagement {
                    sentiment: calculate_slack_sentiment(&reactions),
                    acknowledged: has_reactions || thread_replies > 0,
                    outcome_achieved: thread_replies > 0,
                    details: serde_json::json!({
                        "reactions": reactions.iter().map(|(n, c)| serde_json::json!({"name": n, "count": c})).collect::<Vec<_>>(),
                        "positive_reactions": positive_count,
                        "negative_reactions": negative_count,
                        "thread_replies": thread_replies,
                        "unique_repliers": unique_repliers,
                    }),
                },
                collected_at: Utc::now(),
            }))
        }
        _ => Ok(None),
    }
}

/// Parse a Slack resource_id of the form "CHANNEL:TIMESTAMP" into (channel, ts).
fn parse_slack_resource(resource_id: &str) -> anyhow::Result<(String, String)> {
    let parts: Vec<&str> = resource_id.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("invalid slack resource_id: expected 'CHANNEL:TS', got '{resource_id}'");
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn calculate_slack_sentiment(reactions: &[(String, u32)]) -> f64 {
    let positive = ["+1", "thumbsup", "white_check_mark", "tada", "heart", "fire", "rocket"];
    let negative = ["-1", "thumbsdown", "x", "no_entry"];

    let pos: u32 = reactions.iter()
        .filter(|(name, _)| positive.contains(&name.as_str()))
        .map(|(_, count)| count)
        .sum();
    let neg: u32 = reactions.iter()
        .filter(|(name, _)| negative.contains(&name.as_str()))
        .map(|(_, count)| count)
        .sum();

    let total = pos + neg;
    if total == 0 { return 0.0; }
    (pos as f64 - neg as f64) / total as f64
}
```

### Polling schedule

```rust
/// Determine if we should still poll for feedback on this action.
fn should_poll(action: &ExternalAction) -> bool {
    let age = Utc::now() - action.performed_at;
    match age.num_hours() {
        0..=24 => true,    // First 24h: poll every 15 minutes
        25..=168 => {       // Days 2-7: poll every 6 hours
            // Only if the current 6h window aligns
            Utc::now().hour() % 6 == 0
        }
        _ => false,         // After 7 days: stop polling
    }
}
```

### Checklist — 7.3

- [ ] Create `feedback.rs` module
- [ ] Implement `start_feedback_loop()` with 15-minute interval
- [ ] Implement `collect_github_feedback()` for review, comment, issue actions
- [ ] Implement `collect_slack_feedback()` for message actions
- [ ] Implement sentiment calculation for Slack reactions
- [ ] Implement polling schedule (15min/6h/stop)
- [ ] Wire feedback into ExperimentStore metrics
- [ ] Wire feedback into CascadeRouter quality signals
- [ ] Record feedback signals in `.roko/signals.jsonl`
- [ ] **Verify:** Agent posts PR review → 15min later → feedback signal collected
- [ ] **Verify:** Agent posts Slack message → reactions → feedback captured
- [ ] **Verify:** Feedback metrics appear in experiment store
- [ ] **Verify:** Polling stops after 7 days

---

## 7.4 HDC integration points

### Signal fingerprinting

In the webhook handler (hooks.rs), after constructing the Signal:

```rust
// Compute HDC fingerprint for similarity queries
if let Ok(fingerprint) = bardo_primitives::hdc::fingerprint(&signal.body) {
    // Store alongside the signal
    signal.metadata.insert("hdc_fingerprint".into(), serde_json::to_value(&fingerprint)?);
}
```

### Episode fingerprinting

After episode completion:

```rust
// Fingerprint the episode for similarity-based routing
let episode_text = format!(
    "{} {} {} {}",
    episode.trigger_kind,
    episode.agent_template,
    episode.external_actions.iter().map(|a| &a.action_type).collect::<Vec<_>>().join(" "),
    if episode.success { "success" } else { "failure" },
);
let fingerprint = bardo_primitives::hdc::text_fingerprint(&episode_text);
```

### Similarity-based template recommendation

When no exact subscription matches, use HDC similarity:

```rust
async fn suggest_template(state: &AppState, signal: &Signal) -> Option<String> {
    let signal_fp = signal.metadata.get("hdc_fingerprint")?;

    // Find most similar past episode
    let episodes = load_recent_episodes(&state.workdir, Duration::from_secs(604800)).await.ok()?;

    let mut best_match = None;
    let mut best_similarity = 0.0;

    for ep in &episodes {
        if let Some(ep_fp) = ep.metadata.get("hdc_fingerprint") {
            let sim = bardo_primitives::hdc::cosine_similarity(signal_fp, ep_fp);
            if sim > best_similarity && sim > 0.7 {
                best_similarity = sim;
                best_match = Some(ep.agent_template.clone());
            }
        }
    }

    best_match
}
```

### Checklist — 7.4

- [ ] Add HDC fingerprint computation to webhook handler
- [ ] Add HDC fingerprint to episode metadata
- [ ] Implement similarity-based template suggestion
- [ ] **Verify:** Signals have `hdc_fingerprint` in metadata
- [ ] **Verify:** Similar events cluster (cosine similarity > 0.7)
- [ ] **Verify:** Template suggestion works for unmatched events

---

## 7.5 Cybernetic metrics dashboard

### Quantifiable metrics

| Metric | Computation | Endpoint |
|---|---|---|
| Agent success rate | `successful_episodes / total_episodes` (per template, per trigger kind) | `GET /api/metrics/success_rate` |
| Feedback engagement rate | `acknowledged_actions / total_actions` (per template) | `GET /api/metrics/engagement` |
| Model routing efficiency | Cost per successful episode (via CascadeRouter) | `GET /api/metrics/model_efficiency` |
| Gate pass rate | `passed_gates / total_gates` (per gate type, trending) | `GET /api/metrics/gate_rate` |
| Experiment lift | Metric difference between best and worst variant | `GET /api/metrics/experiments` |
| Time to feedback | Median hours between action and first feedback signal | `GET /api/metrics/feedback_latency` |
| Self-improvement velocity | Rate of change of success rate over time (should be positive) | `GET /api/metrics/velocity` |
| Autonomous coverage | % of events with matching subscriptions (vs unhandled) | `GET /api/metrics/coverage` |

### Aggregate metrics endpoint

`GET /api/metrics/summary`:

```json
{
    "period": "last_7_days",
    "agents_run": 142,
    "success_rate": 0.89,
    "feedback_engagement_rate": 0.73,
    "avg_cost_per_episode_cents": 12,
    "experiments_active": 4,
    "best_experiment_lift": { "name": "review-depth", "lift": 0.15, "winning": "concise" },
    "gate_pass_rate": 0.94,
    "self_improvement_velocity": 0.02,
    "top_templates": [
        { "name": "pr-review-agent", "runs": 38, "success_rate": 0.95 },
        { "name": "pm-board-agent", "runs": 42, "success_rate": 0.98 },
    ]
}
```

### Checklist — 7.5

- [ ] Implement metrics computation from episodes + feedback
- [ ] Add `GET /api/metrics/summary` endpoint
- [ ] Add per-metric endpoints
- [ ] **Verify:** After 10+ agent runs, metrics endpoint returns meaningful data
- [ ] **Verify:** Self-improvement velocity is computed correctly

---

# Phase 8: PRD-Driven Autonomous Development Workflow

> The capstone: humans write PRDs/specs/docs, roko agents autonomously implement them.

---

## 8.1 The end-to-end workflow

```
┌─────────────────────────────────────────────────────────────────┐
│                    HUMAN WORKFLOW                                 │
│                                                                  │
│  1. Write PRD ──► 2. Review ──► 3. Approve ──► 4. Merge to main │
│  (collaboration)   (GitHub PR)   (team)         (canonical)      │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼ webhook: push (canonical PRD)
┌─────────────────────────────────────────────────────────────────┐
│                    AGENT WORKFLOW                                 │
│                                                                  │
│  5. prd-ingestion-agent                                         │
│     ├── Reads canonical PRD                                     │
│     ├── Creates .roko/prd/ entry in target repo                 │
│     └── Emits prd.published signal                              │
│                                                                  │
│  6. auto-plan-agent                                             │
│     ├── Reads PRD                                               │
│     ├── Generates tasks.toml                                    │
│     ├── Creates PR: plan/{prd-slug}                             │
│     ├── Posts to Slack #roko-plans                              │
│     └── Emits prd.plan_generated signal                         │
│                                                                  │
│  7. HUMAN: Review plan PR ──► Approve ──► Merge                 │
│     └── Emits prd.plan_approved signal                          │
│                                                                  │
│  8. code-implementer-agent (cloud)                              │
│     ├── Clones target repo                                      │
│     ├── Creates feature branch: impl/{plan-slug}                │
│     ├── For each task:                                          │
│     │   ├── Implement changes                                   │
│     │   ├── Run gates (compile, test, clippy)                   │
│     │   ├── Auto-fix if gates fail (up to 3x)                  │
│     │   └── Commit                                              │
│     ├── Push branch                                             │
│     ├── Create PR with implementation                           │
│     └── Emits agent.completed signal                            │
│                                                                  │
│  9. review-response-agent                                       │
│     ├── Watches for PR review comments                          │
│     ├── Makes requested changes                                 │
│     ├── Pushes updated commits                                  │
│     └── Replies explaining changes                              │
│                                                                  │
│  10. HUMAN: Final review ──► Approve ──► Merge                  │
│                                                                  │
│  11. Learning                                                    │
│      ├── Full cycle episode recorded                            │
│      ├── Feedback: review cycles, time to merge, post-merge     │
│      ├── CascadeRouter updated                                  │
│      ├── Experiments optimized                                   │
│      └── Next plan benefits from learnings                      │
└─────────────────────────────────────────────────────────────────┘
```

### Signal flow across the workflow

```
webhook.github.push (canonical PRD) ──► prd-ingestion-agent
    │
    ▼
prd.published ──► auto-plan-agent
    │
    ▼
prd.plan_generated (plan PR created)
    │
    ▼ (human reviews and merges)
prd.plan_approved ──► code-implementer-agent
    │
    ▼
agent.completed (implementation PR created)
    │
    ▼
webhook.github.pull_request_review ──► review-response-agent
    │
    ▼
feedback.github.pr_engagement (collected after merge)
```

---

## 8.2 PRD ingestion agent

**Template:** `/Users/will/dev/nunchi/roko/roko/.roko/templates/prd-ingestion-agent.toml`

```toml
name = "prd-ingestion-agent"
description = "Watches collaboration repo for canonical PRDs, syncs to target roko repo"
model = "claude-haiku-4-5-20251001"
role = "operator"
max_turns = 5
max_concurrent = 1

triggers = ["webhook.github.push"]
mcp_servers = ["github"]

system_prompt = """
You are the PRD ingestion agent for Nunchi's roko system.

## Your job
When a document with status "canonical" and tag "prd" is pushed to the collaboration repo:

1. **Read the document** — Use `github.get_file` to get the full content from the collaboration repo.

2. **Determine target** — From the PRD frontmatter:
   - `target_repo`: which repo this PRD applies to
   - `target_crates`: which crates/directories are affected

3. **Create PRD entry** — In the target repo's `.roko/prd/` directory:
   - Filename: `{slug}.md` (derived from title)
   - Content: the PRD content with additional metadata

4. **Create PR** — Use `github.create_pr` in the target repo:
   - Branch: `prd/{slug}`
   - Title: "PRD: {title}"
   - Body: links to original in collaboration repo

5. **Notify** — The signal system will handle notifications via slack-notify-agent.

## Rules
- Don't ingest the same PRD twice. Check if .roko/prd/{slug}.md already exists.
- Preserve the original frontmatter.
- Add `source_repo` and `source_path` to the PRD metadata.
"""
```

**Subscription** (in roko/.roko/subscriptions.toml):
```toml
[[subscription]]
pattern = "webhook.github.push"
agent_template = "prd-ingestion-agent"
filter = { ref = "refs/heads/main" }
path_filter = "docs/**/prd-*.md"
repo_context = "/Users/will/dev/nunchi/collaboration"
cooldown_secs = 60
```

---

## 8.3 Auto-plan generation agent

Already defined in Phase 3.3 as `auto-plan-agent.toml`. The key addition here is how it interfaces with the existing `roko prd plan` system:

### Internal flow

```rust
// In the auto-plan-agent's execution:
// 1. Read the PRD from .roko/prd/{slug}.md
// 2. Call roko's plan generation (equivalent to `roko prd plan {slug}`)
// 3. This produces tasks.toml in plans/{slug}/
// 4. Create a PR with the generated plan
```

The agent doesn't need to call `roko prd plan` as a subprocess — it can use the plan generation prompt directly in its system prompt, leveraging the same Claude model to generate the task breakdown.

---

## 8.4 Code implementer agent (remote orchestrator)

Already defined in Phase 3.3 and Phase 5.4. The key details:

### GitHub App setup

The cloud orchestrator needs a GitHub App (not just a personal token) for:
- **Repo access:** Clone private repos
- **Branch creation:** Push to feature branches
- **PR creation:** Create implementation PRs
- **Status checks:** Read CI results

Required permissions:
- `contents: write` (push code)
- `pull_requests: write` (create/update PRs)
- `issues: write` (create issues, add labels)
- `checks: read` (read CI status)
- `metadata: read`

### Execution flow (detailed)

```rust
async fn implement_plan(
    state: &AppState,
    plan_slug: &str,
    repo: &RepoConfig,
) -> Result<()> {
    // 1. Clone repo
    let workspace = format!("/tmp/roko-workspace/{}", plan_slug);
    git_clone(&repo.url, &workspace, &state.github_token).await?;

    // 2. Create branch
    let branch = format!("impl/{plan_slug}");
    git_checkout_new_branch(&workspace, &branch)?;

    // 3. Load plan
    let tasks = load_tasks(&format!("{workspace}/plans/{plan_slug}/tasks.toml"))?;

    // 4. Execute tasks in dependency order
    let mut results = Vec::new();
    for task in topological_sort(&tasks) {
        // Check cost budget
        if total_cost > state.cloud_config.cost_budget_cents {
            tracing::error!("cost budget exceeded, aborting");
            break;
        }

        // Run the task (invoke Claude agent)
        let result = execute_task(&state, &workspace, &task).await?;

        // Gate check
        for gate in &task.gates {
            let gate_result = run_gate(&workspace, gate).await?;
            if !gate_result.passed {
                // Auto-fix attempt
                for attempt in 1..=3 {
                    let fix_result = run_gate_fixer(&state, &workspace, &gate_result).await?;
                    let retry = run_gate(&workspace, gate).await?;
                    if retry.passed { break; }
                    if attempt == 3 {
                        tracing::error!(task = %task.id, gate = %gate.gate_type, "gate failed after 3 fix attempts");
                    }
                }
            }
        }

        // Commit
        git_commit(&workspace, &format!("feat({}): {}", task.id, task.title))?;
        results.push(result);
    }

    // 5. Push branch
    git_push(&workspace, &branch, &state.github_token).await?;

    // 6. Create PR
    let pr = github_create_pr(
        &state.github_client,
        &repo.owner, &repo.name,
        &format!("Implement: {plan_slug}"),
        &format_pr_body(&results),
        &branch,
        "main",
    ).await?;

    // 7. Notify
    state.event_bus.emit(ServerEvent::AgentCompleted {
        agent_id: plan_slug.to_string(),
        success: true,
    });

    // 8. Cleanup
    tokio::fs::remove_dir_all(&workspace).await?;

    Ok(())
}
```

---

## 8.5 Review response agent

**Template:** `/Users/will/dev/nunchi/roko/roko/.roko/templates/review-response-agent.toml`

```toml
name = "review-response-agent"
description = "Responds to PR review comments by making changes and pushing updates"
model = "claude-sonnet-4-20250514"
role = "implementer"
max_turns = 15

triggers = ["webhook.github.pull_request_review"]
mcp_servers = ["github"]

system_prompt = """
You are the review response agent for Nunchi's roko system.

## Your job
When a PR review is submitted on an implementation PR (branch starts with "impl/"):

1. **Read the review** — Use `github.get_pr` to see the review comments.

2. **Analyze each comment:**
   - Actionable feedback → make the change
   - Question → answer in a reply
   - Nitpick → fix it (it's cheap to be thorough)
   - Disagreement → explain the reasoning, but defer to reviewer

3. **Make changes** — For each actionable comment:
   - Read the relevant file
   - Make the requested change
   - Push a new commit

4. **Reply to each comment** — Use `github.comment_pr` to:
   - Explain what you changed (or why you didn't)
   - Reference the commit SHA

## Rules
- Don't force-push. Add new commits so the reviewer can see the diff.
- If a change would require rearchitecting, say so and ask for guidance.
- Re-run gates after making changes to ensure nothing broke.
- Keep responses concise and professional.
"""
```

**Subscription:**
```toml
[[subscription]]
pattern = "webhook.github.pull_request_review"
agent_template = "review-response-agent"
filter = { action = ["submitted"] }
max_concurrent = 2
```

---

## 8.6 Integration with knowledge-base PM system

### Data flow

```
PRD (collaboration) → Plan (roko) → Tasks (knowledge-base PM) → Implementation → PR
```

Each step updates the PM system:

1. **PRD ingested** → Create PM task: "Review PRD: {title}" in `pm/tasks/`
2. **Plan generated** → Create PM workstream for the plan, tasks for each plan task
3. **Implementation started** → Update PM task status to "in-progress"
4. **Gate passed/failed** → Update PM task with gate results
5. **PR created** → Link PM task to PR URL
6. **PR merged** → Update PM task status to "done"

### pm-sync integration

The `pm_sync` script already syncs GitHub issues ↔ TOML tasks. For autonomous work:
- Implementation PRs are GitHub issues (via linked issues)
- pm-sync picks them up automatically
- Board views show autonomous work alongside human work

### Health report additions

The `pm-health-agent` should include autonomous work metrics:
- Plans in progress (agent-driven)
- Plans completed this week
- Average time from PRD → merged implementation
- Gate failure rate on autonomous PRs
- Review cycle count on autonomous PRs

---

## 8.7 Cross-repo coordination

### Signal routing

The central roko daemon receives events from all repos and routes them:

```
GitHub (collaboration) → webhook → roko daemon
    ├── PRD canonical? → prd-ingestion-agent
    ├── Doc changed? → doc-lifecycle-agent (collaboration context)
    └── Call note? → meeting-agent (collaboration context)

GitHub (knowledge-base) → webhook → roko daemon
    ├── Issue opened? → triage-agent (KB context)
    ├── PM file changed? → pm-board-agent (KB context)
    └── Doc changed? → enrich-agent (KB context)

GitHub (roko) → webhook → roko daemon
    ├── PR opened? → pr-review-agent (roko context)
    ├── Plan merged? → code-implementer-agent (roko context)
    └── PRD published? → auto-plan-agent (roko context)
```

Each subscription specifies its `repo_context`, so agents always operate in the right directory.

---

## 8.8 Safety and human oversight

### Hard rules (non-negotiable)

1. **Never push directly to main** — all changes go through PRs
2. **Gate validation required** — compile + test must pass before PR
3. **Human approval for merge** — PRs require at least 1 human approval
4. **Cost budgets** — per-plan and per-day spending limits
5. **Circuit breakers** — from `roko-conductor`: halt after N consecutive failures
6. **Audit trail** — every action logged in episodes.jsonl

### Configurable safety levels

```toml
# roko.toml
[safety]
# "strict" = human approval for every PR
# "normal" = human approval for code PRs, auto-merge for docs
# "autonomous" = auto-merge if all gates pass (dangerous!)
level = "normal"

# Max cost per plan execution (cents)
max_cost_per_plan = 5000

# Max cost per day across all agents (cents)
max_cost_per_day = 20000

# Circuit breaker: halt after N consecutive failures
consecutive_failure_limit = 5

# Require human approval for these actions
require_approval = ["github.merge_pr", "github.create_pr"]
```

### Escape hatches

- `roko daemon stop` — immediately stops all agents
- `roko agent stop <id>` — stop a specific agent
- Circuit breaker auto-triggers on repeated failures
- Slack alerts on any failure or unusual activity

---

## 8.9 End-to-end verification

### Step-by-step test

```bash
# 1. Start the daemon
roko daemon start --port 9090

# 2. Verify daemon is running
roko daemon status
# Expected: state=running, subscriptions=21, agents=0

# 3. Create a test PRD in the collaboration repo
cd /Users/will/dev/nunchi/collaboration
cat > docs/roadmap/prd-test-feature.md << 'EOF'
---
title: "Test Feature PRD"
status: canonical
domain: roadmap
owner: wp
tags: [prd]
target_repo: roko
target_crates: [roko-core]
created: 2026-04-08
updated: 2026-04-08
---

# Test Feature

## Problem
We need a test feature to verify the autonomous workflow.

## Solution
Add a `pub fn hello() -> &'static str { "world" }` to roko-core/src/lib.rs.

## Tasks
1. Add the function
2. Add a test
3. Verify it compiles
EOF

# 4. Commit and push
git add docs/roadmap/prd-test-feature.md
git commit -m "add test PRD"
git push origin main

# 5. Wait 30 seconds, then check:
# - prd-ingestion-agent should have fired
# - .roko/prd/test-feature.md should exist in roko repo
# - A PR should be created: prd/test-feature

# 6. Merge the PRD PR in the roko repo

# 7. Wait for auto-plan-agent:
# - Should create a plan PR: plan/test-feature
# - tasks.toml should have 3 tasks

# 8. Review and merge the plan PR

# 9. Wait for code-implementer-agent:
# - Should clone roko, create impl/test-feature branch
# - Should add the function and test
# - Should push and create implementation PR

# 10. Review the implementation PR
# - Verify hello() function exists
# - Verify test passes
# - Approve and merge

# 11. Check learning:
curl http://localhost:9090/api/metrics/summary
# Should show successful episodes

# 12. Check feedback (after 15 minutes):
grep "feedback" .roko/signals.jsonl
# Should show engagement signals

# 13. Stop daemon
roko daemon stop
```

### Verification checklist

- [ ] Daemon starts and loads all 21 subscriptions
- [ ] Test PRD push → prd-ingestion-agent fires
- [ ] PRD entry created in target repo's `.roko/prd/`
- [ ] auto-plan-agent generates plan, creates PR
- [ ] Plan PR merged → code-implementer-agent fires
- [ ] Implementation PR created with correct changes
- [ ] Gates pass (compile, test)
- [ ] PR review → review-response-agent handles comments
- [ ] Implementation merged
- [ ] Episodes logged for every agent run
- [ ] Feedback signals collected after 15 minutes
- [ ] PM system updated (if knowledge-base integration active)
- [ ] Slack notifications posted for key events
- [ ] Metrics endpoint shows correct data
- [ ] `roko daemon status` shows agent activity throughout
- [ ] `roko daemon stop` gracefully shuts down all agents
