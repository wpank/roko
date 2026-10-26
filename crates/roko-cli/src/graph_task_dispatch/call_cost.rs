//! Each model call's spend as the call returns (gap-5c3347).
//!
//! A try's spend used to show only once the try settled: through a 20-minute
//! try the run read $0.000, so nobody saw it near its cap. [`MeteredCalls`]
//! wraps the attempt's [`CallAllowance`]. roko's tool loop tells it each model
//! call's usage and cost as the call returns, and it publishes them as a
//! `call_cost` event before passing them on, with the call's reasoning
//! tokens, finish reason and duration (gap-1ba0ac). The attempt's
//! `attempt_settled` stays the record of what it spent.

use super::*;
use crate::dispatch_v2::CallAllowance;

/// An attempt's [`CallAllowance`] that publishes each model call's tokens and
/// cost on the hub as the call returns. It is otherwise the attempt's own
/// allowance, or no limit when the attempt has none.
pub(super) struct MeteredCalls {
    allowance: Option<Arc<dyn CallAllowance>>,
    tui: TuiBridge,
    plan_id: String,
    task_id: String,
    attempt: u32,
    /// The calls recorded so far, and what the priced ones cost in USD.
    spent: parking_lot::Mutex<(u32, f64)>,
}

impl std::fmt::Debug for MeteredCalls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeteredCalls")
            .field("allowance", &self.allowance)
            .field("plan_id", &self.plan_id)
            .field("task_id", &self.task_id)
            .field("attempt", &self.attempt)
            .finish_non_exhaustive()
    }
}

impl CallAllowance for MeteredCalls {
    fn refuse_call(&self) -> Option<String> {
        self.allowance.as_deref()?.refuse_call()
    }

    fn record_call(&self, cost_usd: f64, priced: bool) {
        if let Some(allowance) = &self.allowance {
            allowance.record_call(cost_usd, priced);
        }
    }

    fn record_model_call(&self, model: &str, usage: &roko_core::Usage, priced: bool) {
        self.record_finished_call(
            model,
            usage,
            priced,
            roko_agent::tool_loop::CallFacts::default(),
        );
    }

    fn record_finished_call(
        &self,
        model: &str,
        usage: &roko_core::Usage,
        priced: bool,
        facts: roko_agent::tool_loop::CallFacts<'_>,
    ) {
        if let Some(allowance) = &self.allowance {
            allowance.record_finished_call(model, usage, priced, facts);
        }
        let cost_usd = Some(f64::from(usage.cost_usd))
            .filter(|cost| priced && cost.is_finite() && *cost >= 0.0);
        let (call, attempt_cost_usd) = {
            let mut spent = self.spent.lock();
            spent.0 = spent.0.saturating_add(1);
            spent.1 += cost_usd.unwrap_or(0.0);
            *spent
        };
        self.tui.publish_event(roko_core::DashboardEvent::CallCost {
            plan_id: self.plan_id.clone(),
            task_id: self.task_id.clone(),
            attempt: self.attempt,
            call,
            model: model.to_string(),
            input_tokens: u64::from(usage.input_tokens),
            output_tokens: u64::from(usage.output_tokens),
            cache_read_tokens: u64::from(usage.cache_read_tokens),
            cost_usd,
            attempt_cost_usd,
            // What reasoning cost, why the call ended and how long it took
            // (gap-1ba0ac).
            reasoning_tokens: u64::from(usage.reasoning_tokens),
            finish_reason: facts.finish_reason.map(str::to_string),
            wall_ms: facts.wall_ms,
        });
    }

    fn spend_limit_usd(&self) -> Option<f64> {
        self.allowance.as_deref()?.spend_limit_usd()
    }
}

impl GraphTaskDispatcher {
    /// What attempt `attempt` of `plan_id`/`task_id` may spend, as its
    /// dispatch request carries it: `allowance`, metered so that each model
    /// call's cost reaches the hub as the call returns when the dispatcher
    /// publishes to one (gap-5c3347).
    pub(super) fn metered_calls(
        &self,
        allowance: Option<Arc<dyn CallAllowance>>,
        plan_id: &str,
        task_id: &str,
        attempt: u32,
    ) -> Option<Arc<dyn CallAllowance>> {
        let Some(tui) = &self.tui_bridge else {
            return allowance;
        };
        Some(Arc::new(MeteredCalls {
            allowance,
            tui: tui.clone(),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt,
            spent: parking_lot::Mutex::default(),
        }))
    }
}
