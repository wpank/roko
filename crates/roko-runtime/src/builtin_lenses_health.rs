//! Stateful built-in Error, Drift, and Budget telemetry Lenses.
//!
//! The implementations reduce only facts present in [`ObservableEvent`].
//! Missing timestamps, topology, or knowledge metadata remain explicitly
//! unavailable instead of being inferred.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;
use roko_core::{
    AlertLevel, BudgetAlertPayload, DriftPayload, ErrorCategory, ErrorPayload, LensConfig,
    LensScope, ObservableEvent, ObservableEventKind, Result, RokoError, Signal, TelemetryObserve,
    parse_scope,
};

use crate::{LensPayload, LensSignalEnvelope};

/// Block names accepted by [`ErrorLens`] after an optional `@version` suffix is removed.
pub const ERROR_LENS_BLOCK_ALIASES: &[&str] = &["error-lens", "roko:error-lens"];
/// Block names accepted by [`DriftLens`] after an optional `@version` suffix is removed.
pub const DRIFT_LENS_BLOCK_ALIASES: &[&str] = &["drift-lens", "roko:drift-lens"];
/// Block names accepted by [`BudgetLens`] after an optional `@version` suffix is removed.
pub const BUDGET_LENS_BLOCK_ALIASES: &[&str] = &["budget-lens", "roko:budget-lens"];

const ERROR_OBSERVES: &[ObservableEventKind] = &[
    ObservableEventKind::CellLifecycle,
    ObservableEventKind::GraphLifecycle,
    ObservableEventKind::ExtensionLifecycle,
];
const DRIFT_OBSERVES: &[ObservableEventKind] = &[
    ObservableEventKind::MemoryLifecycle,
    ObservableEventKind::SignalLifecycle,
];
const BUDGET_OBSERVES: &[ObservableEventKind] = &[
    ObservableEventKind::AgentLifecycle,
    ObservableEventKind::CellLifecycle,
];

const DEFAULT_INTERVAL_MS: u64 = 60_000;
const DEFAULT_WINDOW_EVENTS: usize = 100;
const MAX_WINDOW_EVENTS: usize = 1_000_000;
const DEFAULT_MAX_AGENTS: usize = 1_024;
const DEFAULT_COLD_BALANCE_THRESHOLD: f64 = 0.05;

/// Instantiate a recognized health Lens, or return `None` for another built-in/plugin block.
pub fn create_builtin_health_lens(
    config: &LensConfig,
) -> Result<Option<Arc<dyn TelemetryObserve>>> {
    let block = block_base(&config.block);
    if ERROR_LENS_BLOCK_ALIASES.contains(&block.as_str()) {
        ErrorLens::from_config(config).map(|lens| Some(Arc::new(lens) as Arc<dyn TelemetryObserve>))
    } else if DRIFT_LENS_BLOCK_ALIASES.contains(&block.as_str()) {
        DriftLens::from_config(config).map(|lens| Some(Arc::new(lens) as Arc<dyn TelemetryObserve>))
    } else if BUDGET_LENS_BLOCK_ALIASES.contains(&block.as_str()) {
        BudgetLens::from_config(config)
            .map(|lens| Some(Arc::new(lens) as Arc<dyn TelemetryObserve>))
    } else {
        Ok(None)
    }
}

/// Rolling error classification and retry-outcome Lens.
pub struct ErrorLens {
    name: String,
    scope: LensScope,
    target: String,
    interval_ms: u64,
    window_events: usize,
    state: Mutex<ErrorState>,
}

#[derive(Default)]
struct ErrorState {
    outcomes: VecDeque<ErrorOutcome>,
    retries: VecDeque<RetryAttempt>,
}

struct ErrorOutcome {
    category: Option<ErrorCategory>,
    block: String,
}

struct RetryAttempt {
    run: String,
    success: Option<bool>,
}

impl ErrorLens {
    /// Parse and validate one ErrorLens registration.
    pub fn from_config(config: &LensConfig) -> Result<Self> {
        validate_identity(config, ERROR_LENS_BLOCK_ALIASES, "ErrorLens")?;
        validate_param_names(config, &["interval", "interval_ms", "window_events"])?;
        let scope = parse_scope(&config.scope)?;
        reject_chained_scope(&scope, "ErrorLens")?;
        let interval_ms = parse_interval_ms(config)?;
        let window_events = parse_window_events(config)?;
        Ok(Self {
            name: config.name.clone(),
            target: scope_target(&scope),
            scope,
            interval_ms,
            window_events,
            state: Mutex::new(ErrorState::default()),
        })
    }

