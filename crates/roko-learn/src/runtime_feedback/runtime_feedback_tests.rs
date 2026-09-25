use std::time::Duration;

use super::cfactor_snapshot::{
    ContextAttributionRecord, KnowledgeConfirmationRecord, convergence_velocity_from_agreement,
    knowledge_integration_rate, social_perceptiveness_from_attribution,
};
use super::episode_helpers::gate_counts_from_episode;
use super::*;
use crate::model_router::compute_routing_reward_v2;
use crate::prompt_experiment::{PromptExperiment, PromptVariant};
use crate::regression::RegressionThresholds;
use chrono::Utc;
use roko_core::metric::{ConfigHash, TaskMetric};
use serde::Serialize;
use tempfile::TempDir;

fn sample_episode(success: bool) -> Episode {
    let mut ep = Episode::new("claude", "task-1");
    ep.success = success;
    ep.timestamp = Utc::now();
    ep.usage.input_tokens = 123;
    ep.usage.output_tokens = 45;
    ep.usage.cache_read_tokens = 7;
    ep.usage.cost_usd = 0.42;
    ep.usage.wall_ms = 900;
    ep.extra
        .insert("provider".to_string(), serde_json::json!("anthropic"));
    ep.extra
        .insert("model".to_string(), serde_json::json!("claude-opus-4-6"));
    ep.extra
        .insert("role".to_string(), serde_json::json!("Implementer"));
    ep.extra
        .insert("plan_id".to_string(), serde_json::json!("plan-1"));
    ep.extra
        .insert("complexity_band".to_string(), serde_json::json!("standard"));
    ep.extra
        .insert("iteration".to_string(), serde_json::json!(1_u64));
    ep.extra
        .insert("task_tags".to_string(), serde_json::json!(["rust", "fix"]));
    ep.extra.insert(
        "files".to_string(),
        serde_json::json!(["crates/roko-cli/src/run.rs"]),
    );
    ep.extra
        .insert("task_category".to_string(), serde_json::json!("bugfix"));
    ep
}

fn skipped_only_episode() -> Episode {
    let mut ep = sample_episode(true);
    ep.gate_verdicts.clear();
    ep.extra
        .insert("gates_passed".to_string(), serde_json::json!(0_u64));
    ep.extra
        .insert("gates_failed".to_string(), serde_json::json!(0_u64));
    ep.extra
        .insert("gates_skipped".to_string(), serde_json::json!(3_u64));
    ep.extra
        .insert("gates_executed".to_string(), serde_json::json!(0_u64));
    ep
}

#[test]
fn project_paths_keep_learning_state_under_learn_and_episodes_at_root() {
    let tmp = TempDir::new().expect("tempdir");
    let paths = LearningPaths::for_project(tmp.path());
    assert_eq!(paths.root, tmp.path().join(".roko/learn"));
    assert_eq!(
        paths.episodes_jsonl,
        tmp.path().join(".roko/episodes.jsonl")
    );
    assert_eq!(
        paths.cascade_router_json,
        tmp.path().join(".roko/learn/cascade-router.json")
    );
}

#[tokio::test]
async fn project_runtime_appends_only_the_root_episode_log() {
    let tmp = TempDir::new().expect("tempdir");
    let runtime = LearningRuntime::open_for_project(tmp.path())
        .await
        .expect("open project runtime");
    runtime
        .append_episode(&sample_episode(true))
        .await
        .expect("append episode");

    assert!(tmp.path().join(".roko/episodes.jsonl").is_file());
    assert!(!tmp.path().join(".roko/learn/episodes.jsonl").exists());
    assert!(!tmp.path().join(".roko/memory/episodes.jsonl").exists());
}

#[tokio::test]
async fn project_runtime_migrates_v2_episode_stores_before_append() {
    let tmp = TempDir::new().expect("tempdir");
    let layout = roko_fs::RokoLayout::for_project(tmp.path());
    std::fs::create_dir_all(layout.learn_dir()).expect("create learn dir");
    std::fs::create_dir_all(layout.memory_dir()).expect("create memory dir");
    std::fs::write(layout.version_file(), "2").expect("write version");

    let mut legacy = sample_episode(true);
    legacy.id = "legacy-episode".to_string();
    legacy.episode_id = "legacy-episode".to_string();
    write_jsonl(layout.learn_dir().join("episodes.jsonl"), &[legacy]);

    let runtime = LearningRuntime::open_for_project(tmp.path())
        .await
        .expect("open project runtime");
    let mut appended = sample_episode(false);
    appended.id = "new-episode".to_string();
    appended.episode_id = "new-episode".to_string();
    runtime
        .append_episode(&appended)
        .await
        .expect("append episode");

    assert_eq!(
        layout.read_version().await.expect("read version"),
        Some(roko_fs::LayoutVersion::V3)
    );
    let episodes = EpisodeLogger::read_all_lossy(&layout.episodes_path())
        .await
        .expect("read canonical episodes");
    assert_eq!(episodes.len(), 2);
    assert_eq!(episodes[0].episode_id, "legacy-episode");
    assert_eq!(episodes[1].episode_id, "new-episode");
    assert!(!layout.learn_dir().join("episodes.jsonl").exists());
    assert!(layout.learn_dir().join("episodes.jsonl.v2-legacy").exists());
}

fn episode_at(task_id: &str, minutes_ago: i64, success: bool) -> Episode {
    let mut ep = sample_episode(success);
    ep.id = format!("{task_id}-id");
    ep.episode_id = task_id.to_string();
    ep.task_id = task_id.to_string();
    ep.timestamp = Utc::now() - chrono::Duration::minutes(minutes_ago);
    ep
}

fn episode_with_agent(task_id: &str, minutes_ago: i64, success: bool, agent_id: &str) -> Episode {
    let mut ep = episode_at(task_id, minutes_ago, success);
    ep.agent_id = agent_id.to_string();
    ep
}

fn sample_metric(i: u32, passed: bool, cost: f64) -> TaskMetric {
    let mut m = TaskMetric::new(ConfigHash::from("cfg-1".to_string()), "plan-1", "task-1");
    m.timestamp = "2026-04-08T00:00:00Z".to_string();
    m.run_id = format!("run-{i}");
    m.iteration = i;
    m.role = "Implementer".to_string();
    m.backend = "claude".to_string();
    m.model = "claude-opus-4-6".to_string();
    m.complexity_band = "standard".to_string();
    m.gate = "compile".to_string();
    m.gate_passed = passed;
    m.wall_time_ms = 1000 + u64::from(i);
    m.input_tokens = 100;
    m.output_tokens = 20;
    m.cached_tokens = 0;
    m.cost_usd = cost;
    m.sections_included = 3;
    m.sections_dropped = 0;
    m.context_tokens = 400;
    m.cache_hit_rate = 0.0;
    m
}

fn write_jsonl<T: Serialize>(path: impl AsRef<Path>, values: &[T]) {
    let mut contents = String::new();
    for value in values {
        contents.push_str(&serde_json::to_string(value).unwrap());
        contents.push('\n');
    }
    std::fs::write(path, contents).unwrap();
}

