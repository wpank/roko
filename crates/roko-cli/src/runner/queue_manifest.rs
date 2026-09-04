//! Queue manifest — `.roko/queue.toml` milestone system for sequencing
//! groups of plans across named milestones.
//!
//! Milestones execute sequentially: all plans in milestone N must complete
//! before any plan in milestone N+1 is eligible to run. Within a milestone,
//! plans can run in parallel (subject to their own DAG edges).
//!
//! ```toml
//! [run]
//! max_agents = 4
//! mode = "balanced"
//!
//! [[milestone]]
//! name = "mvp"
//! description = "Core execution loop"
//! plans = ["01-task-dag", "02-event-loop", "03-gate-pipeline"]
//!
//! [[milestone]]
//! name = "polish"
//! description = "TUI and UX improvements"
//! plans = ["10-tui-tabs", "11-header-bar"]
//! depends_on = ["mvp"]
//! ```

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

/// Top-level queue manifest parsed from `queue.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueManifest {
    /// Per-run configuration overrides.
    #[serde(default)]
    pub run: RunOverrides,

    /// Ordered milestones.
    #[serde(default, rename = "milestone")]
    pub milestones: Vec<Milestone>,
}

/// Per-run configuration overrides from `[run]` in `queue.toml`.
///
/// These override `roko.toml` settings for the duration of the run.
/// CLI flags take precedence over these overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunOverrides {
    /// Maximum concurrent agents.
    #[serde(default)]
    pub max_agents: Option<usize>,

    /// Execution mode: quality | balanced | cost | speed.
    #[serde(default)]
    pub mode: Option<String>,

    /// Express mode flag.
    #[serde(default)]
    pub express: Option<bool>,

    /// Default model override.
    #[serde(default)]
    pub model: Option<String>,
}

/// A named milestone containing a group of plan IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone name (unique identifier).
    pub name: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Plan IDs in this milestone.
    #[serde(default)]
    pub plans: Vec<String>,

    /// Milestone names this milestone depends on (must complete first).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Free-form tags for filtering.
    #[serde(default)]
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

/// Validation issue found in a queue manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueValidationIssue {
    /// A plan ID referenced in a milestone does not exist on disk.
    MissingPlan { milestone: String, plan_id: String },
    /// A milestone name referenced in `depends_on` does not exist.
    MissingMilestoneDep {
        milestone: String,
        dependency: String,
    },
    /// Two milestones share the same name.
    DuplicateMilestone(String),
    /// A plan appears in more than one milestone.
    DuplicatePlan {
        plan_id: String,
        milestones: Vec<String>,
    },
    /// Milestone dependencies form a cycle.
    CyclicDependency { cycle: Vec<String> },
    /// No milestones defined.
    Empty,
}

impl std::fmt::Display for QueueValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPlan { milestone, plan_id } => {
                write!(f, "milestone '{milestone}': plan '{plan_id}' not found")
            }
            Self::MissingMilestoneDep {
                milestone,
                dependency,
            } => write!(
                f,
                "milestone '{milestone}': depends on unknown milestone '{dependency}'"
            ),
            Self::DuplicateMilestone(name) => write!(f, "duplicate milestone name: '{name}'"),
            Self::DuplicatePlan {
                plan_id,
                milestones,
            } => write!(
                f,
                "plan '{plan_id}' appears in multiple milestones: {}",
                milestones.join(", ")
            ),
            Self::CyclicDependency { cycle } => {
                write!(f, "cyclic milestone dependency: {}", cycle.join(" -> "))
            }
            Self::Empty => write!(f, "no milestones defined"),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

