//! P4-19: Tool recommendation from efficiency history.
//!
//! Queries efficiency.jsonl for past successful similar tasks and extracts
//! which tools were used, injecting "recommended tools" hints into the
//! system prompt at dispatch time.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Maximum number of tools to recommend.
const MAX_RECOMMENDED_TOOLS: usize = 8;

/// Minimum uses before a tool is recommended for a role.
const MIN_USES_FOR_RECOMMENDATION: u64 = 3;

/// A tool recommendation based on historical success patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    /// Tool name.
    pub tool_name: String,
    /// How often this tool was used in successful similar tasks.
    pub success_count: u64,
    /// How often this tool was used overall in similar tasks.
    pub total_count: u64,
    /// Success rate when this tool was used.
    pub success_rate: f64,
    /// Average number of times this tool was called per successful task.
    pub avg_calls_per_task: f64,
}

/// Recommender that learns tool preferences from efficiency history.
#[derive(Debug, Clone, Default)]
pub struct ToolRecommender {
    /// Per-(role, tool_name) aggregate statistics.
    role_tool_stats: HashMap<(String, String), ToolStat>,
}

#[derive(Debug, Clone, Default)]
struct ToolStat {
    success_count: u64,
    total_count: u64,
    total_calls_in_successes: u64,
    successful_tasks: u64,
}

/// Efficiency record subset needed for tool recommendation.
#[derive(Debug, Clone, Deserialize)]
pub struct EfficiencyRecord {
    /// Agent role.
    #[serde(default)]
    pub role: String,
    /// Whether the task succeeded.
    #[serde(default)]
    pub success: bool,
    /// Tools used during the task.
    #[serde(default)]
    pub tools_used: Vec<ToolUsageEntry>,
}

/// A single tool usage entry from an efficiency record.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolUsageEntry {
    /// Tool name.
    #[serde(default)]
    pub name: String,
    /// Number of times the tool was called.
    #[serde(default)]
    pub call_count: u64,
}

impl ToolRecommender {
    /// Create a new empty recommender.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load recommendations from efficiency JSONL data.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn load_from_efficiency(path: &Path) -> Result<Self, std::io::Error> {
        let mut recommender = Self::new();
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(recommender),
            Err(e) => return Err(e),
        };

        for line in contents.lines() {
            if let Ok(record) = serde_json::from_str::<EfficiencyRecord>(line) {
                recommender.ingest_record(&record);
            }
        }
        Ok(recommender)
    }

    fn ingest_record(&mut self, record: &EfficiencyRecord) {
        if record.role.is_empty() {
            return;
        }
        for tool in &record.tools_used {
            if tool.name.is_empty() {
                continue;
            }
            let key = (record.role.clone(), tool.name.clone());
            let stat = self.role_tool_stats.entry(key).or_default();
            stat.total_count += 1;
            if record.success {
                stat.success_count += 1;
                stat.total_calls_in_successes += tool.call_count;
                stat.successful_tasks += 1;
            }
        }
    }

    /// Get recommended tools for a given role, ordered by relevance.
    #[must_use]
    pub fn recommend(&self, role: &str) -> Vec<ToolRecommendation> {
        let mut recommendations: Vec<ToolRecommendation> = self
            .role_tool_stats
            .iter()
            .filter(|((r, _), stat)| {
                r == role && stat.total_count >= MIN_USES_FOR_RECOMMENDATION
            })
            .map(|((_, tool_name), stat)| {
                let success_rate = if stat.total_count == 0 {
                    0.0
                } else {
                    stat.success_count as f64 / stat.total_count as f64
                };
                let avg_calls = if stat.successful_tasks == 0 {
                    0.0
                } else {
                    stat.total_calls_in_successes as f64 / stat.successful_tasks as f64
                };
                ToolRecommendation {
                    tool_name: tool_name.clone(),
                    success_count: stat.success_count,
                    total_count: stat.total_count,
                    success_rate,
                    avg_calls_per_task: avg_calls,
                }
            })
            .collect();

        // Sort by success rate descending, then by usage count.
        recommendations.sort_by(|a, b| {
            b.success_rate
                .total_cmp(&a.success_rate)
                .then(b.success_count.cmp(&a.success_count))
        });

        recommendations.truncate(MAX_RECOMMENDED_TOOLS);
        recommendations
    }

    /// Format recommendations as a prompt section string.
    #[must_use]
    pub fn format_prompt_section(&self, role: &str) -> Option<String> {
        let recs = self.recommend(role);
        if recs.is_empty() {
            return None;
        }

        let mut section = String::from("Previously successful tools for this role:\n");
        for rec in &recs {
            section.push_str(&format!(
                "- {} (success rate: {:.0}%, avg {:.1} calls/task)\n",
                rec.tool_name,
                rec.success_rate * 100.0,
                rec.avg_calls_per_task,
            ));
        }
        Some(section)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommender_learns_from_records() {
        let mut recommender = ToolRecommender::new();

        // Simulate several successful tasks using Bash.
        for _ in 0..5 {
            recommender.ingest_record(&EfficiencyRecord {
                role: "implementer".to_string(),
                success: true,
                tools_used: vec![ToolUsageEntry {
                    name: "Bash".to_string(),
                    call_count: 3,
                }],
            });
        }

        let recs = recommender.recommend("implementer");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].tool_name, "Bash");
        assert!((recs[0].success_rate - 1.0).abs() < 1e-10);
    }

    #[test]
    fn recommender_filters_by_role() {
        let mut recommender = ToolRecommender::new();
        for _ in 0..5 {
            recommender.ingest_record(&EfficiencyRecord {
                role: "reviewer".to_string(),
                success: true,
                tools_used: vec![ToolUsageEntry {
                    name: "Read".to_string(),
                    call_count: 5,
                }],
            });
        }
        assert!(recommender.recommend("implementer").is_empty());
        assert_eq!(recommender.recommend("reviewer").len(), 1);
    }

    #[test]
    fn format_prompt_section_returns_none_when_empty() {
        let recommender = ToolRecommender::new();
        assert!(recommender.format_prompt_section("unknown").is_none());
    }
}