fn sample_pattern_episode(success: bool, suffix: &str) -> Episode {
    let mut ep = sample_episode(success);
    ep.id = format!("episode-{suffix}");
    ep.episode_id = format!("episode-{suffix}");
    ep.task_id = format!("task-{suffix}");
    ep.gate_verdicts = vec![
        crate::episode_logger::EpisodeGateVerdict::new("read", true),
        crate::episode_logger::EpisodeGateVerdict::new("edit", true),
        crate::episode_logger::EpisodeGateVerdict::new("test", true),
    ];
    ep.extra.insert(
        "task_tags".to_string(),
        serde_json::json!(["rust", format!("tag-{suffix}")]),
    );
    ep.extra.insert(
        "files".to_string(),
        serde_json::json!([format!("crates/roko-cli/src/{suffix}.rs")]),
    );
    ep
}

#[tokio::test]
async fn completed_run_updates_episode_cost_provider_and_skill() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    let mut freq = *runtime.update_frequency();
    freq.skill_mining_every_n = 1;
    runtime.set_update_frequency(freq);

    let mut episode = sample_episode(true);
    episode.backend = "claude_cli".to_string();
    let input = CompletedRunInput::from_episode(episode);
    let update = runtime.record_completed_run(input).await.unwrap();

    assert_eq!(update.episode_logged, ApplyStatus::Applied);
    assert_eq!(update.cost_logged, ApplyStatus::Applied);
    assert_eq!(update.provider_updated, ApplyStatus::Applied);
    assert_eq!(update.provider_model_outcome_recorded, ApplyStatus::Applied);
    assert!(update.extracted_skill_id.is_some());
    assert_eq!(runtime.costs_db().len(), 1);
    let pass_rates = runtime.provider_model_pass_rates(25).await.unwrap();
    assert_eq!(pass_rates.total_records, 1);
    assert_eq!(
        pass_rates.actions[0].action_id,
        "provider:anthropic|model:claude-opus-4-6"
    );
    assert_eq!(pass_rates.actions[0].successes, 1);
    assert_eq!(pass_rates.actions[0].task_types, vec!["bugfix"]);

    let episodes_jsonl = std::fs::read_to_string(&runtime.paths().episodes_jsonl).unwrap();
    let persisted: Episode =
        serde_json::from_str(episodes_jsonl.lines().next().unwrap()).expect("persisted episode");
    assert_eq!(persisted.backend, "claude_cli");
    let pad = persisted
        .extra
        .get("pad")
        .and_then(serde_json::Value::as_object)
        .expect("pad signature");
    assert!(pad.contains_key("pleasure"));
    assert!(pad.contains_key("arousal"));
    assert!(pad.contains_key("dominance"));
}

#[tokio::test]
async fn skipped_only_gate_runs_are_blocked_not_passed_and_do_not_update_learning() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_with_models(
        LearningPaths::under(tmp.path()),
        RegressionConfig::default(),
        vec!["claude-opus-4-6".to_string()],
    )
    .await
    .unwrap();
    runtime.set_update_frequency(UpdateFrequency {
        router_every_n_episodes: 1,
        gate_thresholds_every_n: 1,
        experiments_every_n: 1,
        skill_mining_every_n: 1,
        pattern_discovery_every_n: 1,
        distiller_every_n: 1,
    });

    let mut experiment = PromptExperiment::new(
        "skip-only-exp",
        "model-routing",
        vec![PromptVariant {
            id: "blocked".to_string(),
            name: "Blocked".to_string(),
            section_name: "model-routing".to_string(),
            content: String::new(),
            slug: Some("claude-opus-4-6".to_string()),
            active: true,
        }],
    );
    experiment.min_trials_per_variant = 100;
    runtime.experiment_store().lock().register(experiment);

    let mut input = CompletedRunInput::from_episode(skipped_only_episode())
        .with_task_metric(sample_metric(1, true, 0.42));
    input.provider = Some("anthropic".to_string());
    input.playbook_id = Some("playbook-skip-only".to_string());
    input.playbook_rule_id = Some("rule-skip-only".to_string());
    input.matched_skill_id = Some("skill-skip-only".to_string());
    input.experiment_variant_id = Some("blocked".to_string());

    let update = runtime.record_completed_run(input).await.unwrap();

    assert_eq!(update.episode_logged, ApplyStatus::Applied);
    assert_eq!(update.cost_logged, ApplyStatus::Applied);
    assert_eq!(update.provider_model_outcome_recorded, ApplyStatus::Applied);
    assert_eq!(update.efficiency_summary_recorded, ApplyStatus::Applied);
    assert_eq!(update.gate_outcomes_recorded, 0);
    assert_eq!(update.provider_updated, ApplyStatus::Skipped);
    assert_eq!(update.playbook_updated, ApplyStatus::Skipped);
    assert_eq!(update.playbook_rule_updated, ApplyStatus::Skipped);
    assert_eq!(update.matched_skill_updated, ApplyStatus::Skipped);
    assert_eq!(update.reflection_recorded, ApplyStatus::Skipped);
    assert_eq!(update.reflection_candidate_updated, ApplyStatus::Skipped);
    assert_eq!(update.knowledge_seed_recorded, ApplyStatus::Skipped);
    assert_eq!(update.router_updated, false);
    assert!(update.extracted_skill_id.is_none());
    assert!(update.regression_report.is_none());
    assert!(!update.patterns_ingested);

    assert_eq!(runtime.local_reward_score("router", "claude-opus-4-6"), 0.5);
    assert_eq!(runtime.local_reward_score("skill", "skill-skip-only"), 0.5);
    assert_eq!(
        runtime.local_reward_score("playbook_rule", "rule-skip-only"),
        0.5
    );
    assert_eq!(runtime.cascade_router().total_observations(), 0);
    assert_eq!(runtime.skill_library().len(), 0);
    assert_eq!(runtime.pattern_miner().lock().total_episodes(), 0);
    assert!(!runtime.paths().cfactor_jsonl.exists());
    assert!(!runtime.paths().task_metrics_jsonl.exists());

    let persisted = std::fs::read_to_string(&runtime.paths().episodes_jsonl).unwrap();
    let episode: Episode = serde_json::from_str(persisted.lines().next().unwrap())
        .expect("persisted skip-only episode");
    assert!(!episode.success);
    assert_eq!(
        episode
            .failure_reason
            .as_deref()
            .expect("skip-only failure reason"),
        "all gates skipped"
    );
    assert_eq!(
        episode
            .extra
            .get("provider_model_outcome_status")
            .and_then(serde_json::Value::as_str),
        Some("blocked")
    );
    assert_eq!(
        episode
            .extra
            .get("gate_summary")
            .and_then(serde_json::Value::as_str),
        Some("0 passed, 0 failed, 3 skipped")
    );
    assert_eq!(
        episode
            .extra
            .get("gate_counts")
            .and_then(serde_json::Value::as_object)
            .and_then(|counts| counts.get("skipped"))
            .and_then(serde_json::Value::as_u64),
        Some(3)
    );
    assert_eq!(
        episode
            .extra
            .get("gate_pass_rate")
            .and_then(serde_json::Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        gate_counts_from_episode(&episode)
            .expect("gate counts")
            .summary(),
        "0 passed, 0 failed, 3 skipped"
    );
    assert!(
        EfficiencySummaryRecord::from_episode(&episode)
            .gate_passed
            .is_none()
    );
    assert!(GateOutcomeRecord::from_episode(&episode).is_empty());
    assert!(KnowledgeSeedRecord::from_successful_episode(&episode).is_none());

    let experiment_store = runtime.experiment_store().lock();
    let variant_trials = experiment_store
        .get("skip-only-exp")
        .and_then(|exp| exp.stats.get("blocked"))
        .map(|stats| stats.trials);
    assert_eq!(variant_trials, Some(0));
}

