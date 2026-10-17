//! A bounded try per rung (gap-3e770b).
//!
//! A try on the rung the model ladder put a task on is bounded: at most the
//! rung's turns ([`LadderConfig::try_turns`]: fewer below the highest rung
//! the task can reach, unless `[routing.ladder] max_turns` says otherwise)
//! and, when `[routing.ladder] max_usd` names the rung, at most its spend. A
//! try that uses either up fails on its rung, blamed on the agent as a
//! turn-cap stop is, so two such tries climb the ladder as two failed checks
//! do. The plan's budget still bounds every try and is asked first: a stop
//! for the plan's budget stays the harness's, and ends the attempt as
//! before.
//!
//! [`LadderConfig::try_turns`]: roko_core::config::routing::LadderConfig::try_turns

use super::budget::AttemptAllowance;
use super::*;
use crate::dispatch_v2::CallAllowance;

/// The failure class of a try its rung's spend allowance stopped, which its
/// failure reason and its error lead with.
pub(super) const RUNG_ALLOWANCE: &str = "rung_allowance";

/// What one try may use on the rung the ladder put it on.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RungAllowance {
    /// The rung.
    pub(super) rung: String,
    /// Turns the try may take; `None` leaves the tier's cap.
    pub(super) turns: Option<u32>,
    /// What the try may spend, in USD; `None` leaves the plan's budget alone
    /// to bound it.
    pub(super) usd: Option<f64>,
}

/// Why a try's allowance stopped it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TryStop {
    /// The plan's budget cannot pay for its next model call (bug-4fabd0):
    /// the harness stopped it, and no retry can pay either.
    Plan(String),
    /// It spent its rung's allowance: the agent's failure on the rung.
    Rung(String),
}

/// What one try's model calls may spend: its plan's hold, asked first, so
/// the run's cap has the last word, then its rung's spend allowance.
#[derive(Debug)]
pub(super) struct TryAllowance {
    plan: Option<Arc<AttemptAllowance>>,
    rung: Option<RungSpend>,
}

/// A rung's spend allowance for one try, and what the try spent of it.
#[derive(Debug)]
struct RungSpend {
    rung: String,
    limit_usd: f64,
    spent: parking_lot::Mutex<RungSpent>,
}

#[derive(Debug, Default)]
struct RungSpent {
    usd: f64,
    /// What the last model call cost: the try stops before one more like it
    /// would pass its allowance, as the plan's hold does.
    last_call_usd: f64,
    /// Whether the spend limit an agent that keeps its own was handed is the
    /// rung's, not the plan's (gap-e14344).
    own_limit_is_rung: bool,
    /// Why the allowance refused a call, once it did.
    refusal: Option<String>,
}

impl TryAllowance {
    /// The allowance of a try that holds `plan` of its plan's budget and may
    /// spend `rung`'s allowance on its rung; `None` when neither bounds it.
    pub(super) fn new(
        plan: Option<Arc<AttemptAllowance>>,
        rung: Option<&RungAllowance>,
    ) -> Option<Arc<Self>> {
        let rung = rung.and_then(|rung| {
            rung.usd.map(|limit_usd| RungSpend {
                rung: rung.rung.clone(),
                limit_usd,
                spent: parking_lot::Mutex::default(),
            })
        });
        (plan.is_some() || rung.is_some()).then(|| Arc::new(Self { plan, rung }))
    }

    /// Why the try stopped for its allowance, if it did. `own_stop` is why an
    /// agent that keeps its own spend limit stopped there (the Claude CLI at
    /// `--max-budget-usd`): the limit it was handed says whose stop it was.
    pub(super) fn stop(&self, own_stop: Option<String>) -> Option<TryStop> {
        if let Some(reason) = self.plan.as_deref().and_then(AttemptAllowance::refusal) {
            return Some(TryStop::Plan(reason));
        }
        let rung = self.rung.as_ref();
        if let Some(reason) = rung.and_then(|rung| rung.spent.lock().refusal.clone()) {
            return Some(TryStop::Rung(reason));
        }
        let own_stop = own_stop?;
        match rung {
            Some(rung) if rung.spent.lock().own_limit_is_rung => {
                Some(TryStop::Rung(rung.spent_reason(&own_stop)))
            }
            _ => Some(TryStop::Plan(own_stop)),
        }
    }
}

impl RungSpend {
    /// Why the try must stop: it spent its allowance (`detail` says how the
    /// agent put it, when it stopped itself). What it spent is said when it
    /// is known: an agent that keeps its own limit reports it only at the
    /// end, which the plan's budget settles.
    fn spent_reason(&self, detail: &str) -> String {
        let spent = self.spent.lock().usd;
        let amount = if spent > 0.0 {
            format!("${spent:.4} of ${:.4}", self.limit_usd)
        } else {
            format!("${:.4}", self.limit_usd)
        };
        let detail = detail.trim();
        let detail = if detail.is_empty() {
            String::new()
        } else {
            format!(": {detail}")
        };
        format!(
            "the try spent its allowance on rung `{}`: {amount} ([routing.ladder] max_usd){detail}",
            self.rung
        )
    }
}

