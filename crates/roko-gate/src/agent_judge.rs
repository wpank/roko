//! A production [`JudgeOracle`]: the LLM-judge rung's prompt goes to a roko
//! [`Agent`], typically a cheap model, and the first number in its answer is
//! the score (gap-85f102).

use std::sync::Arc;

use async_trait::async_trait;
use roko_agent::Agent;
use roko_core::{Body, Context, Kind, Signal};

use crate::llm_judge_gate::JudgeOracle;

/// What the judge is told about its answer's shape.
const ANSWER_FORMAT: &str = "Answer with the score alone: one number from 0.0 to 1.0.";

/// A [`JudgeOracle`] backed by a roko [`Agent`].
///
/// A failed run, or an answer with no number in it, is an error, which the
/// LLM-judge gate turns into an advisory pass unless it is blocking.
pub struct AgentJudgeOracle {
    agent: Arc<dyn Agent>,
}

impl AgentJudgeOracle {
    /// An oracle that asks `agent` for each score.
    #[must_use]
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        Self { agent }
    }
}

impl std::fmt::Debug for AgentJudgeOracle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentJudgeOracle")
            .field("agent", &self.agent.name())
            .finish()
    }
}

#[async_trait]
impl JudgeOracle for AgentJudgeOracle {
    async fn judge(&self, prompt: &str) -> Result<f32, String> {
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text(format!("{prompt}\n\n{ANSWER_FORMAT}")))
            .build();
        let result = self.agent.run(&input, &Context::now()).await;
        if !result.success {
            return Err(format!("judge agent `{}` failed", self.agent.name()));
        }
        let answer = result
            .output
            .body
            .as_text()
            .map_err(|error| format!("judge answer is not text: {error}"))?;
        parse_judge_score(answer).ok_or_else(|| {
            let start: String = answer.chars().take(80).collect();
            format!("no score in the judge's answer: {start}")
        })
    }
}

/// The first number in `answer`, clamped to `[0, 1]`. A full stop that ends
/// the sentence is not part of it.
fn parse_judge_score(answer: &str) -> Option<f32> {
    answer
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .find_map(|token| token.trim_end_matches('.').parse::<f32>().ok())
        .map(|score| score.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use roko_agent::AgentResult;

    use super::*;

    /// Answers every prompt with `answer`, or fails when `answer` is `None`.
    struct FixedAgent {
        answer: Option<&'static str>,
    }

    #[async_trait]
    impl Agent for FixedAgent {
        async fn run(&self, input: &Signal, _ctx: &Context) -> AgentResult {
            let prompt = input.body.as_text().unwrap_or_default();
            assert!(prompt.ends_with(ANSWER_FORMAT), "{prompt}");
            let output = Signal::builder(Kind::AgentOutput)
                .body(Body::text(self.answer.unwrap_or("provider down")))
                .build();
            if self.answer.is_some() {
                AgentResult::ok(output)
            } else {
                AgentResult::fail(output)
            }
        }

        fn name(&self) -> &str {
            "fixed-judge"
        }
    }

    fn oracle(answer: Option<&'static str>) -> AgentJudgeOracle {
        AgentJudgeOracle::new(Arc::new(FixedAgent { answer }))
    }

    #[tokio::test]
    async fn the_agents_first_number_is_the_score() {
        let score = oracle(Some("Score: 0.75 (clean diff)"))
            .judge("Score this diff on a 0.0-1.0 scale.")
            .await
            .expect("a score");
        assert!((score - 0.75).abs() < 1e-6, "{score}");
        let clamped = oracle(Some("7")).judge("Score it.").await.expect("a score");
        assert!((clamped - 1.0).abs() < 1e-6, "{clamped}");
        let sentence = oracle(Some("I'd say 0.8.")).judge("Score it.").await;
        assert!(sentence.is_ok_and(|score| (score - 0.8).abs() < 1e-6));
    }

    #[tokio::test]
    async fn a_failed_run_or_an_answer_without_a_number_is_an_error() {
        let failed = oracle(None)
            .judge("Score it.")
            .await
            .expect_err("failed run");
        assert!(failed.contains("fixed-judge"), "{failed}");
        let wordy = oracle(Some("looks fine"))
            .judge("Score it.")
            .await
            .expect_err("no number");
        assert!(wordy.contains("no score"), "{wordy}");
    }
}