#[tokio::test]
async fn append_efficiency_event_updates_section_effectiveness_registry() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    let event = AgentEfficiencyEvent {
        role: "Implementer".to_string(),
        gate_passed: Some(true),
        prompt_sections: vec![
            crate::efficiency::PromptSectionMeta {
                name: "workspace_map".to_string(),
                tokens: 120,
                priority: 2,
                was_truncated: false,
                was_dropped: false,
            },
            crate::efficiency::PromptSectionMeta {
                name: "playbook".to_string(),
                tokens: 0,
                priority: 0,
                was_truncated: false,
                was_dropped: true,
            },
        ],
        ..AgentEfficiencyEvent::default()
    };

    runtime.append_efficiency_event(&event).await.unwrap();

    let snapshot = runtime.section_effectiveness_snapshot();
    let included = snapshot
        .get("workspace_map", "Implementer")
        .expect("included section recorded");
    assert_eq!(included.included_trials, 1);
    assert_eq!(included.included_passes, 1);

    let excluded = snapshot
        .get("playbook", "Implementer")
        .expect("dropped section recorded");
    assert_eq!(excluded.excluded_trials, 1);
    assert_eq!(excluded.excluded_passes, 1);
    assert!(runtime.paths().section_effects_json.exists());
}

#[tokio::test]
async fn project_learning_snapshot_reads_episode_efficiency_router_and_knowledge_artifacts() {
    let tmp = TempDir::new().unwrap();
    let workdir = tmp.path();
    let roko = workdir.join(".roko");
    let memory_dir = roko.join("memory");
    let learn_dir = roko.join("learn");
    let neuro_dir = roko.join("neuro");
    std::fs::create_dir_all(&memory_dir).unwrap();
    std::fs::create_dir_all(&learn_dir).unwrap();
    std::fs::create_dir_all(&neuro_dir).unwrap();

    let mut memory_episode = sample_episode(true);
    memory_episode.id = "episode-memory".to_string();
    memory_episode.episode_id = "episode-memory".to_string();
    memory_episode.task_id = "task-memory".to_string();
    let duplicate_episode = memory_episode.clone();
    let mut legacy_episode = sample_episode(false);
    legacy_episode.id = "episode-legacy".to_string();
    legacy_episode.episode_id = "episode-legacy".to_string();
    legacy_episode.task_id = "task-legacy".to_string();

    write_jsonl(memory_dir.join("episodes.jsonl"), &[memory_episode]);
    write_jsonl(
        roko.join("episodes.jsonl"),
        &[duplicate_episode, legacy_episode],
    );

    let efficiency_event = AgentEfficiencyEvent {
        agent_id: "agent-1".to_string(),
        model: "claude-sonnet-4-5".to_string(),
        model_used: "claude-sonnet-4-5".to_string(),
        gate_passed: Some(true),
        cost_usd: 0.25,
        ..AgentEfficiencyEvent::default()
    };
    write_jsonl(learn_dir.join("efficiency.jsonl"), &[efficiency_event]);
    std::fs::write(
        learn_dir.join("cascade-router.json"),
        serde_json::json!({"confidence_stats": {"claude-sonnet-4-5": {"trials": 1}}}).to_string(),
    )
    .unwrap();
    std::fs::write(
        neuro_dir.join("knowledge.jsonl"),
        "{\"id\":\"k1\"}\n{\"id\":\"k2\"}\n",
    )
    .unwrap();

    let snapshot = read_project_learning_snapshot(workdir).await.unwrap();

    assert_eq!(
        snapshot.episodes.len(),
        2,
        "duplicate episode should be skipped"
    );
    assert_eq!(snapshot.efficiency_events.len(), 1);
    assert_eq!(snapshot.knowledge_entries, 2);
    assert!(snapshot.cascade_router.is_some());
    assert_eq!(snapshot.episode_paths, vec![roko.join("episodes.jsonl")]);
}

#[tokio::test]
async fn completed_runs_append_cfactor_history() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    let mut freq = *runtime.update_frequency();
    freq.distiller_every_n = 1;
    runtime.set_update_frequency(freq);

    runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_episode(true)))
        .await
        .unwrap();
    // Use a different task_id so both episodes survive deduplicate_episodes.
    // Dedup key is "{plan_id}:{task_id}:{attempt}"; same key → second episode
    // replaces the first, causing the 2nd snapshot to report episode_count=1.
    runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_pattern_episode(
            true, "2",
        )))
        .await
        .unwrap();

    let cfactor_jsonl = std::fs::read_to_string(&runtime.paths().cfactor_jsonl).unwrap();
    let snapshots: Vec<crate::cfactor::CFactor> = cfactor_jsonl
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid c-factor snapshot"))
        .collect();

    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].episode_count, 1);
    assert_eq!(snapshots[1].episode_count, 2);
}