impl QueueManifest {
    /// Parse a queue manifest from a TOML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("read queue manifest: {}", path.display()))?;
        Self::from_str(&content)
    }

    /// Parse a queue manifest from a TOML string.
    pub fn from_str(content: &str) -> Result<Self> {
        toml::from_str(content).context("parse queue manifest TOML")
    }

    /// Validate the manifest structure, optionally checking plan IDs against
    /// available plan directories.
    pub fn validate(&self, available_plans: Option<&HashSet<String>>) -> Vec<QueueValidationIssue> {
        let mut issues = Vec::new();

        if self.milestones.is_empty() {
            issues.push(QueueValidationIssue::Empty);
            return issues;
        }

        // Check for duplicate milestone names.
        let mut seen_milestones: HashMap<&str, usize> = HashMap::new();
        for ms in &self.milestones {
            let count = seen_milestones.entry(&ms.name).or_insert(0);
            *count += 1;
            if *count == 2 {
                issues.push(QueueValidationIssue::DuplicateMilestone(ms.name.clone()));
            }
        }

        let milestone_names: HashSet<&str> =
            self.milestones.iter().map(|ms| ms.name.as_str()).collect();

        // Check for unknown milestone dependencies.
        for ms in &self.milestones {
            for dep in &ms.depends_on {
                if !milestone_names.contains(dep.as_str()) {
                    issues.push(QueueValidationIssue::MissingMilestoneDep {
                        milestone: ms.name.clone(),
                        dependency: dep.clone(),
                    });
                }
            }
        }

        // Check for plans appearing in multiple milestones.
        let mut plan_to_milestones: HashMap<&str, Vec<&str>> = HashMap::new();
        for ms in &self.milestones {
            for plan_id in &ms.plans {
                plan_to_milestones
                    .entry(plan_id.as_str())
                    .or_default()
                    .push(&ms.name);
            }
        }
        for (plan_id, milestones) in &plan_to_milestones {
            if milestones.len() > 1 {
                issues.push(QueueValidationIssue::DuplicatePlan {
                    plan_id: (*plan_id).to_string(),
                    milestones: milestones.iter().map(|s| (*s).to_string()).collect(),
                });
            }
        }

        // Check that plan IDs exist on disk (when available_plans is provided).
        if let Some(available) = available_plans {
            for ms in &self.milestones {
                for plan_id in &ms.plans {
                    if !available.contains(plan_id) {
                        issues.push(QueueValidationIssue::MissingPlan {
                            milestone: ms.name.clone(),
                            plan_id: plan_id.clone(),
                        });
                    }
                }
            }
        }

        // Check for cyclic milestone dependencies.
        if let Some(cycle) = detect_milestone_cycle(&self.milestones) {
            issues.push(QueueValidationIssue::CyclicDependency { cycle });
        }

        issues
    }

    /// Return plan IDs that are eligible to run given the set of completed plans.
    ///
    /// A plan is eligible when its milestone has all dependencies satisfied
    /// (all plans in all depended-upon milestones are in `completed_plans`).
    pub fn eligible_plans(&self, completed_plans: &HashSet<String>) -> Vec<String> {
        let milestone_names: HashSet<&str> =
            self.milestones.iter().map(|ms| ms.name.as_str()).collect();

        // Determine which milestones are complete.
        let mut milestone_complete: HashMap<&str, bool> = HashMap::new();
        for ms in &self.milestones {
            let all_done = ms.plans.iter().all(|p| completed_plans.contains(p));
            milestone_complete.insert(&ms.name, all_done);
        }

        let mut eligible = Vec::new();
        for ms in &self.milestones {
            // Skip milestones that are already complete.
            if milestone_complete.get(ms.name.as_str()).copied() == Some(true) {
                continue;
            }

            // Check that all dependency milestones are complete.
            let deps_satisfied = ms.depends_on.iter().all(|dep| {
                milestone_names.contains(dep.as_str())
                    && milestone_complete.get(dep.as_str()).copied() == Some(true)
            });

            if deps_satisfied {
                for plan_id in &ms.plans {
                    if !completed_plans.contains(plan_id) {
                        eligible.push(plan_id.clone());
                    }
                }
            }
        }

        eligible
    }

    /// Return the milestone a plan belongs to (if any).
    pub fn milestone_for_plan(&self, plan_id: &str) -> Option<&Milestone> {
        self.milestones
            .iter()
            .find(|ms| ms.plans.iter().any(|p| p == plan_id))
    }

    /// Compute the topological milestone execution order.
    ///
    /// Returns milestone names in an order such that all dependencies
    /// come before their dependents. Milestones without dependencies
    /// appear first, in definition order.
    pub fn milestone_order(&self) -> Vec<&str> {
        let milestone_names: HashSet<&str> =
            self.milestones.iter().map(|ms| ms.name.as_str()).collect();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for ms in &self.milestones {
            in_degree.entry(ms.name.as_str()).or_insert(0);
            for dep in &ms.depends_on {
                if milestone_names.contains(dep.as_str()) {
                    *in_degree.entry(ms.name.as_str()).or_insert(0) += 1;
                    dependents
                        .entry(dep.as_str())
                        .or_default()
                        .push(ms.name.as_str());
                }
            }
        }

        let mut queue: Vec<&str> = self
            .milestones
            .iter()
            .filter(|ms| in_degree.get(ms.name.as_str()).copied().unwrap_or(0) == 0)
            .map(|ms| ms.name.as_str())
            .collect();
        let mut order = Vec::new();
        let mut idx = 0;
        while idx < queue.len() {
            let current = queue[idx];
            idx += 1;
            order.push(current);
            if let Some(deps) = dependents.get(current) {
                for &dep in deps {
                    let count = in_degree.entry(dep).or_insert(1);
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        queue.push(dep);
                    }
                }
            }
        }

        order
    }

    /// Generate a starter queue manifest from a list of plan IDs.
    ///
    /// Creates a single "default" milestone containing all plans.
    pub fn generate_starter(plan_ids: &[String]) -> Self {
        Self {
            run: RunOverrides::default(),
            milestones: vec![Milestone {
                name: "default".to_string(),
                description: "All plans".to_string(),
                plans: plan_ids.to_vec(),
                depends_on: vec![],
                tags: vec![],
            }],
        }
    }

    /// Serialize to a TOML string.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("serialize queue manifest")
    }

    /// Render a human-readable status display.
    ///
    /// Shows each milestone with its plans and completion state.
    pub fn render_show(&self, completed_plans: &HashSet<String>) -> String {
        let mut out = String::new();
        let order = self.milestone_order();

        for (idx, &ms_name) in order.iter().enumerate() {
            let Some(ms) = self.milestones.iter().find(|m| m.name == ms_name) else {
                continue;
            };

            let done_count = ms
                .plans
                .iter()
                .filter(|p| completed_plans.contains(*p))
                .count();
            let total = ms.plans.len();
            let all_done = done_count == total && total > 0;

            let status = if all_done {
                "DONE"
            } else if done_count > 0 {
                "IN PROGRESS"
            } else {
                "PENDING"
            };

            out.push_str(&format!(
                "Milestone {}: {} [{status}] ({done_count}/{total})\n",
                idx, ms.name,
            ));
            if !ms.description.is_empty() {
                out.push_str(&format!("  {}\n", ms.description));
            }
            if !ms.depends_on.is_empty() {
                out.push_str(&format!("  depends_on: {}\n", ms.depends_on.join(", ")));
            }

            for plan_id in &ms.plans {
                let icon = if completed_plans.contains(plan_id) {
                    "\u{2713}" // checkmark
                } else {
                    "\u{25cb}" // circle
                };
                out.push_str(&format!("    {icon} {plan_id}\n"));
            }
            out.push('\n');
        }

        if let Some(ref overrides) = Some(&self.run) {
            let mut has_overrides = false;
            if overrides.max_agents.is_some()
                || overrides.mode.is_some()
                || overrides.express.is_some()
                || overrides.model.is_some()
            {
                has_overrides = true;
            }
            if has_overrides {
                out.push_str("Run overrides:\n");
                if let Some(max_agents) = overrides.max_agents {
                    out.push_str(&format!("  max_agents: {max_agents}\n"));
                }
                if let Some(ref mode) = overrides.mode {
                    out.push_str(&format!("  mode: {mode}\n"));
                }
                if let Some(express) = overrides.express {
                    out.push_str(&format!("  express: {express}\n"));
                }
                if let Some(ref model) = overrides.model {
                    out.push_str(&format!("  model: {model}\n"));
                }
            }
        }

        out
    }
}

