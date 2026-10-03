//! The judge rung's verdict (9123): a rubric judge over a task's artefacts,
//! whose every criterion verdict must quote the artefact.
//!
//! Judges are biased and can be gamed, so a criterion's score counts only
//! when the judge backs it with a passage quoted word for word from an
//! artefact. [`evidence_prompt`] asks for that, [`parse_criterion_answers`]
//! reads the answer, and [`evidence_verdict`] checks each quote: a criterion
//! whose verdict quotes nothing that occurs is `no_evidence`, neither a pass
//! nor a fail.

use roko_core::Verdict;
use serde::Deserialize;

/// The most artefact text a prompt carries, in bytes.
const MAX_PROMPT_ARTEFACT_BYTES: usize = 24 * 1024;

/// The most diff text a prompt carries, in bytes.
const MAX_PROMPT_DIFF_BYTES: usize = 8 * 1024;

/// The shortest quote that backs a score, in characters: a word or two
/// occurs anywhere.
const MIN_QUOTE_CHARS: usize = 12;

/// One criterion's answer from the judge.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CriterionAnswer {
    /// The criterion's number, from 1.
    pub criterion: usize,
    /// Its score, from 0.0 to 1.0.
    pub score: f32,
    /// The passage of an artefact that backs the score, word for word.
    #[serde(default)]
    pub quote: String,
}

/// The prompt that asks a judge to score `artefacts`, each a path and its
/// text, against each of `criteria`, quoting its evidence. `diff`, the
/// attempt's changes, is context only.
#[must_use]
pub fn evidence_prompt(criteria: &[String], artefacts: &[(String, String)], diff: &str) -> String {
    let mut prompt = String::from(
        "Judge the work below against each numbered criterion. For each criterion, give a \
         score from 0.0 to 1.0, and quote word for word the passage of an artefact that \
         supports the score. A score without a quote that occurs in an artefact does not \
         count.\n\nCriteria:\n",
    );
    for (index, criterion) in criteria.iter().enumerate() {
        prompt.push_str(&format!("{}. {criterion}\n", index + 1));
    }
    prompt.push_str("\nArtefacts:\n");
    let mut room = MAX_PROMPT_ARTEFACT_BYTES;
    for (path, text) in artefacts {
        let shown = truncated(text, room);
        room = room.saturating_sub(shown.len());
        prompt.push_str(&format!("=== {path} ===\n{shown}\n=== end of {path} ===\n"));
    }
    if !diff.trim().is_empty() {
        let diff = truncated(diff, MAX_PROMPT_DIFF_BYTES);
        prompt.push_str(&format!("\nThe attempt's changes, for context only:\n{diff}\n"));
    }
    prompt.push_str(
        "\nAnswer with a JSON array and nothing else, one object per criterion, such as \
         [{\"criterion\": 1, \"score\": 0.8, \"quote\": \"words copied from an artefact\"}].",
    );
    prompt
}

/// `text` cut to at most `limit` bytes on a character boundary, marked when
/// cut.
fn truncated(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let cut = text.floor_char_boundary(limit);
    format!("{}\n...[truncated]", &text[..cut])
}

/// The criterion answers in a judge's reply: the JSON array in it, or `None`
/// when it holds none.
#[must_use]
pub fn parse_criterion_answers(reply: &str) -> Option<Vec<CriterionAnswer>> {
    let start = reply.find('[')?;
    let end = reply.rfind(']')?;
    serde_json::from_str(reply.get(start..=end)?).ok()
}