#[tokio::test]
async fn update_frequency_separation() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_with_models(
        LearningPaths::under(tmp.path()),
        RegressionConfig::default(),
        vec!["claude-opus-4-6".to_string()],
    )
    .await
    .unwrap();
    runtime.set_update_frequency(UpdateFrequency {
        router_every_n_episodes: 2,
        gate_thresholds_every_n: 5,
        experiments_every_n: 3,
        skill_mining_every_n: 2,
        pattern_discovery_every_n: 3,
        distiller_every_n: 4,
    });

    let mut experiment = PromptExperiment::new(
        "cadence-exp",
        "model-routing",
        vec![PromptVariant {
            id: "cadence".to_string(),
            name: "Cadence".to_string(),
            section_name: "model-routing".to_string(),
            content: String::new(),
            slug: Some("claude-opus-4-6".to_string()),
            active: true,
        }],
    );
    experiment.min_trials_per_variant = 100;
    runtime.experiment_store().lock().register(experiment);

    let update = runtime
        .record_completed_run(CompletedRunInput {
            experiment_variant_id: Some("cadence".to_string()),
            ..CompletedRunInput::from_episode(sample_pattern_episode(true, "one"))
        })
        .await
        .unwrap();
    assert!(!update.router_updated);
    assert!(update.extracted_skill_id.is_none());
    assert!(!update.patterns_ingested);
    assert_eq!(runtime.cascade_router().total_observations(), 0);
    assert_eq!(runtime.skill_library().len(), 0);
    assert_eq!(runtime.pattern_miner().lock().total_episodes(), 0);
    assert!(!runtime.paths().cfactor_jsonl.exists());
    assert_eq!(
        runtime
            .experiment_store()
            .lock()
            .get("cadence-exp")
            .and_then(|exp| exp.stats.get("cadence"))
            .map(|stats| stats.trials),
        Some(0)
    );

    let update = runtime
        .record_completed_run(CompletedRunInput {
            experiment_variant_id: Some("cadence".to_string()),
            ..CompletedRunInput::from_episode(sample_pattern_episode(true, "two"))
        })
        .await
        .unwrap();
    assert!(update.router_updated);
    assert!(update.extracted_skill_id.is_some());
    assert!(!update.patterns_ingested);
    assert_eq!(runtime.cascade_router().total_observations(), 1);
    assert_eq!(runtime.skill_library().len(), 1);
    assert_eq!(runtime.pattern_miner().lock().total_episodes(), 0);
    assert!(!runtime.paths().cfactor_jsonl.exists());
    assert_eq!(
        runtime
            .experiment_store()
            .lock()
            .get("cadence-exp")
            .and_then(|exp| exp.stats.get("cadence"))
            .map(|stats| stats.trials),
        Some(0)
    );

    let update = runtime
        .record_completed_run(CompletedRunInput {
            experiment_variant_id: Some("cadence".to_string()),
            ..CompletedRunInput::from_episode(sample_pattern_episode(true, "three"))
        })
        .await
        .unwrap();
    assert!(!update.router_updated);
    assert!(update.extracted_skill_id.is_none());
    assert!(update.patterns_ingested);
    assert_eq!(runtime.cascade_router().total_observations(), 1);
    assert_eq!(runtime.skill_library().len(), 1);
    assert_eq!(runtime.pattern_miner().lock().total_episodes(), 1);
    assert!(!runtime.paths().cfactor_jsonl.exists());
    assert_eq!(
        runtime
            .experiment_store()
            .lock()
            .get("cadence-exp")
            .and_then(|exp| exp.stats.get("cadence"))
            .map(|stats| stats.trials),
        Some(1)
    );

    let update = runtime
        .record_completed_run(CompletedRunInput {
            experiment_variant_id: Some("cadence".to_string()),
            ..CompletedRunInput::from_episode(sample_pattern_episode(true, "four"))
        })
        .await
        .unwrap();
    assert!(update.router_updated);
    assert!(update.extracted_skill_id.is_some());
    assert!(!update.patterns_ingested);
    assert_eq!(runtime.cascade_router().total_observations(), 2);
    assert_eq!(runtime.skill_library().len(), 2);
    assert_eq!(runtime.pattern_miner().lock().total_episodes(), 1);
    let cfactor_jsonl = std::fs::read_to_string(&runtime.paths().cfactor_jsonl).unwrap();
    let snapshots: Vec<crate::cfactor::CFactor> = cfactor_jsonl
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid c-factor snapshot"))
        .collect();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].episode_count, 4);
    assert_eq!(
        runtime
            .experiment_store()
            .lock()
            .get("cadence-exp")
            .and_then(|exp| exp.stats.get("cadence"))
            .map(|stats| stats.trials),
        Some(1)
    );
}

#[test]
fn social_perceptiveness_uses_prior_output_attributions() {
    let now = Utc::now();
    let records = vec![
        ContextAttributionRecord {
            ts: now,
            source_type: "prior_output".to_string(),
            referenced: true,
        },
        ContextAttributionRecord {
            ts: now,
            source_type: "prior_output".to_string(),
            referenced: false,
        },
        ContextAttributionRecord {
            ts: now,
            source_type: "file".to_string(),
            referenced: true,
        },
    ];

    let score = social_perceptiveness_from_attribution(&records, Duration::from_secs(60));
    assert!((score - 0.5).abs() < 1e-9);
}

#[test]
fn knowledge_integration_rate_uses_confirmation_chains() {
    let episodes = vec![
        episode_at("task-1", 5, true),
        episode_at("task-2", 4, true),
        episode_at("task-3", 3, true),
    ];
    let records = vec![
        KnowledgeConfirmationRecord {
            created_at: Utc::now(),
            source_episodes: vec!["task-1".to_string(), "task-2".to_string()],
        },
        KnowledgeConfirmationRecord {
            created_at: Utc::now(),
            source_episodes: vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string(),
            ],
        },
        KnowledgeConfirmationRecord {
            created_at: Utc::now(),
            source_episodes: vec!["task-1".to_string()],
        },
    ];

    let score = knowledge_integration_rate(&records, &episodes, Duration::from_secs(60));
    assert!(score > 0.0);
    assert!(score <= 1.0);
}

#[test]
fn convergence_velocity_uses_agreement_across_agents() {
    let episodes = vec![
        episode_with_agent("task-1", 6, true, "agent-a"),
        episode_with_agent("task-2", 4, true, "agent-b"),
        episode_with_agent("task-3", 2, true, "agent-c"),
    ];
    let records = vec![
        KnowledgeConfirmationRecord {
            created_at: Utc::now(),
            source_episodes: vec!["task-1".to_string(), "task-2".to_string()],
        },
        KnowledgeConfirmationRecord {
            created_at: Utc::now(),
            source_episodes: vec![
                "task-1".to_string(),
                "task-2".to_string(),
                "task-3".to_string(),
            ],
        },
    ];

    let score = convergence_velocity_from_agreement(&records, &episodes, Duration::from_secs(60));
    assert!(score > 0.0);
    assert!(score <= 1.0);
}

#[tokio::test]
async fn open_under_loads_persisted_cascade_router_state() {
    let tmp = TempDir::new().unwrap();
    let learn_root = tmp.path().join(".roko").join("learn");
    let paths = LearningPaths::under(&learn_root);

    let router = CascadeRouter::new(vec![
        "claude-sonnet-4-20250514".to_string(),
        "claude-haiku-4-5-20251001".to_string(),
    ]);
    let ctx = RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role: roko_core::agent::AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy: DaimonPolicy::default(),
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    };
    for _ in 0..60 {
        router.record_observation(&ctx, "claude-sonnet-4-20250514", 0.9, true);
    }
    router.save(&paths.cascade_router_json).unwrap();

    let runtime = LearningRuntime::open_under(&learn_root).await.unwrap();
    let loaded_router = runtime.cascade_router();

    assert_eq!(loaded_router.total_observations(), 60);
    assert_eq!(
        loaded_router.current_stage(),
        crate::cascade_router::CascadeStage::Confidence
    );
    let routed = loaded_router.route(&ctx);
    assert_eq!(
        routed.stage,
        crate::cascade_router::CascadeStage::Confidence
    );
}

#[tokio::test]
async fn record_completed_run_persists_cascade_router_immediately() {
    let tmp = TempDir::new().unwrap();
    let learn_root = tmp.path().join(".roko").join("learn");
    let runtime = LearningRuntime::open_under(&learn_root).await.unwrap();
    let router_path = learn_root.join("cascade-router.json");
    assert!(
        !router_path.exists(),
        "router file should not exist before observation"
    );

    let mut ep = sample_episode(true);
    ep.extra
        .insert("model".to_string(), serde_json::json!("claude-sonnet-4-5"));

    let update = runtime
        .record_completed_run(CompletedRunInput::from_episode(ep))
        .await
        .unwrap();

    assert!(
        update.router_updated,
        "completed run should update cascade router"
    );
    assert!(
        router_path.exists(),
        "router file should be written after observation"
    );

    let contents = std::fs::read_to_string(&router_path).unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&contents).unwrap();
    let stats = snapshot
        .get("confidence_stats")
        .and_then(serde_json::Value::as_object)
        .expect("confidence stats should be persisted");
    let sonnet = stats
        .get("claude-sonnet-4-5")
        .and_then(serde_json::Value::as_object)
        .expect("sonnet observation should be persisted");
    assert_eq!(
        sonnet.get("trials").and_then(serde_json::Value::as_u64),
        Some(1),
        "persisted router should reflect the new observation"
    );
    assert_eq!(
        sonnet.get("successes").and_then(serde_json::Value::as_u64),
        Some(1),
        "persisted router should reflect the successful observation"
    );
}

