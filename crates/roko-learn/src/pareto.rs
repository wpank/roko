//! Pareto frontier computation for model cost-quality tradeoffs.
//!
//! This module identifies which models are non-dominated with respect to
//! pass rate and cost per successful task. A model is Pareto-optimal if no
//! other model has both a higher pass rate and a lower cost per successful
//! task.
//!
//! The [`ParetoFrontier`] struct extends this to arbitrary multi-objective
//! optimization (LEARN-06), supporting cost vs quality vs latency tradeoffs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Summary observation for one model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelObservation {
    /// Fraction of tasks that passed (higher is better).
    pub pass_rate: f64,
    /// Total cost divided by the number of successful tasks (lower is better).
    pub cost_per_success: f64,
    /// Average latency in milliseconds (lower is better).
    pub avg_latency_ms: f64,
    /// Fraction of non-error responses (higher is better).
    /// Derived as `non_error_responses / total_responses`.
    pub reliability: f64,
    /// Number of observations contributing to this summary.
    pub observations: u64,
}

/// Configurable weights for Pareto scalarization.
///
/// Allows operators to prioritize certain objectives over others
/// (e.g. cost over latency). All weights should be non-negative;
/// they are internally normalized.
#[derive(Debug, Clone, PartialEq)]
pub struct ParetoWeights {
    /// Weight for quality (pass rate).
    pub quality: f64,
    /// Weight for cost (inverted: lower cost is better).
    pub cost: f64,
    /// Weight for latency (inverted: lower latency is better).
    pub latency: f64,
    /// Weight for reliability (higher is better).
    pub reliability: f64,
}

impl Default for ParetoWeights {
    fn default() -> Self {
        Self {
            quality: 1.0,
            cost: 1.0,
            latency: 1.0,
            reliability: 1.0,
        }
    }
}

impl ParetoWeights {
    /// Create equal weights.
    #[must_use]
    pub fn equal() -> Self {
        Self::default()
    }

    /// Create weights emphasizing quality.
    #[must_use]
    pub fn quality_first() -> Self {
        Self {
            quality: 3.0,
            cost: 1.0,
            latency: 1.0,
            reliability: 1.0,
        }
    }

    /// Create weights emphasizing cost.
    #[must_use]
    pub fn cost_first() -> Self {
        Self {
            quality: 1.0,
            cost: 3.0,
            latency: 1.0,
            reliability: 1.0,
        }
    }
}

/// Scalarize a `ModelObservation` into a single score using weighted combination.
///
/// Quality and reliability are maximized; cost and latency are minimized.
/// Cost and latency are inverted (1 - normalized) so that higher = better.
#[must_use]
pub fn scalarize(obs: &ModelObservation, weights: &ParetoWeights) -> f64 {
    let total_weight =
        (weights.quality + weights.cost + weights.latency + weights.reliability).max(f64::EPSILON);

    // Normalize cost_per_success: assume 100.0 as a reasonable max.
    let cost_normalized = 1.0 - (obs.cost_per_success / 100.0).clamp(0.0, 1.0);
    // Normalize latency: assume 60_000 ms as a reasonable max.
    let latency_normalized = 1.0 - (obs.avg_latency_ms / 60_000.0).clamp(0.0, 1.0);

    (weights.quality * obs.pass_rate
        + weights.cost * cost_normalized
        + weights.latency * latency_normalized
        + weights.reliability * obs.reliability)
        / total_weight
}

