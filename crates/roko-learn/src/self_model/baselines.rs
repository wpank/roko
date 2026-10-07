//! The replay baselines, the production model ladder among them: backlog task 6117.
//!
//! H4 compares the self-model's policies with five baselines (S04 SC2, S09 R-H4), each a
//! [`RoutingPolicy`]:
//!
//! - H4-B0 ([`StaticArm`]): one static arm, or the best single arm by cross-fit (D34);
//! - H4-B1 ([`FrugalCascade`]): FrugalGPT, rung 0 and one rung up on each failure, K_max = 2;
//! - H4-BL ([`ProductionLadder`]): the model ladder as `[routing.ladder]` routes plan tasks
//!   today: the tier's start rung, one rung up after two agent-blamed failures, at most twice;
//! - H4-B2 ([`RouterPolicy`]): the real [`CascadeRouter`], fed with the replayed outcomes, never
//!   a copy of it;
//! - H4-B3 ([`Oracle`]): the cheapest arm that passes on the recorded seed.
//!
//! Every policy is deterministic: none draws at random.

use std::collections::{BTreeMap, HashMap};

use roko_core::config::routing::LadderConfig;
use roko_core::task::{TaskComplexityBand, TaskTier};

use crate::cascade_router::CascadeRouter;
use crate::model_router::RoutingContext;
use crate::provider_health::ProviderHealthRegistry;

/// Agent-blamed failures on one rung before the ladder climbs. This mirrors the private
/// `FAILURES_PER_RUNG` of roko-cli's `graph_task_dispatch/ladder.rs`; change both together.
pub const LADDER_FAILURES_PER_RUNG: u32 = 2;

/// Most rungs the ladder climbs above its start (K_max). This mirrors the private
/// `MAX_ESCALATIONS` of roko-cli's `graph_task_dispatch/ladder.rs`; change both together.
pub const LADDER_MAX_ESCALATIONS: u32 = 2;

/// Most escalations, or retries, of the other cascades (S04 §4.10).
pub const K_MAX: u32 = 2;

/// The most attempts a replayed task gets: two on the start rung and on each of the rungs the
/// ladder may climb, as its retry budget allows.
pub const MAX_ATTEMPTS: u32 = LADDER_FAILURES_PER_RUNG * (LADDER_MAX_ESCALATIONS + 1);

/// A task as a routing policy sees it before an attempt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteTask {
    /// The task (a plan task id, or a benchmark instance id).
    pub task_id: String,
    /// The task family: the tier for plan tasks.
    pub family: String,
    /// The role dispatched.
    pub role: String,
    /// The difficulty tier (`mechanical`, `focused`, `integrative`, `architectural`).
    pub tier: String,
    /// The task's `rung` hint, when it names one.
    pub rung_hint: Option<String>,
}

/// What one attempt on an arm revealed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttemptResult {
    /// The attempt passed.
    pub passed: bool,
    /// The failure is the agent's (a failed gate, a turn cap), not the provider's or the
    /// harness's.
    pub agent_blamed: bool,
    /// What the attempt cost, in USD at the price snapshot.
    pub cost_usd: f64,
}

/// A routing policy: where a task starts, and after each attempt, where it goes next.
pub trait RoutingPolicy {
    /// The policy's id (`H4-B0`, `lcb_aci`, ...).
    fn name(&self) -> &str;
    /// The arm `task` starts on; `None` when the policy does not attempt it.
    fn start(&mut self, task: &RouteTask) -> Option<String>;
    /// After `result` on `arm`, the arm of the next attempt, or `None` to stop. Every attempt
    /// ends in a call, so an online policy learns here, a pass included.
    fn next(&mut self, task: &RouteTask, arm: &str, result: &AttemptResult) -> Option<String>;
}

/// One recorded outcome of an arm, for choosing the best single arm.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmOutcome {
    /// The arm.
    pub arm: String,
    /// Whether it passed.
    pub passed: bool,
    /// What it cost.
    pub cost_usd: f64,
}

/// H4-B0: one static arm, attempted once.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticArm {
    arm: String,
}

impl StaticArm {
    /// The static arm `arm`.
    #[must_use]
    pub fn new(arm: impl Into<String>) -> Self {
        Self { arm: arm.into() }
    }