#[tokio::test]
async fn runtime_experiment_transaction_scopes_duplicate_variant_ids_and_preserves_disk() {
    let tmp = TempDir::new().unwrap();
    let paths = LearningPaths::under(tmp.path());
    let runtime = LearningRuntime::open(paths.clone(), RegressionConfig::default())
        .await
        .unwrap();
    let variant = |section: &str| {
        vec![PromptVariant {
            id: "shared".to_string(),
            name: "Shared".to_string(),
            section_name: section.to_string(),
            content: "Scoped content".to_string(),
            slug: None,
            active: true,
        }]
    };
    runtime
        .experiment_store()
        .lock()
        .register(PromptExperiment::new(
            "exp-a",
            "section-a",
            variant("section-a"),
        ));
    runtime
        .experiment_store()
        .lock()
        .register(PromptExperiment::new(
            "exp-b",
            "section-b",
            variant("section-b"),
        ));

    ExperimentStore::transaction(&paths.experiments_json, |store| {
        store.register(PromptExperiment::new(
            "external-exp",
            "external-section",
            variant("external-section"),
        ));
        Ok(())
    })
    .expect("commit concurrent external experiment");

    runtime
        .record_completed_run(
            CompletedRunInput::from_episode(sample_episode(true))
                .with_experiment_assignment("exp-b", "shared"),
        )
        .await
        .unwrap();

    let committed = ExperimentStore::load_or_new(&paths.experiments_json);
    assert_eq!(committed.get("exp-a").unwrap().stats["shared"].trials, 0);
    assert_eq!(committed.get("exp-b").unwrap().stats["shared"].trials, 1);
    assert!(committed.get("external-exp").is_some());
    let cached = runtime.experiment_store().lock();
    assert!(cached.get("external-exp").is_some());
    assert_eq!(cached.get("exp-b").unwrap().stats["shared"].trials, 1);
}

#[tokio::test]
async fn experiment_updates_static_table() {
    let tmp = TempDir::new().unwrap();
    let learn_root = tmp.path().join(".roko").join("learn");
    let paths = LearningPaths::under(&learn_root);
    let runtime = LearningRuntime::open_with_models(
        paths.clone(),
        RegressionConfig::default(),
        vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-haiku-4-5-20251001".to_string(),
        ],
    )
    .await
    .unwrap();

    let mut experiment = PromptExperiment::new(
        "model-routing-exp",
        "model-routing",
        vec![
            PromptVariant {
                id: "sonnet".to_string(),
                name: "Sonnet".to_string(),
                section_name: "model-routing".to_string(),
                content: String::new(),
                slug: Some("claude-sonnet-4-20250514".to_string()),
                active: true,
            },
            PromptVariant {
                id: "haiku".to_string(),
                name: "Haiku".to_string(),
                section_name: "model-routing".to_string(),
                content: String::new(),
                slug: Some("claude-haiku-4-5-20251001".to_string()),
                active: true,
            },
        ],
    );
    experiment.role = Some("implementer".to_string());
    experiment.min_trials_per_variant = 5;
    experiment.min_effect_size = 0.5;
    runtime.experiment_store().lock().register(experiment);

    let mut before_ctx = RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role: AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy: DaimonPolicy::default(),
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    };
    assert_eq!(
        runtime.cascade_router().route(&before_ctx).primary.slug,
        "claude-sonnet-4-20250514"
    );

    for _ in 0..5 {
        let mut losing_episode = sample_episode(false);
        losing_episode.extra.insert(
            "model".to_string(),
            serde_json::json!("claude-sonnet-4-20250514"),
        );
        runtime
            .record_completed_run(CompletedRunInput {
                experiment_variant_id: Some("sonnet".to_string()),
                ..CompletedRunInput::from_episode(losing_episode)
            })
            .await
            .unwrap();

        let mut winning_episode = sample_episode(true);
        winning_episode.extra.insert(
            "model".to_string(),
            serde_json::json!("claude-haiku-4-5-20251001"),
        );
        runtime
            .record_completed_run(CompletedRunInput {
                experiment_variant_id: Some("haiku".to_string()),
                ..CompletedRunInput::from_episode(winning_episode)
            })
            .await
            .unwrap();
    }

    let winner_artifact = std::fs::read_to_string(&runtime.paths().experiment_winners_json)
        .expect("experiment winners artifact");
    let winner_summaries: Vec<roko_core::ExperimentWinnerSummary> =
        serde_json::from_str(&winner_artifact).expect("winner summary json");
    assert_eq!(winner_summaries.len(), 1);
    assert_eq!(winner_summaries[0].experiment_id, "model-routing-exp");
    assert_eq!(winner_summaries[0].winner_variant_id, "haiku");

    before_ctx.iteration = 1;
    assert_eq!(
        runtime.cascade_router().route(&before_ctx).primary.slug,
        "claude-haiku-4-5-20251001"
    );

    let reloaded = LearningRuntime::open_with_models(
        paths,
        RegressionConfig::default(),
        vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-haiku-4-5-20251001".to_string(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        reloaded.cascade_router().route(&before_ctx).primary.slug,
        "claude-haiku-4-5-20251001"
    );
}

#[tokio::test]
async fn completed_run_updates_playbook_and_rule_outcomes() {
    let tmp = TempDir::new().unwrap();
    let paths = LearningPaths::under(tmp.path());
    let runtime = LearningRuntime::open(paths.clone(), RegressionConfig::default())
        .await
        .unwrap();

    let mut pb = crate::playbook::Playbook::new("pb-1", "goal");
    pb.steps.push(crate::playbook::PlaybookStep::new(
        0,
        "step",
        "edit_file",
        vec!["signal".to_string()],
    ));
    runtime.playbook_store.save(&pb).await.unwrap();

    let mut rule = crate::playbook_rules::Rule {
        rule_id: "r-1".to_string(),
        title: "title".to_string(),
        body: "body".to_string(),
        triggers: crate::playbook_rules::Triggers {
            tags: vec!["rust".to_string()],
            ..Default::default()
        },
        confidence: 0.5,
        validations: 0,
        contradictions: 0,
        last_applied: None,
        created_at: Utc::now(),
        source_episodes: vec![],
        balance: 1.0,
        demurrage_rate: 0.01,
        last_decay_at_ms: Utc::now().timestamp_millis(),
    };
    runtime.playbook_rules.upsert(rule.clone()).unwrap();
    runtime.playbook_rules.save().unwrap();

    let mut ep = sample_episode(false);
    ep.extra
        .insert("playbook_id".to_string(), serde_json::json!("pb-1"));
    ep.extra
        .insert("playbook_rule_id".to_string(), serde_json::json!("r-1"));
    let update = runtime
        .record_completed_run(CompletedRunInput::from_episode(ep))
        .await
        .unwrap();

    assert_eq!(update.playbook_updated, ApplyStatus::Applied);
    assert_eq!(update.playbook_rule_updated, ApplyStatus::Applied);

    let loaded_pb = runtime.playbook_store.load("pb-1").await.unwrap().unwrap();
    assert_eq!(loaded_pb.failure_count, 1);

    let rules = runtime.playbook_rules.snapshot();
    rule = rules.into_iter().find(|r| r.rule_id == "r-1").unwrap();
    assert_eq!(rule.contradictions, 1);
}