/// Compute the 4-objective Pareto frontier over the supplied observations.
///
/// A model is dominated when another model is at least as good on ALL FOUR
/// objectives and strictly better on at least one:
/// - pass_rate: higher is better
/// - cost_per_success: lower is better
/// - avg_latency_ms: lower is better
/// - reliability: higher is better
#[must_use]
pub fn compute_pareto_frontier(stats: &HashMap<String, ModelObservation>) -> Vec<String> {
    let mut frontier = Vec::new();

    for (slug_a, obs_a) in stats {
        let dominated = stats.iter().any(|(slug_b, obs_b)| {
            if slug_b == slug_a {
                return false;
            }
            // b must be >= a on all objectives (using correct direction).
            let quality_ok = obs_b.pass_rate >= obs_a.pass_rate;
            let cost_ok = obs_b.cost_per_success <= obs_a.cost_per_success;
            let latency_ok = obs_b.avg_latency_ms <= obs_a.avg_latency_ms;
            let reliability_ok = obs_b.reliability >= obs_a.reliability;

            // Must be at least as good on all.
            let all_geq = quality_ok && cost_ok && latency_ok && reliability_ok;

            // Must be strictly better on at least one.
            let any_strictly_better = obs_b.pass_rate > obs_a.pass_rate
                || obs_b.cost_per_success < obs_a.cost_per_success
                || obs_b.avg_latency_ms < obs_a.avg_latency_ms
                || obs_b.reliability > obs_a.reliability;

            all_geq && any_strictly_better
        });

        if !dominated {
            frontier.push(slug_a.clone());
        }
    }

    frontier.sort();
    frontier
}

// ─── Multi-objective Pareto frontier (LEARN-06) ─────────────────────────────

/// A single solution in the multi-objective Pareto space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParetoSolution {
    /// Objective values (e.g., cost, quality, latency).
    pub values: Vec<f64>,
    /// Model identifier this solution corresponds to.
    pub model_id: String,
}

/// Multi-objective Pareto frontier over an arbitrary number of objectives.
///
/// Each objective is assumed to be *maximized*. Callers that want to minimize
/// a metric (e.g. cost) should negate it before constructing `ParetoSolution`s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParetoFrontier {
    /// Objective names (e.g., `["cost", "quality", "latency"]`).
    pub objectives: Vec<String>,
    /// Non-dominated solutions on the frontier.
    pub solutions: Vec<ParetoSolution>,
}

impl ParetoFrontier {
    /// Create a new frontier with no solutions.
    #[must_use]
    pub fn new(objectives: Vec<String>) -> Self {
        Self {
            objectives,
            solutions: Vec::new(),
        }
    }

    /// Compute the Pareto frontier from a set of candidate solutions.
    ///
    /// Returns the subset of candidates that are non-dominated.
    #[must_use]
    pub fn compute(objectives: Vec<String>, candidates: &[ParetoSolution]) -> Self {
        let solutions = compute_pareto_frontier_multi(candidates);
        Self {
            objectives,
            solutions,
        }
    }

    /// Number of solutions on the frontier.
    #[must_use]
    pub fn len(&self) -> usize {
        self.solutions.len()
    }

    /// Whether the frontier is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.solutions.is_empty()
    }

    /// Return the model IDs on the frontier.
    #[must_use]
    pub fn model_ids(&self) -> Vec<String> {
        self.solutions.iter().map(|s| s.model_id.clone()).collect()
    }
}

/// Returns `true` if `a` dominates `b` — i.e. `a` is at least as good in
/// every objective and strictly better in at least one.
#[must_use]
pub fn is_dominated(a: &ParetoSolution, b: &ParetoSolution) -> bool {
    let len = a.values.len().min(b.values.len());
    if len == 0 {
        return false;
    }

    let mut all_geq = true;
    let mut any_gt = false;

    for i in 0..len {
        if a.values[i] < b.values[i] {
            all_geq = false;
            break;
        }
        if a.values[i] > b.values[i] {
            any_gt = true;
        }
    }

    all_geq && any_gt
}

/// Compute the non-dominated set from a list of candidate solutions.
#[must_use]
pub fn compute_pareto_frontier_multi(candidates: &[ParetoSolution]) -> Vec<ParetoSolution> {
    let mut frontier = Vec::new();

    for candidate in candidates {
        let dominated = candidates
            .iter()
            .any(|other| other.model_id != candidate.model_id && is_dominated(other, candidate));

        if !dominated {
            frontier.push(candidate.clone());
        }
    }

    frontier.sort_by(|a, b| a.model_id.cmp(&b.model_id));
    frontier
}