    /// The best single arm of `train` by cost per resolved task, fewest dollars per pass
    /// first; an arm that never passed ranks last. Cross-fit by passing only the folds the
    /// policy is not scored on (D34). `None` for no outcomes.
    #[must_use]
    pub fn best_single(train: &[ArmOutcome]) -> Option<Self> {
        let mut arms: BTreeMap<&str, (f64, u32)> = BTreeMap::new();
        for outcome in train {
            let entry = arms.entry(outcome.arm.as_str()).or_insert((0.0, 0));
            entry.0 += outcome.cost_usd;
            entry.1 += u32::from(outcome.passed);
        }
        let cost_per_pass = |(cost, passes): (f64, u32)| {
            if passes == 0 {
                f64::INFINITY
            } else {
                cost / f64::from(passes)
            }
        };
        arms.into_iter()
            .min_by(|a, b| cost_per_pass(a.1).total_cmp(&cost_per_pass(b.1)))
            .map(|(arm, _)| Self::new(arm))
    }
}

impl RoutingPolicy for StaticArm {
    fn name(&self) -> &str {
        "H4-B0"
    }

    fn start(&mut self, _task: &RouteTask) -> Option<String> {
        Some(self.arm.clone())
    }

    fn next(&mut self, _task: &RouteTask, _arm: &str, _result: &AttemptResult) -> Option<String> {
        None
    }
}

/// H4-B1, FrugalGPT: start on rung 0 and climb one rung on each failure, at most `k_max`
/// times, with no forecasts.
#[derive(Debug, Clone, PartialEq)]
pub struct FrugalCascade {
    rungs: Vec<String>,
    k_max: u32,
    index: usize,
}

impl FrugalCascade {
    /// The cascade over `rungs`, cheapest first, with [`K_MAX`] climbs.
    #[must_use]
    pub const fn new(rungs: Vec<String>) -> Self {
        Self {
            rungs,
            k_max: K_MAX,
            index: 0,
        }
    }
}

impl RoutingPolicy for FrugalCascade {
    fn name(&self) -> &str {
        "H4-B1"
    }

    fn start(&mut self, _task: &RouteTask) -> Option<String> {
        self.index = 0;
        self.rungs.first().cloned()
    }

    fn next(&mut self, _task: &RouteTask, _arm: &str, result: &AttemptResult) -> Option<String> {
        let climbs = u32::try_from(self.index).unwrap_or(u32::MAX);
        if result.passed || climbs >= self.k_max || self.index + 1 >= self.rungs.len() {
            return None;
        }
        self.index += 1;
        self.rungs.get(self.index).cloned()
    }
}

/// H4-BL: the production model ladder. The start rung comes from [`LadderConfig::resolve`]
/// for the task's role, tier and `rung` hint; the task climbs one usable rung after
/// [`LADDER_FAILURES_PER_RUNG`] agent-blamed failures, at most [`LADDER_MAX_ESCALATIONS`]
/// times, and stops once the rung it may not climb from has failed that often. Provider and
/// harness failures retry the rung without counting, within [`MAX_ATTEMPTS`].
#[derive(Debug, Clone, PartialEq)]
pub struct ProductionLadder {
    config: LadderConfig,
    /// Each rung model's arm; a rung whose model has no arm cannot run.
    arms: BTreeMap<String, String>,
    /// The arms of the task's usable rungs, cheapest first.
    usable: Vec<String>,
    position: usize,
    failures_on_rung: u32,
    escalations: u32,
    attempts: u32,
}

impl ProductionLadder {
    /// The ladder `config`, whose rung models run as `arms` (model to arm id).
    #[must_use]
    pub const fn new(config: LadderConfig, arms: BTreeMap<String, String>) -> Self {
        Self {
            config,
            arms,
            usable: Vec::new(),
            position: 0,
            failures_on_rung: 0,
            escalations: 0,
            attempts: 0,
        }
    }
}

impl RoutingPolicy for ProductionLadder {
    fn name(&self) -> &str {
        "H4-BL"
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        let tier = TaskTier::parse(&task.tier).unwrap_or_default();
        let arms = &self.arms;
        let resolved =
            self.config
                .resolve(&task.role, tier, task.rung_hint.as_deref(), |model| {
                    arms.contains_key(model)
                })?;
        self.usable = resolved
            .usable
            .iter()
            .filter_map(|&index| arms.get(&resolved.rungs[index].model).cloned())
            .collect();
        self.position = resolved
            .usable
            .iter()
            .position(|&index| index == resolved.start)
            .unwrap_or(0);
        self.failures_on_rung = 0;
        self.escalations = 0;
        self.attempts = 0;
        self.usable.get(self.position).cloned()
    }

