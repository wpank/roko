//! Provider failover: a planned model whose provider cannot take the task hands
//! it to the next usable candidate within the same attempt. A provider that
//! rate-limits the attempt fails it over the same way, or, with nowhere to go,
//! the attempt waits for the limit and calls again, so a 429 never spends one
//! of the task's tries (bug-4b6844).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Duration;

use parking_lot::Mutex;
use roko_agent::live_output::LiveAgentEvent;
use roko_core::agent::ProviderKind;
use roko_core::config::harness_params::{HarnessLadders, HarnessParams};
use roko_learn::provider_failover::{
    FailoverCandidate as DispatchCandidate, credentials_fix, credentials_hint, format_local_ms,
    missing_credentials_reason, provider_brings_own_tools, same_model_candidates,
};

use super::helper_calls::SideCall;
use super::try_allowance::{RungBounds, TryAllowance};
use super::*;

// ─── Provider failover ──────────────────────────────────────────────────────

/// `RokoError::Gateway` category for a task left without a usable provider.
///
/// The error is non-retryable, so `TaskExecutorCell` fails the attempt at once
/// instead of re-running a dispatch that would be refused again.
const PROVIDER_EXHAUSTED_CATEGORY: &str = "provider_exhausted";

/// Failover refusal class, and `RokoError::Gateway` category, of a CLI agent
/// roko cannot guard that an attempt in the operator's shared checkout passes
/// over (decision 1214). The error is non-retryable like
/// [`PROVIDER_EXHAUSTED_CATEGORY`], and the attempt settles as a harness
/// failure, charged to no provider.
pub(super) const UNGUARDED_IN_CHECKOUT: &str = "unguarded_in_checkout";

/// Agents whose commands roko cannot check before they run: they bring their
/// own shells, and Codex's stream broker acts only once a command has
/// started. They run only in per-task worktrees, where a `git stash` or
/// `git clean -fdx` cannot reach the operator's work (decision 1214).
const UNGUARDED_CLI_KINDS: [ProviderKind; 4] = [
    ProviderKind::CodexCli,
    ProviderKind::CursorCli,
    ProviderKind::CursorAcp,
    ProviderKind::GeminiCli,
];

/// `role` of the cost and efficiency rows of a call failover refused, by
/// roko-core's name for it: roko-serve's run summary picks them out by it.
const FAILOVER_REFUSED_ROLE: &str = roko_core::dashboard_snapshot::FAILOVER_REFUSED_SIDE_CALL;

/// Failover refusal class of a provider that rejected roko's credentials: a
/// login does not fix itself within a run, so the refusal is definitive and
/// the error says how to log in (backlog 1115).
const AUTH_FAILURE: &str = "auth_failure";

/// Failover refusal classes that waiting does not end: a login or a change
/// to the workspace's configuration does, such as a key for a model, or an
/// API model for a task a CLI agent cannot run (backlog 1115, gap-cf427e).
const WAITING_DOES_NOT_HELP: [&str; 8] = [
    AUTH_FAILURE,
    "not_configured",
    "not_dispatchable",
    "contract_unsupported",
    "disabled",
    "no_credentials",
    UNGUARDED_IN_CHECKOUT,
    UNPRICED_UNDER_CAP,
];

/// Failover refusal class of a model roko has no price for, which a run held
/// to a budget cap passes over before any call: the cap could not count the
/// call (gap-8548e3).
const UNPRICED_UNDER_CAP: &str = "unpriced_under_cap";

/// Why a model of [`UNPRICED_UNDER_CAP`] is passed over.
const UNPRICED_REASON: &str = "has no price, and a budget cap can count only priced calls";

/// Failover refusal class of a provider that answered a call with a rate
/// limit (HTTP 429) the tool loop's own retries did not get past
/// (bug-4b6844). Waiting ends it, so it is not definitive.
const RATE_LIMITED: &str = "rate_limited";

/// The first wait of an attempt whose provider rate-limited it and that
/// failover had nowhere to send, when the provider named no `Retry-After`.
/// Each further wait doubles, up to [`RATE_LIMIT_WAIT_CAP`] (bug-4b6844).
const RATE_LIMIT_WAIT_BASE: Duration = Duration::from_secs(5);

/// The longest wait between two calls of a rate-limited attempt when the
/// provider named no `Retry-After` (bug-4b6844).
const RATE_LIMIT_WAIT_CAP: Duration = Duration::from_secs(60);

/// How long a rate-limited attempt with nowhere to fail over waits before it
/// calls its provider again, after `waits` such waits: the `Retry-After` the
/// provider named, else a backoff from [`RATE_LIMIT_WAIT_BASE`] that doubles
/// up to [`RATE_LIMIT_WAIT_CAP`] (bug-4b6844).
fn rate_limit_wait(waits: u32, retry_after: Option<Duration>) -> Duration {
    retry_after.unwrap_or_else(|| {
        RATE_LIMIT_WAIT_BASE
            .saturating_mul(1_u32 << waits.min(4))
            .min(RATE_LIMIT_WAIT_CAP)
    })
}

/// How often a try that waits for a slot at a provider the run's own calls
/// fill looks again (bug-62ae1e).
const SLOT_POLL: Duration = Duration::from_secs(2);

/// Failover refusal class of a provider whose circuit is open.
const CIRCUIT_OPEN: &str = "circuit_open";

/// The failover refusal classes of a routed rung's provider that turned a try
/// away for its capacity, a rate limit or an open circuit, not for anything
/// about the try: the substitute's failure then counts on the routed rung, so
/// a task its provider keeps turning away climbs instead of looping on
/// substitutes (bug-62ae1e).
pub(super) const COUNTS_ON_ROUTED_RUNG: [&str; 2] = [RATE_LIMITED, CIRCUIT_OPEN];

/// `RokoError::Gateway` category of an attempt denied in a way no retry can
/// change (backlog 1116). The error is non-retryable like
/// [`PROVIDER_EXHAUSTED_CATEGORY`], so the task fails at once with how to
/// recover instead of spending its retries on the same denial.
pub(super) const PROVIDER_DENIED_CATEGORY: &str = "provider_denied";

/// How to recover once the workspace's immune isolation ledger cannot be read
/// and every agent is denied before dispatch (`isolation_state_unavailable`).
const UNREADABLE_LEDGER_FIX: &str = "`roko safety controls` says why \
     .roko/immune/agent-controls.json cannot be read; repair that file, or move it aside to drop \
     its controls";

/// How to recover from an attempt agent id the immune boundary refuses
/// (`invalid_agent_identity`), which every attempt of the task shares.
const INVALID_AGENT_ID_FIX: &str = "the plan or task id makes the attempt's agent id invalid \
     (over 256 bytes, a control character, or secret-shaped text); rename it";

/// Why the provider behind a model cannot take this dispatch.
#[derive(Debug, Clone)]
struct ProviderRefusal {
    model_key: String,
    /// Wire slug of the refused model, used to find the same model elsewhere.
    model_slug: String,
    provider_id: String,
    provider_kind: roko_core::agent::ProviderKind,
    /// The provider's own words, or why it cannot be called.
    reason: String,
    /// When the provider is expected to accept work again (unix ms).
    until_ms: Option<i64>,
    /// Calling the provider again cannot help (missing, not dispatchable, no
    /// credentials, rejected credentials, out of usage, billing), unlike an
    /// open circuit that may already have recovered.
    definitive: bool,
    /// Why, as a class ([`roko_learn::telemetry::FailoverRefusal::class`]).
    class: &'static str,
    /// Whether a call reached the provider before it refused.
    called: bool,
    /// When the refusal happened (unix ms).
    at_ms: i64,
}

/// The provider's own words in an exhaustion's `message`: what follows
/// roko's marker, else the whole message (a CLI's own usage-limit line).
fn exhaustion_words(message: &str) -> &str {
    use roko_agent::provider::error_classify::{BILLING_REFUSAL_MARKER, PROVIDER_EXHAUSTED_MARKER};
    let lower = message.to_ascii_lowercase();
    if let Some(at) = lower.find(PROVIDER_EXHAUSTED_MARKER) {
        return message[at + PROVIDER_EXHAUSTED_MARKER.len()..]
            .trim_start_matches(':')
            .trim();
    }
    if let Some(at) = lower.find(BILLING_REFUSAL_MARKER) {
        return message[at..].trim();
    }
    message.trim()
}

/// The hub's word that `refusal`'s provider refused a call of attempt
/// `attempt_key` for want of usage, an account it can't bill included, and is
/// skipped until it may be called again (bug-390e4b). `ladder` is whether
/// the model ladder routed the attempt.
fn provider_exhausted_event(
    spec: &TaskExecutionSpec,
    task_id: &str,
    attempt_key: &str,
    refusal: &ProviderRefusal,
    ladder: bool,
) -> roko_core::DashboardEvent {
    let reason = exhaustion_words(&refusal.reason).to_string();
    let billing =
        roko_agent::provider::error_classify::is_billing_message(&reason.to_ascii_lowercase());
    let provider = &refusal.provider_id;
    let until = refusal.until_ms.map(format_local_ms).unwrap_or_default();
    let detail = if billing {
        let without = if ladder {
            "the ladder runs without it"
        } else {
            "the run goes on without it"
        };
        format!("{provider}: {reason}: recharge, or {without} until {until}")
    } else {
        format!("{provider}: {reason}: skipped until {until}")
    };
    roko_core::DashboardEvent::ProviderExhausted {
        plan_id: spec.plan_id.clone(),
        task_id: task_id.to_string(),
        attempt: roko_learn::telemetry::records::AttemptKey::parse(attempt_key)
            .map_or(0, |key| key.attempt),
        provider: provider.clone(),
        model: refusal.model_slug.clone(),
        billing,
        reason,
        until_ms: refusal.until_ms,
        detail,
    }
}

/// The [`DashboardEvent::RouteDecided`](roko_core::DashboardEvent::RouteDecided)
/// of an attempt failover moved to `target` after `refusals` (gap-2a3298):
/// the models passed over are its ineligible candidates, the last refusal
/// its reason. `rung` is the ladder rung the target runs on, and `step` the
/// rungs the task climbed, when the ladder routed the attempt (gap-aaaaae).
fn failover_route_event(
    plan_id: &str,
    task_id: &str,
    attempt: u32,
    target: &crate::dispatch_v2::ProviderDispatchSpec,
    refusals: &[ProviderRefusal],
    rung: Option<String>,
    step: u32,
) -> roko_core::DashboardEvent {
    let reason = refusals.last().map_or_else(String::new, |refusal| {
        let words: String = refusal.reason.chars().take(200).collect();
        format!(
            "{} on {} passed over ({}): {words}",
            refusal.model_key, refusal.provider_id, refusal.class
        )
    });
    let mut candidates: Vec<roko_core::RouteCandidate> = refusals
        .iter()
        .rev()
        .take(roko_core::dashboard_snapshot::ROUTE_EVENT_CANDIDATES - 1)
        .map(|refusal| roko_core::RouteCandidate {
            model: refusal.model_slug.clone(),
            provider: refusal.provider_id.clone(),
            score: 0.0,
            p: None,
            eligible: false,
            ineligible_reason: Some(refusal.class.to_string()),
        })
        .collect();
    candidates.insert(
        0,
        roko_core::RouteCandidate {
            model: target.model_slug.clone(),
            provider: target.provider_id.clone(),
            score: 1.0,
            p: Some(1.0),
            eligible: true,
            ineligible_reason: None,
        },
    );
    roko_core::DashboardEvent::RouteDecided {
        plan_id: plan_id.to_string(),
        task_id: task_id.to_string(),
        attempt,
        model: target.model_slug.clone(),
        provider: target.provider_id.clone(),
        stage: "failover".to_string(),
        reason,
        source: "failover".to_string(),
        candidates,
        ladder: Box::new(roko_core::dashboard_snapshot::RouteLadder {
            rung,
            ladder_step: step,
            ladder_reason: super::ladder::TryReason::Failover.as_str().to_string(),
            forecast: None,
        }),
    }
}

/// The models failover passed over before the one that ran (bug-35379d).
/// Empty when the planned model ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct FailoverChain {
    /// Refused model keys, in order: the planned model first.
    pub(super) models: Vec<String>,
    /// Why the planned model did not run.
    pub(super) reason: Option<String>,
    /// Every refusal, with its class and whether a call was made
    /// (bug-220385).
    pub(super) refusals: Vec<roko_learn::telemetry::FailoverRefusal>,
    /// The ladder rung of the model that ran, when failover moved an attempt
    /// the ladder routed (backlog 1120).
    pub(super) rung: Option<FailoverRung>,
}

/// The ladder rung failover ran an attempt on in place of the routed one:
/// the routed rung itself when the same model ran on another provider, else a
/// rung above it (backlog 1120).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FailoverRung {
    /// Index of the rung among the role's rungs, cheapest first.
    pub(super) index: u32,
    /// The rung's name.
    pub(super) name: String,
}

/// Where `[routing.ladder]` routed an attempt. Failover moves such an
/// attempt to the same model on another provider, then up the role's
/// runnable rungs, never to a cheaper model (decision 1119, backlog 1120),
/// and a substitute on a rung above runs under that rung's bounds
/// (bug-62ae1e).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LadderRoute {
    /// The task's role, whose rungs the ladder routes along.
    pub(super) role: String,
    /// Index of the routed rung among the role's rungs, cheapest first.
    pub(super) rung: usize,
    /// Rungs the task climbed above its start rung.
    pub(super) step: u32,
    /// What the try may use on each runnable rung above the routed one.
    pub(super) above: Vec<RungBounds>,
}

impl LadderRoute {
    /// Where `plan` put `task` on the ladder, `step` rungs above its start
    /// rung; `None` for an attempt the ladder did not route (a pin, a task
    /// hint, the router or a default).
    pub(super) fn of(
        task: &TaskDef,
        plan: &crate::dispatch::RunnerDispatchPlan,
        step: u32,
    ) -> Option<Self> {
        let rung = match plan.source {
            ModelChoiceSource::Ladder { rung, .. } | ModelChoiceSource::SelfModel { rung } => rung,
            _ => return None,
        };
        Some(Self {
            role: task.role.as_deref().unwrap_or("implementer").to_string(),
            rung,
            step,
            above: Vec::new(),
        })
    }

    /// This route, with what the try may use on each rung above it.
    pub(super) fn with_above(mut self, above: Vec<RungBounds>) -> Self {
        self.above = above;
        self
    }

    /// What the try may use on the rung `chain`'s substitute runs on, when
    /// failover moved it up from the routed rung (bug-62ae1e); `None` on the
    /// routed rung.
    pub(super) fn substitute_bounds(&self, chain: &FailoverChain) -> Option<&RungBounds> {
        let index = usize::try_from(chain.rung.as_ref()?.index).ok()?;
        self.above.iter().find(|bounds| bounds.index == index)
    }
}

/// The dashboard row an attempt's `agent_spawned` opened before dispatch,
/// which [`GraphTaskDispatcher::run_bridge_with_failover`] republishes with
/// the model and provider that run once failover passes the planned model
/// over (backlog 1128).
#[derive(Debug, Clone, Copy)]
pub(super) struct DashboardRow<'a> {
    /// The dashboard's agent id (`plan/cell`).
    pub(super) agent_id: &'a str,
    /// The task's role.
    pub(super) role: &'a str,
    /// The attempt's ordinal, as its attempt key numbers it (bug-369c21).
    pub(super) attempt: u32,
}

impl FailoverChain {
    fn of(refusals: &[ProviderRefusal]) -> Self {
        Self {
            models: refusals
                .iter()
                .map(|refusal| refusal.model_key.clone())
                .collect(),
            reason: refusals.first().map(|refusal| {
                format!(
                    "`{}` on `{}`: {}",
                    refusal.model_key, refusal.provider_id, refusal.reason
                )
            }),
            refusals: refusals
                .iter()
                .map(|refusal| roko_learn::telemetry::FailoverRefusal {
                    model: refusal.model_key.clone(),
                    provider: refusal.provider_id.clone(),
                    class: refusal.class.to_string(),
                    reason: refusal.reason.clone(),
                    called: refusal.called,
                    at: Some(refusal.at_ms),
                    until: refusal.until_ms,
                })
                .collect(),
            rung: None,
        }
    }
}

