//! What the operator sends a running plan with `roko inject` (gap-f118b3).
//!
//! The plan run queues each directive or context for the plan it names
//! ([`OperatorDirectives::queue`]), and the next task of that plan to start
//! gets them in its prompt, once, under an "Operator directive" or
//! "Operator context" heading. A plan's queue and each text are bounded, and
//! a request id queues its text once for the run's lifetime, so a request
//! sent again is not delivered twice.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::execution_control::InjectedKind;

/// The most bytes one directive or context may carry.
pub const MAX_INJECTED_BYTES: usize = 8 * 1024;
/// The most texts a plan may have waiting for its next task.
pub const MAX_WAITING_PER_PLAN: usize = 8;
/// How many request ids the run remembers, oldest forgotten first.
const MAX_REMEMBERED_REQUESTS: usize = 1024;

/// Operator texts waiting for each running plan's next task.
#[derive(Clone, Default)]
pub struct OperatorDirectives {
    queues: Arc<parking_lot::Mutex<Queues>>,
}

#[derive(Default)]
struct Queues {
    waiting: BTreeMap<String, Vec<(InjectedKind, String)>>,
    seen: HashSet<String>,
    seen_order: VecDeque<String>,
}

impl OperatorDirectives {
    /// Queue `text` for the next task of `plan_id` to start. Returns whether
    /// it was queued now: a request id seen before queues nothing again.
    pub fn queue(
        &self,
        plan_id: &str,
        request_id: &str,
        kind: InjectedKind,
        text: &str,
    ) -> Result<bool, String> {
        if text.trim().is_empty() {
            return Err("there is no text to deliver".to_string());
        }
        if text.len() > MAX_INJECTED_BYTES {
            return Err(format!("the text is longer than {MAX_INJECTED_BYTES} bytes"));
        }
        let mut queues = self.queues.lock();
        if queues.seen.contains(request_id) {
            return Ok(false);
        }
        let waiting = queues.waiting.entry(plan_id.to_string()).or_default();
        if waiting.len() >= MAX_WAITING_PER_PLAN {
            return Err(format!(
                "plan '{plan_id}' already has {MAX_WAITING_PER_PLAN} texts waiting"
            ));
        }
        waiting.push((kind, text.to_string()));
        queues.remember(request_id);
        Ok(true)
    }

    /// The prompt section of what waits for the next task of `plan_id`,
    /// which this takes; `None` when nothing waits.
    pub(crate) fn take_section(&self, plan_id: &str) -> Option<String> {
        let waiting = self.queues.lock().waiting.remove(plan_id)?;
        Some(
            waiting
                .iter()
                .map(|(kind, text)| {
                    let heading = match kind {
                        InjectedKind::Directive => "Operator directive",
                        InjectedKind::Context => "Operator context",
                    };
                    format!(
                        "\n\n## {heading}\n\nThe operator sent this with `roko inject` while \
                         the plan was running.\n\n{text}\n"
                    )
                })
                .collect(),
        )
    }
}

impl Queues {
    /// Remember `request_id`, forgetting the oldest beyond the bound.
    fn remember(&mut self, request_id: &str) {
        if self.seen_order.len() >= MAX_REMEMBERED_REQUESTS
            && let Some(oldest) = self.seen_order.pop_front()
        {
            self.seen.remove(&oldest);
        }
        self.seen.insert(request_id.to_string());
        self.seen_order.push_back(request_id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use roko_graph::cell::CellContext;
    use roko_graph::cells::TaskDispatcher;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::GraphFeedbackContext;
    use crate::graph_task_dispatch::tests::{
        make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };

    const PLAN: &str = "p1";
    const TEXT: &str = "keep the API stable";

    /// gap-f118b3: a queued text reaches one prompt, and a request sent
    /// again is not queued again.
    #[test]
    fn a_text_waits_for_one_prompt_and_is_queued_once() {
        let directives = OperatorDirectives::default();
        let queue = |request_id: &str| {
            directives.queue(PLAN, request_id, InjectedKind::Directive, TEXT)
        };

        assert_eq!(queue("req-1"), Ok(true));
        assert_eq!(queue("req-1"), Ok(false), "the same request again");
        let section = directives.take_section(PLAN).expect("a section");
        assert_eq!(section.matches(TEXT).count(), 1);
        assert!(section.contains("## Operator directive"), "{section}");
        assert_eq!(directives.take_section(PLAN), None, "taken once");
        assert_eq!(queue("req-1"), Ok(false), "and never delivered again");
    }

    /// gap-f118b3: empty and oversized texts are refused, and so is a text
    /// beyond a plan's waiting bound; another plan's queue is its own.
    #[test]
    fn texts_and_queues_are_bounded() {
        let directives = OperatorDirectives::default();
        let context = InjectedKind::Context;
        let long = "x".repeat(MAX_INJECTED_BYTES + 1);
        assert!(directives.queue(PLAN, "empty", context, "  ").is_err());
        assert!(directives.queue(PLAN, "long", context, &long).is_err());
        for index in 0..MAX_WAITING_PER_PLAN {
            let request_id = format!("req-{index}");
            let queued = directives.queue(PLAN, &request_id, context, "note");
            assert_eq!(queued, Ok(true));
        }
        assert!(directives.queue(PLAN, "more", context, "note").is_err());
        assert_eq!(directives.queue("p2", "other", context, "note"), Ok(true));
    }

    /// Writes the provider's prompt (argv, then stdin) to `prompt.txt`.
    const PROMPT_LOG_PROVIDER: &str = r#"#!/bin/sh
set -eu
input="$(cat)"
printf '%s\n---\n%s\n' "$*" "$input" > prompt.txt
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// gap-f118b3: a directive queued for a running plan is in the prompt
    /// of its next task, once, and that dispatch takes it from the queue.
    #[tokio::test]
    async fn a_queued_directive_reaches_the_next_prompt_once() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            PROMPT_LOG_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "true")];
        let spec = make_spec(&task);
        let queued = dispatcher.operator_directives().queue(
            &spec.plan_id,
            "req-1",
            InjectedKind::Directive,
            "DIRECTIVE-5c2e: keep the public API stable",
        );
        assert_eq!(queued, Ok(true));

        dispatcher
            .dispatch(&spec, Vec::new(), &CellContext::new())
            .await
            .expect("the task runs");
        let sent = std::fs::read_to_string(temp.path().join("prompt.txt")).expect("prompt");
        assert_eq!(sent.matches("DIRECTIVE-5c2e").count(), 1, "{sent}");
        assert!(sent.contains("## Operator directive"), "{sent}");
        let waiting = dispatcher.operator_directives().take_section(&spec.plan_id);
        assert_eq!(waiting, None, "the dispatch took it");
    }
}