    fn next(&mut self, _task: &RouteTask, _arm: &str, result: &AttemptResult) -> Option<String> {
        self.attempts += 1;
        if result.passed || self.attempts >= MAX_ATTEMPTS {
            return None;
        }
        if result.agent_blamed {
            self.failures_on_rung += 1;
        }
        if self.failures_on_rung >= LADDER_FAILURES_PER_RUNG {
            let can_climb = self.position + 1 < self.usable.len();
            if !can_climb || self.escalations >= LADDER_MAX_ESCALATIONS {
                // The rung it may not climb from is exhausted.
                return None;
            }
            self.position += 1;
            self.escalations += 1;
            self.failures_on_rung = 0;
        }
        self.usable.get(self.position).cloned()
    }
}

/// H4-B2: the real [`CascadeRouter`], routing each attempt with
/// [`CascadeRouter::route_with_health_scored`] and learning each replayed outcome through
/// [`CascadeRouter::observe_outcome`]. It retries on the router's pick at most [`K_MAX`]
/// times. Nothing of the router is copied, so the baseline follows the router as it changes.
pub struct RouterPolicy {
    router: CascadeRouter,
    health: ProviderHealthRegistry,
    /// Each model's provider.
    providers: HashMap<String, String>,
    /// Each model's arm, and back.
    arms: BTreeMap<String, String>,
    models: BTreeMap<String, String>,
    context: RoutingContext,
    attempts: u32,
}

impl RouterPolicy {
    /// A fresh router over `models`: (model slug, provider, arm id) triples. `None` for none.
    #[must_use]
    pub fn new(models: &[(&str, &str, &str)]) -> Option<Self> {
        if models.is_empty() {
            return None;
        }
        let slugs = models
            .iter()
            .map(|(slug, _, _)| (*slug).to_string())
            .collect();
        Some(Self {
            router: CascadeRouter::new(slugs),
            health: ProviderHealthRegistry::new(),
            providers: models
                .iter()
                .map(|(slug, provider, _)| ((*slug).to_string(), (*provider).to_string()))
                .collect(),
            arms: models
                .iter()
                .map(|(slug, _, arm)| ((*slug).to_string(), (*arm).to_string()))
                .collect(),
            models: models
                .iter()
                .map(|(slug, _, arm)| ((*arm).to_string(), (*slug).to_string()))
                .collect(),
            context: RoutingContext::default(),
            attempts: 0,
        })
    }

    /// The routing context of attempt `attempt` of `task`, after `previous` failed.
    #[must_use]
    pub fn context(task: &RouteTask, attempt: u32, previous: Option<&str>) -> RoutingContext {
        let complexity = match TaskTier::parse(&task.tier).unwrap_or_default() {
            TaskTier::Mechanical => TaskComplexityBand::Fast,
            TaskTier::Focused | TaskTier::Integrative => TaskComplexityBand::Standard,
            TaskTier::Architectural => TaskComplexityBand::Complex,
        };
        RoutingContext {
            complexity,
            iteration: attempt,
            has_prior_failure: attempt > 1,
            previous_model: previous.map(str::to_string),
            ..RoutingContext::default()
        }
    }

    /// The router's pick for the current context, as an arm.
    fn route(&self) -> Option<String> {
        let pick = self.router.route_with_health_scored(
            &self.context,
            &self.health,
            &self.providers,
            None,
            None,
        );
        self.arms.get(&pick.primary.slug).cloned()
    }

    /// The router, for reading its state.
    #[must_use]
    pub const fn router(&self) -> &CascadeRouter {
        &self.router
    }
}

impl RoutingPolicy for RouterPolicy {
    fn name(&self) -> &str {
        "H4-B2"
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        self.attempts = 0;
        self.context = Self::context(task, 1, None);
        self.route()
    }

    fn next(&mut self, task: &RouteTask, arm: &str, result: &AttemptResult) -> Option<String> {
        self.attempts += 1;
        let model = self.models.get(arm)?.clone();
        if let Some(index) = self.router.model_index_for_slug(&model) {
            let features = self.context.to_features_for_model(Some(model.as_str()));
            let reward = if result.passed { 1.0 } else { 0.0 };
            self.router
                .observe_outcome(features, index, reward, result.passed);
        }
        if result.passed || self.attempts > K_MAX {
            return None;
        }
        self.context = Self::context(task, self.attempts + 1, Some(model.as_str()));
        self.route()
    }
}