#[cfg(test)]
mod tests {
    use super::{ModelObservation, ParetoWeights, compute_pareto_frontier, scalarize};
    use std::collections::HashMap;

    #[test]
    fn pareto_frontier_keeps_non_dominated_models_4d() {
        let mut stats = HashMap::new();
        // model-a: best quality but worst latency and cost.
        stats.insert(
            "model-a".to_string(),
            ModelObservation {
                pass_rate: 0.95,
                cost_per_success: 15.0,
                avg_latency_ms: 2000.0,
                reliability: 0.90,
                observations: 20,
            },
        );
        // model-b: worst on everything.
        stats.insert(
            "model-b".to_string(),
            ModelObservation {
                pass_rate: 0.60,
                cost_per_success: 20.0,
                avg_latency_ms: 3000.0,
                reliability: 0.70,
                observations: 20,
            },
        );
        // model-c: best cost and latency but lower quality.
        stats.insert(
            "model-c".to_string(),
            ModelObservation {
                pass_rate: 0.80,
                cost_per_success: 5.0,
                avg_latency_ms: 500.0,
                reliability: 0.95,
                observations: 20,
            },
        );

        let frontier = compute_pareto_frontier(&stats);

        // model-a and model-c are non-dominated (tradeoff between quality and cost/latency).
        // model-b is dominated by model-c on ALL 4 objectives.
        assert!(frontier.contains(&"model-a".to_string()));
        assert!(frontier.contains(&"model-c".to_string()));
        assert!(!frontier.contains(&"model-b".to_string()));
    }

    #[test]
    fn pareto_frontier_all_on_frontier_when_no_dominance() {
        let mut stats = HashMap::new();
        stats.insert(
            "model-a".to_string(),
            ModelObservation {
                pass_rate: 0.90,
                cost_per_success: 10.0,
                avg_latency_ms: 1000.0,
                reliability: 0.80,
                observations: 20,
            },
        );
        stats.insert(
            "model-b".to_string(),
            ModelObservation {
                pass_rate: 0.70,
                cost_per_success: 12.0,
                avg_latency_ms: 900.0,
                reliability: 0.95,
                observations: 20,
            },
        );

        let frontier = compute_pareto_frontier(&stats);
        // Neither dominates the other (model-a better quality, model-b better latency + reliability).
        assert_eq!(frontier.len(), 2);
    }

    #[test]
    fn scalarize_equal_weights() {
        let obs = ModelObservation {
            pass_rate: 0.90,
            cost_per_success: 10.0,
            avg_latency_ms: 1000.0,
            reliability: 0.95,
            observations: 20,
        };
        let score = scalarize(&obs, &ParetoWeights::equal());
        assert!(score > 0.0 && score <= 1.0, "scalarized score = {score}");
    }

    #[test]
    fn scalarize_quality_first_favors_high_pass_rate() {
        let high_quality = ModelObservation {
            pass_rate: 0.95,
            cost_per_success: 50.0,
            avg_latency_ms: 5000.0,
            reliability: 0.90,
            observations: 20,
        };
        let low_quality = ModelObservation {
            pass_rate: 0.40,
            cost_per_success: 5.0,
            avg_latency_ms: 500.0,
            reliability: 0.95,
            observations: 20,
        };
        let qw = ParetoWeights::quality_first();
        assert!(
            scalarize(&high_quality, &qw) > scalarize(&low_quality, &qw),
            "quality-first should prefer high pass rate"
        );
    }