impl GraphTaskDispatcher {
    /// Run the provider bridge, treating the planned model as a preference.
    ///
    /// A model whose provider is missing, not dispatchable, without
    /// credentials, disabled, or has an open circuit is skipped before any
    /// call. One that refuses with a usage-window error ("You've hit your
    /// session limit · resets 4pm") is quarantined in the persisted health
    /// registry until its reported reset (else
    /// `routing.exhaustion_cooldown_secs`). Either way the next candidate from
    /// [`Self::failover_candidates`] runs in the same attempt, so no task
    /// retry is burned. An attempt the model ladder routed (`ladder`) moves
    /// only to the same model elsewhere or up its rungs, and records the rung
    /// that ran (backlog 1120). An explicit `--model` override is a pin and
    /// never fails over. When nothing usable remains the attempt fails with a
    /// non-retryable error that says how to recover.
    ///
    /// A call its provider refused with a rate limit (HTTP 429), past the
    /// tool loop's own retries, fails over the same way, as a refusal that is
    /// not definitive. With nowhere to fail over, a pinned model included,
    /// the attempt waits for the limit ([`rate_limit_wait`]) and calls again,
    /// marked queued so the stall watchdog does not take the wait for
    /// silence. It waits while the limit lasts, until the call is served or
    /// fails otherwise, failover finds a model, or the plan run stops it, so a
    /// 429 never settles a try (bug-4b6844). While the run's own calls fill
    /// the provider, at the calls at once its rate limits showed it takes,
    /// the attempt first waits for one of them to end and calls the same
    /// model again, for up to `[routing.ladder] rate_limit_wait_secs`
    /// ([`Self::wait_for_own_slot`], bug-62ae1e). A substitute on a rung
    /// above the routed one runs under that rung's turns, and `allowance`
    /// under its time and spend.
    ///
    /// In the operator's shared checkout, a Codex, Cursor or Gemini CLI agent
    /// is passed over the same way, pinned or not, unless `[runner]
    /// allow_unguarded_agents_in_checkout` is set (decision 1214).
    ///
    /// Returns the dispatch with the models failover passed over, which the
    /// attempt's records carry beside the one that ran. A call a provider
    /// refused gets cost and efficiency rows of its own, keyed by
    /// `attempt_key` (role `failover_refused`, bug-220385). Each call starts
    /// on `progress`, so a call the watchdog cancels is recorded against the
    /// model it ran on (bug-aa2044). Once failover passed a model over, the
    /// attempt's `dashboard` row names the model and provider that run.
    /// `theta`, the θ the attempt runs, orders the providers failover tries
    /// (M1's `provider_order`, 8124).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_bridge_with_failover(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        attempt_key: String,
        mut request: AgentDispatchRequest,
        progress: Option<&super::watchdog::AttemptProgress>,
        ladder: Option<&LadderRoute>,
        allowance: Option<&TryAllowance>,
        dashboard: Option<DashboardRow<'_>>,
        theta: Option<&HarnessParams>,
    ) -> Result<(crate::dispatch_v2::AgentResultDispatch, FailoverChain)> {
        let pinned = self.cli_model_override.is_some();
        let mut candidate = DispatchCandidate {
            model_key: request.model_key.clone(),
            config: None,
        };
        let mut refusals: Vec<ProviderRefusal> = Vec::new();
        // Calls a provider refused, each with cost and efficiency rows of its
        // own, and the waits of a rate-limited attempt with nowhere to go.
        let mut refused_calls = 0_usize;
        let mut rate_limit_waits = 0_u32;
        // What the attempt waited for slots at a provider its run's own
        // calls fill, and the rung above the routed one whose bounds it runs
        // under (bug-62ae1e).
        let mut slot_waited = Duration::ZERO;
        let mut bound_to: Option<usize> = None;
        // A pinned model never fails over, and never runs unguarded in the
        // shared checkout either.
        if pinned {
            let target = self.resolve_candidate(&candidate);
            if let Some(kind) = self.unguarded_in_checkout(&target) {
                log_unguarded_skip(&spec.plan_id, kind);
                let pinned_model = format!(
                    "`{}` on `{}`: {}",
                    candidate.model_key,
                    target.provider_id,
                    unguarded_reason(kind)
                );
                return Err(no_guarded_provider(&[pinned_model]));
            }
            // A pinned model is never called unpriced under a cap either.
            if self.unpriced_under_cap(&target) {
                let refusal = ProviderRefusal {
                    model_key: candidate.model_key.clone(),
                    model_slug: target.model_slug.clone(),
                    provider_id: target.provider_id.clone(),
                    provider_kind: target.provider_kind,
                    reason: UNPRICED_REASON.to_string(),
                    until_ms: None,
                    definitive: true,
                    class: UNPRICED_UNDER_CAP,
                    called: false,
                    at_ms: chrono::Utc::now().timestamp_millis(),
                };
                return Err(self.no_usable_provider(&[refusal], &[], true, None));
            }
        }
        loop {
            if !pinned && let Some(refusal) = self.blocked_provider(&candidate, &request) {
                if refusal.class == UNGUARDED_IN_CHECKOUT {
                    log_unguarded_skip(&spec.plan_id, refusal.provider_kind);
                }
                let definitive = refusal.definitive;
                refusals.push(refusal);
                match self.failover_model(spec, task_id, &refusals, ladder, theta, &request) {
                    Ok(next) => {
                        candidate = next;
                        continue;
                    }
                    Err(error) if definitive => return Err(error),
                    // An open circuit may already have recovered; with no
                    // usable alternative, dispatch the planned model.
                    Err(_) => {
                        refusals.pop();
                    }
                }
            }

            request.model_key = candidate.model_key.clone();
            // A substitute on a rung above the routed one runs under that
            // rung's bounds, not the routed rung's (bug-62ae1e).
            if let Some(bounds) = ladder.and_then(|route| {
                route.substitute_bounds(&self.failover_chain(&refusals, &candidate, Some(route)))
            }) && bound_to != Some(bounds.index)
            {
                bound_to = Some(bounds.index);
                request.max_turns = Some(bounds.max_turns);
                if let Some(allowance) = allowance {
                    allowance.rebind(bounds.allowance.as_ref());
                }
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id,
                    model = %candidate.model_key,
                    max_turns = bounds.max_turns,
                    max_usd = ?bounds.allowance.as_ref().and_then(|rung| rung.usd),
                    max_secs = ?bounds.allowance.as_ref().and_then(|rung| rung.secs),
                    "failover moved the try up the ladder; it runs under that rung's bounds"
                );
            }
            // Once failover passed a model over, the dashboard row the
            // attempt opened with the planned model names the one that runs
            // instead; the hub upserts the row by its agent id (backlog 1128).
            if !refusals.is_empty()
                && let (Some(tui), Some(row)) = (&self.tui_bridge, dashboard)
            {
                let target = self.resolve_candidate(&candidate);
                // Failover re-routed the attempt: say to what, and why
                // (gap-2a3298), and on which rung (gap-aaaaae).
                let rung = self
                    .failover_chain(&refusals, &candidate, ladder)
                    .rung
                    .map(|rung| rung.name);
                tui.publish_event(failover_route_event(
                    &spec.plan_id,
                    task_id,
                    row.attempt,
                    &target,
                    &refusals,
                    rung,
                    ladder.map_or(0, |ladder| ladder.step),
                ));
                tui.agent_spawned(
                    row.agent_id,
                    &spec.plan_id,
                    task_id,
                    row.attempt,
                    row.role,
                    &target.model_slug,
                    &target.provider_id,
                );
            }
            if let Some(progress) = progress {
                progress.call_started(
                    self.resolve_candidate(&candidate),
                    self.failover_chain(&refusals, &candidate, ladder),
                );
            }
            let call_started = Instant::now();
            let dispatch = match &candidate.config {
                Some(config) => {
                    self.factory
                        .run_shared_agent_bridge_with_config(request.clone(), Arc::clone(config))
                        .await
                }
                None => self.factory.run_shared_agent_bridge(request.clone()).await,
            }
            .map_err(|error| RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: error.to_string(),
            })?;
            if dispatch.result.success {
                let chain = self.failover_chain(&refusals, &candidate, ladder);
                return Ok((dispatch, chain));
            }
            let text = dispatch.result.output.body.as_text().unwrap_or_default();
            let exhaustion = roko_agent::provider::error_classify::detect_provider_exhaustion(text);
            let rate_limited = exhaustion.is_none()
                && roko_agent::provider::error_classify::classify_failure_text(
                    &text.to_ascii_lowercase(),
                ) == "rate_limit";
            if exhaustion.is_none() && !rate_limited {
                let chain = self.failover_chain(&refusals, &candidate, ladder);
                return Ok((dispatch, chain));
            }

            // The caller settles only the result it receives; account the
            // refused call here.
            let refused_cost_usd = f64::from(dispatch.result.usage.cost_usd);
            self.record_dispatch_spend(&spec.plan_id, task_id, &dispatch);
            self.budget_ledger
                .settle(&spec.plan_id, None, refused_cost_usd)?;
            // The attempt's verdict counts it beside the call that runs
            // (bug-8bcd98).
            if let Some(progress) = progress {
                progress.call_refused(refused_cost_usd);
            }
            refused_calls += 1;
            self.write_side_call_rows(
                spec,
                task_id,
                &attempt_key,
                &format!("{attempt_key}/refused-{refused_calls}"),
                FAILOVER_REFUSED_ROLE,
                &SideCall::of(
                    &dispatch,
                    u64::try_from(call_started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    self.pricing_snapshot().as_deref(),
                ),
            )
            .await;
            let provider_id = dispatch.target.provider_id.clone();
            let Some(exhaustion) = exhaustion else {
                // A rate limit while the run's own calls fill the provider
                // waits for a slot among them, then calls the same model
                // again: a substitute would run another rung's model on the
                // try (bug-62ae1e).
                if self
                    .wait_for_own_slot(
                        spec,
                        task_id,
                        &attempt_key,
                        &dispatch.target,
                        &mut slot_waited,
                        progress,
                    )
                    .await
                {
                    continue;
                }
                // A rate limit (bug-4b6844): fail over, as for an open
                // circuit, or wait for the limit and call the same model
                // again.
                let now_ms = chrono::Utc::now().timestamp_millis();
                let retry_after = RokoError::rate_limited(text).retry_after();
                let reason = text.lines().next().unwrap_or("rate limited").to_string();
                refusals.push(ProviderRefusal {
                    model_key: candidate.model_key.clone(),
                    model_slug: dispatch.target.model_slug.clone(),
                    provider_id: provider_id.clone(),
                    provider_kind: dispatch.target.provider_kind,
                    reason: reason.clone(),
                    until_ms: retry_after.map(|wait| {
                        now_ms.saturating_add(i64::try_from(wait.as_millis()).unwrap_or(i64::MAX))
                    }),
                    definitive: false,
                    class: RATE_LIMITED,
                    called: true,
                    at_ms: now_ms,
                });
                if !pinned
                    && let Ok(next) =
                        self.failover_model(spec, task_id, &refusals, ladder, theta, &request)
                {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id,
                        provider = %provider_id,
                        model = %candidate.model_key,
                        next = %next.model_key,
                        %reason,
                        "provider rate-limited the attempt; failing over within it"
                    );
                    candidate = next;
                    continue;
                }
                refusals.pop();
                let wait = rate_limit_wait(rate_limit_waits, retry_after);
                rate_limit_waits = rate_limit_waits.saturating_add(1);
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id,
                    provider = %provider_id,
                    model = %candidate.model_key,
                    wait_ms = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX),
                    waits = rate_limit_waits,
                    %reason,
                    "provider rate-limited the attempt and failover has nowhere to send it; \
                     waiting inside the attempt, then calling again"
                );
                // The dashboard says why the attempt is quiet.
                if let Some(tui) = &self.tui_bridge {
                    tui.diagnosis(roko_core::DiagnosisSummary {
                        id: format!("rate-limit:{attempt_key}:{rate_limit_waits}"),
                        ts: chrono::Utc::now(),
                        severity: roko_core::DiagnosisSeverity::Warn,
                        subject: format!("{}/{task_id}", spec.plan_id),
                        detail: format!(
                            "{provider_id} rate-limited the attempt ({reason}) and failover has \
                             nowhere to send it: it waits {}s, then calls again",
                            wait.as_secs()
                        ),
                        suggested_action: None,
                        intervention_taken: Some("waiting out a rate limit".to_string()),
                    });
                }
                if let Some(progress) = progress {
                    progress.observe(&LiveAgentEvent::Queued { waiting: true });
                }
                tokio::time::sleep(wait).await;
                if let Some(progress) = progress {
                    progress.observe(&LiveAgentEvent::Queued { waiting: false });
                }
                continue;
            };

            let cooldown_ms = i64::try_from(
                self.config
                    .routing
                    .exhaustion_cooldown_secs
                    .saturating_mul(1_000),
            )
            .unwrap_or(i64::MAX);
            let until_ms = exhaustion.resets_at_ms.unwrap_or_else(|| {
                chrono::Utc::now()
                    .timestamp_millis()
                    .saturating_add(cooldown_ms)
            });
            self.factory
                .health_registry
                .record_exhaustion(&provider_id, until_ms);
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id,
                provider = %provider_id,
                model = %candidate.model_key,
                until = %format_local_ms(until_ms),
                reason = %exhaustion.message,
                "provider is out of usage; skipping it until its reset"
            );
            refusals.push(ProviderRefusal {
                model_key: candidate.model_key.clone(),
                model_slug: dispatch.target.model_slug.clone(),
                provider_id,
                provider_kind: dispatch.target.provider_kind,
                reason: exhaustion.message,
                until_ms: Some(until_ms),
                definitive: true,
                class: "provider_exhausted",
                called: true,
                at_ms: chrono::Utc::now().timestamp_millis(),
            });
            // The hub hears why, an account the provider can't bill included
            // (bug-390e4b).
            if let (Some(tui), Some(refusal)) = (&self.tui_bridge, refusals.last()) {
                tui.publish_event(provider_exhausted_event(
                    spec,
                    task_id,
                    &attempt_key,
                    refusal,
                    ladder.is_some(),
                ));
            }
            if pinned {
                return Err(self.no_usable_provider(&refusals, &[], true, None));
            }
            candidate = self.failover_model(spec, task_id, &refusals, ladder, theta, &request)?;
        }
    }

    /// Wait, while the run's own calls fill `target`'s provider at the calls
    /// at once its rate limits showed it takes, for one of them to end
    /// (bug-62ae1e): the attempt's call the provider refused, or the run's
    /// limiter held back, then calls the same model again instead of
    /// failover handing the try to a substitute on another rung. The wait is
    /// marked queued, so neither the stall watchdog nor the try's time bound
    /// counts it, and lasts no more than `[routing.ladder]
    /// rate_limit_wait_secs` over the attempt, `waited` so far. Returns
    /// whether to call again: `false` when none of the run's calls is in
    /// flight there, so the limit is the provider's own, once the wait is
    /// spent, and once the run is stopping.
    async fn wait_for_own_slot(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        attempt_key: &str,
        target: &crate::dispatch_v2::ProviderDispatchSpec,
        waited: &mut Duration,
        progress: Option<&super::watchdog::AttemptProgress>,
    ) -> bool {
        use roko_agent::provider::openai_compat::own_calls;

        let Some(provider) = target.provider_config.as_ref() else {
            return false;
        };
        let Some(own) = own_calls(provider) else {
            return false;
        };
        // One of them ended already.
        if !own.full() {
            return true;
        }
        let bound = Duration::from_secs(self.config.routing.ladder.rate_limit_wait_secs());
        if *waited >= bound || self.is_stopping() {
            return false;
        }
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id,
            provider = %target.provider_id,
            model = %target.model_key,
            in_flight = own.in_flight,
            limit = own.limit,
            waited_secs = waited.as_secs(),
            "the run's own calls fill the provider at the calls at once its rate limits showed \
             it takes; waiting for one of them to end, then calling the same model again"
        );
        // The dashboard says why the attempt is quiet.
        if let Some(tui) = &self.tui_bridge {
            tui.diagnosis(roko_core::DiagnosisSummary {
                id: format!("rate-limit-slot:{attempt_key}:{}", waited.as_millis()),
                ts: chrono::Utc::now(),
                severity: roko_core::DiagnosisSeverity::Info,
                subject: format!("{}/{task_id}", spec.plan_id),
                detail: format!(
                    "{} takes {} calls at once and the run has {} in flight there: the attempt \
                     waits for one to end, then calls {} again",
                    target.provider_id, own.limit, own.in_flight, target.model_key
                ),
                suggested_action: None,
                intervention_taken: Some(
                    "waiting for a slot at a rate-limited provider".to_string(),
                ),
            });
        }
        if let Some(progress) = progress {
            progress.observe(&LiveAgentEvent::Queued { waiting: true });
        }
        let free = loop {
            let step = SLOT_POLL.min(bound.saturating_sub(*waited));
            tokio::time::sleep(step).await;
            *waited += step;
            if own_calls(provider).is_none_or(|own| !own.full()) {
                break true;
            }
            if *waited >= bound || self.is_stopping() {
                break false;
            }
        };
        if let Some(progress) = progress {
            progress.observe(&LiveAgentEvent::Queued { waiting: false });
        }
        if !free {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id,
                provider = %target.provider_id,
                model = %target.model_key,
                waited_secs = waited.as_secs(),
                "the attempt waited its [routing.ladder] rate_limit_wait_secs for a slot at the \
                 provider; failover may now hand it to a substitute"
            );
        }
        free
    }

    /// The chain of `refusals` before `candidate` ran, with the ladder rung
    /// it ran on when failover moved an attempt the ladder routed: the routed
    /// rung when it is the same model elsewhere, else the rung above whose
    /// model it is (backlog 1120).
    fn failover_chain(
        &self,
        refusals: &[ProviderRefusal],
        candidate: &DispatchCandidate,
        ladder: Option<&LadderRoute>,
    ) -> FailoverChain {
        let mut chain = FailoverChain::of(refusals);
        let (Some(route), Some(routed)) = (ladder, refusals.first()) else {
            return chain;
        };
        let Some(routing) = self.factory.dispatcher().routing_ladder() else {
            return chain;
        };
        let index = if self.resolve_candidate(candidate).model_slug == routed.model_slug {
            Some(route.rung)
        } else {
            routing
                .rung_models_above(&route.role, route.rung)
                .into_iter()
                .find(|rung| rung.model == candidate.model_key)
                .map(|rung| rung.index)
        };
        chain.rung = index.and_then(|index| {
            Some(FailoverRung {
                index: u32::try_from(index).ok()?,
                name: routing.rung_name(&route.role, index)?.to_string(),
            })
        });
        chain
    }

    fn resolve_candidate(
        &self,
        candidate: &DispatchCandidate,
    ) -> crate::dispatch_v2::ProviderDispatchSpec {
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        crate::dispatch_v2::ProviderDispatchResolver::new(Arc::clone(config))
            .resolve(&candidate.model_key)
    }

    /// The refusal for `candidate` when its provider must not be called now:
    /// missing, not dispatchable, unable to enforce `request`'s agent
    /// contract, an agent that would run unguarded in the operator's shared
    /// checkout, without credentials, statically disabled, a model with no
    /// price under a budget cap (gap-8548e3), or its circuit is open in the
    /// health registry.
    fn blocked_provider(
        &self,
        candidate: &DispatchCandidate,
        request: &AgentDispatchRequest,
    ) -> Option<ProviderRefusal> {
        use crate::dispatch_v2::ProviderRuntime;
        use roko_learn::provider_health::ErrorClass;

        let target = self.resolve_candidate(candidate);
        let provider_id = target.provider_id.clone();
        // Skipped before any call.
        let refusal = |class: &'static str,
                       reason: String,
                       until_ms: Option<i64>,
                       definitive: bool| ProviderRefusal {
            model_key: candidate.model_key.clone(),
            model_slug: target.model_slug.clone(),
            provider_id: provider_id.clone(),
            provider_kind: target.provider_kind,
            reason,
            until_ms,
            definitive,
            class,
            called: false,
            at_ms: chrono::Utc::now().timestamp_millis(),
        };
        let Some(provider) = target.provider_config.as_ref() else {
            return Some(refusal(
                "not_configured",
                format!("provider `{provider_id}` is not configured"),
                None,
                true,
            ));
        };
        if let ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Some(refusal(
                "not_dispatchable",
                format!(
                    "provider `{provider_id}` is not dispatchable: {}",
                    unsupported.detail
                ),
                None,
                true,
            ));
        }
        if let Err(error) = crate::dispatch_v2::validate_contract_support(request, &target) {
            return Some(refusal(
                "contract_unsupported",
                error.to_string(),
                None,
                true,
            ));
        }
        if let Some(kind) = self.unguarded_in_checkout(&target) {
            return Some(refusal(
                UNGUARDED_IN_CHECKOUT,
                unguarded_reason(kind),
                None,
                true,
            ));
        }
        if self
            .config
            .routing
            .disabled_providers
            .contains(&provider_id)
        {
            return Some(refusal(
                "disabled",
                "listed in routing.disabled_providers".to_string(),
                None,
                false,
            ));
        }
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Some(refusal(
                "no_credentials",
                missing_credentials_reason(provider, &provider_id),
                None,
                true,
            ));
        }
        if self.unpriced_under_cap(&target) {
            return Some(refusal(
                UNPRICED_UNDER_CAP,
                UNPRICED_REASON.to_string(),
                None,
                true,
            ));
        }
        let registry = &self.factory.health_registry;
        if registry.is_available(&provider_id) {
            return None;
        }
        let health = registry.get(&provider_id);
        let (class, reason, definitive) = match health.failure_window.back().map(|r| r.error_class)
        {
            Some(ErrorClass::Exhausted) => ("provider_exhausted", "out of usage", true),
            Some(ErrorClass::Billing) => ("billing", "billing failure", true),
            Some(ErrorClass::AuthFailure) => (AUTH_FAILURE, "not logged in or key rejected", true),
            _ => (CIRCUIT_OPEN, "circuit open after repeated failures", false),
        };
        Some(refusal(
            class,
            reason.to_string(),
            health.cooldown_until,
            definitive,
        ))
    }

    /// Candidates after `refusals`, in order: the first refused model's slug
    /// on another configured provider of its family (claude_cli can run any
    /// Claude slug), `[routing] fallback_models`, `agent.fallback_model`, and
    /// `agent.default_model`. The order is the shared failover policy's
    /// (`roko_learn::provider_failover`), which serve and ACP apply too.
    ///
    /// An attempt the model ladder routed (`ladder`) takes the first group,
    /// then the runnable rungs above its own, and nothing cheaper (decision
    /// 1119, backlog 1120). An attempt the ladder did not route that holds
    /// its outbound tool calls for approval (`stages`, as a chat host's run
    /// does) can run only in roko's own tool loop, so it takes the models
    /// that run there last ([`RokoConfig::tool_loop_models`], gap-cf427e).
    /// M1's `provider_order` in `theta` then reorders the candidates
    /// ([`Self::order_by_provider`], 8124).
    fn failover_candidates(
        &self,
        refusals: &[ProviderRefusal],
        ladder: Option<&LadderRoute>,
        theta: Option<&HarnessParams>,
        stages: bool,
    ) -> Vec<DispatchCandidate> {
        let first = refusals
            .first()
            .map(|first| roko_learn::provider_failover::RefusedModel {
                model_key: first.model_key.clone(),
                model_slug: first.model_slug.clone(),
                provider_id: first.provider_id.clone(),
                provider_kind: first.provider_kind,
                profile: self
                    .resolve_candidate(&DispatchCandidate {
                        model_key: first.model_key.clone(),
                        config: None,
                    })
                    .model_profile,
            });
        let routing = self.factory.dispatcher().routing_ladder();
        let Some((route, routing)) = ladder.zip(routing) else {
            let first = first.as_ref();
            let mut candidates =
                roko_learn::provider_failover::failover_candidates(&self.config, first);
            if stages {
                for model_key in self.config.tool_loop_models() {
                    if !candidates
                        .iter()
                        .any(|candidate| candidate.model_key == model_key)
                    {
                        candidates.push(DispatchCandidate {
                            model_key,
                            config: None,
                        });
                    }
                }
            }
            self.order_by_provider(&mut candidates, theta);
            return candidates;
        };
        let mut candidates = first
            .map(|first| same_model_candidates(&self.config, &first))
            .unwrap_or_default();
        for rung in routing.rung_models_above(&route.role, route.rung) {
            candidates.push(DispatchCandidate {
                model_key: rung.model,
                config: None,
            });
        }
        self.order_by_provider(&mut candidates, theta);
        candidates
    }

    /// `candidates` in M1's provider order (B1 `provider_order`, 8124): the
    /// candidates on providers whose place in `theta`'s order differs from
    /// their place in `[providers]` trade slots among themselves, in θ's
    /// order, and every other candidate keeps its slot. θ₀ keeps the config's
    /// order, so it moves nothing. Nothing is added, so a reorder never adds
    /// a provider or re-enables a disabled one.
    fn order_by_provider(
        &self,
        candidates: &mut [DispatchCandidate],
        theta: Option<&HarnessParams>,
    ) {
        let Some(theta) = theta else {
            return;
        };
        let providers = HarnessLadders::from_config(&self.config).providers;
        // Each provider M1 moved, with its place in θ's order.
        let moved: HashMap<&str, usize> = theta
            .provider_order
            .iter()
            .enumerate()
            .filter(|&(place, &index)| place != index)
            .filter_map(|(place, &index)| Some((providers.get(index)?.as_str(), place)))
            .collect();
        if moved.is_empty() {
            return;
        }
        let mut slots: Vec<(usize, usize)> = Vec::new();
        for (slot, candidate) in candidates.iter().enumerate() {
            let provider = self.resolve_candidate(candidate).provider_id;
            if let Some(&place) = moved.get(provider.as_str()) {
                slots.push((slot, place));
            }
        }
        let mut reordered: Vec<(usize, DispatchCandidate)> = slots
            .iter()
            .map(|&(slot, place)| (place, candidates[slot].clone()))
            .collect();
        reordered.sort_by_key(|&(place, _)| place);
        for (&(slot, _), (_, candidate)) in slots.iter().zip(reordered) {
            candidates[slot] = candidate;
        }
    }

    /// The first usable model in [`Self::failover_candidates`], with the
    /// substitution and its reason logged at WARN. A candidate that would run
    /// unguarded in the shared checkout is passed over, and when that is all
    /// that stopped every candidate the error says so. So is one that cannot
    /// enforce `request`'s agent contract, such as a CLI agent for a task
    /// that holds its outbound tool calls for approval (gap-cf427e).
    fn failover_model(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        refusals: &[ProviderRefusal],
        ladder: Option<&LadderRoute>,
        theta: Option<&HarnessParams>,
        request: &AgentDispatchRequest,
    ) -> Result<DispatchCandidate> {
        let mut skipped = Vec::new();
        let mut only_unguarded = refusals
            .iter()
            .all(|refusal| refusal.class == UNGUARDED_IN_CHECKOUT);
        let stages = request.agent_contract.as_ref().is_some_and(|contract| {
            contract.outbound_policy() != roko_core::tool::OutboundPolicy::Allow
        });
        for candidate in self.failover_candidates(refusals, ladder, theta, stages) {
            let target = self.resolve_candidate(&candidate);
            if let Some(kind) = self.unguarded_in_checkout(&target) {
                log_unguarded_skip(&spec.plan_id, kind);
                let why = unguarded_reason(kind);
                skipped.push(format!("{}: {why}", candidate.model_key));
                continue;
            }
            only_unguarded = false;
            if let Err(error) = crate::dispatch_v2::validate_contract_support(request, &target) {
                skipped.push(format!("{}: {error}", candidate.model_key));
                continue;
            }
            match self.failover_candidate(&candidate, refusals) {
                Ok(provider_id) => {
                    if let Some(refused) = refusals.last() {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id,
                            from_model = %refused.model_key,
                            from_provider = %refused.provider_id,
                            to_model = %candidate.model_key,
                            to_provider = %provider_id,
                            reason = %refused.reason,
                            "model substitution: `{}` on `{}` is unusable ({}); running `{}` on `{provider_id}` instead",
                            refused.model_key,
                            refused.provider_id,
                            refused.reason,
                            candidate.model_key,
                        );
                    }
                    return Ok(candidate);
                }
                Err(why) => skipped.push(format!("{}: {why}", candidate.model_key)),
            }
        }
        if only_unguarded {
            let passed_over: Vec<String> = refusals
                .iter()
                .map(|refusal| {
                    format!(
                        "`{}` on `{}`: {}",
                        refusal.model_key, refusal.provider_id, refusal.reason
                    )
                })
                .chain(skipped)
                .collect();
            return Err(no_guarded_provider(&passed_over));
        }
        Err(self.no_usable_provider(refusals, &skipped, false, ladder))
    }

    /// The kind of agent `target` runs, when it is one roko cannot guard
    /// ([`UNGUARDED_CLI_KINDS`]) and this attempt would run it in the
    /// operator's shared checkout: the dispatcher has no per-task worktrees,
    /// and `[runner] allow_unguarded_agents_in_checkout` is off. The kind is
    /// the CLI protocol's when `target` runs a CLI, so a legacy
    /// `openai_compat` provider whose command is `codex` counts as Codex.
    fn unguarded_in_checkout(
        &self,
        target: &crate::dispatch_v2::ProviderDispatchSpec,
    ) -> Option<ProviderKind> {
        if self.workspace_provider.is_some()
            || self.config.runner.allow_unguarded_agents_in_checkout
        {
            return None;
        }
        let kind = match &target.runtime {
            crate::dispatch_v2::ProviderRuntime::Cli(cli) => cli.descriptor.provider_kind,
            _ => target.provider_kind,
        };
        UNGUARDED_CLI_KINDS.contains(&kind).then_some(kind)
    }

    /// The provider id of `candidate` when it can take the task now, else why not.
    fn failover_candidate(
        &self,
        candidate: &DispatchCandidate,
        refusals: &[ProviderRefusal],
    ) -> std::result::Result<String, String> {
        let target = self.resolve_candidate(candidate);
        let provider_id = &target.provider_id;
        let (Some(profile), Some(provider)) = (&target.model_profile, &target.provider_config)
        else {
            return Err(format!("provider `{provider_id}` is not configured"));
        };
        // A model passed over for want of a price leaves its provider usable.
        if refusals.iter().any(|refusal| {
            &refusal.provider_id == provider_id && refusal.class != UNPRICED_UNDER_CAP
        }) {
            return Err(format!("provider `{provider_id}` is unavailable too"));
        }
        if let crate::dispatch_v2::ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Err(format!(
                "provider `{provider_id}` is not dispatchable: {}",
                unsupported.detail
            ));
        }
        if self.config.routing.disabled_providers.contains(provider_id) {
            return Err(format!(
                "provider `{provider_id}` is in routing.disabled_providers"
            ));
        }
        if !profile.supports_tools && !provider_brings_own_tools(provider) {
            return Err("model does not support tools".to_string());
        }
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Err(missing_credentials_reason(provider, provider_id));
        }
        if self.unpriced_under_cap(&target) {
            return Err(UNPRICED_REASON.to_string());
        }
        let registry = &self.factory.health_registry;
        if !registry.is_available(provider_id) {
            let until = registry
                .get(provider_id)
                .cooldown_until
                .map(|ms| format!(" until {}", format_local_ms(ms)))
                .unwrap_or_default();
            return Err(format!("provider `{provider_id}` is quarantined{until}"));
        }
        Ok(provider_id.clone())
    }

    /// Non-retryable error for a task left without a usable provider, naming
    /// each refusal, each skipped fallback, and how to recover. A task the
    /// model ladder routed (`ladder`) had only the same model elsewhere and
    /// the rungs above its own to fall back on (backlog 1120).
    fn no_usable_provider(
        &self,
        refusals: &[ProviderRefusal],
        skipped: &[String],
        pinned: bool,
        ladder: Option<&LadderRoute>,
    ) -> RokoError {
        let refused = refusals
            .iter()
            .map(|refusal| {
                let until = refusal
                    .until_ms
                    .map(|ms| format!(" (skipped until {})", format_local_ms(ms)))
                    .unwrap_or_default();
                format!(
                    "`{}` on `{}`: {}{until}",
                    refusal.model_key, refusal.provider_id, refusal.reason
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let routed_rung = ladder.and_then(|route| {
            let routing = self.factory.dispatcher().routing_ladder()?;
            Some(routing.rung_name(&route.role, route.rung)?.to_string())
        });
        let fallback = if pinned {
            "The --model override pins this model, so no failover was attempted; drop --model to \
             allow it."
                .to_string()
        } else if let Some(rung) = routed_rung {
            let skipped = if skipped.is_empty() {
                String::new()
            } else {
                format!(" ({})", skipped.join("; "))
            };
            format!(
                "The model ladder routed this task to rung `{rung}`, and failover never moves it \
                 to a cheaper model: neither the same model elsewhere nor a rung above is \
                 usable{skipped}."
            )
        } else if skipped.is_empty() {
            format!(
                "No fallback model is configured: set [routing] fallback_models in roko.toml{} \
                 and put that provider's API key in ~/.roko/.env (loaded automatically at startup).",
                self.api_key_model_examples()
            )
        } else if skipped.iter().all(|skip| skip.contains(UNPRICED_REASON)) {
            // A price, not a key, would make them usable (gap-8548e3).
            format!(
                "No fallback model is usable ({}): extend [routing] fallback_models with a model \
                 that has a price.",
                skipped.join("; ")
            )
        } else {
            format!(
                "No fallback model is usable ({}): put the missing API key in ~/.roko/.env \
                 (loaded automatically at startup) or extend [routing] fallback_models.",
                skipped.join("; ")
            )
        };
        // A provider that rejected roko's credentials needs a login, and one
        // this task cannot use needs a configuration change: waiting brings
        // neither (backlog 1115, gap-cf427e).
        let mut fixes: Vec<String> = refusals
            .iter()
            .filter(|refusal| refusal.class == AUTH_FAILURE)
            .map(|refusal| self.credentials_hint(refusal))
            .collect();
        // A model with no price under a cap needs a price (gap-8548e3).
        let unpriced: Vec<&str> = refusals
            .iter()
            .filter(|refusal| refusal.class == UNPRICED_UNDER_CAP)
            .map(|refusal| refusal.model_key.as_str())
            .collect();
        if !unpriced.is_empty() || skipped.iter().any(|skip| skip.contains(UNPRICED_REASON)) {
            let named = if unpriced.is_empty() {
                "each model".to_string()
            } else {
                unpriced.join(", ")
            };
            fixes.push(format!(
                "Give {named} a price: cost_input_per_m and cost_output_per_m on its [models.*] \
                 entry in roko.toml (0 for a free local model), or run with --no-budget."
            ));
        }
        let waiting_helps =
            |refusal: &ProviderRefusal| !WAITING_DOES_NOT_HELP.contains(&refusal.class);
        fixes.push(fallback);
        if !refusals.iter().any(waiting_helps) {
            fixes.push("Then re-run.".to_string());
        } else {
            let wait = refusals
                .iter()
                .filter(|refusal| waiting_helps(refusal))
                .filter_map(|refusal| refusal.until_ms)
                .min()
                .map_or_else(
                    || "wait for the provider to recover".to_string(),
                    |ms| format!("wait until {}", format_local_ms(ms)),
                );
            fixes.push(format!("Otherwise {wait} and re-run."));
        }
        let fixes = fixes.join(" ");
        RokoError::Gateway {
            category: PROVIDER_EXHAUSTED_CATEGORY,
            retryable: false,
            message: format!("no usable provider for this task: {refused}. {fixes}"),
        }
    }

    /// What to do about `refusal`'s provider rejecting roko's credentials,
    /// and how to use it again before its skip ends (backlog 1115).
    fn credentials_hint(&self, refusal: &ProviderRefusal) -> String {
        let providers = self.config.effective_providers();
        let key_env = providers
            .get(&refusal.provider_id)
            .and_then(|provider| provider.api_key_env.as_deref());
        // The wording ACP gives too (gap-ade918).
        credentials_hint(&refusal.provider_id, refusal.provider_kind, key_env)
    }

    /// " (e.g. kimi-k2-5 needs MOONSHOT_API_KEY, …)" for configured API-key models.
    fn api_key_model_examples(&self) -> String {
        let providers = self.config.effective_providers();
        let mut examples: Vec<String> = self
            .config
            .effective_models()
            .into_iter()
            .filter(|(_, profile)| profile.supports_tools)
            .filter_map(|(key, profile)| {
                let env = providers.get(&profile.provider)?.api_key_env.clone()?;
                Some(format!("{key} needs {env}"))
            })
            .collect();
        examples.sort();
        examples.truncate(3);
        if examples.is_empty() {
            String::new()
        } else {
            format!(" (e.g. {})", examples.join(", "))
        }
    }
}

/// The non-retryable error for `dispatch`'s failed result when no retry can
/// change it, naming the reason and how to recover (backlog 1116):
/// - an immune preflight denial that holds for every attempt: the
///   workspace's isolation ledger cannot be read
///   (`isolation_state_unavailable`), or the attempt's agent id is not a
///   valid provider identity (`invalid_agent_identity`);
/// - a provider that rejected roko's credentials.
///
/// `None` for any other failure, which stays retryable. That includes
/// `agent_isolated`: each attempt runs under its own agent id (decision
/// 1107), so the next attempt is not isolated, and a quarantined output or
/// the stream cap, which a new call can pass.
pub(super) fn permanent_provider_denial(
    dispatch: &crate::dispatch_v2::AgentResultDispatch,
) -> Option<RokoError> {
    let output = &dispatch.result.output;
    let text = output.body.as_text().unwrap_or_default();
    let class = crate::dispatch_v2::classify_provider_error(&text.to_ascii_lowercase());
    let (denial, recovery) = if output.tag("immune_denied") == Some("true") {
        let reason = output.tag("immune_reason").unwrap_or_default();
        let recovery = match reason {
            "isolation_state_unavailable" => UNREADABLE_LEDGER_FIX,
            "invalid_agent_identity" => INVALID_AGENT_ID_FIX,
            _ => return None,
        };
        (
            format!("the immune boundary denied the attempt before any call ({reason})"),
            recovery.to_string(),
        )
    } else if class == AUTH_FAILURE {
        let key_env = dispatch
            .target
            .provider_config
            .as_ref()
            .and_then(|provider| provider.api_key_env.as_deref());
        (
            format!(
                "provider `{}` rejected its credentials ({text})",
                dispatch.target.provider_id
            ),
            credentials_fix(dispatch.target.provider_kind, key_env),
        )
    } else {
        return None;
    };
    Some(RokoError::Gateway {
        category: PROVIDER_DENIED_CATEGORY,
        retryable: false,
        message: format!("{denial}, which no retry can change: {recovery}"),
    })
}

/// Why an agent of `kind` may not take an attempt in the operator's shared
/// checkout.
fn unguarded_reason(kind: ProviderKind) -> String {
    format!("{kind} has no roko command guard, so it runs only in a per-task worktree")
}

/// Non-retryable error for an attempt in the operator's shared checkout that
/// only agents roko cannot guard could take; `passed_over` names each
/// (decision 1214).
fn no_guarded_provider(passed_over: &[String]) -> RokoError {
    RokoError::Gateway {
        category: UNGUARDED_IN_CHECKOUT,
        retryable: false,
        message: format!(
            "no guarded provider for a shared-checkout attempt: {}. Codex, Cursor and \
             Gemini CLI agents have no roko command guard, so they run only in per-task \
             worktrees: run the plan with --worktree-per-task, route the task to a guarded \
             provider such as claude_cli, or set [runner] allow_unguarded_agents_in_checkout \
             = true to accept the risk.",
            passed_over.join("; ")
        ),
    }
}

/// Log, once per plan and agent kind, that the plan's attempts in the shared
/// checkout pass over agents of `kind`.
fn log_unguarded_skip(plan_id: &str, kind: ProviderKind) {
    static LOGGED: OnceLock<Mutex<HashSet<(String, ProviderKind)>>> = OnceLock::new();
    let first = LOGGED
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .insert((plan_id.to_string(), kind));
    if first {
        tracing::warn!(
            plan_id,
            agent = %kind,
            "{kind} agents have no roko command guard, so this plan's attempts in the shared \
             checkout pass them over; run it with --worktree-per-task, or set [runner] \
             allow_unguarded_agents_in_checkout = true to accept the risk"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_graph::Cell;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_HANG_GUARD_SECS, FIXTURE_PROVIDER_TIMEOUT_MS, cli_provider, final_turn,
        fixture_config, jsonl_rows_where, make_bare_dispatcher, make_spec, make_task_def, model,
        rate_limited_turn, recording_feedback, refused_turn, spawn_openai_mock, tool_call_turn,
    };

    // ─── Provider failover on usage exhaustion ──────────────────────────────

    fn write_executable(path: &Path, body: &str) {
        std::fs::write(path, body).expect("write script");
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("make script executable");
    }

    /// A fake `claude` that refuses exactly as the CLI did on 2026-09-26 and
    /// appends one line to `calls` per invocation.
    fn session_limit_claude(path: &Path, calls: &Path) {
        write_executable(
            path,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo called >> '{}'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":true,"total_cost_usd":0,"result":"You’ve hit your session limit · resets 4pm (Europe/Berlin)"}}'
exit 1
"#,
                calls.display()
            ),
        );
    }

    fn invocations(calls: &Path) -> usize {
        std::fs::read_to_string(calls).map_or(0, |log| log.lines().count())
    }

    /// `claude_cli` (fake script) plus two OpenAI-compatible fallbacks: one
    /// whose key env var is never set, and one pointed at `api_base_url`.
    fn failover_config(claude: &Path, api_base_url: &str, fallback_models: &[&str]) -> RokoConfig {
        let provider =
            |kind, command: Option<String>, base_url: Option<&str>, key_env: Option<&str>| {
                ProviderConfig {
                    kind,
                    base_url: base_url.map(str::to_string),
                    api_key_env: key_env.map(str::to_string),
                    command,
                    args: None,
                    timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                    require_confirmation: false,
                    stream_usage: None,
                    billing: None,
                }
            };
        let api_model = |provider: &str, slug: &str| ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            context_window: 128_000,
            max_output: Some(1_024),
            // Unknown slugs default to a 3-tool cap, which would drop `bash`.
            max_tools: Some(32),
            supports_tools: true,
            tool_format: "openai_json".to_string(),
            ..ModelProfile::default()
        };
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "claude_cli".to_string(),
            provider(
                ProviderKind::ClaudeCli,
                Some(claude.display().to_string()),
                None,
                None,
            ),
        );
        // `PATH` is always set, standing in for a key loaded from ~/.roko/.env.
        config.providers.insert(
            "mock_api".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some(api_base_url),
                Some("PATH"),
            ),
        );
        config.providers.insert(
            "keyless_api".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some("http://127.0.0.1:9/v1"),
                Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"),
            ),
        );
        config.models.insert(
            "claude-sonnet".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        config.models.insert(
            "keyless-model".to_string(),
            api_model("keyless_api", "keyless-1"),
        );
        config.models.insert(
            "api-model".to_string(),
            api_model("mock_api", "api-model-1"),
        );
        config.routing.fallback_models = fallback_models.iter().map(|m| m.to_string()).collect();
        // The mock API answers without SSE. The stall watchdog would attach
        // live output, over which the tool loop streams, so keep it off.
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        config
    }

    /// A live cell for an implementer task hinted at `model_hint`.
    fn failover_cell(
        dispatcher: Arc<GraphTaskDispatcher>,
        model_hint: &str,
    ) -> roko_graph::cells::TaskExecutorCell {
        failover_cell_with_retries(dispatcher, model_hint, 2)
    }

    /// [`failover_cell`] with `max_retries` retries.
    fn failover_cell_with_retries(
        dispatcher: Arc<GraphTaskDispatcher>,
        model_hint: &str,
        max_retries: u32,
    ) -> roko_graph::cells::TaskExecutorCell {
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            description: Some("Edit notes and write hello.txt".to_string()),
            model_hint: Some(model_hint.to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            max_retries,
            ..make_task_def("focused")
        };
        let config = toml::Value::Table(toml::map::Map::from_iter([
            (
                "plan_id".to_string(),
                toml::Value::String("p-failover".to_string()),
            ),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            (
                "timeout_secs".to_string(),
                toml::Value::Integer(FIXTURE_HANG_GUARD_SECS as i64),
            ),
            (
                "max_retries".to_string(),
                toml::Value::Integer(i64::from(max_retries)),
            ),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        roko_graph::cells::TaskExecutorCell::live(config, dispatcher)
    }

    #[tokio::test]
    async fn exhausted_claude_cli_fails_over_to_api_model_with_file_and_shell_tools() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        std::fs::write(workdir.join("notes.txt"), "draft notes\n").expect("seed notes");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let (base_url, requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "call-read",
                "read_file",
                serde_json::json!({ "path": "notes.txt" }),
            ),
            tool_call_turn(
                "call-edit",
                "edit_file",
                serde_json::json!({
                    "path": "notes.txt", "old_string": "draft", "new_string": "final"
                }),
            ),
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "hello.txt", "content": "hi from fallback" }),
            ),
            tool_call_turn(
                "call-shell",
                "bash",
                serde_json::json!({ "command": "cat hello.txt" }),
            ),
            final_turn("fallback finished"),
            final_turn("second task on fallback"),
        ]);
        let config = Arc::new(failover_config(
            &claude,
            &base_url,
            &["keyless-model", "api-model"],
        ));
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(Arc::clone(&health)),
        );
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            workdir.clone(),
        ));
        let cell = failover_cell(dispatcher, "claude-sonnet-4-6");

        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect("failover completes the task within one attempt");
        assert_eq!(output[0].body.as_text().expect("text"), "fallback finished");
        assert_eq!(invocations(&calls), 1, "the refusal is not retried");
        // Tool results the fallback model saw, in order: read, edit, write, shell.
        let tool_results: Vec<String> = requests.lock()[4]["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .filter(|message| message["role"] == "tool")
            .map(|message| message["content"].to_string())
            .collect();
        assert_eq!(tool_results.len(), 4, "{tool_results:#?}");
        assert_eq!(
            std::fs::read_to_string(workdir.join("notes.txt")).expect("notes"),
            "final notes\n",
            "{tool_results:#?}"
        );
        assert_eq!(
            std::fs::read_to_string(workdir.join("hello.txt")).expect("hello"),
            "hi from fallback",
            "{tool_results:#?}"
        );
        assert!(
            tool_results[3].contains("hi from fallback"),
            "shell output reaches the model: {tool_results:#?}"
        );

        // claude_cli stays quarantined until the parsed "resets 4pm".
        let claude_health = health.get("claude_cli");
        assert_eq!(
            claude_health.state,
            roko_learn::provider_health::CircuitState::Open
        );
        let until = claude_health.cooldown_until.expect("quarantine end");
        let now = chrono::Utc::now().timestamp_millis();
        assert!(until > now && until <= now + 24 * 3_600_000, "{until}");

        // The next dispatch skips the quarantined CLI without calling it.
        let second = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08-next".to_string()),
            )
            .await
            .expect("next dispatch runs on the fallback");
        assert_eq!(
            second[0].body.as_text().expect("text"),
            "second task on fallback"
        );
        assert_eq!(invocations(&calls), 1);
        let requests = requests.lock();
        assert_eq!(requests.len(), 6);
        assert!(
            requests
                .iter()
                .all(|request| request["model"] == "api-model-1"),
            "every API turn names the fallback model"
        );
    }

    /// One attempt of a task planned for `claude-sonnet-4-6` on a Claude CLI
    /// that refuses with its session limit, which fails over to
    /// `api-model`, answering as `api-model-1`, with every record written
    /// under the returned workdir's `.roko` for the returned run.
    async fn fail_over_from_an_exhausted_cli(temp: &tempfile::TempDir) -> (PathBuf, &'static str) {
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let mut answer = final_turn("fallback finished");
        answer["model"] = serde_json::json!("api-model-1");
        let (base_url, _requests) = spawn_openai_mock(vec![answer]);
        let config = Arc::new(failover_config(&claude, &base_url, &["api-model"]));
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(health),
        );
        let dispatcher = GraphTaskDispatcher::new(factory, Arc::clone(&config), workdir.clone())
            .with_feedback(recording_feedback(&workdir));
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            model_hint: Some("claude-sonnet-4-6".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let run = "graph-failover-run";
        dispatcher
            .dispatch(
                &make_spec(&task),
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .expect("the fallback runs the task");
        drop(dispatcher);
        assert_eq!(invocations(&calls), 1);
        (workdir, run)
    }

    /// A call a provider refused for its usage limit is on the attempt's
    /// records (bug-220385): the verdict and episode list the refusal with
    /// its class, and the refused call has its own cost and efficiency rows.
    #[tokio::test]
    async fn an_exhaustion_refusal_during_failover_is_recorded() {
        let temp = tempdir().expect("tempdir");
        let (workdir, run) = fail_over_from_an_exhausted_cli(&temp).await;

        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        let key = verdict["attempt_key"].as_str().expect("attempt key");
        let planned = verdict["executed"]["model_requested"]
            .as_str()
            .expect("planned model");
        let refusals = verdict["executed"]["failover_refusals"]
            .as_array()
            .expect("failover refusals");
        assert_eq!(refusals.len(), 1, "{verdict}");
        let refusal = &refusals[0];
        assert_eq!(refusal["model"], planned);
        assert_eq!(refusal["provider"], "claude_cli");
        assert_eq!(refusal["class"], "provider_exhausted");
        assert_eq!(refusal["called"], true);
        assert!(
            refusal["at"].is_i64() && refusal["until"].is_i64(),
            "{refusal}"
        );
        let reason = refusal["reason"].as_str().unwrap_or_default();
        assert!(reason.contains("session limit"), "{reason}");

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(
            &workdir.join(".roko/episodes.jsonl"),
        )
        .await
        .expect("episodes");
        assert_eq!(
            episodes[0].extra["failover_refusals"][0]["class"],
            "provider_exhausted"
        );

        let refused = |row: &serde_json::Value| row["role"] == FAILOVER_REFUSED_ROLE;
        let costs = jsonl_rows_where(&workdir.join(".roko/learn/costs.jsonl"), 1, refused).await;
        assert_eq!(costs.len(), 1);
        assert_eq!(costs[0]["attempt_key"], key);
        assert_eq!(costs[0]["provider"], "claude_cli");
        assert_eq!(costs[0]["success"], false);
        let efficiency =
            jsonl_rows_where(&workdir.join(".roko/learn/efficiency.jsonl"), 1, |row| {
                row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
                    && refused(row)
            })
            .await;
        assert_eq!(efficiency[0]["attempt_id"], format!("{key}/refused-1"));
    }

    /// A task planned for an exhausted provider runs on a fallback: the
    /// attempt's verdict, episode, cost and efficiency rows name the planned
    /// model, why it did not run, and the model that did.
    #[tokio::test]
    async fn failover_records_planned_and_substitute_model() {
        let temp = tempdir().expect("tempdir");
        let (workdir, run) = fail_over_from_an_exhausted_cli(&temp).await;

        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let executed = &verdicts[0]["executed"];
        let planned = executed["model_requested"].as_str().expect("planned model");
        assert_eq!(executed["failover_chain"], serde_json::json!([planned]));
        let reason = executed["failover_reason"]
            .as_str()
            .expect("failover reason");
        assert!(reason.contains("session limit"), "{reason}");
        assert!(reason.contains("claude_cli"), "{reason}");
        assert_eq!(executed["provider"], "mock_api");
        assert_eq!(executed["model_dispatched"], "api-model-1");
        assert_eq!(executed["model_reported"], "api-model-1");
        assert_eq!(executed["model_mismatch"], false);

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(
            &workdir.join(".roko/episodes.jsonl"),
        )
        .await
        .expect("episodes");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].model, "api-model-1");
        assert_eq!(episodes[0].extra["substituted_from"], planned);
        assert_eq!(episodes[0].extra["failover_reason"], reason);
        // The refused call has rows of its own; these are the call that ran.
        let ran = |row: &serde_json::Value| row["role"] != FAILOVER_REFUSED_ROLE;
        let costs = jsonl_rows_where(&workdir.join(".roko/learn/costs.jsonl"), 1, ran).await;
        assert_eq!(costs[0]["model"], "api-model-1");
        assert_eq!(costs[0]["substituted_from"], planned);
        assert_eq!(costs[0]["substitution_reason"], reason);
        let efficiency =
            jsonl_rows_where(&workdir.join(".roko/learn/efficiency.jsonl"), 1, |row| {
                row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA && ran(row)
            })
            .await;
        assert_eq!(efficiency[0]["model"], "api-model-1");
        assert_eq!(efficiency[0]["substituted_from"], planned);
    }

    /// bug-0b7695: an attempt whose planned model refused it (a usage-limit
    /// provider error) fails over, and its records name the model that
    /// answered while keeping the one routing planned: the verdict's
    /// `model_dispatched` beside its `model_requested`, and the episode's
    /// `successful_model` beside its `initial_model`, so the self-model reads
    /// the failover as a routing miss. The attempt's cost row, its verdict
    /// and the plan budget are priced at the answering model's rate, never
    /// at the planned one's.
    #[tokio::test]
    async fn failover_attempt_records_the_model_that_answered() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let mut answer = final_turn("fallback finished");
        answer["model"] = serde_json::json!("api-model-1");
        let (base_url, _requests) = spawn_openai_mock(vec![answer]);
        let mut config = failover_config(&claude, &base_url, &["api-model"]);
        // The planned model is priced far above the one that answers, so a
        // cost at the planned rate would show.
        for (model, input, output) in [("claude-sonnet", 1_000.0, 1_000.0), ("api-model", 2.0, 8.0)]
        {
            let profile = config.models.get_mut(model).expect("configured model");
            profile.cost_input_per_m = Some(input);
            profile.cost_output_per_m = Some(output);
        }
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = GraphTaskDispatcher::new(factory, Arc::clone(&config), workdir.clone())
            .with_plan_budget(1.0, 0.5, false)
            .with_feedback(recording_feedback(&workdir));
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            model_hint: Some("claude-sonnet-4-6".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let spec = make_spec(&task);
        let run = "graph-failover-answered";
        dispatcher
            .dispatch(
                &spec,
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .expect("the fallback runs the task");
        // The answer's 12 input and 3 output tokens, at the answering
        // model's $2 and $8 a million.
        let answered_usd = (12.0 * 2.0 + 3.0 * 8.0) / 1_000_000.0;
        let spent = dispatcher.plan_budget_snapshot(&spec.plan_id).spent_usd;
        assert!((spent - answered_usd).abs() < 1e-9, "plan spend {spent}");
        drop(dispatcher);

        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        let executed = &verdict["executed"];
        let planned = executed["model_requested"].as_str().expect("planned model");
        assert_ne!(planned, "api-model-1", "{executed}");
        assert_eq!(executed["model_dispatched"], "api-model-1", "{executed}");
        assert_eq!(executed["provider"], "mock_api", "{executed}");
        let billed = verdict["cost"]["billed_usd"].as_f64().expect("billed_usd");
        assert!((billed - answered_usd).abs() < 1e-9, "{verdict}");

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(
            &workdir.join(".roko/episodes.jsonl"),
        )
        .await
        .expect("episodes");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].model, "api-model-1");
        assert_eq!(episodes[0].extra["successful_model"], "api-model-1");
        assert_eq!(episodes[0].extra["initial_model"], planned);

        // The refused call is accounted on a row of its own.
        let ran = |row: &serde_json::Value| row["role"] != FAILOVER_REFUSED_ROLE;
        let costs = jsonl_rows_where(&workdir.join(".roko/learn/costs.jsonl"), 1, ran).await;
        assert_eq!(costs[0]["model"], "api-model-1");
        let cost = costs[0]["cost_usd"].as_f64().expect("cost_usd");
        assert!((cost - answered_usd).abs() < 1e-9, "{}", costs[0]);
    }

    /// bug-8bcd98: in run 7b8cb121, T2#1's verdict billed $0.0354 while its
    /// calls cost $0.2430: glm-4.7 worked until z.ai's 429 failed it over,
    /// and the verdict counted only the model that ran after. The routed
    /// model here reads a file, then its provider answers 429 past the tool
    /// loop's retries, and a backup model finishes the try. The verdict
    /// carries the refused call's cost beside the billed one, and the two
    /// add up to the try's `call_cost` events and to the plan's spend.
    #[tokio::test]
    async fn a_failed_over_try_s_verdict_counts_its_refused_calls() {
        use roko_core::DashboardEvent;

        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        std::fs::write(workdir.join("notes.txt"), "notes\n").expect("seed notes");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &temp.path().join("claude-calls.log"));
        // The routed model's read: 100k tokens in and 1k out, $0.102 at $1
        // and $2 a million.
        let mut read = tool_call_turn(
            "call-read",
            "read_file",
            serde_json::json!({ "path": "notes.txt" }),
        );
        read["usage"] = serde_json::json!({
            "prompt_tokens": 100_000,
            "completion_tokens": 1_000,
            "total_tokens": 101_000
        });
        let tries = roko_agent::retry::RetryPolicy::default().max_attempts as usize;
        let mut routed = vec![read];
        routed.extend(std::iter::repeat_with(|| rate_limited_turn(1)).take(tries));
        let (routed_url, routed_requests) = spawn_openai_mock(routed);
        // The backup's answer: 10k in and 500 out, $0.011 at the same rates.
        let mut answer = final_turn("finished on the backup");
        answer["model"] = serde_json::json!("backup-1");
        answer["usage"] = serde_json::json!({
            "prompt_tokens": 10_000,
            "completion_tokens": 500,
            "total_tokens": 10_500
        });
        let (backup_url, _backup_requests) = spawn_openai_mock(vec![answer]);
        let mut config = failover_config(&claude, &routed_url, &["backup-model"]);
        config.agent.default_model = "api-model".to_string();
        let mut backup_api = config.providers["mock_api"].clone();
        backup_api.base_url = Some(backup_url);
        config
            .providers
            .insert("backup_api".to_string(), backup_api);
        let mut backup = config.models["api-model"].clone();
        backup.provider = "backup_api".to_string();
        backup.slug = "backup-1".to_string();
        config.models.insert("backup-model".to_string(), backup);
        for model in ["api-model", "backup-model"] {
            let profile = config.models.get_mut(model).expect("configured model");
            profile.cost_input_per_m = Some(1.0);
            profile.cost_output_per_m = Some(2.0);
        }
        let hub = crate::state_hub::StateHub::default_capacity();
        let mut events = hub.subscribe_events();
        let runs_dir = workdir.join(".roko/runs");
        let dispatcher = make_bare_dispatcher(config, &workdir)
            .await
            .with_plan_budget(1.0, 0.5, false)
            .with_tui_bridge(crate::runner::tui_bridge::TuiBridge::new(hub.sender()))
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            });
        let task = TaskDef {
            model_hint: Some("api-model".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let spec = make_spec(&task);
        let run = "refused-spend";
        dispatcher
            .dispatch(
                &spec,
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .expect("the backup finishes the try");
        assert_eq!(
            routed_requests.lock().len(),
            1 + tries,
            "the read, then 429s"
        );
        let spent = dispatcher.plan_budget_snapshot(&spec.plan_id).spent_usd;
        drop(dispatcher);

        let verdicts = jsonl_rows_where(&runs_dir.join(run).join("attempts.jsonl"), 1, |row| {
            row["schema_version"] == "roko.verdict/1"
        })
        .await;
        let verdict = &verdicts[0];
        assert_eq!(
            verdict["executed"]["model_dispatched"], "backup-1",
            "{verdict}"
        );
        // A call's cost is an f32: equal to about seven digits.
        let usd = |field: &str| verdict["cost"][field].as_f64().unwrap_or(f64::NAN);
        assert!((usd("billed_usd") - 0.011).abs() < 1e-6, "{verdict}");
        assert!((usd("refused_usd") - 0.102).abs() < 1e-6, "{verdict}");

        // Every call the try made reached the hub with its cost, and they
        // add up to the verdict's billed and refused calls, as the plan's
        // budget counted them.
        let mut calls = Vec::new();
        while let Ok(envelope) = events.try_recv() {
            if let DashboardEvent::CallCost {
                model, cost_usd, ..
            } = envelope.payload
            {
                calls.push((model, cost_usd.expect("a priced call")));
            }
        }
        let models: Vec<&str> = calls.iter().map(|(model, _)| model.as_str()).collect();
        assert_eq!(models, ["api-model-1", "backup-1"], "{calls:?}");
        let calls_usd: f64 = calls.iter().map(|(_, usd)| usd).sum();
        assert!((calls_usd - 0.113).abs() < 1e-6, "{calls:?}");
        assert!((spent - 0.113).abs() < 1e-6, "plan spend {spent}");
    }

    #[tokio::test]
    async fn exhausted_provider_without_usable_fallback_fails_once_with_fix_hint() {
        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let config = Arc::new(failover_config(
            &claude,
            "http://127.0.0.1:9/v1",
            &["keyless-model"],
        ));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let error = failover_cell(dispatcher, "claude-sonnet-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect_err("no usable provider fails the task");

        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_EXHAUSTED_CATEGORY);
        assert!(!retryable);
        assert!(
            message.contains("You’ve hit your session limit"),
            "{message}"
        );
        assert!(
            message.contains("keyless-model: ROKO_TEST_FAILOVER_KEY_NEVER_SET is not set"),
            "{message}"
        );
        assert!(message.contains("~/.roko/.env"), "{message}");
        assert_eq!(
            invocations(&calls),
            1,
            "max_retries = 2 must not re-run an exhausted provider"
        );
    }

    /// bug-c55f1c: one call refused for usage exhaustion is one failure in
    /// its provider's health, though the bridge's classifier and failover's
    /// quarantine both see it, and the quarantine ends at the reset the
    /// provider reported, not at the classifier's default cooldown.
    #[tokio::test]
    async fn one_exhaustion_counts_as_one_failure_record() {
        use roko_agent::provider::error_classify::detect_provider_exhaustion;
        use roko_learn::provider_health::{ErrorClass, ProviderHealthRegistry};

        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let (base_url, _requests) = spawn_openai_mock(vec![final_turn("fallback finished")]);
        let config = Arc::new(failover_config(&claude, &base_url, &["api-model"]));
        let health = Arc::new(ProviderHealthRegistry::new());
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(Arc::clone(&health)),
        );
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        failover_cell(dispatcher, "claude-sonnet-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect("the task fails over to the API model");

        assert_eq!(invocations(&calls), 1);
        let claude_health = health.get("claude_cli");
        assert_eq!(claude_health.total_failures, 1, "{claude_health:?}");
        let classes: Vec<ErrorClass> = claude_health
            .failure_window
            .iter()
            .map(|failure| failure.error_class)
            .collect();
        assert_eq!(classes, [ErrorClass::Exhausted]);
        let refusal = "You\u{2019}ve hit your session limit \u{b7} resets 4pm (Europe/Berlin)";
        let reset = detect_provider_exhaustion(refusal).and_then(|refused| refused.resets_at_ms);
        assert!(reset.is_some());
        assert_eq!(claude_health.cooldown_until, reset);
    }

    /// backlog 1115: one auth failure takes its provider out of the run.
    /// With no usable alternative the attempt fails non-retryably before any
    /// call, and the error says how to log in rather than to wait.
    #[tokio::test]
    async fn auth_failure_refuses_provider_with_login_hint() {
        use roko_learn::provider_health::{ErrorClass, ProviderHealthRegistry};

        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo called >> '{}'
echo 'Not logged in' >&2
exit 1
"#,
                calls.display()
            ),
        );
        let config = Arc::new(failover_config(
            &claude,
            "http://127.0.0.1:9/v1",
            &["keyless-model"],
        ));
        let health = Arc::new(ProviderHealthRegistry::new());
        health.record_failure("claude_cli", ErrorClass::AuthFailure);
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(health),
        );
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let error = failover_cell(dispatcher, "claude-sonnet-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect_err("a provider that rejected its credentials fails the task");

        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_EXHAUSTED_CATEGORY);
        assert!(!retryable);
        assert!(
            message.contains("not logged in or key rejected"),
            "{message}"
        );
        assert!(message.contains("run `claude /login`"), "{message}");
        assert!(message.contains("USER and HOME"), "{message}");
        assert!(message.contains("reset-health claude_cli"), "{message}");
        assert!(message.contains("Then re-run."), "{message}");
        assert!(!message.contains("wait until"), "{message}");
        assert_eq!(invocations(&calls), 0, "no call reaches the provider");
    }

    /// backlog 1116: a denial no retry can change fails the task at once,
    /// with how to recover, though five retries are left: a CLI that is not
    /// logged in is called once, and an isolation ledger the immune boundary
    /// cannot read denies one attempt before any call, and no other.
    #[tokio::test]
    async fn permanent_provider_denial_is_not_retried() {
        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo called >> '{}'
echo 'Not logged in. Please run /login' >&2
exit 1
"#,
                calls.display()
            ),
        );
        let config = Arc::new(failover_config(&claude, "http://127.0.0.1:9/v1", &[]));

        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let error = failover_cell_with_retries(dispatcher, "claude-sonnet-4-6", 5)
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect_err("a CLI that is not logged in fails the task");
        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_DENIED_CATEGORY);
        assert!(!retryable);
        assert!(message.contains("rejected its credentials"), "{message}");
        assert!(message.contains("run `claude /login`"), "{message}");
        assert_eq!(invocations(&calls), 1, "the denial is not retried");

        // An isolation ledger the immune boundary cannot read denies every
        // attempt before its call. The new factory's registry does not hold
        // the CLI's open circuit from above.
        let immune_dir = temp.path().join(".roko/immune");
        std::fs::create_dir_all(&immune_dir).expect("create immune dir");
        std::fs::write(immune_dir.join("agent-controls.json"), "not json").expect("corrupt");
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let error = failover_cell_with_retries(Arc::clone(&dispatcher), "claude-sonnet-4-6", 5)
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect_err("an unreadable isolation ledger fails the task");
        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_DENIED_CATEGORY);
        assert!(!retryable);
        assert!(message.contains("isolation_state_unavailable"), "{message}");
        assert!(message.contains("roko safety controls"), "{message}");
        assert_eq!(invocations(&calls), 1, "no call reaches the provider");
        assert_eq!(
            dispatcher.task_attempts.lock().get("p-failover/T08"),
            Some(&1)
        );
    }

    // ─── Failover along the model ladder (backlog 1120) ─────────────────────

    /// A fake `claude` that appends the model of each call to `models`, then
    /// answers.
    fn model_logging_claude(path: &Path, models: &Path) {
        write_executable(
            path,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
previous=
for arg in "$@"; do
  if [ "$previous" = "--model" ]; then
    printf '%s\n' "$arg" >> '{}'
  fi
  previous=$arg
done
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ran"}}}}'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}}'
"#,
                models.display()
            ),
        );
    }

    /// The models [`model_logging_claude`] was called with, in order.
    fn logged_models(models: &Path) -> Vec<String> {
        std::fs::read_to_string(models)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// A model ladder of `rungs` (name, model key) over three models:
    /// `cheap-model` and `strong-model` on Claude CLIs that run `claude`, and
    /// `mid-model` on an OpenAI-compatible API that is not reached unless a
    /// test points it at a mock. The cheap model is also the default and the
    /// fallback, where the failover of a task the ladder did not route goes.
    fn ladder_failover_config(claude: &Path, rungs: &[(&str, &str)]) -> RokoConfig {
        let claude = claude.display().to_string();
        let mut mid_api = cli_provider(&claude);
        mid_api.kind = ProviderKind::OpenAiCompat;
        mid_api.command = None;
        mid_api.base_url = Some("http://127.0.0.1:9/v1".to_string());
        // `PATH` is always set, standing in for a key.
        mid_api.api_key_env = Some("PATH".to_string());
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.bare_mode = false;
        config
            .providers
            .insert("cheap_cli".to_string(), cli_provider(&claude));
        config
            .providers
            .insert("strong_cli".to_string(), cli_provider(&claude));
        config.providers.insert("mid_api".to_string(), mid_api);
        config.models.insert(
            "cheap-model".to_string(),
            model("cheap_cli", "claude-haiku-4-5", None),
        );
        config.models.insert(
            "mid-model".to_string(),
            ModelProfile {
                context_window: 128_000,
                max_output: Some(1_024),
                max_tools: Some(32),
                tool_format: "openai_json".to_string(),
                ..model("mid_api", "mid-1", None)
            },
        );
        config.models.insert(
            "strong-model".to_string(),
            model("strong_cli", "claude-sonnet-4-6", None),
        );
        config.agent.default_model = "cheap-model".to_string();
        config.routing.fallback_models = vec!["cheap-model".to_string()];
        config.routing.ladder.rungs = rungs
            .iter()
            .map(|&(name, model_key)| roko_core::config::routing::LadderRung::new(name, model_key))
            .collect();
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        config
    }

    /// backlog 1120 (decision 1119): the ladder routes a task to its `mid`
    /// rung, whose provider's circuit is open. Failover runs the task on the
    /// `strong` rung's model, not on the cheap default and fallback model,
    /// and the verdict records the rung that ran. With no usable rung above
    /// `mid`, the attempt fails with the no-usable-provider error, and the
    /// cheap model still never runs.
    #[tokio::test]
    async fn open_circuit_on_rung_fails_over_to_next_rung() {
        use roko_learn::provider_health::ErrorClass;

        const RUN: &str = "ladder-failover";
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let mut task = make_task_def("mechanical");
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        task.hints.rung = Some("mid".to_string());
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let runs_dir = temp.path().join(".roko/runs");
        let config = ladder_failover_config(&claude, &rungs);
        let dispatcher = make_bare_dispatcher(config, temp.path())
            .await
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            });
        for _ in 0..3 {
            dispatcher
                .factory
                .health_registry
                .record_failure("mid_api", ErrorClass::ServerError);
        }
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the strong rung's model runs the task");
        assert_eq!(logged_models(&models), ["claude-sonnet-4-6"]);
        let attempts = runs_dir.join(RUN).join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 1, is_verdict).await;
        let verdict = &verdicts[0];
        assert_eq!(verdict["ladder"]["rung"], "strong", "{verdict}");
        assert_eq!(verdict["ladder"]["reason"], "failover", "{verdict}");
        assert_eq!(verdict["executed"]["provider"], "strong_cli", "{verdict}");
        assert_eq!(
            verdict["executed"]["failover_chain"],
            serde_json::json!(["mid-1"]),
            "{verdict}"
        );

        // `mid` tops this ladder, and its provider rejected roko's
        // credentials, so no model is left that the ladder allows.
        let rungs = [("cheap", "cheap-model"), ("mid", "mid-model")];
        let config = ladder_failover_config(&claude, &rungs);
        let dispatcher = make_bare_dispatcher(config, temp.path()).await;
        dispatcher
            .factory
            .health_registry
            .record_failure("mid_api", ErrorClass::AuthFailure);
        let error = dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("no rung above `mid` can take the task");
        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_EXHAUSTED_CATEGORY);
        assert!(!retryable);
        assert!(message.contains("rung `mid`"), "{message}");
        assert!(message.contains("cheaper model"), "{message}");
        assert_eq!(logged_models(&models), ["claude-sonnet-4-6"]);
    }

    // ─── A rate limit (HTTP 429) never spends a try (bug-4b6844) ───────────

    /// [`failover_config`] whose task model is the mock's `api-model`, with
    /// nothing to fail over to: no fallback models, and the default model is
    /// `api-model` itself.
    fn rate_limited_config(claude: &Path, api_base_url: &str) -> RokoConfig {
        let mut config = failover_config(claude, api_base_url, &[]);
        config.agent.default_model = "api-model".to_string();
        config.agent.fallback_model = None;
        config
    }

    /// A dispatcher over `config` working in `workdir`, its provider health
    /// kept in `temp`.
    async fn rate_limited_dispatcher(
        config: RokoConfig,
        temp: &tempfile::TempDir,
        workdir: &Path,
    ) -> Arc<GraphTaskDispatcher> {
        let config = Arc::new(config);
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(health),
        );
        Arc::new(GraphTaskDispatcher::new(
            factory,
            config,
            workdir.to_path_buf(),
        ))
    }

    /// bug-4b6844: in round v6, z.ai answered 429 to a task's calls on mid,
    /// and each half-second refusal settled as one of the task's tries, so the
    /// task ended with no real try. A provider that answers 429 twice and then
    /// serves the call costs the task no try: with no retries at all, it still
    /// passes, in the one try.
    #[tokio::test]
    async fn a_429_from_the_rungs_provider_does_not_spend_a_try() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let (base_url, requests) = spawn_openai_mock(vec![
            rate_limited_turn(1),
            rate_limited_turn(1),
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "hello.txt", "content": "hi after the limit" }),
            ),
            final_turn("wrote hello.txt"),
        ]);
        let config = rate_limited_config(&claude, &base_url);
        let dispatcher = rate_limited_dispatcher(config, &temp, &workdir).await;
        let cell = failover_cell_with_retries(dispatcher, "api-model", 0);

        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect("the rate limit passes and the one try completes the task");
        assert_eq!(output[0].body.as_text().expect("text"), "wrote hello.txt");
        assert_eq!(
            std::fs::read_to_string(workdir.join("hello.txt")).expect("hello.txt"),
            "hi after the limit"
        );
        assert_eq!(requests.lock().len(), 4, "429, 429, write, done");
        assert_eq!(invocations(&calls), 0, "nothing failed over");
    }

    /// bug-4b6844: a rung whose provider keeps answering 429 fails over
    /// within the try, to the rung above, as an open circuit does: one try,
    /// run by the strong rung's model, with the mid model in its failover
    /// chain.
    #[tokio::test]
    async fn a_rung_whose_provider_keeps_answering_429_fails_over_within_the_try() {
        const RUN: &str = "rate-limited-rung";
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let tries = roko_agent::retry::RetryPolicy::default().max_attempts as usize;
        let (base_url, requests) = spawn_openai_mock(
            std::iter::repeat_with(|| rate_limited_turn(1))
                .take(tries)
                .collect(),
        );
        let mut task = make_task_def("mechanical");
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        task.hints.rung = Some("mid".to_string());
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let mut config = ladder_failover_config(&claude, &rungs);
        let mid_api = config.providers.get_mut("mid_api").expect("mid_api");
        mid_api.base_url = Some(base_url);
        let runs_dir = temp.path().join(".roko/runs");
        let dispatcher = make_bare_dispatcher(config, temp.path())
            .await
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            });

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the strong rung's model runs the task in the same try");
        assert_eq!(requests.lock().len(), tries, "the mid model's calls");
        assert_eq!(logged_models(&models), ["claude-sonnet-4-6"]);
        let attempts = runs_dir.join(RUN).join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 1, is_verdict).await;
        assert_eq!(verdicts.len(), 1, "{verdicts:?}");
        let verdict = &verdicts[0];
        // The task has no checks, so its one try settles unverified: served,
        // not a provider error.
        assert_eq!(verdict["outcome"], "unverified", "{verdict}");
        assert_eq!(verdict["ladder"]["rung"], "strong", "{verdict}");
        assert_eq!(verdict["ladder"]["reason"], "failover", "{verdict}");
        assert_eq!(
            verdict["executed"]["failover_chain"],
            serde_json::json!(["mid-1"]),
            "{verdict}"
        );
    }

    /// bug-390e4b: a provider that answers that it can't bill the account,
    /// z.ai's 429 with code 1113 or a 402 as Cerebras sends it, is out of
    /// usage at its first answer. The try fails over to the rung above with
    /// no retry and no wait, the hub says why in plain words, and the next try
    /// routes past the provider without calling it.
    #[tokio::test]
    async fn a_provider_that_cannot_bill_is_skipped_at_its_first_answer() {
        let refusals = [
            (
                refused_turn(
                    429,
                    serde_json::json!({ "error": {
                        "code": "1113",
                        "message": "Insufficient balance or no resource package. Please recharge."
                    } }),
                ),
                "Insufficient balance or no resource package. Please recharge. (code 1113)",
            ),
            (
                refused_turn(
                    402,
                    serde_json::json!({
                        "message": "Payment required to access this resource. Visit your billing tab.",
                        "type": "payment_required_error",
                        "code": "payment_required"
                    }),
                ),
                "Payment required to access this resource. Visit your billing tab. (code \
                 payment_required)",
            ),
        ];
        for (index, (refusal, words)) in refusals.into_iter().enumerate() {
            let temp = tempdir().expect("tempdir");
            let models = temp.path().join("claude-models.log");
            let claude = temp.path().join("fake-claude.sh");
            model_logging_claude(&claude, &models);
            // A second answer, so a call from the next try would show.
            let (base_url, requests) = spawn_openai_mock(vec![refusal.clone(), refusal]);
            let mut task = make_task_def("mechanical");
            task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
            task.hints.rung = Some("mid".to_string());
            let spec = make_spec(&task);
            let ctx = CellContext::new().with_run_id(format!("cannot-bill-{index}"));
            let rungs = [
                ("cheap", "cheap-model"),
                ("mid", "mid-model"),
                ("strong", "strong-model"),
            ];
            let mut config = ladder_failover_config(&claude, &rungs);
            let mid_api = config.providers.get_mut("mid_api").expect("mid_api");
            mid_api.base_url = Some(base_url);
            let hub = crate::state_hub::StateHub::default_capacity();
            let dispatcher = make_bare_dispatcher(config, temp.path())
                .await
                .with_tui_bridge(crate::runner::tui_bridge::TuiBridge::new(hub.sender()));

            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect("the strong rung's model runs the task in the same try");
            assert_eq!(requests.lock().len(), 1, "one answer, no retry");
            assert_eq!(logged_models(&models), ["claude-sonnet-4-6"]);
            let said: Vec<(String, bool, String, String)> = hub
                .replay_from(0)
                .into_iter()
                .filter_map(|envelope| match envelope.payload {
                    roko_core::DashboardEvent::ProviderExhausted {
                        provider,
                        billing,
                        reason,
                        detail,
                        ..
                    } => Some((provider, billing, reason, detail)),
                    _ => None,
                })
                .collect();
            assert_eq!(said.len(), 1, "{said:?}");
            let (provider, billing, reason, detail) = &said[0];
            assert_eq!(provider, "mid_api");
            assert!(*billing, "{said:?}");
            assert_eq!(reason, words);
            assert!(
                detail.starts_with(&format!(
                    "mid_api: {words}: recharge, or the ladder runs without it until "
                )),
                "{detail}"
            );

            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect("the next try runs");
            assert_eq!(
                requests.lock().len(),
                1,
                "the next try did not call the provider out of usage"
            );
            assert_eq!(
                logged_models(&models),
                ["claude-sonnet-4-6", "claude-sonnet-4-6"]
            );
        }
    }

    /// bug-4b6844: with nowhere to fail over, a try whose provider is still
    /// rate-limited after the tool loop's own retries waits for the limit
    /// inside the try and calls again, instead of settling: with no retries
    /// at all, the task still passes.
    #[tokio::test]
    async fn a_429_with_nowhere_to_fail_over_waits_inside_the_try() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let tries = roko_agent::retry::RetryPolicy::default().max_attempts as usize;
        let mut turns: Vec<serde_json::Value> = std::iter::repeat_with(|| rate_limited_turn(1))
            .take(tries)
            .collect();
        turns.push(tool_call_turn(
            "call-write",
            "write_file",
            serde_json::json!({ "path": "hello.txt", "content": "hi after the wait" }),
        ));
        turns.push(final_turn("wrote hello.txt"));
        let (base_url, requests) = spawn_openai_mock(turns);
        let config = rate_limited_config(&claude, &base_url);
        let dispatcher = rate_limited_dispatcher(config, &temp, &workdir).await;
        let cell = failover_cell_with_retries(dispatcher, "api-model", 0);

        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect("the try waits the limit out and completes the task");
        assert_eq!(output[0].body.as_text().expect("text"), "wrote hello.txt");
        assert_eq!(
            std::fs::read_to_string(workdir.join("hello.txt")).expect("hello.txt"),
            "hi after the wait"
        );
        assert_eq!(requests.lock().len(), tries + 2);
        assert_eq!(invocations(&calls), 0, "nothing failed over");
    }

    /// bug-80f0d0: in phase 4's run 16fc88ae three HTTP 400 refusals of
    /// gpt-5.4-mini's request opened the openai circuit, so routing skipped
    /// gpt-5.4 as well. A refused request is the request's fault or its
    /// model's: three refusals of api-model's request, a 403 that refuses
    /// the model among them, leave the provider's circuit closed, and the
    /// next try, on another model of the same provider, is dispatched and
    /// completes the task.
    #[tokio::test]
    async fn refused_requests_leave_the_providers_circuit_closed() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let refused = refused_turn(
            400,
            serde_json::json!({ "error": {
                "message": "Function tools with reasoning_effort are not supported for \
                            gpt-5.4-mini in /v1/chat/completions.",
                "type": "invalid_request_error",
                "param": "reasoning_effort",
                "code": null
            } }),
        );
        let no_access = refused_turn(
            403,
            serde_json::json!({ "error": {
                "message": "Project `proj_1` does not have access to model `api-model-1`",
                "type": "invalid_request_error",
                "code": "model_not_found"
            } }),
        );
        let (base_url, requests) = spawn_openai_mock(vec![
            refused.clone(),
            no_access,
            refused,
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "hello.txt", "content": "hi from the other model" }),
            ),
            final_turn("wrote hello.txt"),
        ]);
        let mut config = rate_limited_config(&claude, &base_url);
        // The provider's other model, as gpt-5.4 is beside gpt-5.4-mini.
        let mut other = config.models["api-model"].clone();
        other.slug = "api-model-2".to_string();
        config.models.insert("api-model-full".to_string(), other);
        let dispatcher = rate_limited_dispatcher(config, &temp, &workdir).await;
        let ctx = CellContext::new().with_cell_id("T08".to_string());

        for _ in 0..3 {
            let cell = failover_cell_with_retries(Arc::clone(&dispatcher), "api-model", 0);
            cell.execute(Vec::new(), &ctx)
                .await
                .expect_err("the provider refuses the request");
        }
        assert_eq!(requests.lock().len(), 3, "each refused request once");
        let health = &dispatcher.factory.health_registry;
        assert!(
            health.is_available("mock_api"),
            "{:?}",
            health.get("mock_api")
        );

        let cell = failover_cell_with_retries(Arc::clone(&dispatcher), "api-model-full", 0);
        let output = cell
            .execute(Vec::new(), &ctx)
            .await
            .expect("the provider's other model runs the task");
        assert_eq!(output[0].body.as_text().expect("text"), "wrote hello.txt");
        assert_eq!(requests.lock().len(), 5);
        assert_eq!(invocations(&calls), 0, "nothing failed over");
    }

    /// A stand-in provider that serves `capacity` calls at once, each after
    /// `hold`, with `answer`, and refuses any call beyond them with HTTP 429
    /// and a `Retry-After` of one second, as z.ai refuses calls past an
    /// account's concurrency. It takes `calls` calls and records each one's
    /// arrival and status.
    fn spawn_concurrency_limited_mock(
        capacity: usize,
        hold: std::time::Duration,
        answer: serde_json::Value,
        calls: usize,
    ) -> (String, Arc<parking_lot::Mutex<Vec<(Instant, u16)>>>) {
        use std::io::Write;

        use crate::graph_task_dispatch::tests::{openai_mock_wire, read_mock_request};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base_url = format!("http://{}/v1", listener.local_addr().expect("mock addr"));
        let served = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let log = Arc::clone(&served);
        let in_flight = Arc::new(parking_lot::Mutex::new(0_usize));
        std::thread::spawn(move || {
            for _ in 0..calls {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let (log, in_flight, answer) =
                    (Arc::clone(&log), Arc::clone(&in_flight), answer.clone());
                std::thread::spawn(move || {
                    let Some(request) = read_mock_request(&mut stream) else {
                        return;
                    };
                    let arrived = Instant::now();
                    let admitted = {
                        let mut in_flight = in_flight.lock();
                        let admitted = *in_flight < capacity;
                        if admitted {
                            *in_flight += 1;
                        }
                        admitted
                    };
                    let streamed = request["stream"] == serde_json::Value::Bool(true);
                    if !admitted {
                        log.lock().push((arrived, 429));
                        let wire = openai_mock_wire(&rate_limited_turn(1), streamed);
                        let _ = stream.write_all(wire.as_bytes());
                        return;
                    }
                    log.lock().push((arrived, 200));
                    std::thread::sleep(hold);
                    let _ = stream.write_all(openai_mock_wire(&answer, streamed).as_bytes());
                    *in_flight.lock() -= 1;
                });
            }
        });
        (base_url, served)
    }

    /// bug-4b6844, in the shape of round v7's run 0bed5c58: three tasks started
    /// at once on mid, glm-4.7 on z.ai, which took two of their calls and
    /// answered the third task with 429s. That task spent three of its four
    /// tries in two seconds, 168 ms and 202 ms apart, and only its last try
    /// failed over. Here a stand-in provider takes two calls at once and
    /// refuses a third with a 429. The refused task waits for the
    /// `Retry-After`; while the other two still hold the provider, the
    /// run's shared limit holds its calls back without calling, so the
    /// provider sees one 429 in all; and every task passes in its one try,
    /// on the rung's model, with nowhere to fail over.
    #[tokio::test]
    async fn three_tasks_at_once_on_one_provider_share_its_rate_limit_and_spend_no_try() {
        const RUN: &str = "three-at-once";
        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let (base_url, served) = spawn_concurrency_limited_mock(
            2,
            std::time::Duration::from_secs(3),
            final_turn("done"),
            4,
        );
        let runs_dir = temp.path().join(".roko/runs");
        let dispatcher = make_bare_dispatcher(rate_limited_config(&claude, &base_url), temp.path())
            .await
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            });
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let specs: Vec<TaskExecutionSpec> = (1..=3)
            .map(|n| {
                let task = TaskDef {
                    id: format!("T{n}"),
                    title: format!("Task {n}"),
                    model_hint: Some("api-model".to_string()),
                    timeout_secs: FIXTURE_HANG_GUARD_SECS,
                    ..make_task_def("focused")
                };
                make_spec(&task)
            })
            .collect();

        let (t1, t2, t3) = tokio::join!(
            dispatcher.dispatch(&specs[0], Vec::new(), &ctx),
            dispatcher.dispatch(&specs[1], Vec::new(), &ctx),
            dispatcher.dispatch(&specs[2], Vec::new(), &ctx),
        );
        for (task, result) in ["T1", "T2", "T3"].into_iter().zip([t1, t2, t3]) {
            result.unwrap_or_else(|error| panic!("{task} passes in its one try: {error}"));
        }

        let served = served.lock().clone();
        let refused: Vec<Instant> = served
            .iter()
            .filter(|(_, status)| *status == 429)
            .map(|(at, _)| *at)
            .collect();
        assert_eq!(refused.len(), 1, "one 429 in all: {served:?}");
        assert_eq!(
            served.len(),
            4,
            "two at once, one refused, then its call: {served:?}"
        );
        // The refused task called again only after the `Retry-After`, never
        // in a few hundred milliseconds.
        let last = served.iter().map(|(at, _)| *at).max().expect("calls");
        assert!(
            last.duration_since(refused[0]) >= std::time::Duration::from_secs(1),
            "{served:?}"
        );
        assert_eq!(invocations(&calls), 0, "nothing failed over");
        let attempts = runs_dir.join(RUN).join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 3, is_verdict).await;
        assert_eq!(verdicts.len(), 3, "{verdicts:?}");
        for verdict in &verdicts {
            assert_eq!(verdict["attempt"], 1, "{verdict}");
            assert_eq!(verdict["outcome"], "unverified", "{verdict}");
        }
    }

    // ─── A rung our own calls hold (bug-62ae1e) ────────────────────────────

    /// A fake `claude` that appends the `--max-turns` of each call to `turns`,
    /// then answers.
    fn turns_logging_claude(path: &Path, turns: &Path) {
        write_executable(
            path,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
previous=
for arg in "$@"; do
  if [ "$previous" = "--max-turns" ]; then
    printf '%s\n' "$arg" >> '{}'
  fi
  previous=$arg
done
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ran"}}}}'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}}'
"#,
                turns.display()
            ),
        );
    }

    /// A dispatcher over [`ladder_failover_config`] (cheap, mid on the mock's
    /// API at `base_url`, strong), `configure`d, working in `temp`, with its
    /// provider health kept there and its run's records under `.roko/runs`.
    async fn held_rung_dispatcher(
        claude: &Path,
        base_url: &str,
        temp: &tempfile::TempDir,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (
        GraphTaskDispatcher,
        Arc<roko_learn::provider_health::ProviderHealthRegistry>,
    ) {
        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let mut config = ladder_failover_config(claude, &rungs);
        config
            .providers
            .get_mut("mid_api")
            .expect("the mid rung's API provider")
            .base_url = Some(base_url.to_string());
        configure(&mut config);
        crate::graph_task_dispatch::tests::fence_audits(&mut config);
        let config = Arc::new(config);
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(Arc::clone(&health)),
        );
        let dispatcher = GraphTaskDispatcher::new(factory, config, temp.path().to_path_buf())
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(temp.path().join(".roko/runs")),
                ..GraphFeedbackContext::default()
            });
        (dispatcher, health)
    }

    /// A task routed to the mid rung, by its `rung` hint.
    fn mid_rung_task() -> TaskDef {
        let mut task = make_task_def("focused");
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        task.hints.rung = Some("mid".to_string());
        task
    }

    /// bug-62ae1e, phase 2.2's loop: another try's call holds the mid rung's
    /// endpoint at the one call at once a rate limit showed it takes, so the
    /// limiter holds the task's call back. The call waits for its slot
    /// (bug-33a61f) instead of the try handing itself to the strong rung's
    /// model: once that call ends, it goes out and the mid model finishes the
    /// task. The held call never counts against the provider's health.
    #[tokio::test]
    async fn a_held_call_waits_for_a_slot_instead_of_a_substitute() {
        const RUN: &str = "held-for-a-slot";
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let (base_url, requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "hello.txt", "content": "hi on mid" }),
            ),
            final_turn("wrote hello.txt"),
        ]);
        let (dispatcher, health) = held_rung_dispatcher(&claude, &base_url, &temp, |_| {}).await;
        let spec = make_spec(&mid_rung_task());
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        // Another try's call is in flight on the endpoint, which a 429 taught
        // takes one call at once; it ends six seconds in, after the tool
        // loop's own three calls gave up.
        let limiter = roko_agent::openai_compat_backend::shared_rate_limiter();
        let endpoint = roko_agent::openai_compat_backend::chat_endpoint(&base_url);
        let other = limiter.admit(&endpoint).expect("the other try's call");
        limiter.note_rate_limited(&endpoint);
        let release = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(6)).await;
            drop(other);
        });

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the mid model runs the task once the slot is free");
        release.await.expect("the other call ended");
        assert!(logged_models(&models).is_empty(), "no substitute ran");
        assert_eq!(
            requests.lock().len(),
            2,
            "write, done: the held calls never reached the provider"
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("hello.txt")).expect("hello.txt"),
            "hi on mid"
        );
        let attempts = temp
            .path()
            .join(".roko/runs")
            .join(RUN)
            .join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 1, is_verdict).await;
        assert_eq!(verdicts[0]["ladder"]["rung"], "mid", "{}", verdicts[0]);
        assert_eq!(
            verdicts[0]["executed"]["failover_chain"],
            serde_json::json!([]),
            "{}",
            verdicts[0]
        );
        let failures = health
            .snapshot()
            .get("mid_api")
            .map_or(0, |health| health.total_failures);
        assert_eq!(failures, 0, "a held call is not the provider's failure");
    }

    /// [`spawn_openai_mock`], calling `on_request` with each request's index
    /// and the mock's base URL before it answers that request.
    fn spawn_hooked_openai_mock(
        responses: Vec<serde_json::Value>,
        mut on_request: impl FnMut(usize, &str) + Send + 'static,
    ) -> (String, Arc<parking_lot::Mutex<Vec<serde_json::Value>>>) {
        use std::io::Write;

        use crate::graph_task_dispatch::tests::{openai_mock_wire, read_mock_request};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base_url = format!("http://{}/v1", listener.local_addr().expect("mock addr"));
        let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let requests = Arc::clone(&captured);
        let url = base_url.clone();
        std::thread::spawn(move || {
            for (index, response) in responses.into_iter().enumerate() {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let Some(request) = read_mock_request(&mut stream) else {
                    return;
                };
                let streamed = request["stream"] == serde_json::Value::Bool(true);
                requests.lock().push(request);
                on_request(index, &url);
                let wire = openai_mock_wire(&response, streamed);
                let _ = stream.write_all(wire.as_bytes());
            }
        });
        (base_url, captured)
    }

    /// bug-33a61f: a call the limiter holds back mid-run waits for its slot
    /// in place, marked queued, and the agent's conversation goes on. When
    /// the agent's first call reaches the mid rung's provider, another try's
    /// call joins it and a 429 teaches the endpoint takes one call at once,
    /// so the agent's next call is held until that call ends, 8 s later. It
    /// then goes out in the same run, carrying the first call's tool result.
    /// The mid rung bounds a try at 5 s, and the attempt's watchdog leaves the
    /// queued wait out of that time, as it does out of a stall's silence, so
    /// the run's third call still goes out.
    #[tokio::test]
    async fn a_call_held_mid_run_waits_and_continues_its_conversation() {
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let (base_url, requests) = spawn_hooked_openai_mock(
            vec![
                tool_call_turn(
                    "call-write",
                    "write_file",
                    serde_json::json!({ "path": "hello.txt", "content": "hi on mid" }),
                ),
                tool_call_turn(
                    "call-read",
                    "read_file",
                    serde_json::json!({ "path": "hello.txt" }),
                ),
                final_turn("wrote hello.txt"),
            ],
            |index, base_url| {
                if index > 0 {
                    return;
                }
                let endpoint = roko_agent::openai_compat_backend::chat_endpoint(base_url);
                let limiter = roko_agent::openai_compat_backend::shared_rate_limiter();
                let other = limiter.admit(&endpoint).expect("another try's call");
                limiter.note_rate_limited(&endpoint);
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(8));
                    drop(other);
                });
            },
        );
        let (dispatcher, _health) = held_rung_dispatcher(&claude, &base_url, &temp, |config| {
            config.routing.ladder.max_secs = [("mid".to_string(), 5)].into();
        })
        .await;
        let spec = make_spec(&mid_rung_task());
        let ctx = CellContext::new().with_run_id("held-mid-run".to_string());

        let started = Instant::now();
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("mid finishes the task in one run");
        assert!(
            started.elapsed() >= Duration::from_secs(7),
            "the held call waited for its slot"
        );
        assert!(logged_models(&models).is_empty(), "no substitute ran");
        let requests = requests.lock();
        assert_eq!(
            requests.len(),
            3,
            "write, read, done: one run, never restarted nor stopped at its time bound"
        );
        let continued = requests[1]["messages"].as_array().is_some_and(|messages| {
            messages
                .iter()
                .any(|message| message["role"] == "tool" && message["tool_call_id"] == "call-write")
        });
        assert!(
            continued,
            "the held call carries the first call's tool result: {}",
            requests[1]
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("hello.txt")).expect("hello.txt"),
            "hi on mid"
        );
    }

    /// bug-33a61f with bug-5e597a: a call held for a slot that then waits long
    /// for its answer keeps both rules. Its hold is queued, so the stall
    /// watchdog leaves it out, and the wait in flight is bounded by its
    /// provider's `timeout_ms`, not by `task_stall_secs`. Held about 4 s, then
    /// answered 5 s after it goes out, with `task_stall_secs = 2` and
    /// `timeout_ms = 8000`, the run goes on in one go: past 120 s at full
    /// scale, the same two rules.
    #[tokio::test]
    async fn a_held_call_that_then_waits_long_keeps_its_queue_and_its_providers_timeout() {
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let (base_url, requests) = spawn_hooked_openai_mock(
            vec![
                tool_call_turn(
                    "call-write",
                    "write_file",
                    serde_json::json!({ "path": "hello.txt", "content": "hi on mid" }),
                ),
                final_turn("wrote hello.txt"),
            ],
            |index, base_url| {
                if index > 0 {
                    // The held call, once out, waits 5 s for its answer.
                    std::thread::sleep(Duration::from_secs(5));
                    return;
                }
                let endpoint = roko_agent::openai_compat_backend::chat_endpoint(base_url);
                let limiter = roko_agent::openai_compat_backend::shared_rate_limiter();
                let other = limiter.admit(&endpoint).expect("another try's call");
                limiter.note_rate_limited(&endpoint);
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(4));
                    drop(other);
                });
            },
        );
        let (dispatcher, _health) = held_rung_dispatcher(&claude, &base_url, &temp, |config| {
            config.conductor.task_stall_secs = 2;
            config
                .providers
                .get_mut("mid_api")
                .expect("the mid rung's API provider")
                .timeout_ms = Some(8_000);
        })
        .await;
        let spec = make_spec(&mid_rung_task());
        let ctx = CellContext::new().with_run_id("held-then-slow".to_string());

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("mid finishes the task in one run");
        assert!(logged_models(&models).is_empty(), "no substitute ran");
        let requests = requests.lock();
        assert_eq!(requests.len(), 2, "write, then done: one run");
        let continued = requests[1]["messages"].as_array().is_some_and(|messages| {
            messages
                .iter()
                .any(|message| message["role"] == "tool" && message["tool_call_id"] == "call-write")
        });
        assert!(continued, "the held call continues the conversation");
    }

    /// bug-62ae1e: a try the mid rung's provider rate-limits, with nothing
    /// else of ours in flight there, still fails over to the strong rung's
    /// model, and runs under the strong rung's bounds: its 13 turns, not
    /// mid's 7.
    #[tokio::test]
    async fn a_rate_limit_failover_try_runs_under_its_substitute_rungs_bounds() {
        let temp = tempdir().expect("tempdir");
        let turns = temp.path().join("claude-turns.log");
        let claude = temp.path().join("fake-claude.sh");
        turns_logging_claude(&claude, &turns);
        let tries = roko_agent::retry::RetryPolicy::default().max_attempts as usize;
        let (base_url, _requests) = spawn_openai_mock(
            std::iter::repeat_with(|| rate_limited_turn(1))
                .take(tries)
                .collect(),
        );
        let (dispatcher, _health) = held_rung_dispatcher(&claude, &base_url, &temp, |config| {
            config.routing.ladder.max_turns =
                [("mid".to_string(), 7), ("strong".to_string(), 13)].into();
        })
        .await;
        let spec = make_spec(&mid_rung_task());
        let ctx = CellContext::new().with_run_id("substitute-bounds".to_string());

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the strong rung's model runs the try");
        let logged = std::fs::read_to_string(&turns).unwrap_or_default();
        assert_eq!(logged.trim(), "13", "the substitute's own rung's turns");
    }

    /// bug-62ae1e: in phase 2.2 a substitute's failure counted nothing, so a
    /// task its routed rung's provider kept refusing looped on substitutes.
    /// Two tries the mid rung's provider rate-limited, run by the strong
    /// rung's model and failed, now count on mid: the task climbs, and its
    /// third try is routed to the strong rung itself, without calling mid.
    #[tokio::test]
    async fn repeated_rate_limit_failovers_count_toward_the_climb() {
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let tries = roko_agent::retry::RetryPolicy::default().max_attempts as usize;
        let (base_url, requests) = spawn_openai_mock(
            std::iter::repeat_with(|| rate_limited_turn(1))
                .take(2 * tries)
                .collect(),
        );
        let (dispatcher, _health) = held_rung_dispatcher(&claude, &base_url, &temp, |_| {}).await;
        let mut task = mid_rung_task();
        task.verify = vec![crate::graph_task_dispatch::tests::verify_step(
            "test", "false",
        )];
        let mut spec = make_spec(&task);
        spec.max_retries = 2;
        let ctx = CellContext::new().with_run_id("rate-limit-climb".to_string());

        for _ in 0..2 {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("the substitute's try fails its check");
        }
        assert_eq!(
            dispatcher.ladder_step(&spec, &task),
            1,
            "two rate-limit failovers climb the task"
        );
        let mid_calls = requests.lock().len();
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("the strong rung's own try fails its check too");
        assert_eq!(
            requests.lock().len(),
            mid_calls,
            "the third try is routed to strong: mid is not called"
        );
        // Each failed try's error enrichment runs on the cheap helper model.
        let strong_tries = logged_models(&models)
            .into_iter()
            .filter(|model| model == "claude-sonnet-4-6")
            .count();
        assert_eq!(strong_tries, 3, "two substitutes, then strong's own try");
    }

    /// gap-cf427e: a chat host's run holds its outbound tool calls for
    /// approval, which only roko's own tool loop can do, so its ladder keeps
    /// only the rungs that run there (`run_graph_plan` keeps them for a run
    /// with a `stage` floor). A task whose start rung is a CLI rung runs on
    /// the nearest API rung below it, and no CLI is called; a run from the
    /// CLI routes the same task to the CLI rung, as before.
    #[tokio::test]
    async fn chat_run_ladder_skips_rungs_whose_agent_runs_its_own_tools() {
        const RUN: &str = "chat-ladder";
        let temp = tempdir().expect("tempdir");
        let models = temp.path().join("claude-models.log");
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &models);
        let (base_url, requests) = spawn_openai_mock(vec![final_turn("done"); 2]);
        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let mut config = ladder_failover_config(&claude, &rungs);
        config
            .providers
            .get_mut("mid_api")
            .expect("the mid rung's API provider")
            .base_url = Some(base_url);
        let config = Arc::new(config);
        let mut task = make_task_def("mechanical");
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        task.hints.rung = Some("strong".to_string());
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let runs_dir = temp.path().join(".roko/runs");

        let factory = SharedAgentFactory::new(Arc::clone(&config), None, None, None)
            .await
            .keep_tool_loop_rungs();
        let chat = GraphTaskDispatcher::new(
            Arc::new(factory),
            Arc::clone(&config),
            temp.path().to_path_buf(),
        )
        .with_outbound_floor(Some(roko_core::tool::OutboundPolicy::Stage))
        .with_feedback(GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        });
        chat.dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the mid rung's API model runs the task");
        assert!(logged_models(&models).is_empty(), "no CLI runs");
        assert_eq!(requests.lock().len(), 1, "one call to the API rung");
        let attempts = runs_dir.join(RUN).join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 1, is_verdict).await;
        let verdict = &verdicts[0];
        assert_eq!(verdict["ladder"]["rung"], "mid", "{verdict}");
        assert_eq!(verdict["executed"]["provider"], "mid_api", "{verdict}");

        let cli = make_bare_dispatcher(config.as_ref().clone(), temp.path()).await;
        cli.dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the strong rung's CLI runs the task");
        assert_eq!(logged_models(&models), ["claude-sonnet-4-6"]);

        // Without the API model's key, nothing can run the chat run's task:
        // the error names the CLI that cannot keep its policy, and asks for
        // a configuration change, not for waiting.
        let mut keyless = config.as_ref().clone();
        keyless
            .providers
            .get_mut("mid_api")
            .expect("the mid rung's API provider")
            .api_key_env = Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET".to_string());
        let keyless = Arc::new(keyless);
        let factory = SharedAgentFactory::new(Arc::clone(&keyless), None, None, None)
            .await
            .keep_tool_loop_rungs();
        let chat = GraphTaskDispatcher::new(
            Arc::new(factory),
            Arc::clone(&keyless),
            temp.path().to_path_buf(),
        )
        .with_outbound_floor(Some(roko_core::tool::OutboundPolicy::Stage));
        let error = chat
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("no model of roko's tool loop has its key");
        let RokoError::Gateway { message, .. } = &error else {
            panic!("expected a gateway error, got {error:?}");
        };
        assert!(message.contains("runs its own tools"), "{message}");
        assert!(message.contains("Then re-run."), "{message}");
        assert!(!message.contains("wait for the provider"), "{message}");
        assert_eq!(logged_models(&models), ["claude-sonnet-4-6"], "no CLI runs");
    }

    /// bug-ef82eb (shakedown D1): an API model that answers blank fails its
    /// attempt as an empty response, and the task is retried, not isolated.
    /// The retry reads the file before it writes it, as the implementer's
    /// contract requires (`RequireToolBeforeEdit` refused D1's scripted write
    /// to a file the attempt never read, which was why D1 ended gate-failed),
    /// and the task completes: its second attempt passes its verify step.
    #[tokio::test]
    async fn retry_after_a_blank_answer_completes() {
        use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
        use crate::graph_task_dispatch::tests::verify_step;

        const RUN: &str = "blank-retry";
        let temp = tempdir().expect("tempdir");
        let one = "pub fn one() -> u8 {\n    1\n}\n";
        let two = format!("{one}\npub fn two() -> u8 {{\n    one() + one()\n}}\n");
        commit_repo(temp.path(), &[("src/lib.rs", one)]);
        let (base_url, requests) = spawn_openai_mock(vec![
            final_turn(""),
            tool_call_turn(
                "call-read",
                "read_file",
                serde_json::json!({ "path": "src/lib.rs" }),
            ),
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "src/lib.rs", "content": two }),
            ),
            final_turn("Done."),
        ]);
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "api-model".to_string();
        config.agent.bare_mode = false;
        // `PATH` is always set, standing in for an API key.
        config.providers.insert(
            "mock_api".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some(base_url),
                api_key_env: Some("PATH".to_string()),
                command: None,
                ..cli_provider("")
            },
        );
        config.models.insert(
            "api-model".to_string(),
            ModelProfile {
                context_window: 128_000,
                max_output: Some(1_024),
                max_tools: Some(32),
                tool_format: "openai_json".to_string(),
                ..model("mock_api", "api-model-1", None)
            },
        );
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        let runs_dir = temp.path().join(".roko/runs");
        let dispatcher = make_bare_dispatcher(config, temp.path())
            .await
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            });
        let task = TaskDef {
            id: "T01".to_string(),
            title: "Add two".to_string(),
            description: Some("Add `two` to src/lib.rs".to_string()),
            model_hint: Some("api-model".to_string()),
            files: vec!["src/lib.rs".to_string()],
            verify: vec![verify_step("check", "grep -q 'fn two' src/lib.rs")],
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            max_retries: 1,
            ..make_task_def("focused")
        };
        let cell_config = toml::Value::Table(toml::map::Map::from_iter([
            (
                "plan_id".to_string(),
                toml::Value::String("p-blank".to_string()),
            ),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            (
                "timeout_secs".to_string(),
                toml::Value::Integer(FIXTURE_HANG_GUARD_SECS as i64),
            ),
            ("max_retries".to_string(), toml::Value::Integer(1)),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        let cell = roko_graph::cells::TaskExecutorCell::live(cell_config, Arc::new(dispatcher));
        let ctx = CellContext::new()
            .with_cell_id("T01".to_string())
            .with_run_id(RUN.to_string());
        cell.execute(Vec::new(), &ctx)
            .await
            .expect("the retry after the blank answer completes the task");

        assert_eq!(requests.lock().len(), 4, "blank, read, write, done");
        let lib = std::fs::read_to_string(temp.path().join("src/lib.rs")).expect("src/lib.rs");
        assert!(lib.contains("fn two"), "{lib}");
        let attempts = runs_dir.join(RUN).join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 2, is_verdict).await;
        let outcomes: Vec<&str> = verdicts
            .iter()
            .map(|verdict| verdict["outcome"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(outcomes, ["provider_error", "passed"], "{verdicts:?}");
    }

    /// backlog 1121 (decision 1119, 3-A): at plan start each rung model that
    /// roko's tool loop drives gets one tool-use probe; the CLI rungs bring
    /// their own tools and get none. The `mid` rung's model answers with
    /// reasoning only, so the bound ladder leaves `mid` out and the probe
    /// cache records why. Within a day the cached verdict stands, with no
    /// second call.
    #[tokio::test]
    async fn preflight_skips_rung_that_returns_blank_answer() {
        use crate::dispatch::RoutingLadder;
        use crate::dispatch::rung_probe::{RungProbes, probe_ladder};

        let temp = tempdir().expect("tempdir");
        let reasoning_only = serde_json::json!({
            "id": "chatcmpl-probe",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "reasoning_content": "The user wants the echo tool. Thinking."
                },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15 }
        });
        let (base_url, requests) = spawn_openai_mock(vec![reasoning_only]);
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &temp.path().join("claude-models.log"));
        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let mut config = ladder_failover_config(&claude, &rungs);
        let mid_api = config.providers.get_mut("mid_api").expect("mid_api");
        mid_api.base_url = Some(base_url);
        let ladder = RoutingLadder::from_config(&config).expect("every rung can run");
        let run_started_ms = chrono::Utc::now().timestamp_millis();

        let failed = probe_ladder(&config, &ladder, temp.path(), run_started_ms).await;
        assert_eq!(requests.lock().len(), 1, "one probe, of the one API rung");
        let reason = failed.get("mid-1").expect("the mid rung failed its probe");
        assert!(reason.contains("empty_response"), "{reason}");
        let bound = ladder
            .clone()
            .without_models(&failed)
            .expect("two rungs remain");
        assert_eq!(
            bound.rung_models(),
            ["claude-haiku-4-5", "claude-sonnet-4-6"]
        );
        let cached = RungProbes::load(&RungProbes::path(temp.path()));
        let probe = cached.models.get("mid-1").expect("the cached verdict");
        assert!(!probe.passed);
        assert!(probe.reason.contains("empty_response"), "{probe:?}");

        let again = probe_ladder(&config, &ladder, temp.path(), run_started_ms).await;
        assert_eq!(again, failed);
        assert_eq!(requests.lock().len(), 1, "the fresh verdict is reused");
    }

    /// bug-35b190: a rung whose provider refuses the probe's request fails
    /// its probe, and the probe records the refusal in the provider's own
    /// words and its class, as OpenAI refused reasoning_effort beside
    /// function tools in phase 4's run 16fc88ae; it read "unknown: agent error
    /// (openai): network error: http 400: {…" before. The refused request is
    /// not sent again.
    #[tokio::test]
    async fn a_rung_probe_records_a_refusals_words() {
        use crate::dispatch::RoutingLadder;
        use crate::dispatch::rung_probe::{RungProbes, probe_ladder};

        let temp = tempdir().expect("tempdir");
        let refused = refused_turn(
            400,
            serde_json::json!({ "error": {
                "message": "Function tools with reasoning_effort are not supported for \
                            gpt-5.4-mini in /v1/chat/completions.",
                "type": "invalid_request_error",
                "param": "reasoning_effort",
                "code": null
            } }),
        );
        // A second answer, so a call sent again would show.
        let (base_url, requests) = spawn_openai_mock(vec![refused.clone(), refused]);
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &temp.path().join("claude-models.log"));
        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let mut config = ladder_failover_config(&claude, &rungs);
        let mid_api = config.providers.get_mut("mid_api").expect("mid_api");
        mid_api.base_url = Some(base_url);
        let ladder = RoutingLadder::from_config(&config).expect("every rung can run");
        let run_started_ms = chrono::Utc::now().timestamp_millis();

        let failed = probe_ladder(&config, &ladder, temp.path(), run_started_ms).await;
        assert_eq!(
            requests.lock().len(),
            1,
            "the refused request is not sent again"
        );
        let reason = failed.get("mid-1").expect("the mid rung failed its probe");
        assert_eq!(
            reason,
            "request_refused: mid_api refused the request (HTTP 400): Function tools with \
             reasoning_effort are not supported for gpt-5.4-mini in /v1/chat/completions. \
             (param reasoning_effort)"
        );
        let cached = RungProbes::load(&RungProbes::path(temp.path()));
        assert_eq!(&cached.models["mid-1"].reason, reason);
    }

    /// bug-374384: a rung whose cached probe failed on its provider's
    /// billing before this run is probed again once in the run. The provider
    /// now answers with the probe's tool call (the account was recharged), so
    /// the rung runs and the cache says so. A second probe in the same run
    /// reads that verdict with no call, and a billing failure probed in this
    /// run stands for the run.
    #[tokio::test]
    async fn a_rung_refused_on_billing_before_this_run_is_probed_again() {
        use crate::dispatch::RoutingLadder;
        use crate::dispatch::rung_probe::{RungProbe, RungProbes, probe_ladder};

        let temp = tempdir().expect("tempdir");
        let (base_url, requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "probe-1",
                "echo",
                serde_json::json!({ "text": "roko rung probe" }),
            ),
            final_turn("done"),
        ]);
        let claude = temp.path().join("fake-claude.sh");
        model_logging_claude(&claude, &temp.path().join("claude-models.log"));
        let rungs = [
            ("cheap", "cheap-model"),
            ("mid", "mid-model"),
            ("strong", "strong-model"),
        ];
        let mut config = ladder_failover_config(&claude, &rungs);
        let mid_api = config.providers.get_mut("mid_api").expect("mid_api");
        mid_api.base_url = Some(base_url);
        let ladder = RoutingLadder::from_config(&config).expect("every rung can run");
        let run_started_ms = chrono::Utc::now().timestamp_millis();
        let path = RungProbes::path(temp.path());
        std::fs::create_dir_all(path.parent().expect("the learn directory")).expect("mkdir");
        let billed = |probed_at_ms: i64| {
            let probe = RungProbe {
                passed: false,
                reason: "insufficient_credits: mid_api refused the request (HTTP 402): payment \
                         required"
                    .to_string(),
                probed_at_ms,
            };
            let probes = RungProbes {
                models: [("mid-1".to_string(), probe)].into(),
            };
            std::fs::write(&path, serde_json::to_vec(&probes).expect("json")).expect("probes");
        };
        // An earlier run's verdict, an hour old.
        billed(run_started_ms - 3_600_000);

        let failed = probe_ladder(&config, &ladder, temp.path(), run_started_ms).await;
        assert!(failed.is_empty(), "the recharged rung runs: {failed:?}");
        let probed = requests.lock().len();
        assert!(probed >= 1, "the rung was probed again");
        let cached = RungProbes::load(&path);
        assert!(cached.models["mid-1"].passed, "{cached:?}");

        let again = probe_ladder(&config, &ladder, temp.path(), run_started_ms).await;
        assert!(again.is_empty(), "{again:?}");
        assert_eq!(requests.lock().len(), probed, "this run's verdict stands");

        // A billing failure this run probed stands for the run.
        billed(run_started_ms + 1);
        let refused = probe_ladder(&config, &ladder, temp.path(), run_started_ms).await;
        assert!(
            refused
                .get("mid-1")
                .is_some_and(|reason| reason.contains("(HTTP 402)")),
            "{refused:?}"
        );
        assert_eq!(requests.lock().len(), probed, "no probe");
    }

    /// gap-baab0a: Codex cannot honour a task's tool allowlist, so failover
    /// passes it over before any call, runs the task on a provider that can,
    /// and records why.
    #[tokio::test]
    async fn codex_with_a_tool_allowlist_fails_over_before_any_call() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let codex_calls = temp.path().join("codex-calls.log");
        let codex = temp.path().join("fake-codex.sh");
        write_executable(
            &codex,
            &format!(
                "#!/bin/sh\ncat >/dev/null\necho called >> '{}'\nexit 1\n",
                codex_calls.display()
            ),
        );
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"ran"}}'
printf '%s\n' '{"type":"result","session_id":"s","total_cost_usd":0,"usage":{"input_tokens":1,"output_tokens":1}}'
"#,
        );
        let provider = |kind, command: Option<&Path>, key_env: Option<&str>| ProviderConfig {
            kind,
            base_url: None,
            api_key_env: key_env.map(str::to_string),
            command: command.map(|path| path.display().to_string()),
            args: None,
            timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
            stream_usage: None,
            billing: None,
        };
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet-4-6".to_string();
        config.agent.bare_mode = false;
        let claude_cli = provider(ProviderKind::ClaudeCli, Some(&claude), None);
        config
            .providers
            .insert("claude_cli".to_string(), claude_cli);
        let codex_cli = provider(ProviderKind::CodexCli, Some(&codex), None);
        config.providers.insert("codex_cli".to_string(), codex_cli);
        for (key, provider_id, slug) in [
            ("claude-sonnet-4-6", "claude_cli", "claude-sonnet-4-6"),
            ("codex-model", "codex_cli", "gpt-5-codex"),
        ] {
            let profile = ModelProfile {
                provider: provider_id.to_string(),
                slug: slug.to_string(),
                ..ModelProfile::default()
            };
            config.models.insert(key.to_string(), profile);
        }
        // Keys in the environment must not synthesize other usable providers.
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            let keyless = provider(kind, None, Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"));
            config.providers.insert(id.to_string(), keyless);
        }
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = GraphTaskDispatcher::new(factory, Arc::clone(&config), workdir.clone())
            .with_feedback(recording_feedback(&workdir));
        let task = TaskDef {
            id: "T09".to_string(),
            title: "Read with an allowlist".to_string(),
            model_hint: Some("codex-model".to_string()),
            allowed_tools: Some(vec!["read_file".to_string(), "grep".to_string()]),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let run = "graph-contract-failover-run";
        dispatcher
            .dispatch(
                &make_spec(&task),
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .expect("claude_cli runs the task");
        drop(dispatcher);

        assert!(!codex_calls.exists(), "codex was called");
        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let executed = &verdicts[0]["executed"];
        assert_eq!(executed["provider"], "claude_cli");
        assert_eq!(
            executed["failover_chain"],
            serde_json::json!(["codex-model"])
        );
        let reason = executed["failover_reason"]
            .as_str()
            .expect("failover reason");
        assert!(
            reason.contains("cannot enforce the resolved agent contract"),
            "{reason}"
        );
    }

    /// gap-baab0a: an implementer may not search the web, so the broker stops
    /// a Codex run at its first `web_search`, and the attempt's verdict
    /// records the policy the contract asked for, what the broker enforced,
    /// and the denial.
    #[tokio::test]
    async fn a_denied_codex_operation_is_recorded_with_the_tool_policy() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let codex = temp.path().join("fake-codex.sh");
        write_executable(
            &codex,
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"item.started","item":{"id":"item_0","type":"web_search","query":"rust"}}'
exec sleep 5
"#,
        );
        let provider = |kind, command: Option<&Path>, key_env: Option<&str>| ProviderConfig {
            kind,
            base_url: None,
            api_key_env: key_env.map(str::to_string),
            command: command.map(|path| path.display().to_string()),
            args: None,
            timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
            stream_usage: None,
            billing: None,
        };
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "codex-model".to_string();
        // The dispatcher has no per-task worktrees, so Codex runs only when
        // the operator accepts it in the shared checkout (decision 1214).
        config.runner.allow_unguarded_agents_in_checkout = true;
        let codex_cli = provider(ProviderKind::CodexCli, Some(&codex), None);
        config.providers.insert("codex_cli".to_string(), codex_cli);
        let profile = ModelProfile {
            provider: "codex_cli".to_string(),
            slug: "gpt-5-codex".to_string(),
            ..ModelProfile::default()
        };
        config.models.insert("codex-model".to_string(), profile);
        // Keys in the environment must not synthesize other usable providers.
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            let keyless = provider(kind, None, Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"));
            config.providers.insert(id.to_string(), keyless);
        }
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = GraphTaskDispatcher::new(factory, Arc::clone(&config), workdir.clone())
            .with_feedback(recording_feedback(&workdir));
        let task = TaskDef {
            id: "T10".to_string(),
            title: "Implement without the web".to_string(),
            model_hint: Some("codex-model".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let run = "graph-codex-denial-run";
        let dispatched = dispatcher
            .dispatch(
                &make_spec(&task),
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await;
        assert!(dispatched.is_err(), "the denied run fails the task");
        drop(dispatcher);

        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let policy = &verdicts[0]["executed"]["tool_policy"];
        assert_eq!(policy["enforcement"], "broker", "{policy}");
        let forbidden = policy["forbidden_tools"]
            .as_array()
            .expect("forbidden tools");
        assert!(
            forbidden.contains(&serde_json::json!("web_search")),
            "{policy}"
        );
        assert_eq!(
            policy["denied_operations"],
            serde_json::json!(["web_search"])
        );
        assert_eq!(policy["network_off"], true);
        assert_eq!(policy["denial"], "web_search denied by policy: rust");
    }

    /// `claude_cli` and `codex_cli`, each a fake script, with `default_model`
    /// the model failover falls back to.
    fn shared_checkout_config(
        claude: &Path,
        codex: &Path,
        default_model: &str,
        allow_unguarded: bool,
    ) -> Arc<RokoConfig> {
        let provider = |kind, command: Option<&Path>, key_env: Option<&str>| ProviderConfig {
            kind,
            base_url: None,
            api_key_env: key_env.map(str::to_string),
            command: command.map(|path| path.display().to_string()),
            args: None,
            timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
            stream_usage: None,
            billing: None,
        };
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = default_model.to_string();
        config.agent.bare_mode = false;
        config.runner.allow_unguarded_agents_in_checkout = allow_unguarded;
        let claude_cli = provider(ProviderKind::ClaudeCli, Some(claude), None);
        config
            .providers
            .insert("claude_cli".to_string(), claude_cli);
        let codex_cli = provider(ProviderKind::CodexCli, Some(codex), None);
        config.providers.insert("codex_cli".to_string(), codex_cli);
        for (key, provider_id, slug) in [
            ("claude-sonnet-4-6", "claude_cli", "claude-sonnet-4-6"),
            ("codex-model", "codex_cli", "gpt-5-codex"),
        ] {
            let profile = ModelProfile {
                provider: provider_id.to_string(),
                slug: slug.to_string(),
                ..ModelProfile::default()
            };
            config.models.insert(key.to_string(), profile);
        }
        // Keys in the environment must not synthesize other usable providers.
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            let keyless = provider(kind, None, Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"));
            config.providers.insert(id.to_string(), keyless);
        }
        Arc::new(config)
    }

    /// Dispatch `task` as run `run` from a dispatcher without per-task
    /// worktrees, so its attempt runs in the shared checkout `workdir`.
    async fn dispatch_in_shared_checkout(
        config: Arc<RokoConfig>,
        workdir: &Path,
        task: &TaskDef,
        run: &str,
    ) -> Result<()> {
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = GraphTaskDispatcher::new(factory, config, workdir.to_path_buf())
            .with_feedback(recording_feedback(workdir));
        dispatcher
            .dispatch(
                &make_spec(task),
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .map(|_| ())
    }

    /// 1216 (decision 1214): roko cannot check a Codex, Cursor or Gemini CLI
    /// agent's commands before they run, so an attempt in the operator's
    /// shared checkout passes Codex over for Claude's CLI; with Codex alone it
    /// fails before any call, saying why; and `[runner]
    /// allow_unguarded_agents_in_checkout` lets the operator accept the risk.
    #[tokio::test]
    async fn unguarded_cli_agents_never_run_in_the_shared_checkout() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let codex_calls = temp.path().join("codex-calls.log");
        let codex = temp.path().join("fake-codex.sh");
        write_executable(
            &codex,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo called >> '{}'
printf '%s\n' '{{"type":"item.completed","item":{{"id":"item_0","type":"agent_message","text":"codex ran"}}}}'
"#,
                codex_calls.display()
            ),
        );
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"ran"}}'
printf '%s\n' '{"type":"result","session_id":"s","total_cost_usd":0,"usage":{"input_tokens":1,"output_tokens":1}}'
"#,
        );
        let task = TaskDef {
            id: "T11".to_string(),
            title: "Implement in the shared checkout".to_string(),
            model_hint: Some("codex-model".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };

        // Codex is planned; Claude's CLI runs the task instead, and the
        // attempt records why.
        let config = shared_checkout_config(&claude, &codex, "claude-sonnet-4-6", false);
        let run = "graph-unguarded-failover-run";
        dispatch_in_shared_checkout(config, &workdir, &task, run)
            .await
            .expect("claude_cli runs the task");
        assert_eq!(invocations(&codex_calls), 0, "codex was called");
        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let executed = &verdicts[0]["executed"];
        assert_eq!(executed["provider"], "claude_cli");
        assert_eq!(
            executed["failover_chain"],
            serde_json::json!(["codex-model"])
        );
        let reason = executed["failover_reason"]
            .as_str()
            .expect("failover reason");
        assert!(reason.contains("no roko command guard"), "{reason}");

        // With Codex alone, the attempt fails before any call, saying why.
        let config = shared_checkout_config(&claude, &codex, "codex-model", false);
        let run = "graph-unguarded-alone-run";
        let error = dispatch_in_shared_checkout(config, &workdir, &task, run)
            .await
            .expect_err("no guarded provider is left");
        let message = error.to_string();
        assert!(
            message.contains("no guarded provider for a shared-checkout attempt"),
            "{message}"
        );
        assert_eq!(invocations(&codex_calls), 0, "codex was called");

        // The operator may accept the risk.
        let config = shared_checkout_config(&claude, &codex, "claude-sonnet-4-6", true);
        let run = "graph-unguarded-allowed-run";
        dispatch_in_shared_checkout(config, &workdir, &task, run)
            .await
            .expect("codex runs once the operator allows it");
        assert_eq!(invocations(&codex_calls), 1);
    }

    /// `roko init` workspaces configure only `claude_cli` and the default
    /// model; a hint naming another model must never fail the task.
    #[tokio::test]
    async fn hint_for_unconfigured_provider_runs_same_slug_then_default_model() {
        let temp = tempdir().expect("tempdir");
        let args_log = temp.path().join("claude-args.log");
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo "$@" >> '{}'
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ran"}}}}'
printf '%s\n' '{{"type":"result","session_id":"s","total_cost_usd":0,"usage":{{"input_tokens":1,"output_tokens":1}}}}'
"#,
                args_log.display()
            ),
        );
        // What `roko init` + `synthesize_claude_cli_config` produce: one CLI
        // provider and a `[models.*]` entry for the default model only.
        let mut config = fixture_config();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet-4-6".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "claude_cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(claude.display().to_string()),
                args: None,
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
                stream_usage: None,
                billing: None,
            },
        );
        config.models.insert(
            "claude-sonnet-4-6".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        // An API key in the environment synthesizes a usable standard provider
        // (`OPENAI_API_KEY` would send `gpt-4o` to the real OpenAI API), and
        // gate commands inherit the keys roko loads from `~/.roko/.env`. Shadow
        // each one with a provider whose key is never set.
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            config.providers.insert(
                id.to_string(),
                ProviderConfig {
                    kind,
                    base_url: None,
                    api_key_env: Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET".to_string()),
                    command: None,
                    args: None,
                    timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                    require_confirmation: false,
                    stream_usage: None,
                    billing: None,
                },
            );
        }
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));

        // 1. `claude-opus-4-6` resolves to an unconfigured `anthropic_api`;
        //    claude_cli serves the same slug.
        let output = failover_cell(Arc::clone(&dispatcher), "claude-opus-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T1".to_string()),
            )
            .await
            .expect("hinted slug runs on claude_cli");
        assert_eq!(output[0].body.as_text().expect("text"), "ran");
        // 2. No configured provider can serve `gpt-4o`: the default model runs.
        failover_cell(dispatcher, "gpt-4o")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T2".to_string()),
            )
            .await
            .expect("default model runs");

        let calls = std::fs::read_to_string(&args_log).expect("claude invocations");
        let models: Vec<&str> = calls
            .lines()
            .filter_map(|line| line.split("--model ").nth(1)?.split_whitespace().next())
            .collect();
        assert_eq!(models, ["claude-opus-4-6", "claude-sonnet-4-6"], "{calls}");
    }

    /// backlog 1128: the planned provider's circuit is open, so failover
    /// runs the fallback without calling it. The dashboard row the attempt
    /// opened with the planned model before dispatch is republished with
    /// the fallback's model and provider, so the hub's last word on the
    /// agent is the model that ran.
    #[tokio::test]
    async fn failover_publishes_fallback_slug_to_hub() {
        use roko_learn::provider_health::ErrorClass;

        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let mut answer = final_turn("fallback finished");
        answer["model"] = serde_json::json!("api-model-1");
        let (base_url, _requests) = spawn_openai_mock(vec![answer]);
        let config = Arc::new(failover_config(&claude, &base_url, &["api-model"]));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        for _ in 0..3 {
            factory
                .health_registry
                .record_failure("claude_cli", ErrorClass::ServerError);
        }
        let hub = crate::state_hub::shared_state_hub();
        let mut events = hub.subscribe_events();
        let dispatcher =
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_tui_bridge(crate::runner::tui_bridge::TuiBridge::new(hub.sender()));
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            model_hint: Some("claude-sonnet-4-6".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the fallback runs the task");
        assert_eq!(invocations(&calls), 0, "the open circuit is never called");

        let spawned = crate::graph_task_dispatch::tests::spawned_agents(&mut events);
        let agent = format!("stream-plan/{}", task.id);
        let rows: Vec<(&str, &str)> = spawned
            .iter()
            .filter(|(agent_id, _, _)| *agent_id == agent)
            .map(|(_, model, provider)| (model.as_str(), provider.as_str()))
            .collect();
        assert_eq!(
            rows,
            [
                ("claude-sonnet-4-6", "claude_cli"),
                ("api-model-1", "mock_api"),
            ],
            "{spawned:?}"
        );
    }
}