#[tokio::test]
async fn completed_gate_run_persists_post_gate_reflection() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    let mut ep = sample_episode(false);
    ep.gate_verdicts.push(
        crate::episode_logger::EpisodeGateVerdict::new("compile", false).with_signature("E0308"),
    );
    ep.reflection =
        Some("Fix crates/roko-learn/src/lib.rs before retrying E0308 type_mismatch".to_string());

    let update = runtime
        .record_completed_run(CompletedRunInput::from_episode(ep))
        .await
        .unwrap();

    assert_eq!(update.reflection_recorded, ApplyStatus::Applied);
    assert_eq!(update.reflection_candidate_updated, ApplyStatus::Applied);
    let store = PostGateReflectionStore::load(&runtime.paths().post_gate_reflections_json);
    assert_eq!(store.records.len(), 1);
    assert_eq!(store.candidates.len(), 1);
}

#[tokio::test]
async fn completed_run_emits_regression_report_when_enough_metrics() {
    let tmp = TempDir::new().unwrap();
    let cfg = RegressionConfig {
        thresholds: RegressionThresholds {
            min_records: 2,
            pass_rate_drop: 0.1,
            cost_increase: 0.1,
            duration_increase: 0.1,
            iterations_increase: 0.1,
        },
        current_window: 2,
    };
    let runtime = LearningRuntime::open(LearningPaths::under(tmp.path()), cfg)
        .await
        .unwrap();

    // Baseline: good + cheap.
    for i in 1..=2_u32 {
        let input = CompletedRunInput::from_episode(sample_episode(true))
            .with_task_metric(sample_metric(i, true, 0.1));
        let update = runtime.record_completed_run(input).await.unwrap();
        assert!(update.regression_report.is_none());
    }

    // Current window: worse + expensive.
    let update = runtime
        .record_completed_run(
            CompletedRunInput::from_episode(sample_episode(false))
                .with_task_metric(sample_metric(3, false, 1.0)),
        )
        .await
        .unwrap();
    assert!(update.regression_report.is_none());

    let update = runtime
        .record_completed_run(
            CompletedRunInput::from_episode(sample_episode(false))
                .with_task_metric(sample_metric(4, false, 1.1)),
        )
        .await
        .unwrap();
    let report = update.regression_report.expect("regression report");
    assert!(report.sufficient_data);
    assert!(!report.alerts.is_empty());
}

#[tokio::test]
async fn health_filters_routing() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_with_models(
        LearningPaths::under(tmp.path()),
        RegressionConfig::default(),
        vec![
            "moonshot-fast".to_string(),
            "moonshot-premium".to_string(),
            "anthropic-safe".to_string(),
        ],
    )
    .await
    .unwrap();

    for _ in 0..3 {
        runtime.provider_health().record_failure("moonshot");
    }

    let all_model_slugs = runtime
        .cascade_router()
        .linucb()
        .arm_stats()
        .into_iter()
        .map(|arm| arm.slug)
        .collect::<Vec<_>>();
    let healthy_models = runtime.healthy_model_slugs(&all_model_slugs, |model_slug| {
        if model_slug.starts_with("moonshot") {
            "moonshot".to_string()
        } else {
            "anthropic".to_string()
        }
    });

    assert_eq!(healthy_models, vec!["anthropic-safe".to_string()]);

    let ctx = RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role: AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy: DaimonPolicy::default(),
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    };
    let selected = runtime
        .cascade_router()
        .select_for_frequency_among(
            roko_core::OperatingFrequency::Theta,
            Some(&ctx),
            None,
            Some("Implementer"),
            &healthy_models,
        )
        .expect("theta should route to a healthy model");

    assert_eq!(selected.slug, "anthropic-safe");
}

#[tokio::test]
async fn conductor_negative_feedback_records_failed_router_observation() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_with_models(
        LearningPaths::under(tmp.path()),
        RegressionConfig::default(),
        vec!["claude-opus-4-6".to_string()],
    )
    .await
    .unwrap();

    let routing_context = RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration: 1,
        role: AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy: DaimonPolicy::default(),
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    };

    let recorded = runtime.record_conductor_intervention(
        &routing_context,
        "claude-opus-4-6",
        &ConductorDecision::restart("stuck-pattern", "repeated output"),
    );
    assert!(recorded);

    let stats = runtime.cascade_router().observation_snapshot();
    let opus = stats.get("claude-opus-4-6").expect("router stats");
    assert_eq!(opus.trials, 1);
    assert_eq!(opus.successes, 0);

    let reloaded = LearningRuntime::open_with_models(
        LearningPaths::under(tmp.path()),
        RegressionConfig::default(),
        vec!["claude-opus-4-6".to_string()],
    )
    .await
    .unwrap();
    let persisted = reloaded.cascade_router().observation_snapshot();
    let opus = persisted
        .get("claude-opus-4-6")
        .expect("persisted router stats");
    assert_eq!(opus.trials, 1);
    assert_eq!(opus.successes, 0);
}

#[tokio::test]
async fn latency_aware_reward_prefers_faster_models() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();

    let event = AgentEfficiencyEvent {
        backend: "anthropic".to_string(),
        model: "claude-opus-4-6".to_string(),
        model_used: "claude-opus-4-6".to_string(),
        wall_time_ms: 1_000,
        duration_ms: 1_000,
        output_tokens: 128,
        ..AgentEfficiencyEvent::default()
    };
    runtime.append_efficiency_event(&event).await.unwrap();

    let faster = runtime.compute_routing_reward_with_latency(
        true,
        0.25,
        1_000,
        "claude-opus-4-6",
        "anthropic",
    );
    let slower = runtime.compute_routing_reward_with_latency(
        true,
        0.25,
        60_000,
        "claude-opus-4-6",
        "anthropic",
    );

    assert!(faster > slower, "faster={faster}, slower={slower}");
}

#[tokio::test]
async fn latency_aware_reward_uses_latency_registry_fallback() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();

    for wall_time_ms in [10_000_u64, 20_000, 30_000] {
        let event = AgentEfficiencyEvent {
            backend: "anthropic".to_string(),
            model: "claude-opus-4-6".to_string(),
            model_used: "claude-opus-4-6".to_string(),
            wall_time_ms,
            duration_ms: wall_time_ms,
            output_tokens: 64,
            ..AgentEfficiencyEvent::default()
        };
        runtime.append_efficiency_event(&event).await.unwrap();
    }

    let reloaded = LearningRuntime::open_under(tmp.path()).await.unwrap();
    let stats = reloaded
        .latency_registry()
        .get("claude-opus-4-6", "anthropic")
        .expect("latency stats");
    assert_eq!(stats.p50_ms(), 20_000.0);

    let reward =
        reloaded.compute_routing_reward_with_latency(true, 0.25, 0, "claude-opus-4-6", "anthropic");
    let expected = compute_routing_reward_v2(1.0, 0.25_f64 / 5.0, 20_000.0, 120_000.0);
    assert!((reward - expected).abs() < 1e-9);
}