    #[test]
    fn scalarize_cost_first_favors_cheap_model() {
        let expensive = ModelObservation {
            pass_rate: 0.95,
            cost_per_success: 80.0,
            avg_latency_ms: 1000.0,
            reliability: 0.90,
            observations: 20,
        };
        let cheap = ModelObservation {
            pass_rate: 0.60,
            cost_per_success: 2.0,
            avg_latency_ms: 1000.0,
            reliability: 0.90,
            observations: 20,
        };
        let cw = ParetoWeights::cost_first();
        assert!(
            scalarize(&cheap, &cw) > scalarize(&expensive, &cw),
            "cost-first should prefer cheap model"
        );
    }

    #[test]
    fn is_dominated_basic() {
        use super::{ParetoSolution, is_dominated};

        let a = ParetoSolution {
            values: vec![0.9, 0.8, 0.7],
            model_id: "a".into(),
        };
        let b = ParetoSolution {
            values: vec![0.5, 0.5, 0.5],
            model_id: "b".into(),
        };
        // a dominates b (better in all objectives)
        assert!(is_dominated(&a, &b));
        // b does not dominate a
        assert!(!is_dominated(&b, &a));
    }

    #[test]
    fn multi_objective_frontier() {
        use super::{ParetoFrontier, ParetoSolution};

        let candidates = vec![
            // High quality, high cost (negate cost so higher = cheaper).
            ParetoSolution {
                values: vec![0.9, 0.3],
                model_id: "high-quality".into(),
            },
            // Low quality, low cost.
            ParetoSolution {
                values: vec![0.3, 0.9],
                model_id: "low-cost".into(),
            },
            // Dominated: worse quality than high-quality AND worse cost than low-cost,
            // but NOT dominated by either individually since each is only better on one axis.
            // To actually be dominated we need a point that loses on ALL axes to another.
            ParetoSolution {
                values: vec![0.2, 0.2],
                model_id: "dominated".into(),
            },
        ];

        let frontier =
            ParetoFrontier::compute(vec!["quality".into(), "neg_cost".into()], &candidates);

        // "dominated" (0.2, 0.2) is dominated by both "high-quality" (0.9>0.2, 0.3>0.2)
        // and "low-cost" (0.3>0.2, 0.9>0.2).
        assert_eq!(frontier.len(), 2);
        let ids = frontier.model_ids();
        assert!(ids.contains(&"high-quality".to_string()));
        assert!(ids.contains(&"low-cost".to_string()));
        assert!(!ids.contains(&"dominated".to_string()));
    }

    #[test]
    fn empty_frontier() {
        use super::ParetoFrontier;
        let frontier = ParetoFrontier::compute(vec!["a".into()], &[]);
        assert!(frontier.is_empty());
    }

    // ── scalarize() with known inputs ──────────────────────────────────────

    #[test]
    fn scalarize_known_inputs_equal_weights() {
        // All objectives at their normalized midpoints.
        // pass_rate=0.5, cost_per_success=50 → cost_norm=1-(50/100)=0.5,
        // avg_latency_ms=30000 → lat_norm=1-(30000/60000)=0.5,
        // reliability=0.5.
        // Equal weights (1,1,1,1) → (0.5+0.5+0.5+0.5)/4 = 0.5
        let obs = ModelObservation {
            pass_rate: 0.5,
            cost_per_success: 50.0,
            avg_latency_ms: 30_000.0,
            reliability: 0.5,
            observations: 10,
        };
        let score = scalarize(&obs, &ParetoWeights::equal());
        assert!((score - 0.5).abs() < 1e-10, "expected 0.5, got {score}");
    }

    #[test]
    fn scalarize_known_inputs_quality_first() {
        // pass_rate=1.0, cost=0 → cost_norm=1.0, latency=0 → lat_norm=1.0,
        // reliability=1.0.
        // quality_first weights: (3,1,1,1), total=6.
        // score = (3*1.0 + 1*1.0 + 1*1.0 + 1*1.0)/6 = 6/6 = 1.0
        let obs = ModelObservation {
            pass_rate: 1.0,
            cost_per_success: 0.0,
            avg_latency_ms: 0.0,
            reliability: 1.0,
            observations: 10,
        };
        let score = scalarize(&obs, &ParetoWeights::quality_first());
        assert!(
            (score - 1.0).abs() < 1e-10,
            "perfect model should score 1.0, got {score}"
        );
    }