impl CallAllowance for TryAllowance {
    fn refuse_call(&self) -> Option<String> {
        if let Some(reason) = self.plan.as_deref().and_then(CallAllowance::refuse_call) {
            return Some(reason);
        }
        let rung = self.rung.as_ref()?;
        let spent = rung.spent.lock();
        if spent.usd + spent.last_call_usd <= rung.limit_usd && spent.usd < rung.limit_usd {
            return None;
        }
        drop(spent);
        let reason = rung.spent_reason("");
        rung.spent.lock().refusal = Some(reason.clone());
        Some(reason)
    }

    fn record_call(&self, cost_usd: f64, priced: bool) {
        if let Some(plan) = &self.plan {
            plan.record_call(cost_usd, priced);
        }
        // An unpriced call's cost is unknown: the plan refuses the next call
        // (bug-4fabd0), and the rung counts nothing.
        if let Some(rung) = &self.rung
            && priced
            && cost_usd.is_finite()
            && cost_usd > 0.0
        {
            let mut spent = rung.spent.lock();
            spent.usd += cost_usd;
            spent.last_call_usd = cost_usd;
        }
    }

    fn spend_limit_usd(&self) -> Option<f64> {
        let plan = self
            .plan
            .as_deref()
            .and_then(CallAllowance::spend_limit_usd);
        let Some(rung) = &self.rung else {
            return plan;
        };
        let mut spent = rung.spent.lock();
        let left = (rung.limit_usd - spent.usd).max(0.0);
        spent.own_limit_is_rung = plan.is_none_or(|plan| left < plan);
        Some(plan.map_or(left, |plan| plan.min(left)))
    }
}

impl GraphTaskDispatcher {
    /// Owe `plan_id/task_id`'s next attempt a note that its last try
    /// stopped at one of its limits, for `reason`: its rung's spend
    /// allowance, or its model's output limit (bug-f6b99f). Its work is on
    /// disk.
    pub(super) fn keep_allowance_stop(&self, plan_id: &str, task_id: &str, reason: &str) {
        self.allowance_stops
            .lock()
            .insert(format!("{plan_id}/{task_id}"), reason.to_string());
    }

    /// The prompt section owed to `plan_id/task_id`'s next attempt after a
    /// try that spent its rung's allowance, which takes it.
    pub(super) fn take_allowance_stop_note(&self, plan_id: &str, task_id: &str) -> Option<String> {
        let reason = self
            .allowance_stops
            .lock()
            .remove(&format!("{plan_id}/{task_id}"))?;
        Some(format!(
            "\n\n# Resuming a partially completed task\n\n\
             Your previous attempt at this task was stopped: {reason}. Its edits are still in the \
             working tree. Inspect the current state of the files in scope (for example with \
             `git diff`), keep what is already correct, and continue from there instead of \
             starting over.\n",
            reason = reason.trim_end_matches('.'),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rung's allowance stops a try once one more call like the last
    /// would pass it; an unpriced call counts nothing.
    /// With no plan hold, a stop is the rung's, and so is an agent's own stop
    /// at the limit the rung handed it.
    #[test]
    fn a_rung_allowance_refuses_the_call_that_would_pass_it() {
        let rung = RungAllowance {
            rung: "cheap".to_string(),
            turns: None,
            usd: Some(0.25),
        };
        let allowance = TryAllowance::new(None, Some(&rung)).expect("an allowance");
        assert_eq!(allowance.refuse_call(), None);
        allowance.record_call(0.125, true);
        allowance.record_call(1.0, false);
        assert_eq!(allowance.refuse_call(), None);
        assert_eq!(allowance.spend_limit_usd(), Some(0.125));
        allowance.record_call(0.125, true);
        let reason = allowance.refuse_call().expect("the allowance is spent");
        assert!(reason.contains("rung `cheap`"), "{reason}");
        assert!(reason.contains("$0.2500 of $0.2500"), "{reason}");
        assert_eq!(allowance.stop(None), Some(TryStop::Rung(reason)));

        let fresh = TryAllowance::new(None, Some(&rung)).expect("an allowance");
        assert_eq!(fresh.spend_limit_usd(), Some(0.25));
        let own = fresh.stop(Some("budget exhausted".to_string()));
        assert!(
            matches!(&own, Some(TryStop::Rung(reason)) if reason.ends_with(": budget exhausted")),
            "{own:?}"
        );

        assert!(TryAllowance::new(None, None).is_none());
        let turns_only = RungAllowance { usd: None, ..rung };
        assert!(TryAllowance::new(None, Some(&turns_only)).is_none());
    }
}