/// H4-B3: the oracle, the cheapest arm that passes on the recorded seed. It abstains from a
/// task no arm passes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Oracle {
    choices: BTreeMap<String, String>,
}

impl Oracle {
    /// The oracle of `cells`, keyed (task, arm): each task's cheapest passing arm, the first
    /// by arm id on a tie.
    #[must_use]
    pub fn new(cells: &BTreeMap<(String, String), AttemptResult>) -> Self {
        let mut best: BTreeMap<String, (String, f64)> = BTreeMap::new();
        for ((task, arm), result) in cells.iter().filter(|(_, result)| result.passed) {
            let cheaper = best
                .get(task)
                .is_none_or(|(_, cost)| result.cost_usd < *cost);
            if cheaper {
                best.insert(task.clone(), (arm.clone(), result.cost_usd));
            }
        }
        Self {
            choices: best
                .into_iter()
                .map(|(task, (arm, _))| (task, arm))
                .collect(),
        }
    }
}

impl RoutingPolicy for Oracle {
    fn name(&self) -> &str {
        "H4-B3"
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        self.choices.get(&task.task_id).cloned()
    }

    fn next(&mut self, _task: &RouteTask, _arm: &str, _result: &AttemptResult) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAIL: AttemptResult = AttemptResult {
        passed: false,
        agent_blamed: true,
        cost_usd: 0.01,
    };
    const INFRA: AttemptResult = AttemptResult {
        passed: false,
        agent_blamed: false,
        cost_usd: 0.0,
    };

    fn task(tier: &str) -> RouteTask {
        RouteTask {
            task_id: "T1".to_string(),
            family: tier.to_string(),
            role: "implementer".to_string(),
            tier: tier.to_string(),
            rung_hint: None,
        }
    }

    /// The arms the policy runs `task` on, each attempt ending in `results` in turn.
    fn run(
        policy: &mut dyn RoutingPolicy,
        task: &RouteTask,
        results: &[AttemptResult],
    ) -> Vec<String> {
        let mut tried = Vec::new();
        let mut arm = policy.start(task);
        for result in results {
            let Some(current) = arm else {
                break;
            };
            arm = policy.next(task, &current, result);
            tried.push(current);
        }
        tried
    }

    fn default_ladder() -> ProductionLadder {
        let config = LadderConfig::default();
        let arms = config
            .rungs
            .iter()
            .map(|rung| (rung.model.clone(), rung.name.clone()))
            .collect();
        ProductionLadder::new(config, arms)
    }

    #[test]
    fn production_ladder_baseline_follows_ladder_rules() {
        let mut ladder = default_ladder();
        // A focused task starts on `cheap` (D11), climbs after exactly two agent-blamed
        // failures, ignores a provider failure, and climbs at most twice.
        let results = [FAIL, INFRA, FAIL, FAIL, FAIL, FAIL, FAIL, FAIL];
        let tried = run(&mut ladder, &task("focused"), &results);
        assert_eq!(tried, ["cheap", "cheap", "cheap", "mid", "mid", "strong"]);
        assert_eq!(u32::try_from(tried.len()).ok(), Some(MAX_ATTEMPTS));

        let mut ladder = default_ladder();
        let results = [FAIL; 8];
        let tried = run(&mut ladder, &task("focused"), &results);
        assert_eq!(tried, ["cheap", "cheap", "mid", "mid", "strong", "strong"]);
        assert!(
            !tried.iter().any(|arm| arm == "top"),
            "never more than two climbs"
        );

        // An architectural task starts on `top` and stops once it has failed there twice.
        let mut ladder = default_ladder();
        let tried = run(&mut ladder, &task("architectural"), &[FAIL; 8]);
        assert_eq!(tried, ["top", "top"]);

        // A pass stops the task, and a rung hint picks the start rung.
        let mut ladder = default_ladder();
        let passed = AttemptResult {
            passed: true,
            ..FAIL
        };
        let hinted = RouteTask {
            rung_hint: Some("mid".to_string()),
            ..task("focused")
        };
        assert_eq!(
            run(&mut ladder, &hinted, &[FAIL, passed, FAIL]),
            ["mid", "mid"]
        );
    }