    #[test]
    fn scalarize_known_inputs_worst_case() {
        // pass_rate=0.0, cost=100 → cost_norm=0.0, latency=60000 → lat_norm=0.0,
        // reliability=0.0.
        // Equal weights → (0+0+0+0)/4 = 0.0
        let obs = ModelObservation {
            pass_rate: 0.0,
            cost_per_success: 100.0,
            avg_latency_ms: 60_000.0,
            reliability: 0.0,
            observations: 10,
        };
        let score = scalarize(&obs, &ParetoWeights::equal());
        assert!(
            score.abs() < 1e-10,
            "worst model should score 0.0, got {score}"
        );
    }

    #[test]
    fn scalarize_cost_clamped_above_max() {
        // cost_per_success = 200 → clamped to 100 → cost_norm = 0.0
        let obs = ModelObservation {
            pass_rate: 0.5,
            cost_per_success: 200.0,
            avg_latency_ms: 30_000.0,
            reliability: 0.5,
            observations: 10,
        };
        let score = scalarize(&obs, &ParetoWeights::equal());
        // (0.5 + 0.0 + 0.5 + 0.5)/4 = 0.375
        assert!(
            (score - 0.375).abs() < 1e-10,
            "expected 0.375 with clamped cost, got {score}"
        );
    }

    #[test]
    fn scalarize_latency_clamped_above_max() {
        // avg_latency_ms = 120_000 → clamped to 60_000 → lat_norm = 0.0
        let obs = ModelObservation {
            pass_rate: 0.5,
            cost_per_success: 50.0,
            avg_latency_ms: 120_000.0,
            reliability: 0.5,
            observations: 10,
        };
        let score = scalarize(&obs, &ParetoWeights::equal());
        // (0.5 + 0.5 + 0.0 + 0.5)/4 = 0.375
        assert!(
            (score - 0.375).abs() < 1e-10,
            "expected 0.375 with clamped latency, got {score}"
        );
    }

    #[test]
    fn scalarize_asymmetric_weights() {
        // pass_rate=0.8, cost=20 → cost_norm=0.8, latency=6000 → lat_norm=0.9,
        // reliability=0.7.
        // Weights: quality=2, cost=3, latency=0, reliability=5 → total=10.
        // score = (2*0.8 + 3*0.8 + 0*0.9 + 5*0.7)/10 = (1.6+2.4+0+3.5)/10 = 7.5/10 = 0.75
        let obs = ModelObservation {
            pass_rate: 0.8,
            cost_per_success: 20.0,
            avg_latency_ms: 6_000.0,
            reliability: 0.7,
            observations: 10,
        };
        let weights = ParetoWeights {
            quality: 2.0,
            cost: 3.0,
            latency: 0.0,
            reliability: 5.0,
        };
        let score = scalarize(&obs, &weights);
        assert!((score - 0.75).abs() < 1e-10, "expected 0.75, got {score}");
    }

    // ── ParetoWeights preset verification ──────────────────────────────────