    fn emit(&self, state: &ErrorState) -> Result<Vec<Signal>> {
        let mut by_category = BTreeMap::new();
        let mut by_block = BTreeMap::new();
        let mut total_errors = 0_u64;
        for outcome in &state.outcomes {
            let Some(category) = &outcome.category else {
                continue;
            };
            total_errors = total_errors.saturating_add(1);
            *by_category
                .entry(error_category_name(category))
                .or_default() += 1;
            *by_block.entry(outcome.block.clone()).or_default() += 1;
        }
        let resolved_retries = state
            .retries
            .iter()
            .filter_map(|attempt| attempt.success)
            .collect::<Vec<_>>();
        let retry_successes = resolved_retries.iter().filter(|success| **success).count();
        let payload = ErrorPayload {
            target: self.target.clone(),
            interval_ms: self.interval_ms,
            total_errors,
            by_category,
            by_block,
            retry_count: state.retries.len() as u64,
            retry_success_rate: usize_ratio(retry_successes, resolved_retries.len()),
            error_rate: u64_ratio(total_errors, state.outcomes.len() as u64),
        };
        encode(&self.name, LensPayload::Error(payload), "ErrorLens")
    }

    fn push_outcome(state: &mut ErrorState, outcome: ErrorOutcome, window_events: usize) {
        state.outcomes.push_back(outcome);
        while state.outcomes.len() > window_events {
            state.outcomes.pop_front();
        }
    }

    fn resolve_retry(state: &mut ErrorState, run: &str, success: bool) {
        if let Some(attempt) = state
            .retries
            .iter_mut()
            .rev()
            .find(|attempt| attempt.run == run && attempt.success.is_none())
        {
            attempt.success = Some(success);
        }
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for ErrorLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let mut state = self.state.lock();
        let outcome = match event {
            ObservableEvent::CellCompleted { block, run, .. } => {
                Self::resolve_retry(&mut state, run, true);
                Some(ErrorOutcome {
                    category: None,
                    block: block.clone(),
                })
            }
            ObservableEvent::CellFailed {
                block, run, error, ..
            } => {
                Self::resolve_retry(&mut state, run, false);
                Some(ErrorOutcome {
                    category: Some(classify_error(error, false)),
                    block: block.clone(),
                })
            }
            ObservableEvent::CellCancelled { block, run } => {
                Self::resolve_retry(&mut state, run, false);
                Some(ErrorOutcome {
                    category: Some(ErrorCategory::Cancelled),
                    block: block.clone(),
                })
            }
            ObservableEvent::GraphCompleted { graph, .. } => Some(ErrorOutcome {
                category: None,
                block: format!("graph:{graph}"),
            }),
            ObservableEvent::GraphFailed { graph, error, .. } => Some(ErrorOutcome {
                category: Some(classify_error(error, false)),
                block: format!("graph:{graph}"),
            }),
            ObservableEvent::ExtensionHookCalled {
                extension, hook, ..
            } => Some(ErrorOutcome {
                category: None,
                block: format!("{extension}:{hook}"),
            }),
            ObservableEvent::ExtensionHookFailed {
                extension,
                hook,
                error,
            } => Some(ErrorOutcome {
                category: Some(classify_error(error, true)),
                block: format!("{extension}:{hook}"),
            }),
            ObservableEvent::CellRetried { run, .. } => {
                state.retries.push_back(RetryAttempt {
                    run: run.clone(),
                    success: None,
                });
                while state.retries.len() > self.window_events {
                    state.retries.pop_front();
                }
                return self.emit(&state);
            }
            _ => return Ok(Vec::new()),
        };
        if let Some(outcome) = outcome {
            Self::push_outcome(&mut state, outcome, self.window_events);
        }
        self.emit(&state)
    }

