//! STATUS: UNWIRED — no production caller since `plan run` stopped scoring
//! failed verifies with it for the gate-gaming detector (S05 F1).
//!
//! Lightweight LLM judge for tasks without compilable output.
//!
//! Some routing tasks, such as research, documentation, and architecture
//! work, do not produce a binary pass/fail artifact. This module provides a
//! cheap judge call that asks a model to score the response on a 0.0 to 1.0
//! scale and returns that value for routing feedback.

use roko_agent::Agent;
use roko_core::{Body, Context, Kind, Signal};

/// Ask a cheap judge model to score a response in `[0.0, 1.0]`.
///
/// The prompt asks the judge model to rate the response against the provided
/// rubric. If the agent call fails, the output is not readable as text, or the
/// answer holds no score on the 0–1 scale (a list number, a scale echo and a
/// `7` or `7/10` are none), this returns `0.0`.
#[must_use]
pub async fn judge_quality(agent: &dyn Agent, prompt: &str, response: &str, rubric: &str) -> f64 {
    let judge_prompt = format!(
        "Rate the quality of this response on a scale of 0.0 to 1.0.\n\
         Rubric: {rubric}\n\
         Prompt: {prompt}\n\
         Response: {response}\n\
         Score (0.0-1.0):"
    );
    let input = Signal::builder(Kind::Prompt)
        .body(Body::text(judge_prompt))
        .build();
    let result = agent.run(&input, &Context::now()).await;
    if !result.success {
        return 0.0;
    }

    let text = match result.output.body.as_text() {
        Ok(text) => text,
        Err(_) => return 0.0,
    };
    parse_score(text).unwrap_or(0.0)
}

/// The score in a judge's answer, or `None` when it holds no score on the 0–1
/// scale.
///
/// The first line that offers a score decides: a line holding only a number,
/// or a `score:` or `rating:` label followed by a number. A list marker
/// (`1. Root cause`), a range and a scale echo (`0.0-1.0`, `0 to 1`, the `10`
/// of `out of 10`) are never the score. A line that names a scale other than
/// 0–1 (`7/10`, `Score (0-10): 1`) holds no score, and a number outside
/// `[0, 1]` is no score rather than a clamped one. roko-gate's
/// `agent_judge::parse_judge_score` keeps a copy of this rule: roko-learn does
/// not depend on roko-gate.
fn parse_score(text: &str) -> Option<f64> {
    text.lines()
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
    Number(f64, bool),
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
            let value = rest[..end].parse().unwrap_or(f64::INFINITY);
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
fn labelled(line: &[Token<'_>]) -> Option<f64> {
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
fn alone(line: &[Token<'_>]) -> Option<f64> {
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
            scale && (bound - 1.0).abs() > f64::EPSILON
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_agent::MockAgent;

    #[tokio::test]
    async fn quality_judge_parses_mock_score() {
        let agent = MockAgent::reply("0.73");
        let score = judge_quality(
            &agent,
            "Summarize the architecture",
            "The service uses a three-layer adapter model.",
            "Prefer correctness, clarity, and completeness.",
        )
        .await;

        assert!((score - 0.73).abs() < 1e-9);
        assert!((0.0..=1.0).contains(&score));
    }

    #[tokio::test]
    async fn quality_judge_returns_zero_when_parse_fails() {
        let agent = MockAgent::reply("cannot score this response");
        let score = judge_quality(&agent, "prompt", "response", "rubric").await;
        assert_eq!(score, 0.0);
    }

    #[tokio::test]
    async fn quality_judge_ignores_list_numbers() {
        let unscored = [
            "1. Root cause: the retry is unbounded.\n2. No test covers it.",
            "On a 0.0-1.0 scale I'd give 0.4",
            "7",
            "7/10",
        ];
        for answer in unscored {
            assert_eq!(parse_score(answer), None, "{answer}");
            let agent = MockAgent::reply(answer);
            let score = judge_quality(&agent, "prompt", "response", "rubric").await;
            assert_eq!(score, 0.0, "{answer}");
        }
        assert_eq!(parse_score("Score: 0.6"), Some(0.6));
        assert_eq!(parse_score("0.73"), Some(0.73));
        assert_eq!(parse_score("Score (0.0-1.0): 0.4"), Some(0.4));
    }
}