#[tokio::test]
async fn local_reward_functions_observe_and_persist_across_runs() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_with_models(
        LearningPaths::under(tmp.path()),
        RegressionConfig::default(),
        vec!["claude-opus-4-6".to_string()],
    )
    .await
    .unwrap();

    // Before any observations, unknown decisions return the 0.5 prior.
    assert_eq!(runtime.local_reward_score("router", "claude-opus-4-6"), 0.5);

    // Record a successful run with model + skill metadata.
    let mut ep = sample_pattern_episode(true, "local-reward-test");
    ep.extra
        .insert("model".to_string(), serde_json::json!("claude-opus-4-6"));
    let mut input = CompletedRunInput::from_episode(ep);
    input.matched_skill_id = Some("rust-impl".to_string());
    input.playbook_rule_id = Some("rule-001".to_string());
    runtime.record_completed_run(input).await.unwrap();

    // Route subsystem should have observed the model decision.
    assert_eq!(
        runtime.local_reward_score("router", "claude-opus-4-6"),
        1.0,
        "single success should give 1.0"
    );
    // Skill subsystem should track the matched skill.
    assert_eq!(runtime.local_reward_score("skill", "rust-impl"), 1.0);
    // Playbook rule subsystem.
    assert_eq!(runtime.local_reward_score("playbook_rule", "rule-001"), 1.0);

    // Record a failed run for the same model.
    let mut ep2 = sample_pattern_episode(false, "local-reward-fail");
    ep2.extra
        .insert("model".to_string(), serde_json::json!("claude-opus-4-6"));
    ep2.success = false;
    runtime
        .record_completed_run(CompletedRunInput::from_episode(ep2))
        .await
        .unwrap();
    assert!(
        (runtime.local_reward_score("router", "claude-opus-4-6") - 0.5).abs() < 1e-9,
        "1 success + 1 failure = 0.5"
    );

    // Verify persistence: reload from disk and check scores survive.
    let reloaded = LearningRuntime::open_under(tmp.path()).await.unwrap();
    assert!(
        (reloaded.local_reward_score("router", "claude-opus-4-6") - 0.5).abs() < 1e-9,
        "persisted score should survive reload"
    );
    assert_eq!(reloaded.local_reward_score("skill", "rust-impl"), 1.0);
}

#[tokio::test]
async fn generation_outcome_partial_success_does_not_seed_learning() {
    let tmp = TempDir::new().unwrap();
    let runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();

    let outcome = GenerationOutcome {
        process_success: true,
        artifact_valid: false,
        validation_report: None,
    };

    runtime
        .record_generation_outcome("prd:plan:test", "claude-sonnet-4-6", &outcome)
        .await
        .unwrap();

    let snapshot = runtime
        .query_feedback(&RuntimeFeedbackQuery::default())
        .await
        .unwrap();

    assert_eq!(snapshot.episodes.len(), 1);
    assert!(!snapshot.episodes[0].success);
    assert!(snapshot.knowledge_seeds.is_empty());
    assert_eq!(snapshot.provider_model_outcomes.len(), 1);
}

#[tokio::test]
async fn provider_health_tracker_bootstraps_from_persisted_state() {
    use crate::provider_health::{ErrorClass, ProviderHealthRegistry};

    let tmp = TempDir::new().unwrap();
    let learn_root = tmp.path().join("learn");
    std::fs::create_dir_all(&learn_root).unwrap();

    // Write a persisted registry with provider "zai" in Open state
    // (3 failures trips the circuit).
    let health_path = learn_root.join("provider-health.json");
    let registry = ProviderHealthRegistry::load_or_new(&health_path);
    registry.record_failure("zai", ErrorClass::Unknown);
    registry.record_failure("zai", ErrorClass::Unknown);
    registry.record_failure("zai", ErrorClass::Unknown);
    registry.save(&health_path).unwrap();
    drop(registry);

    // Open a LearningRuntime and verify the in-memory tracker reflects
    // the persisted Open state.
    let runtime = LearningRuntime::open_under(&learn_root).await.unwrap();
    assert!(
        !runtime.provider_health().is_healthy("zai"),
        "provider with persisted Open state should be unhealthy on construction"
    );

    // Now record a manual success in the registry, save, reopen, and
    // verify the tracker reflects the healthy state.
    let registry2 = ProviderHealthRegistry::load_or_new(&health_path);
    registry2.record_success("zai");
    registry2.save(&health_path).unwrap();
    drop(registry2);

    let runtime2 = LearningRuntime::open_under(&learn_root).await.unwrap();
    assert!(
        runtime2.provider_health().is_healthy("zai"),
        "provider with persisted Closed state should be healthy on construction"
    );
}

#[tokio::test]
async fn wal_replay_on_open_restores_cascade_observations() {
    use crate::wal::{WalEntry, WalWriter};

    let dir = TempDir::new().unwrap();
    let learn_root = dir.path().join("learn");
    std::fs::create_dir_all(&learn_root).unwrap();

    // Pre-populate a WAL with cascade observations.
    let wal_path = learn_root.join("wal.jsonl");
    {
        let mut w = WalWriter::open(&wal_path).unwrap();
        for i in 0..5 {
            w.append(&WalEntry::CascadeObservation {
                model_slug: "claude-sonnet-4-5".into(),
                context_features: vec![0.0; 18],
                model_idx: 0,
                reward: 0.8,
                success: i % 2 == 0,
                ts_ms: 1000 + i as i64,
            })
            .unwrap();
        }
        assert_eq!(w.entry_count(), 5);
    }

    // Open the LearningRuntime -- it should replay the WAL.
    let runtime = LearningRuntime::open_under(&learn_root).await.unwrap();

    // After replay + snapshot, the WAL should be truncated.
    let reopened = WalWriter::open(&wal_path).unwrap();
    assert_eq!(
        reopened.entry_count(),
        0,
        "WAL should be truncated after replay"
    );

    // The cascade router should reflect 5 observations (from the replay).
    let obs = runtime.cascade_router().total_observations();
    assert!(
        obs >= 5,
        "cascade router should have at least 5 observations from WAL replay, got {obs}"
    );
}

#[tokio::test]
async fn wal_append_gate_threshold_writes_entry() {
    use crate::wal::{WalEntry, replay_wal};

    let dir = TempDir::new().unwrap();
    let learn_root = dir.path().join("learn");

    let runtime = LearningRuntime::open_under(&learn_root).await.unwrap();
    runtime.wal_append_gate_threshold(2, true);
    runtime.wal_append_gate_threshold(3, false);

    let wal_path = learn_root.join("wal.jsonl");
    let entries = replay_wal(&wal_path).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(matches!(
        entries[0],
        WalEntry::GateThresholdUpdate {
            rung: 2,
            passed: true,
            ..
        }
    ));
    assert!(matches!(
        entries[1],
        WalEntry::GateThresholdUpdate {
            rung: 3,
            passed: false,
            ..
        }
    ));
}