// ---------------------------------------------------------------------------
// Cycle detection
// ---------------------------------------------------------------------------

fn detect_milestone_cycle(milestones: &[Milestone]) -> Option<Vec<String>> {
    let names: HashSet<&str> = milestones.iter().map(|ms| ms.name.as_str()).collect();
    let adjacency: HashMap<&str, Vec<&str>> = milestones
        .iter()
        .map(|ms| {
            let deps: Vec<&str> = ms
                .depends_on
                .iter()
                .filter(|d| names.contains(d.as_str()))
                .map(|d| d.as_str())
                .collect();
            (ms.name.as_str(), deps)
        })
        .collect();

    // DFS-based cycle detection.
    let mut visited: HashSet<&str> = HashSet::new();
    let mut stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for ms in milestones {
        if !visited.contains(ms.name.as_str())
            && dfs_cycle(
                ms.name.as_str(),
                &adjacency,
                &mut visited,
                &mut stack,
                &mut path,
            )
        {
            return Some(path.iter().map(|s| (*s).to_string()).collect());
        }
    }
    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    adjacency: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> bool {
    visited.insert(node);
    stack.insert(node);
    path.push(node);

    if let Some(deps) = adjacency.get(node) {
        for &dep in deps {
            if !visited.contains(dep) {
                if dfs_cycle(dep, adjacency, visited, stack, path) {
                    return true;
                }
            } else if stack.contains(dep) {
                path.push(dep);
                return true;
            }
        }
    }

    stack.remove(node);
    path.pop();
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest() {
        let toml = r#"
[run]
max_agents = 4
mode = "balanced"

[[milestone]]
name = "mvp"
description = "Core execution loop"
plans = ["01-task-dag", "02-event-loop"]

[[milestone]]
name = "polish"
description = "TUI and UX improvements"
plans = ["10-tui-tabs"]
depends_on = ["mvp"]
"#;
        let manifest = QueueManifest::from_str(toml).unwrap();
        assert_eq!(manifest.milestones.len(), 2);
        assert_eq!(manifest.run.max_agents, Some(4));
        assert_eq!(manifest.milestones[0].name, "mvp");
        assert_eq!(manifest.milestones[1].depends_on, vec!["mvp"]);
    }

    #[test]
    fn validate_missing_plan() {
        let manifest = QueueManifest {
            run: RunOverrides::default(),
            milestones: vec![Milestone {
                name: "test".to_string(),
                description: String::new(),
                plans: vec!["nonexistent".to_string()],
                depends_on: vec![],
                tags: vec![],
            }],
        };
        let available: HashSet<String> = HashSet::from(["existing".to_string()]);
        let issues = manifest.validate(Some(&available));
        assert!(issues.iter().any(|i| matches!(i, QueueValidationIssue::MissingPlan { plan_id, .. } if plan_id == "nonexistent")));
    }

    #[test]
    fn validate_duplicate_milestone() {
        let manifest = QueueManifest {
            run: RunOverrides::default(),
            milestones: vec![
                Milestone {
                    name: "same".to_string(),
                    description: String::new(),
                    plans: vec!["a".to_string()],
                    depends_on: vec![],
                    tags: vec![],
                },
                Milestone {
                    name: "same".to_string(),
                    description: String::new(),
                    plans: vec!["b".to_string()],
                    depends_on: vec![],
                    tags: vec![],
                },
            ],
        };
        let issues = manifest.validate(None);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i, QueueValidationIssue::DuplicateMilestone(n) if n == "same"))
        );
    }

    #[test]
    fn validate_cyclic_dependency() {
        let manifest = QueueManifest {
            run: RunOverrides::default(),
            milestones: vec![
                Milestone {
                    name: "a".to_string(),
                    description: String::new(),
                    plans: vec!["p1".to_string()],
                    depends_on: vec!["b".to_string()],
                    tags: vec![],
                },
                Milestone {
                    name: "b".to_string(),
                    description: String::new(),
                    plans: vec!["p2".to_string()],
                    depends_on: vec!["a".to_string()],
                    tags: vec![],
                },
            ],
        };
        let issues = manifest.validate(None);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i, QueueValidationIssue::CyclicDependency { .. }))
        );
    }

    #[test]
    fn eligible_plans_respects_milestones() {
        let manifest = QueueManifest {
            run: RunOverrides::default(),
            milestones: vec![
                Milestone {
                    name: "first".to_string(),
                    description: String::new(),
                    plans: vec!["a".to_string(), "b".to_string()],
                    depends_on: vec![],
                    tags: vec![],
                },
                Milestone {
                    name: "second".to_string(),
                    description: String::new(),
                    plans: vec!["c".to_string()],
                    depends_on: vec!["first".to_string()],
                    tags: vec![],
                },
            ],
        };

        // Nothing completed: only first milestone is eligible.
        let completed = HashSet::new();
        let eligible = manifest.eligible_plans(&completed);
        assert_eq!(eligible, vec!["a", "b"]);

        // First milestone partially complete: second still blocked.
        let completed = HashSet::from(["a".to_string()]);
        let eligible = manifest.eligible_plans(&completed);
        assert_eq!(eligible, vec!["b"]);

        // First milestone complete: second is eligible.
        let completed = HashSet::from(["a".to_string(), "b".to_string()]);
        let eligible = manifest.eligible_plans(&completed);
        assert_eq!(eligible, vec!["c"]);
    }

    #[test]
    fn milestone_order_topological() {
        let manifest = QueueManifest {
            run: RunOverrides::default(),
            milestones: vec![
                Milestone {
                    name: "c".to_string(),
                    description: String::new(),
                    plans: vec![],
                    depends_on: vec!["a".to_string(), "b".to_string()],
                    tags: vec![],
                },
                Milestone {
                    name: "a".to_string(),
                    description: String::new(),
                    plans: vec![],
                    depends_on: vec![],
                    tags: vec![],
                },
                Milestone {
                    name: "b".to_string(),
                    description: String::new(),
                    plans: vec![],
                    depends_on: vec!["a".to_string()],
                    tags: vec![],
                },
            ],
        };
        let order = manifest.milestone_order();
        let a_pos = order.iter().position(|&n| n == "a").unwrap();
        let b_pos = order.iter().position(|&n| n == "b").unwrap();
        let c_pos = order.iter().position(|&n| n == "c").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn generate_starter_roundtrip() {
        let plan_ids = vec!["plan-a".to_string(), "plan-b".to_string()];
        let manifest = QueueManifest::generate_starter(&plan_ids);
        let toml = manifest.to_toml().unwrap();
        let parsed = QueueManifest::from_str(&toml).unwrap();
        assert_eq!(parsed.milestones.len(), 1);
        assert_eq!(parsed.milestones[0].plans, plan_ids);
    }

    #[test]
    fn validate_empty() {
        let manifest = QueueManifest {
            run: RunOverrides::default(),
            milestones: vec![],
        };
        let issues = manifest.validate(None);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i, QueueValidationIssue::Empty))
        );
    }
}