    #[test]
    fn pareto_weights_quality_first_values() {
        let w = ParetoWeights::quality_first();
        assert!((w.quality - 3.0).abs() < f64::EPSILON);
        assert!((w.cost - 1.0).abs() < f64::EPSILON);
        assert!((w.latency - 1.0).abs() < f64::EPSILON);
        assert!((w.reliability - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pareto_weights_cost_first_values() {
        let w = ParetoWeights::cost_first();
        assert!((w.quality - 1.0).abs() < f64::EPSILON);
        assert!((w.cost - 3.0).abs() < f64::EPSILON);
        assert!((w.latency - 1.0).abs() < f64::EPSILON);
        assert!((w.reliability - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pareto_weights_equal_values() {
        let w = ParetoWeights::equal();
        assert!((w.quality - 1.0).abs() < f64::EPSILON);
        assert!((w.cost - 1.0).abs() < f64::EPSILON);
        assert!((w.latency - 1.0).abs() < f64::EPSILON);
        assert!((w.reliability - 1.0).abs() < f64::EPSILON);
    }

    // ── compute_pareto_frontier() 2-objective cases ────────────────────────

    #[test]
    fn pareto_frontier_2d_simple_tradeoff() {
        // Two models trading off quality vs cost. Both should be on the frontier.
        let mut stats = HashMap::new();
        stats.insert(
            "fast-cheap".into(),
            ModelObservation {
                pass_rate: 0.50,
                cost_per_success: 2.0,
                avg_latency_ms: 500.0,
                reliability: 0.90,
                observations: 20,
            },
        );
        stats.insert(
            "slow-good".into(),
            ModelObservation {
                pass_rate: 0.95,
                cost_per_success: 50.0,
                avg_latency_ms: 5000.0,
                reliability: 0.90,
                observations: 20,
            },
        );

        let frontier = compute_pareto_frontier(&stats);
        assert_eq!(frontier.len(), 2);
        assert!(frontier.contains(&"fast-cheap".to_string()));
        assert!(frontier.contains(&"slow-good".to_string()));
    }

    // ── Edge case: single point ────────────────────────────────────────────

    #[test]
    fn pareto_frontier_single_point() {
        let mut stats = HashMap::new();
        stats.insert(
            "only-model".into(),
            ModelObservation {
                pass_rate: 0.80,
                cost_per_success: 10.0,
                avg_latency_ms: 1000.0,
                reliability: 0.90,
                observations: 5,
            },
        );

        let frontier = compute_pareto_frontier(&stats);
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0], "only-model");
    }

    #[test]
    fn pareto_frontier_multi_single_point() {
        use super::{ParetoFrontier, ParetoSolution};

        let candidates = vec![ParetoSolution {
            values: vec![0.5, 0.5],
            model_id: "lonely".into(),
        }];
        let frontier = ParetoFrontier::compute(vec!["x".into(), "y".into()], &candidates);
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier.model_ids(), vec!["lonely".to_string()]);
    }

    // ── Edge case: all dominated (one clearly best) ────────────────────────

    #[test]
    fn pareto_frontier_all_dominated_by_one() {
        let mut stats = HashMap::new();
        // "king" dominates every other model on all 4 objectives.
        stats.insert(
            "king".into(),
            ModelObservation {
                pass_rate: 0.99,
                cost_per_success: 1.0,
                avg_latency_ms: 100.0,
                reliability: 0.99,
                observations: 50,
            },
        );
        stats.insert(
            "pawn-a".into(),
            ModelObservation {
                pass_rate: 0.50,
                cost_per_success: 10.0,
                avg_latency_ms: 2000.0,
                reliability: 0.70,
                observations: 50,
            },
        );
        stats.insert(
            "pawn-b".into(),
            ModelObservation {
                pass_rate: 0.60,
                cost_per_success: 15.0,
                avg_latency_ms: 3000.0,
                reliability: 0.60,
                observations: 50,
            },
        );
        stats.insert(
            "pawn-c".into(),
            ModelObservation {
                pass_rate: 0.40,
                cost_per_success: 20.0,
                avg_latency_ms: 5000.0,
                reliability: 0.50,
                observations: 50,
            },
        );

        let frontier = compute_pareto_frontier(&stats);
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0], "king");
    }

    #[test]
    fn pareto_frontier_multi_all_dominated() {
        use super::{ParetoFrontier, ParetoSolution};

        let candidates = vec![
            ParetoSolution {
                values: vec![1.0, 1.0],
                model_id: "best".into(),
            },
            ParetoSolution {
                values: vec![0.5, 0.5],
                model_id: "mid".into(),
            },
            ParetoSolution {
                values: vec![0.1, 0.1],
                model_id: "worst".into(),
            },
        ];
        let frontier = ParetoFrontier::compute(vec!["a".into(), "b".into()], &candidates);
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier.model_ids(), vec!["best".to_string()]);
    }