    fn observes(&self) -> &[ObservableEventKind] {
        ERROR_OBSERVES
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

/// Streaming knowledge-tier, demurrage, and explicit metadata Lens.
pub struct DriftLens {
    name: String,
    scope: LensScope,
    memory: String,
    interval_ms: u64,
    window_events: usize,
    cold_balance_threshold: f64,
    state: Mutex<DriftState>,
}

#[derive(Default)]
struct DriftState {
    pending: BTreeMap<String, PendingSignal>,
    pending_order: VecDeque<String>,
    entries: BTreeMap<String, EntryEvidence>,
    entry_aliases: BTreeMap<String, String>,
    entry_order: VecDeque<String>,
    balance_deltas: VecDeque<f64>,
    promoted: u64,
    demoted: u64,
}

#[derive(Clone)]
struct PendingSignal {
    full_id: String,
    short_id: String,
    evidence: SignalEvidence,
}

#[derive(Clone)]
struct SignalEvidence {
    balance: f64,
    tier: String,
    anti_knowledge: bool,
    heuristic_calibration: Option<f64>,
}

struct EntryEvidence {
    tier: String,
    balance: Option<f64>,
    anti_knowledge: bool,
    heuristic_calibration: Option<f64>,
}

impl DriftLens {
    /// Parse and validate one DriftLens registration.
    pub fn from_config(config: &LensConfig) -> Result<Self> {
        validate_identity(config, DRIFT_LENS_BLOCK_ALIASES, "DriftLens")?;
        validate_param_names(
            config,
            &[
                "interval",
                "interval_ms",
                "window_events",
                "cold_balance_threshold",
            ],
        )?;
        let scope = parse_scope(&config.scope)?;
        if !matches!(
            scope,
            LensScope::Agent(_) | LensScope::Space(_) | LensScope::Global
        ) {
            return Err(RokoError::config(
                "DriftLens scope must be agent, space, or global",
            ));
        }
        let cold_balance_threshold = parse_fraction(
            config,
            "cold_balance_threshold",
            DEFAULT_COLD_BALANCE_THRESHOLD,
        )?;
        Ok(Self {
            name: config.name.clone(),
            memory: scope_target(&scope),
            scope,
            interval_ms: parse_interval_ms(config)?,
            window_events: parse_window_events(config)?,
            cold_balance_threshold,
            state: Mutex::new(DriftState::default()),
        })
    }

    fn emit(&self, state: &DriftState) -> Result<Vec<Signal>> {
        let mut tier_distribution = BTreeMap::new();
        let mut known_balances = Vec::new();
        let mut cold_entries = 0_u64;
        let mut anti_knowledge_count = 0_u64;
        let mut calibrations = Vec::new();
        for entry in state.entries.values() {
            *tier_distribution.entry(entry.tier.clone()).or_default() += 1;
            if tier_is_cold(&entry.tier)
                || entry
                    .balance
                    .is_some_and(|balance| balance <= self.cold_balance_threshold)
            {
                cold_entries = cold_entries.saturating_add(1);
            }
            if entry.anti_knowledge {
                anti_knowledge_count = anti_knowledge_count.saturating_add(1);
            }
            if let Some(balance) = entry.balance {
                known_balances.push(balance);
            }
            if let Some(calibration) = entry.heuristic_calibration {
                calibrations.push(calibration);
            }
        }
        let transitions = state.promoted.saturating_add(state.demoted);
        let payload = DriftPayload {
            memory: self.memory.clone(),
            interval_ms: self.interval_ms,
            total_entries: state.entries.len() as u64,
            tier_distribution,
            avg_balance: mean(&known_balances),
            balance_delta: mean_deque(&state.balance_deltas),
            promotion_rate: u64_ratio(state.promoted, transitions),
            demotion_rate: u64_ratio(state.demoted, transitions),
            cold_entries,
            anti_knowledge_count,
            heuristic_calibration_avg: mean(&calibrations),
        };
        encode(&self.name, LensPayload::Drift(payload), "DriftLens")
    }

    fn push_balance_delta(&self, state: &mut DriftState, delta: f64) {
        state.balance_deltas.push_back(delta);
        while state.balance_deltas.len() > self.window_events {
            state.balance_deltas.pop_front();
        }
    }

    fn insert_pending(
        &self,
        state: &mut DriftState,
        full_id: String,
        short_id: String,
        evidence: SignalEvidence,
    ) {
        if let Some(existing) = state.pending.get(&full_id).cloned() {
            state.pending.remove(&existing.full_id);
            state.pending.remove(&existing.short_id);
            state.pending_order.retain(|id| id != &existing.full_id);
        }
        let pending = PendingSignal {
            full_id: full_id.clone(),
            short_id: short_id.clone(),
            evidence,
        };
        state.pending.insert(full_id.clone(), pending.clone());
        state.pending.insert(short_id, pending);
        state.pending_order.push_back(full_id);
        while state.pending_order.len() > self.window_events {
            let Some(evicted_id) = state.pending_order.pop_front() else {
                break;
            };
            if let Some(evicted) = state.pending.remove(&evicted_id) {
                state.pending.remove(&evicted.short_id);
            }
        }
    }

    fn take_pending(state: &mut DriftState, signal: &str) -> Option<PendingSignal> {
        let pending = state.pending.get(signal)?.clone();
        state.pending.remove(&pending.full_id);
        state.pending.remove(&pending.short_id);
        state.pending_order.retain(|id| id != &pending.full_id);
        Some(pending)
    }

    fn entry_id(state: &DriftState, signal: &str) -> Option<String> {
        state.entry_aliases.get(signal).cloned().or_else(|| {
            state
                .entries
                .contains_key(signal)
                .then(|| signal.to_owned())
        })
    }

    fn insert_entry(
        &self,
        state: &mut DriftState,
        canonical_id: String,
        aliases: &[String],
        evidence: EntryEvidence,
    ) {
        if !state.entries.contains_key(&canonical_id) {
            state.entry_order.push_back(canonical_id.clone());
        }
        state.entries.insert(canonical_id.clone(), evidence);
        state
            .entry_aliases
            .insert(canonical_id.clone(), canonical_id.clone());
        for alias in aliases {
            state
                .entry_aliases
                .insert(alias.clone(), canonical_id.clone());
        }
        while state.entries.len() > self.window_events {
            let Some(evicted_id) = state.entry_order.pop_front() else {
                break;
            };
            state.entries.remove(&evicted_id);
            state.entry_aliases.retain(|_, id| id != &evicted_id);
        }
    }

    fn remove_entry(state: &mut DriftState, signal: &str) {
        let Some(canonical_id) = Self::entry_id(state, signal) else {
            return;
        };
        state.entries.remove(&canonical_id);
        state.entry_aliases.retain(|_, id| id != &canonical_id);
        state.entry_order.retain(|id| id != &canonical_id);
    }

    fn store_memory(&self, state: &mut DriftState, signal: &str, tier: &str) -> Result<()> {
        if let Some(canonical_id) = Self::entry_id(state, signal) {
            if !tier.trim().is_empty()
                && let Some(entry) = state.entries.get_mut(&canonical_id)
            {
                entry.tier = normalize_label(tier);
            }
            return Ok(());
        }
        if let Some(pending) = Self::take_pending(state, signal) {
            let stored_tier = if tier.trim().is_empty() {
                pending.evidence.tier.clone()
            } else {
                normalize_label(tier)
            };
            let aliases = [
                signal.to_owned(),
                pending.full_id.clone(),
                pending.short_id.clone(),
            ];
            self.insert_entry(
                state,
                pending.full_id,
                &aliases,
                EntryEvidence {
                    tier: stored_tier,
                    balance: Some(pending.evidence.balance),
                    anti_knowledge: pending.evidence.anti_knowledge || tier_is_anti_knowledge(tier),
                    heuristic_calibration: pending.evidence.heuristic_calibration,
                },
            );
            return Ok(());
        }
        if tier.trim().is_empty() {
            return Err(RokoError::config(
                "MemoryStored tier must be non-empty without correlated signal evidence",
            ));
        }
        self.insert_entry(
            state,
            signal.to_owned(),
            &[signal.to_owned()],
            EntryEvidence {
                tier: normalize_label(tier),
                balance: None,
                anti_knowledge: tier_is_anti_knowledge(tier),
                heuristic_calibration: None,
            },
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for DriftLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let mut state = self.state.lock();
        match event {
            ObservableEvent::SignalCreated(signal) => {
                let evidence = signal_evidence(signal)?;
                self.insert_pending(&mut state, signal.id.to_hex(), signal.id.short(), evidence);
                return Ok(Vec::new());
            }
            ObservableEvent::MemoryStored { signal, tier } => {
                self.store_memory(&mut state, signal, tier)?;
            }
            ObservableEvent::SignalDemurrageApplied(signal, loss) => {
                validate_nonnegative_finite(*loss, "SignalDemurrageApplied loss")?;
                let entry_id = Self::entry_id(&state, signal);
                let actual_delta =
                    if let Some(entry) = entry_id.and_then(|id| state.entries.get_mut(&id)) {
                        entry.balance.map_or(-*loss, |balance| {
                            let next = (balance - loss).max(0.0);
                            entry.balance = Some(next);
                            next - balance
                        })
                    } else {
                        -*loss
                    };
                self.push_balance_delta(&mut state, actual_delta);
            }
            ObservableEvent::DemurrageApplied {
                count,
                total_balance_lost,
            } => {
                validate_nonnegative_finite(
                    *total_balance_lost,
                    "DemurrageApplied total_balance_lost",
                )?;
                if *count == 0 && *total_balance_lost > 0.0 {
                    return Err(RokoError::config(
                        "DemurrageApplied cannot lose balance when count is zero",
                    ));
                }
                if *count > 0 {
                    self.push_balance_delta(&mut state, -total_balance_lost / *count as f64);
                }
            }
            ObservableEvent::SignalPromoted(signal, old, new) => {
                let entry_id = Self::entry_id(&state, signal);
                if let Some(entry) = entry_id.and_then(|id| state.entries.get_mut(&id)) {
                    entry.tier = normalize_label(new);
                }
                match tier_direction(old, new) {
                    Some(true) => state.promoted = state.promoted.saturating_add(1),
                    Some(false) => state.demoted = state.demoted.saturating_add(1),
                    None => {}
                }
            }
            ObservableEvent::MemoryConsolidated {
                promoted, demoted, ..
            } => {
                state.promoted = state
                    .promoted
                    .saturating_add(u64::try_from(*promoted).unwrap_or(u64::MAX));
                state.demoted = state
                    .demoted
                    .saturating_add(u64::try_from(*demoted).unwrap_or(u64::MAX));
            }
            ObservableEvent::SignalPruned(signal) => {
                Self::remove_entry(&mut state, signal);
            }
            _ => return Ok(Vec::new()),
        }
        self.emit(&state)
    }

    fn observes(&self) -> &[ObservableEventKind] {
        DRIFT_OBSERVES
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

/// Threshold-crossing Agent budget and vitality Lens.
pub struct BudgetLens {
    name: String,
    scope: LensScope,
    interval_ms: u64,
    thresholds: BudgetThresholds,
    max_agents: usize,
    state: Mutex<BudgetState>,
}

#[derive(Clone, Copy)]
struct BudgetThresholds {
    info: f64,
    warning: f64,
    critical: f64,
}

#[derive(Default)]
struct AgentBudgetState {
    phase: String,
    last_spent: Option<f64>,
    last_level: Option<AlertLevel>,
}

#[derive(Default)]
struct BudgetState {
    agents: BTreeMap<String, AgentBudgetState>,
    order: VecDeque<String>,
}

impl BudgetLens {
    /// Parse and validate one BudgetLens registration.
    pub fn from_config(config: &LensConfig) -> Result<Self> {
        validate_identity(config, BUDGET_LENS_BLOCK_ALIASES, "BudgetLens")?;
        validate_param_names(
            config,
            &[
                "interval",
                "interval_ms",
                "info_pct",
                "warning_pct",
                "budget_warn_pct",
                "critical_pct",
                "budget_critical_pct",
                "max_agents",
            ],
        )?;
        let scope = parse_scope(&config.scope)?;
        if !matches!(
            scope,
            LensScope::Agent(_) | LensScope::Space(_) | LensScope::Global
        ) {
            return Err(RokoError::config(
                "BudgetLens scope must be agent, space, or global",
            ));
        }
        let info = parse_fraction(config, "info_pct", 0.50)?;
        let warning = parse_aliased_fraction(config, "warning_pct", "budget_warn_pct", 0.80)?;
        let critical = parse_aliased_fraction(config, "critical_pct", "budget_critical_pct", 0.95)?;
        if !(info < warning && warning < critical) {
            return Err(RokoError::config(
                "BudgetLens thresholds must satisfy info_pct < warning_pct < critical_pct",
            ));
        }
        Ok(Self {
            name: config.name.clone(),
            scope,
            interval_ms: parse_interval_ms(config)?,
            thresholds: BudgetThresholds {
                info,
                warning,
                critical,
            },
            max_agents: parse_bounded_count(config, "max_agents", DEFAULT_MAX_AGENTS)?,
            state: Mutex::new(BudgetState::default()),
        })
    }

    fn level(&self, spent_fraction: f64) -> Option<AlertLevel> {
        if spent_fraction >= self.thresholds.critical {
            Some(AlertLevel::Critical)
        } else if spent_fraction >= self.thresholds.warning {
            Some(AlertLevel::Warning)
        } else if spent_fraction >= self.thresholds.info {
            Some(AlertLevel::Info)
        } else {
            None
        }
    }

    fn agent_state<'a>(&self, state: &'a mut BudgetState, agent: &str) -> &'a mut AgentBudgetState {
        if !state.agents.contains_key(agent) {
            while state.agents.len() >= self.max_agents {
                let Some(evicted) = state.order.pop_front() else {
                    break;
                };
                state.agents.remove(&evicted);
            }
            state.order.push_back(agent.to_owned());
            state
                .agents
                .insert(agent.to_owned(), AgentBudgetState::default());
        }
        state
            .agents
            .get_mut(agent)
            .expect("agent state was inserted")
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for BudgetLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let ObservableEvent::AgentBudgetUpdate {
            agent,
            spent_usd,
            remaining_usd,
            vitality,
        } = event
        else {
            if let ObservableEvent::AgentPhaseChange {
                agent, new_phase, ..
            } = event
            {
                let mut state = self.state.lock();
                self.agent_state(&mut state, agent).phase = new_phase.clone();
            }
            return Ok(Vec::new());
        };
        validate_nonnegative_finite(*spent_usd, "AgentBudgetUpdate spent_usd")?;
        validate_nonnegative_finite(*remaining_usd, "AgentBudgetUpdate remaining_usd")?;
        validate_fraction_value(*vitality, "AgentBudgetUpdate vitality")?;
        let total = spent_usd + remaining_usd;
        validate_nonnegative_finite(total, "AgentBudgetUpdate budget total")?;
        if total <= f64::EPSILON {
            let mut states = self.state.lock();
            let state = self.agent_state(&mut states, agent);
            state.last_spent = Some(0.0);
            state.last_level = None;
            return Ok(Vec::new());
        }
        let level = self.level(spent_usd / total);
        let mut states = self.state.lock();
        let state = self.agent_state(&mut states, agent);
        let previous_spent = state.last_spent.replace(*spent_usd);
        if state.last_level == level {
            return Ok(Vec::new());
        }
        state.last_level = level.clone();
        let Some(level) = level else {
            return Ok(Vec::new());
        };
        let interval_hours = self.interval_ms as f64 / 3_600_000.0;
        let burn_rate = previous_spent
            .map(|previous| (spent_usd - previous).max(0.0) / interval_hours)
            .unwrap_or(0.0);
        let payload = BudgetAlertPayload {
            target: format!("agent:{agent}"),
            budget_total: total,
            budget_spent: *spent_usd,
            budget_remaining: *remaining_usd,
            vitality: *vitality,
            vitality_phase: state.phase.clone(),
            projected_exhaustion_ms: None,
            burn_rate,
            level,
        };
        encode(&self.name, LensPayload::BudgetAlert(payload), "BudgetLens")
    }

    fn observes(&self) -> &[ObservableEventKind] {
        BUDGET_OBSERVES
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

fn encode(name: &str, payload: LensPayload, lens: &str) -> Result<Vec<Signal>> {
    LensSignalEnvelope::new(name, payload)
        .to_signal()
        .map(|signal| vec![signal])
        .map_err(|error| RokoError::config(format!("{lens} envelope encoding failed: {error}")))
}

fn validate_identity(config: &LensConfig, aliases: &[&str], lens: &str) -> Result<()> {
    if config.name.trim().is_empty() || config.name != config.name.trim() {
        return Err(RokoError::config(format!(
            "{lens} name must be non-empty without surrounding whitespace"
        )));
    }
    if let Some((_, version)) = config.block.trim().split_once('@')
        && (version.trim().is_empty() || version.contains('@'))
    {
        return Err(RokoError::config(format!(
            "{lens} block has an invalid version requirement"
        )));
    }
    let block = block_base(&config.block);
    if !aliases.contains(&block.as_str()) {
        return Err(RokoError::config(format!(
            "{lens} does not accept block `{}`",
            config.block
        )));
    }
    Ok(())
}

fn block_base(block: &str) -> String {
    block
        .trim()
        .split_once('@')
        .map_or(block.trim(), |(base, _)| base)
        .to_ascii_lowercase()
}

fn validate_param_names(config: &LensConfig, allowed: &[&str]) -> Result<()> {
    if let Some(name) = config
        .params
        .keys()
        .find(|name| !allowed.contains(&name.as_str()))
    {
        return Err(RokoError::config(format!(
            "lens `{}` has unsupported parameter `{name}`",
            config.name
        )));
    }
    Ok(())
}

fn reject_chained_scope(scope: &LensScope, lens: &str) -> Result<()> {
    if matches!(scope, LensScope::Lens(_)) {
        Err(RokoError::config(format!(
            "{lens} observes lifecycle events and cannot use lens scope"
        )))
    } else {
        Ok(())
    }
}

fn parse_interval_ms(config: &LensConfig) -> Result<u64> {
    if config.params.contains_key("interval") && config.params.contains_key("interval_ms") {
        return Err(RokoError::config(
            "lens params cannot set both interval and interval_ms",
        ));
    }
    let interval_ms = if config.params.contains_key("interval_ms") {
        parse_positive_integer(config, "interval_ms")?.expect("interval_ms was checked as present")
    } else if let Some(value) = config.params.get("interval") {
        let Some(value) = value.as_str() else {
            return Err(RokoError::config("lens interval must be a duration string"));
        };
        parse_duration_ms(value)?
    } else {
        DEFAULT_INTERVAL_MS
    };
    if interval_ms == 0 {
        Err(RokoError::config("lens interval must be greater than zero"))
    } else {
        Ok(interval_ms)
    }
}

fn parse_duration_ms(value: &str) -> Result<u64> {
    let value = value.trim();
    let (number, multiplier) = if let Some(number) = value.strip_suffix("ms") {
        (number, 1_u64)
    } else if let Some(number) = value.strip_suffix('s') {
        (number, 1_000)
    } else if let Some(number) = value.strip_suffix('m') {
        (number, 60_000)
    } else if let Some(number) = value.strip_suffix('h') {
        (number, 3_600_000)
    } else {
        return Err(RokoError::config(
            "lens interval must end in ms, s, m, or h",
        ));
    };
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .filter(|duration| *duration > 0)
        .ok_or_else(|| RokoError::config("lens interval is invalid or overflows milliseconds"))
}

fn parse_window_events(config: &LensConfig) -> Result<usize> {
    parse_bounded_count(config, "window_events", DEFAULT_WINDOW_EVENTS)
}

fn parse_bounded_count(config: &LensConfig, name: &str, default: usize) -> Result<usize> {
    let value = parse_positive_integer(config, name)?.unwrap_or(default as u64);
    let value = usize::try_from(value)
        .map_err(|_| RokoError::config(format!("{name} does not fit usize")))?;
    if value == 0 || value > MAX_WINDOW_EVENTS {
        Err(RokoError::config(format!(
            "{name} must be between 1 and {MAX_WINDOW_EVENTS}"
        )))
    } else {
        Ok(value)
    }
}

fn parse_positive_integer(config: &LensConfig, name: &str) -> Result<Option<u64>> {
    let Some(value) = config.params.get(name) else {
        return Ok(None);
    };
    let Some(value) = value.as_integer() else {
        return Err(RokoError::config(format!("{name} must be an integer")));
    };
    u64::try_from(value)
        .map(Some)
        .map_err(|_| RokoError::config(format!("{name} must be non-negative")))
}

fn parse_fraction(config: &LensConfig, name: &str, default: f64) -> Result<f64> {
    config.params.get(name).map_or(Ok(default), |value| {
        let value = value
            .as_float()
            .or_else(|| value.as_integer().map(|value| value as f64))
            .ok_or_else(|| RokoError::config(format!("{name} must be numeric")))?;
        validate_fraction_value(value, name)?;
        Ok(value)
    })
}

fn parse_aliased_fraction(
    config: &LensConfig,
    primary: &str,
    alias: &str,
    default: f64,
) -> Result<f64> {
    if config.params.contains_key(primary) && config.params.contains_key(alias) {
        return Err(RokoError::config(format!(
            "lens params cannot set both {primary} and {alias}"
        )));
    }
    if let Some(value) = config
        .params
        .get(primary)
        .or_else(|| config.params.get(alias))
    {
        let value = value
            .as_float()
            .or_else(|| value.as_integer().map(|value| value as f64))
            .ok_or_else(|| RokoError::config(format!("{primary} must be numeric")))?;
        validate_fraction_value(value, primary)?;
        Ok(value)
    } else {
        Ok(default)
    }
}

fn validate_fraction_value(value: f64, name: &str) -> Result<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(RokoError::config(format!(
            "{name} must be finite and between 0 and 1"
        )))
    }
}

fn validate_nonnegative_finite(value: f64, name: &str) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(RokoError::config(format!(
            "{name} must be finite and non-negative"
        )))
    }
}

fn scope_target(scope: &LensScope) -> String {
    match scope {
        LensScope::Cell(name) => format_target("cell", name),
        LensScope::Graph(name) => format_target("graph", name),
        LensScope::Agent(name) => format_target("agent", name),
        LensScope::Space(name) => format_target("space", name),
        LensScope::Lens(name) => format_target("lens", name),
        LensScope::Global => "global".into(),
    }
}

fn format_target(kind: &str, name: &str) -> String {
    format!("{kind}:{}", if name.is_empty() { "*" } else { name })
}

fn classify_error(error: &str, external_default: bool) -> ErrorCategory {
    let error = error.to_ascii_lowercase();
    if contains_any(&error, &["timeout", "timed out", "deadline exceeded"]) {
        ErrorCategory::Timeout
    } else if contains_any(
        &error,
        &[
            "capability denied",
            "permission denied",
            "access denied",
            "forbidden",
            "unauthorized",
        ],
    ) {
        ErrorCategory::CapabilityDenied
    } else if contains_any(&error, &["cancelled", "canceled"]) {
        ErrorCategory::Cancelled
    } else if contains_any(
        &error,
        &[
            "invalid input",
            "validation",
            "malformed",
            "bad request",
            "parse error",
        ],
    ) {
        ErrorCategory::InputInvalid
    } else if external_default
        || contains_any(
            &error,
            &[
                "external",
                "network",
                "connection",
                "provider",
                "http ",
                "dns",
                "i/o",
            ],
        )
    {
        ErrorCategory::External
    } else {
        ErrorCategory::LogicError
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn error_category_name(category: &ErrorCategory) -> String {
    match category {
        ErrorCategory::Timeout => "Timeout",
        ErrorCategory::CapabilityDenied => "CapabilityDenied",
        ErrorCategory::External => "External",
        ErrorCategory::LogicError => "LogicError",
        ErrorCategory::InputInvalid => "InputInvalid",
        ErrorCategory::Cancelled => "Cancelled",
    }
    .into()
}

fn signal_evidence(signal: &Signal) -> Result<SignalEvidence> {
    validate_fraction_value(signal.balance, "Signal balance")?;
    let kind = normalize_label(signal.kind.as_str());
    let knowledge_type = signal
        .tag("knowledge.type")
        .or_else(|| signal.tag("type"))
        .map(normalize_label);
    let anti_knowledge = tier_is_anti_knowledge(&kind)
        || knowledge_type
            .as_deref()
            .is_some_and(tier_is_anti_knowledge);
    let is_heuristic = kind == "heuristic" || knowledge_type.as_deref() == Some("heuristic");
    let heuristic_calibration = if is_heuristic {
        signal
            .tag("heuristic.calibration")
            .or_else(|| signal.tag("calibration"))
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
    } else {
        None
    };
    Ok(SignalEvidence {
        balance: signal.balance,
        tier: normalize_label(&format!("{:?}", signal.status)),
        anti_knowledge,
        heuristic_calibration,
    })
}

fn normalize_label(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn tier_is_anti_knowledge(tier: &str) -> bool {
    normalize_label(tier) == "anti_knowledge"
}

fn tier_is_cold(tier: &str) -> bool {
    matches!(
        normalize_label(tier).as_str(),
        "cold" | "archive" | "archived"
    )
}

fn tier_direction(old: &str, new: &str) -> Option<bool> {
    let old = tier_rank(old)?;
    let new = tier_rank(new)?;
    match new.cmp(&old) {
        std::cmp::Ordering::Greater => Some(true),
        std::cmp::Ordering::Less => Some(false),
        std::cmp::Ordering::Equal => None,
    }
}

fn tier_rank(tier: &str) -> Option<u8> {
    match normalize_label(tier).as_str() {
        "cold" | "archive" | "archived" => Some(0),
        "transient" => Some(1),
        "working" => Some(2),
        "consolidated" => Some(3),
        "persistent" => Some(4),
        _ => None,
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn mean_deque(values: &VecDeque<f64>) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn u64_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn usize_ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
