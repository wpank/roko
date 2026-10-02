//! A production [`JudgeOracle`]: the LLM-judge rung's prompt goes to a roko
//! [`Agent`], typically a cheap model, and its answer must hold one score on
//! the 0–1 scale (gap-85f102).

use std::sync::Arc;

use async_trait::async_trait;
use roko_agent::Agent;
use roko_core::{Body, Context, Kind, Signal};

use crate::llm_judge_gate::JudgeOracle;

/// What the judge is told about its answer's shape.
const ANSWER_FORMAT: &str = "Answer with the score alone: one number from 0.0 to 1.0.";

/// A [`JudgeOracle`] backed by a roko [`Agent`].
///
/// A failed run, or an answer that holds no score on the 0–1 scale (a list
/// number, a scale echo, a `7` or a `7/10`), is an error, which the LLM-judge
/// gate turns into an advisory pass unless it is blocking.
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

/// The score in a judge's answer, or `None` when it holds no score on the 0–1
/// scale.
///
/// The first line that offers a score decides: a line holding only a number,
/// or a `score:` or `rating:` label followed by a number. A list marker
/// (`1. Root cause`), a range and a scale echo (`0.0-1.0`, `0 to 1`, the `10`
/// of `out of 10`) are never the score. A line that names a scale other than
/// 0–1 (`7/10`, `Score (0-10): 1`) holds no score, and a number outside
/// `[0, 1]` is no score rather than a clamped one. roko-learn's
/// `quality_judge::parse_score` keeps a copy of this rule: roko-learn does not
/// depend on roko-gate.
fn parse_judge_score(answer: &str) -> Option<f32> {
    answer
        .lines()
        .map(tokens)
        .find_map(|line| {
            let score = labelled(&line).or_else(|| alone(&line))?;
            let on_scale = !names_another_scale(&line) && (0.0..=1.0).contains(&score);
            Some(on_scale.then_some(score))
        })
        .flatten()
}

/// One piece of a judge's answer line.
#[derive(Clone, Copy)]
enum Token<'a> {
    /// A number, `\d+(\.\d+)?`, and whether it is an integer.
    Number(f32, bool),
    /// A run of letters.
    Word(&'a str),
    /// Any other character that is not a space.
    Mark(char),
}

/// A line as numbers, words and marks. Markdown emphasis and code ticks are
/// dropped, so `**0.8**` is the number 0.8.
fn tokens(line: &str) -> Vec<Token<'_>> {
    let digits = |text: &str| run_len(text, |c| c.is_ascii_digit());
    let mut tokens = Vec::new();
    let mut rest = line;
    while let Some(first) = rest.chars().next() {
        let len = if first.is_ascii_digit() {
            let whole = digits(rest);
            let end = match digits(rest[whole..].strip_prefix('.').unwrap_or_default()) {
                0 => whole,
                fraction => whole + 1 + fraction,
            };
            let value = rest[..end].parse().unwrap_or(f32::INFINITY);
            tokens.push(Token::Number(value, end == whole));
            end
        } else if first.is_alphabetic() {
            let end = run_len(rest, char::is_alphabetic);
            tokens.push(Token::Word(&rest[..end]));
            end
        } else {
            if !first.is_whitespace() && !matches!(first, '*' | '_' | '`') {
                tokens.push(Token::Mark(first));
            }
            first.len_utf8()
        };
        rest = &rest[len..];
    }
    tokens
}

/// The length of the run of characters at the start of `text` that pass
/// `keep`.
fn run_len(text: &str, keep: fn(char) -> bool) -> usize {
    text.find(|c: char| !keep(c)).unwrap_or(text.len())
}