    // ── Edge case: all non-dominated ───────────────────────────────────────

    #[test]
    fn pareto_frontier_all_non_dominated() {
        // Each model is best on exactly one objective, so none is dominated.
        let mut stats = HashMap::new();
        stats.insert(
            "quality-king".into(),
            ModelObservation {
                pass_rate: 0.99,
                cost_per_success: 90.0,
                avg_latency_ms: 50_000.0,
                reliability: 0.50,
                observations: 20,
            },
        );
        stats.insert(
            "cost-king".into(),
            ModelObservation {
                pass_rate: 0.30,
                cost_per_success: 1.0,
                avg_latency_ms: 50_000.0,
                reliability: 0.50,
                observations: 20,
            },
        );
        stats.insert(
            "latency-king".into(),
            ModelObservation {
                pass_rate: 0.30,
                cost_per_success: 90.0,
                avg_latency_ms: 50.0,
                reliability: 0.50,
                observations: 20,
            },
        );
        stats.insert(
            "reliability-king".into(),
            ModelObservation {
                pass_rate: 0.30,
                cost_per_success: 90.0,
                avg_latency_ms: 50_000.0,
                reliability: 0.99,
                observations: 20,
            },
        );

        let frontier = compute_pareto_frontier(&stats);
        assert_eq!(frontier.len(), 4);
    }

    #[test]
    fn pareto_frontier_multi_all_non_dominated() {
        use super::{ParetoFrontier, ParetoSolution};

        // Three points along the Pareto front: each trades off differently.
        let candidates = vec![
            ParetoSolution {
                values: vec![1.0, 0.0],
                model_id: "a".into(),
            },
            ParetoSolution {
                values: vec![0.0, 1.0],
                model_id: "b".into(),
            },
            ParetoSolution {
                values: vec![0.5, 0.5],
                model_id: "c".into(),
            },
        ];
        let frontier = ParetoFrontier::compute(vec!["x".into(), "y".into()], &candidates);
        // None dominates any other.
        assert_eq!(frontier.len(), 3);
    }

    // ── 4-objective multi-objective frontier ───────────────────────────────

    #[test]
    fn pareto_frontier_multi_4_objectives() {
        use super::{ParetoFrontier, ParetoSolution};

        let candidates = vec![
            // Best on obj0, poor elsewhere.
            ParetoSolution {
                values: vec![1.0, 0.1, 0.1, 0.1],
                model_id: "specialist-0".into(),
            },
            // Best on obj1, poor elsewhere.
            ParetoSolution {
                values: vec![0.1, 1.0, 0.1, 0.1],
                model_id: "specialist-1".into(),
            },
            // Best on obj2, poor elsewhere.
            ParetoSolution {
                values: vec![0.1, 0.1, 1.0, 0.1],
                model_id: "specialist-2".into(),
            },
            // Best on obj3, poor elsewhere.
            ParetoSolution {
                values: vec![0.1, 0.1, 0.1, 1.0],
                model_id: "specialist-3".into(),
            },
            // Dominated: worse than specialist-0 on obj0, and not better on any other.
            ParetoSolution {
                values: vec![0.05, 0.05, 0.05, 0.05],
                model_id: "dominated-4d".into(),
            },
        ];

        let frontier = ParetoFrontier::compute(
            vec!["q".into(), "c".into(), "l".into(), "r".into()],
            &candidates,
        );
        assert_eq!(frontier.len(), 4);
        let ids = frontier.model_ids();
        assert!(ids.contains(&"specialist-0".to_string()));
        assert!(ids.contains(&"specialist-1".to_string()));
        assert!(ids.contains(&"specialist-2".to_string()));
        assert!(ids.contains(&"specialist-3".to_string()));
        assert!(!ids.contains(&"dominated-4d".to_string()));
    }