#[tokio::test]
async fn gate_thresholds_flush_due_at_cadence() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    // gate_thresholds_every_n = 1 means every episode triggers a flush signal.
    runtime.set_update_frequency(UpdateFrequency {
        gate_thresholds_every_n: 1,
        ..UpdateFrequency::default()
    });

    let update = runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_episode(true)))
        .await
        .unwrap();
    assert!(
        update.gate_thresholds_flush_due,
        "gate_thresholds_flush_due must be true when cadence = 1"
    );

    // With cadence = 3, only the 3rd episode triggers the flush signal.
    runtime.set_update_frequency(UpdateFrequency {
        gate_thresholds_every_n: 3,
        ..UpdateFrequency::default()
    });

    let update2 = runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_episode(true)))
        .await
        .unwrap();
    assert!(
        !update2.gate_thresholds_flush_due,
        "episode 2 of 3 must not trigger flush"
    );

    let update3 = runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_episode(true)))
        .await
        .unwrap();
    assert!(
        update3.gate_thresholds_flush_due,
        "episode 3 of 3 must trigger flush at cadence"
    );
}

/// Regression guard: with gate_thresholds_every_n = 1 the very first
/// completed run signals an incremental threshold flush, proving
/// thresholds are written before shutdown.
#[tokio::test]
async fn gate_thresholds_incremental_flush_before_shutdown() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    // Cadence = 1: every completed run must trigger a flush signal.
    runtime.set_update_frequency(UpdateFrequency {
        gate_thresholds_every_n: 1,
        ..UpdateFrequency::default()
    });

    // First completed run should already signal a flush.
    let update1 = runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_episode(true)))
        .await
        .unwrap();
    assert!(
        update1.gate_thresholds_flush_due,
        "incremental threshold flush must fire on the very first completed run at cadence 1"
    );

    // The gate_thresholds.json path is available for the caller to persist
    // adaptive thresholds — verify it points inside the learn directory.
    let expected_path = tmp.path().join("gate-thresholds.json");
    assert_eq!(
        runtime.paths().gate_thresholds_json,
        expected_path,
        "gate_thresholds_json path must be inside the learn root"
    );

    // Second run also fires (cadence 1 means every episode).
    let update2 = runtime
        .record_completed_run(CompletedRunInput::from_episode(sample_episode(false)))
        .await
        .unwrap();
    assert!(
        update2.gate_thresholds_flush_due,
        "incremental threshold flush must fire on every completed run at cadence 1"
    );
}

/// E07-T10: Prove that when `gate_thresholds_snapshot` is supplied in
/// `CompletedRunInput` and cadence = 1, the threshold JSON is written
/// to disk *during* `record_completed_run` — before any shutdown.
#[tokio::test]
async fn gate_thresholds_written_before_shutdown() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    runtime.set_update_frequency(UpdateFrequency {
        gate_thresholds_every_n: 1,
        ..UpdateFrequency::default()
    });

    let thresholds_path = runtime.paths().gate_thresholds_json.clone();

    // Threshold file must not exist yet.
    assert!(
        !thresholds_path.exists(),
        "gate-thresholds.json must not exist before the first run"
    );

    // Simulate serialized adaptive thresholds from the orchestrator.
    let fake_thresholds = serde_json::json!({
        "rungs": {
            "0": { "pass_count": 5, "total_count": 10, "ema_pass_rate": 0.6 }
        }
    });
    let snapshot_json = serde_json::to_string_pretty(&fake_thresholds).unwrap();

    let input = CompletedRunInput::from_episode(sample_episode(true))
        .with_gate_thresholds(snapshot_json.clone());

    let update = runtime.record_completed_run(input).await.unwrap();

    // Both flags should be set.
    assert!(
        update.gate_thresholds_flush_due,
        "gate_thresholds_flush_due must be true at cadence 1"
    );
    assert!(
        update.gate_thresholds_flushed,
        "gate_thresholds_flushed must be true when snapshot was provided"
    );

    // The file must now exist on disk — written before any shutdown.
    assert!(
        thresholds_path.exists(),
        "gate-thresholds.json must exist after incremental flush"
    );

    let on_disk = std::fs::read_to_string(&thresholds_path).unwrap();
    assert_eq!(
        on_disk, snapshot_json,
        "on-disk content must match the supplied snapshot"
    );
}

/// E07-T10: When `gate_thresholds_snapshot` is NOT supplied, the flush
/// flag is set but no disk write occurs (backward compat with callers
/// that handle the flush themselves).
#[tokio::test]
async fn gate_thresholds_not_flushed_without_snapshot() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    runtime.set_update_frequency(UpdateFrequency {
        gate_thresholds_every_n: 1,
        ..UpdateFrequency::default()
    });

    let thresholds_path = runtime.paths().gate_thresholds_json.clone();

    let input = CompletedRunInput::from_episode(sample_episode(true));
    let update = runtime.record_completed_run(input).await.unwrap();

    assert!(
        update.gate_thresholds_flush_due,
        "gate_thresholds_flush_due must be true at cadence 1"
    );
    assert!(
        !update.gate_thresholds_flushed,
        "gate_thresholds_flushed must be false when no snapshot was provided"
    );
    assert!(
        !thresholds_path.exists(),
        "gate-thresholds.json must not be created when no snapshot is supplied"
    );
}

/// E07-T10: Honour gate_thresholds_every_n > 1 — the incremental flush
/// only fires at the configured cadence, not on every run.
#[tokio::test]
async fn gate_thresholds_cadence_honoured_with_snapshot() {
    let tmp = TempDir::new().unwrap();
    let mut runtime = LearningRuntime::open_under(tmp.path()).await.unwrap();
    // Cadence = 3: only the 3rd run should flush.
    runtime.set_update_frequency(UpdateFrequency {
        gate_thresholds_every_n: 3,
        ..UpdateFrequency::default()
    });

    let thresholds_path = runtime.paths().gate_thresholds_json.clone();
    let snapshot_json = r#"{"rungs":{}}"#.to_string();

    // Run 1: no flush.
    let input1 = CompletedRunInput::from_episode(sample_episode(true))
        .with_gate_thresholds(snapshot_json.clone());
    let u1 = runtime.record_completed_run(input1).await.unwrap();
    assert!(!u1.gate_thresholds_flushed, "run 1 of 3 must not flush");
    assert!(!thresholds_path.exists(), "file must not exist after run 1");

    // Run 2: no flush.
    let input2 = CompletedRunInput::from_episode(sample_episode(true))
        .with_gate_thresholds(snapshot_json.clone());
    let u2 = runtime.record_completed_run(input2).await.unwrap();
    assert!(!u2.gate_thresholds_flushed, "run 2 of 3 must not flush");
    assert!(!thresholds_path.exists(), "file must not exist after run 2");

    // Run 3: flush fires.
    let input3 = CompletedRunInput::from_episode(sample_episode(true))
        .with_gate_thresholds(snapshot_json.clone());
    let u3 = runtime.record_completed_run(input3).await.unwrap();
    assert!(u3.gate_thresholds_flushed, "run 3 of 3 must flush");
    assert!(
        thresholds_path.exists(),
        "file must exist after cadence fires"
    );
}