    #[test]
    fn frugal_cascade_never_exceeds_k_max() {
        let rungs = ["a", "b", "c", "d"].map(str::to_string).to_vec();
        let mut cascade = FrugalCascade::new(rungs);
        let tried = run(&mut cascade, &task("focused"), &[INFRA; 6]);
        assert_eq!(tried, ["a", "b", "c"], "K_max = 2 climbs, on any failure");
    }

    #[test]
    fn router_policy_returns_the_routers_own_pick() {
        let models = [
            ("gpt-oss-120b", "cerebras", "cheap"),
            ("glm-4.7", "zai", "mid"),
            ("claude-sonnet-4-6", "anthropic", "top"),
        ];
        let mut policy = RouterPolicy::new(&models).expect("models");
        let slugs = models
            .iter()
            .map(|(slug, _, _)| (*slug).to_string())
            .collect();
        let twin = CascadeRouter::new(slugs);
        let health = ProviderHealthRegistry::new();
        let providers: HashMap<String, String> = models
            .iter()
            .map(|(slug, provider, _)| ((*slug).to_string(), (*provider).to_string()))
            .collect();
        let tiers = ["mechanical", "focused", "integrative", "architectural"];
        let mut compared = 0;
        for i in 0..120 {
            let task = RouteTask {
                task_id: format!("T{i}"),
                ..task(tiers[i % 4])
            };
            let picked = policy.start(&task).expect("a pick");
            let context = RouterPolicy::context(&task, 1, None);
            let direct = twin.route_with_health_scored(&context, &health, &providers, None, None);
            let expected = models
                .iter()
                .find(|(slug, _, _)| *slug == direct.primary.slug)
                .map(|(_, _, arm)| (*arm).to_string());
            assert_eq!(Some(picked.clone()), expected, "context {i}");
            compared += 1;
            // Both learn the same outcome: the cheap rung passes every third task.
            let passed = i.is_multiple_of(3) || picked != "cheap";
            let result = AttemptResult { passed, ..FAIL };
            let slug = models
                .iter()
                .find(|(_, _, arm)| *arm == picked)
                .map(|(slug, _, _)| *slug)
                .expect("a model");
            let index = twin.model_index_for_slug(slug).expect("an index");
            let features = context.to_features_for_model(Some(slug));
            let reward = if passed { 1.0 } else { 0.0 };
            twin.observe_outcome(features, index, reward, passed);
            policy.next(&task, &picked, &result);
        }
        assert!(compared >= 100);
    }

    #[test]
    fn the_oracle_is_deterministic() {
        let cell = |passed: bool, cost_usd: f64| AttemptResult {
            passed,
            agent_blamed: !passed,
            cost_usd,
        };
        let cells: BTreeMap<(String, String), AttemptResult> = [
            (("T1", "cheap"), cell(false, 0.01)),
            (("T1", "mid"), cell(true, 0.04)),
            (("T1", "top"), cell(true, 0.30)),
            (("T2", "cheap"), cell(true, 0.01)),
            (("T2", "top"), cell(true, 0.30)),
            (("T3", "cheap"), cell(false, 0.01)),
        ]
        .into_iter()
        .map(|((task, arm), result)| ((task.to_string(), arm.to_string()), result))
        .collect();
        let (mut first, second) = (Oracle::new(&cells), Oracle::new(&cells));
        assert_eq!(first, second);
        let start = |oracle: &mut Oracle, id: &str| {
            let task = RouteTask {
                task_id: id.to_string(),
                ..task("focused")
            };
            oracle.start(&task)
        };
        assert_eq!(start(&mut first, "T1").as_deref(), Some("mid"));
        assert_eq!(start(&mut first, "T2").as_deref(), Some("cheap"));
        assert_eq!(start(&mut first, "T3"), None, "no arm passes: it abstains");

        let train = [
            ArmOutcome {
                arm: "cheap".to_string(),
                passed: true,
                cost_usd: 0.03,
            },
            ArmOutcome {
                arm: "cheap".to_string(),
                passed: false,
                cost_usd: 0.03,
            },
            ArmOutcome {
                arm: "top".to_string(),
                passed: true,
                cost_usd: 0.30,
            },
        ];
        // cheap: $0.06 a pass; top: $0.30.
        assert_eq!(
            StaticArm::best_single(&train),
            Some(StaticArm::new("cheap"))
        );
    }
}