    #[test]
    fn pareto_frontier_multi_4_objectives_balanced_survivor() {
        use super::{ParetoFrontier, ParetoSolution};

        // A balanced model that is not dominated by any specialist.
        let candidates = vec![
            ParetoSolution {
                values: vec![0.9, 0.2, 0.2, 0.2],
                model_id: "specialist".into(),
            },
            ParetoSolution {
                values: vec![0.6, 0.6, 0.6, 0.6],
                model_id: "balanced".into(),
            },
        ];

        let frontier = ParetoFrontier::compute(
            vec!["a".into(), "b".into(), "c".into(), "d".into()],
            &candidates,
        );
        // Neither dominates the other: specialist better on a, balanced better on b,c,d.
        assert_eq!(frontier.len(), 2);
    }

    // ── is_dominated edge cases ────────────────────────────────────────────

    #[test]
    fn is_dominated_equal_points() {
        use super::{ParetoSolution, is_dominated};

        let a = ParetoSolution {
            values: vec![0.5, 0.5],
            model_id: "a".into(),
        };
        let b = ParetoSolution {
            values: vec![0.5, 0.5],
            model_id: "b".into(),
        };
        // Equal on all objectives: neither dominates the other (need strictly better on at least one).
        assert!(!is_dominated(&a, &b));
        assert!(!is_dominated(&b, &a));
    }

    #[test]
    fn is_dominated_empty_values() {
        use super::{ParetoSolution, is_dominated};

        let a = ParetoSolution {
            values: vec![],
            model_id: "a".into(),
        };
        let b = ParetoSolution {
            values: vec![],
            model_id: "b".into(),
        };
        assert!(!is_dominated(&a, &b));
    }

    #[test]
    fn is_dominated_strictly_better_on_one() {
        use super::{ParetoSolution, is_dominated};

        let a = ParetoSolution {
            values: vec![0.5, 0.6],
            model_id: "a".into(),
        };
        let b = ParetoSolution {
            values: vec![0.5, 0.5],
            model_id: "b".into(),
        };
        // a >= b on both, strictly better on second → a dominates b.
        assert!(is_dominated(&a, &b));
        assert!(!is_dominated(&b, &a));
    }

    // ── compute_pareto_frontier with identical models ──────────────────────

    #[test]
    fn pareto_frontier_identical_models_all_survive() {
        let mut stats = HashMap::new();
        let obs = ModelObservation {
            pass_rate: 0.80,
            cost_per_success: 10.0,
            avg_latency_ms: 1000.0,
            reliability: 0.90,
            observations: 20,
        };
        stats.insert("clone-a".into(), obs.clone());
        stats.insert("clone-b".into(), obs.clone());
        stats.insert("clone-c".into(), obs);

        let frontier = compute_pareto_frontier(&stats);
        // Identical points cannot dominate each other (need strictly better on at least one).
        assert_eq!(frontier.len(), 3);
    }

    // ── Empty map ──────────────────────────────────────────────────────────

    #[test]
    fn pareto_frontier_empty_map() {
        let stats: HashMap<String, ModelObservation> = HashMap::new();
        let frontier = compute_pareto_frontier(&stats);
        assert!(frontier.is_empty());
    }

    // ── Sorted output ──────────────────────────────────────────────────────

    #[test]
    fn pareto_frontier_output_is_sorted() {
        let mut stats = HashMap::new();
        // Insert in reverse alphabetical order; output should still be sorted.
        for name in ["zulu", "mike", "alpha", "bravo"] {
            stats.insert(
                name.into(),
                ModelObservation {
                    pass_rate: 0.80,
                    cost_per_success: 10.0,
                    avg_latency_ms: 1000.0,
                    reliability: 0.90,
                    observations: 10,
                },
            );
        }
        let frontier = compute_pareto_frontier(&stats);
        let mut sorted = frontier.clone();
        sorted.sort();
        assert_eq!(frontier, sorted);
    }
}
