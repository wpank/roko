//! Decision provenance: knowledge cards and provenance chain rendering.

use std::collections::HashSet;
use std::path::Path;

use roko_learn::{
    episode_logger::EpisodeLogger,
    playbook::Playbook,
};
use roko_neuro::{KnowledgeKind, KnowledgeQueryHit, KnowledgeTier};
use roko_dreams::{load_dream_routing_advice, relevant_pattern_summaries};
use tokio::{sync::mpsc, task};
use tracing::warn;

use crate::knowledge::DispatchKnowledge;
use crate::types::{ContentBlock, ToolCallKind, ToolCallStatus};

use super::CognitiveEvent;
use super::context::truncate_with_limit;
use super::cost::{knowledge_tier_label, score_to_confidence};

pub(crate) async fn emit_knowledge_card(
    knowledge: &DispatchKnowledge,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    let Some(card) = knowledge.card() else {
        return;
    };

    let tool_call_id = "knowledge-query".to_string();
    let _ = event_sender
        .send(CognitiveEvent::ToolCallStart {
            tool_call_id: tool_call_id.clone(),
            title: card.title,
            kind: ToolCallKind::Other,
            locations: None,
        })
        .await;
    let _ = event_sender
        .send(CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Completed,
            content: vec![ContentBlock::Text { text: card.body }],
        })
        .await;
}

/// A chain tracing why Roko chose a particular approach.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProvenanceChain {
    pub(crate) sources: Vec<ProvenanceSource>,
    pub(crate) confidence: f64,
}

/// One source in a decision provenance chain.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProvenanceSource {
    Playbook {
        id: String,
        goal: String,
        total_outcomes: u64,
        success_rate: f64,
    },
    Episode {
        task_id: String,
        success: bool,
        gate_summary: String,
    },
    Knowledge {
        kind: KnowledgeKind,
        tier: KnowledgeTier,
        score: f64,
        summary: String,
    },
    DreamPattern {
        description: String,
        guidance: String,
        confidence: f64,
    },
}