/// The judge rung `gate`'s verdict on `answers` to `criteria` about
/// `artefacts`.
///
/// A criterion's score counts when its answer quotes, word for word up to
/// runs of whitespace, at least [`MIN_QUOTE_CHARS`] characters of an
/// artefact; otherwise the criterion is `no_evidence`. The rung fails when a
/// counted score is below `min_score`, is skipped when no score counts, and
/// passes otherwise with the lowest counted score. Its detail gives each
/// criterion's outcome.
#[must_use]
pub fn evidence_verdict(
    gate: &str,
    criteria: &[String],
    artefacts: &[(String, String)],
    answers: &[CriterionAnswer],
    min_score: f32,
) -> Verdict {
    let mut lines = Vec::new();
    let mut scores = Vec::new();
    let mut low = Vec::new();
    for (index, criterion) in criteria.iter().enumerate() {
        let number = index + 1;
        let evidenced = answers
            .iter()
            .find(|answer| answer.criterion == number)
            .filter(|answer| quoted(&answer.quote, artefacts));
        let Some(answer) = evidenced else {
            lines.push(format!("{number}. {criterion}: no_evidence"));
            continue;
        };
        let score = answer.score.clamp(0.0, 1.0);
        let quote = answer.quote.trim();
        lines.push(format!("{number}. {criterion}: {score:.2}, quoting \"{quote}\""));
        if score < min_score {
            low.push(format!("criterion {number} scored {score:.2}"));
        }
        scores.push(score);
    }
    let verdict = if !low.is_empty() {
        let reason = format!("below {min_score:.2} with evidence: {}", low.join(", "));
        Verdict::fail(gate, reason)
    } else if scores.is_empty() {
        Verdict::skip(gate, "no_evidence: no criterion's verdict quotes an artefact")
    } else {
        let lowest = scores.iter().copied().fold(1.0_f32, f32::min);
        let mut verdict = Verdict::pass(gate).with_score(lowest);
        verdict.reason = format!(
            "{} of {} criteria scored with evidence",
            scores.len(),
            criteria.len()
        );
        verdict
    };
    verdict.with_detail(lines.join("\n"))
}

/// Whether `quote` occurs in one of `artefacts`, word for word up to runs of
/// whitespace, and is long enough to back a score.
fn quoted(quote: &str, artefacts: &[(String, String)]) -> bool {
    let normal = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let quote = normal(quote);
    quote.chars().count() >= MIN_QUOTE_CHARS
        && artefacts
            .iter()
            .any(|(_, text)| normal(text).contains(&quote))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 9123: a score counts only with a quote that occurs in the artefact; a
    /// low evidenced score fails, no evidence at all is no score, and the
    /// reply's JSON array is read through the prose around it.
    #[test]
    fn evidence_verdict_counts_only_quoted_scores() {
        let criteria = vec![
            "Every claim cites a source".to_string(),
            "The summary is under 200 words".to_string(),
        ];
        let artefacts = vec![(
            "report.md".to_string(),
            "Transformers replaced recurrence (Vaswani et al., 2017).\nSummary: short."
                .to_string(),
        )];
        let reply = "Here you go:\n[{\"criterion\": 1, \"score\": 0.3, \"quote\": \
                     \"Transformers replaced  recurrence\"}, {\"criterion\": 2, \"score\": 0.9, \
                     \"quote\": \"not in the report at all\"}]";
        let answers = parse_criterion_answers(reply).expect("a JSON array");
        let verdict = evidence_verdict("rung[rubric]", &criteria, &artefacts, &answers, 0.8);
        assert!(!verdict.passed && !verdict.skipped, "{verdict:?}");
        assert_eq!(
            verdict.reason,
            "below 0.80 with evidence: criterion 1 scored 0.30"
        );
        let detail = verdict.detail.unwrap_or_default();
        assert!(
            detail.contains("2. The summary is under 200 words: no_evidence"),
            "{detail}"
        );

        let unbacked = vec![CriterionAnswer {
            criterion: 1,
            score: 0.1,
            quote: String::new(),
        }];
        let verdict = evidence_verdict("rung[rubric]", &criteria, &artefacts, &unbacked, 0.8);
        assert!(verdict.skipped && !verdict.passed, "{verdict:?}");
        assert!(parse_criterion_answers("no verdicts here").is_none());
    }
}