/// The number after a line's first `score:` or `rating:` label. The label may
/// carry its scale in brackets, as in `Score (0.0-1.0): 0.6`.
fn labelled(line: &[Token<'_>]) -> Option<f32> {
    (0..line.len()).find_map(|at| {
        let [Token::Word(word), rest @ ..] = &line[at..] else {
            return None;
        };
        if !word.eq_ignore_ascii_case("score") && !word.eq_ignore_ascii_case("rating") {
            return None;
        }
        let rest = match rest {
            [Token::Mark('('), ..] => {
                let close = rest.iter().position(|t| matches!(t, Token::Mark(')')))?;
                &rest[close + 1..]
            }
            _ => rest,
        };
        let [Token::Mark(':' | '='), Token::Number(score, _), after @ ..] = rest else {
            return None;
        };
        (!starts_a_range(after)).then_some(*score)
    })
}

/// Whether `after`, what follows a number, makes that number the start of a
/// range, as `-1.0` does after the `0.0` of `0.0-1.0`.
fn starts_a_range(after: &[Token<'_>]) -> bool {
    match after {
        [Token::Mark('-' | '–' | '—'), Token::Number(..), ..] => true,
        [Token::Word(word), Token::Number(..), ..] => word.eq_ignore_ascii_case("to"),
        _ => false,
    }
}

/// The number of a line that holds only a number, perhaps with its scale
/// (`0.7`, `0.7 out of 1`, `7/10`) or a closing full stop. An integer before
/// a full stop is a list marker (`1.`), not a number alone.
fn alone(line: &[Token<'_>]) -> Option<f32> {
    let line = match line {
        [rest @ .., Token::Mark('.')] if !matches!(rest, [Token::Number(_, true)]) => rest,
        line => line,
    };
    let [Token::Number(score, _), scale @ ..] = line else {
        return None;
    };
    let bare = match scale {
        [] | [Token::Mark('/'), Token::Number(..)] => true,
        [Token::Word(out), Token::Word(of), Token::Number(..)] => {
            out.eq_ignore_ascii_case("out") && of.eq_ignore_ascii_case("of")
        }
        _ => false,
    };
    bare.then_some(*score)
}

/// Whether a line names a scale other than 0–1: a number that is not 1 ends a
/// range (`0-10`, `0 to 100`) or bounds a ratio (`7/10`, `out of 5`).
fn names_another_scale(line: &[Token<'_>]) -> bool {
    line.windows(3).any(|window| match window {
        [first, joiner, Token::Number(bound, _)] => {
            let scale = match (first, joiner) {
                (Token::Number(..), Token::Mark('-' | '–' | '—' | '/')) => true,
                (Token::Number(..), Token::Word(word)) => word.eq_ignore_ascii_case("to"),
                (Token::Word(out), Token::Word(of)) => {
                    out.eq_ignore_ascii_case("out") && of.eq_ignore_ascii_case("of")
                }
                _ => false,
            };
            scale && (bound - 1.0).abs() > f32::EPSILON
        }
        _ => false,
    })
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
    async fn a_lone_or_labelled_number_in_range_is_the_score() {
        let score = oracle(Some("Score: 0.75 (clean diff)"))
            .judge("Score this diff on a 0.0-1.0 scale.")
            .await
            .expect("a score");
        assert!((score - 0.75).abs() < 1e-6, "{score}");
        let sentence = oracle(Some("0.8.")).judge("Score it.").await;
        assert!(sentence.is_ok_and(|score| (score - 0.8).abs() < 1e-6));
        let unclamped = oracle(Some("7")).judge("Score it.").await;
        assert!(unclamped.is_err(), "{unclamped:?}");
        let prose = oracle(Some("I'd say 0.8.")).judge("Score it.").await;
        assert!(prose.is_err(), "{prose:?}");
    }

    #[tokio::test]
    async fn judge_scores_ignore_list_numbers() {
        let unscored = [
            "1. The diff does not implement the retry.\n2. It adds no test.",
            "On a 0.0-1.0 scale I'd give 0.4",
            "7",
            "7/10",
            "Score: 1/10",
            "Score (0-10): 1",
        ];
        for answer in unscored {
            let error = oracle(Some(answer))
                .judge("Score it.")
                .await
                .expect_err(answer);
            assert!(error.contains("no score"), "{answer}: {error}");
        }
        let scored = [
            ("Score: 0.6", 0.6),
            ("0.73", 0.73),
            ("Score (0.0-1.0): 0.4", 0.4),
            (
                "1. Tests pass.\n2. The retry is bounded.\n\nRating: 0.9",
                0.9,
            ),
            ("**0.8** out of 1", 0.8),
        ];
        for (answer, expected) in scored {
            let score = oracle(Some(answer))
                .judge("Score it.")
                .await
                .expect("a score");
            assert!((score - expected).abs() < 1e-6, "{answer}: {score}");
        }
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