/// Build provenance from already-queried knowledge/playbook results and
/// best-effort episode/dream lookups.
pub(crate) async fn build_provenance(
    knowledge_hits: &[KnowledgeQueryHit],
    playbooks: &[Playbook],
    prompt: &str,
    workdir: &Path,
) -> Option<ProvenanceChain> {
    let mut sources = Vec::new();
    let mut has_playbook_source = false;

    for playbook in playbooks {
        let Some(success_rate) = playbook.success_rate() else {
            continue;
        };

        has_playbook_source = true;
        sources.push(ProvenanceSource::Playbook {
            id: playbook.id.clone(),
            goal: truncate_with_limit(playbook.goal.trim(), 80, "..."),
            total_outcomes: playbook.total_outcomes(),
            success_rate,
        });
    }

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let prompt_keywords = prompt_keywords(prompt);
    let episodes_future = EpisodeLogger::read_all_lossy(&episodes_path);
    let dreams_future = async {
        if prompt_keywords.is_empty() {
            return Vec::new();
        }

        match task::spawn_blocking({
            let workdir = workdir.to_path_buf();
            move || load_dream_routing_advice(&workdir)
        })
        .await
        {
            Ok(Ok(advice)) => {
                let mut seen_signatures = HashSet::new();
                let mut dream_sources = Vec::new();
                for keyword in prompt_keywords {
                    for pattern in relevant_pattern_summaries(&advice, &keyword, 0.5, 2) {
                        if !seen_signatures.insert(pattern.signature) {
                            continue;
                        }

                        dream_sources.push(ProvenanceSource::DreamPattern {
                            description: truncate_with_limit(&pattern.description, 80, "..."),
                            guidance: truncate_with_limit(&pattern.guidance, 80, "..."),
                            confidence: pattern.confidence,
                        });

                        if dream_sources.len() == 2 {
                            return dream_sources;
                        }
                    }
                }

                dream_sources
            }
            Ok(Err(err)) => {
                warn!(
                    workdir = %workdir.display(),
                    error = %err,
                    "dream routing advice load failed"
                );
                Vec::new()
            }
            Err(err) => {
                warn!(
                    workdir = %workdir.display(),
                    error = %err,
                    "dream routing advice task failed"
                );
                Vec::new()
            }
        }
    };

    let (episodes_result, dream_sources) = tokio::join!(episodes_future, dreams_future);

    match episodes_result {
        Ok(episodes) => {
            let matched_ids: HashSet<&str> = playbooks.iter().map(|pb| pb.id.as_str()).collect();
            let mut episode_count = 0usize;
            for episode in episodes.iter().rev().take(100) {
                if !matched_ids.contains(episode.task_id.as_str()) {
                    continue;
                }

                let gate_summary = if episode.gate_verdicts.is_empty() {
                    String::from("no gate verdicts")
                } else {
                    episode
                        .gate_verdicts
                        .iter()
                        .map(|verdict| {
                            format!(
                                "{}:{}",
                                verdict.gate,
                                if verdict.passed { "pass" } else { "fail" }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                sources.push(ProvenanceSource::Episode {
                    task_id: episode.task_id.clone(),
                    success: episode.success,
                    gate_summary,
                });

                episode_count += 1;
                if episode_count == 3 {
                    break;
                }
            }
        }
        Err(err) => {
            warn!(
                workdir = %workdir.display(),
                error = %err,
                "episode log read failed"
            );
        }
    }

    for hit in knowledge_hits.iter().take(3) {
        sources.push(ProvenanceSource::Knowledge {
            kind: hit.entry.kind,
            tier: hit.entry.tier,
            score: hit.total_score,
            summary: truncate_with_limit(hit.entry.content.trim(), 80, "..."),
        });
    }

    for source in dream_sources {
        sources.push(source);
    }

    if sources.is_empty() || (!has_playbook_source && sources.len() < 2) {
        return None;
    }

    let scores = sources
        .iter()
        .map(|source| match source {
            ProvenanceSource::Playbook { success_rate, .. } => *success_rate,
            ProvenanceSource::Episode { success, .. } => {
                if *success {
                    1.0
                } else {
                    0.0
                }
            }
            ProvenanceSource::Knowledge { score, .. } => score_to_confidence(*score),
            ProvenanceSource::DreamPattern { confidence, .. } => *confidence,
        })
        .collect::<Vec<_>>();

    let confidence = if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    };

    Some(ProvenanceChain {
        sources,
        confidence,
    })
}

/// Emit a provenance card into ACP updates.
pub(crate) async fn emit_provenance_card(
    chain: &ProvenanceChain,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) {
    let tool_call_id = format!("decision-provenance-{}", uuid::Uuid::new_v4());
    let _ = event_sender
        .send(CognitiveEvent::ToolCallStart {
            tool_call_id: tool_call_id.clone(),
            title: "Decision provenance".to_string(),
            kind: ToolCallKind::Other,
            locations: None,
        })
        .await;
    let _ = event_sender
        .send(CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status: ToolCallStatus::Completed,
            content: vec![ContentBlock::Text {
                text: render_provenance_card(chain),
            }],
        })
        .await;
}

pub(crate) fn render_provenance_card(chain: &ProvenanceChain) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} source{}, {:.0}% confidence",
        chain.sources.len(),
        if chain.sources.len() == 1 { "" } else { "s" },
        chain.confidence * 100.0
    ));
    lines.push(String::new());

    for source in &chain.sources {
        match source {
            ProvenanceSource::Playbook {
                id,
                goal,
                total_outcomes,
                success_rate,
            } => {
                lines.push(format!(
                    "- Playbook `{id}` ({} runs, {:.0}% success)",
                    total_outcomes,
                    success_rate * 100.0
                ));
                lines.push(format!("  Goal: {}", goal));
            }
            ProvenanceSource::Episode {
                task_id,
                success,
                gate_summary,
            } => {
                lines.push(format!(
                    "- Episode `{task_id}` [{}]",
                    if *success { "pass" } else { "fail" }
                ));
                lines.push(format!(
                    "  Gates: {}",
                    truncate_with_limit(gate_summary, 80, "...")
                ));
            }
            ProvenanceSource::Knowledge {
                kind,
                tier,
                score,
                summary,
            } => {
                lines.push(format!(
                    "- Knowledge [{}/{}] ({:.2})",
                    kind.as_str(),
                    knowledge_tier_label(*tier),
                    score
                ));
                lines.push(format!("  {}", summary));
            }
            ProvenanceSource::DreamPattern {
                description,
                guidance,
                confidence,
            } => {
                lines.push(format!("- Dream pattern ({:.0}%)", confidence * 100.0));
                lines.push(format!("  Description: {}", description));
                lines.push(format!("  Guidance: {}", guidance));
            }
        }
        lines.push(String::new());
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

pub(crate) fn prompt_keywords(prompt: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    for raw in prompt.split(|ch: char| !ch.is_alphanumeric()) {
        let keyword = raw.trim().to_ascii_lowercase();
        if keyword.len() <= 4 || keywords.iter().any(|existing| existing == &keyword) {
            continue;
        }

        keywords.push(keyword);
        if keywords.len() == 5 {
            break;
        }
    }

    keywords
}

