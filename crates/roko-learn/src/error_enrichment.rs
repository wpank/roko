//! STATUS: WIRED — used by gate failure replan (`build_gate_failure_plan_revision`).
//!
//! Cheap enrichment pass for noisy gate failures before retry.
//!
//! The caller is expected to provide a low-cost agent (for example a
//! mechanical/haiku-tier model). This helper constrains the prompt size,
//! requests a short diagnosis, and falls back to a deterministic summary when
//! the enrichment agent fails.

use roko_agent::Agent;
use roko_core::{Body, Context, Kind, Signal};

const RAW_ERROR_LIMIT: usize = 4_000;
const TASK_CONTEXT_LIMIT: usize = 1_000;
const DIAGNOSIS_SENTENCE_LIMIT: usize = 2;
/// The longest diagnosis kept, in characters: two sentences such as phase 5's
/// stand whole (bug-cdd885), and the raw output below them is short.
const DIAGNOSIS_CHAR_LIMIT: usize = 600;

/// The most failing cases a fallback diagnosis names one by one.
const CASES_NAMED: usize = 6;

/// The most characters of a failing case's name, and of its first failed
/// assertion, that a fallback diagnosis quotes.
const CASE_PART_CHARS: usize = 120;

/// Summarize raw compiler or test output into a short retry-ready diagnosis.
///
/// The supplied `agent` should already be configured for the cheapest
/// mechanical-tier summarization pass. If the call fails or returns unusable
/// output, this falls back to a deterministic two-sentence digest extracted
/// from the raw error text.
pub async fn enrich_error_digest(raw_error: &str, agent: &dyn Agent, task_context: &str) -> String {
    let prompt = diagnosis_prompt(raw_error, task_context);
    let input = Signal::builder(Kind::Prompt)
        .body(Body::text(prompt))
        .build();
    let result = agent.run(&input, &Context::now()).await;

    if result.success
        && let Ok(text) = result.output.body.as_text()
    {
        let normalized = normalize_diagnosis(text);
        if !normalized.is_empty() {
            return normalized;
        }
    }

    fallback_diagnosis(raw_error, task_context)
}

/// The prompt that asks the cheap model to diagnose `raw_error`: what failed
/// against what it expected, for a test or a check as for a compiler
/// (bug-cdd885).
fn diagnosis_prompt(raw_error: &str, task_context: &str) -> String {
    format!(
        "Diagnose this build or test failure in 2 sentences. \
         Say which file and line, or which test or check, failed, and what it found against what \
         it expected; then name what else fails, if anything.\n\
         Error output:\n{}\n\
         Task context:\n{}\n\
         Diagnosis:",
        truncate_chars(raw_error, RAW_ERROR_LIMIT),
        truncate_chars(task_context, TASK_CONTEXT_LIMIT),
    )
}

/// The model's `text` as a diagnosis: as many of its first sentences as fit
/// whole in [`DIAGNOSIS_CHAR_LIMIT`], up to [`DIAGNOSIS_SENTENCE_LIMIT`]. A
/// first sentence longer than the limit is cut at a word boundary and marked
/// with "…", so a diagnosis never ends mid-word (bug-cdd885).
fn normalize_diagnosis(text: &str) -> String {
    let compact = collapse_whitespace(text);
    if compact.is_empty() {
        return String::new();
    }

    let sentences = leading_sentences(&compact, DIAGNOSIS_SENTENCE_LIMIT);
    let mut kept = String::new();
    for sentence in &sentences {
        let joined = if kept.is_empty() {
            sentence.clone()
        } else {
            format!("{kept} {sentence}")
        };
        if joined.chars().count() > DIAGNOSIS_CHAR_LIMIT {
            break;
        }
        kept = joined;
    }
    if kept.is_empty() {
        let first = sentences.first().unwrap_or(&compact);
        return cut_at_word(first, DIAGNOSIS_CHAR_LIMIT);
    }
    kept
}

fn fallback_diagnosis(raw_error: &str, task_context: &str) -> String {
    let cases = crate::failure_text::failed_cases(raw_error);
    if !cases.is_empty() {
        return cases_diagnosis(&cases);
    }
    let file_line = extract_file_line(raw_error);
    let headline = extract_headline(raw_error);
    let mismatch = extract_mismatch(raw_error);

    let first = match (file_line.as_deref(), headline.as_deref()) {
        (Some(location), Some(detail)) if detail.contains(location) => {
            format!("The primary error is {detail}")
        }
        (Some(location), Some(detail)) => {
            format!("The failure points to {location}, where {detail}")
        }
        (Some(location), None) => format!("The failure points to {location}"),
        (None, Some(detail)) => format!("The primary error is {detail}"),
        (None, None) => {
            "The gate failed with compiler or test output that needs a targeted fix".to_string()
        }
    };

    let second = if let Some(detail) = mismatch {
        format!("The retry should fix {detail}")
    } else if !task_context.trim().is_empty() {
        format!(
            "The retry should focus on the failing code path for {}",
            truncate_chars(&collapse_whitespace(task_context), 140)
        )
    } else {
        "The retry should focus on the cited file and align the implementation with the failing check"
            .to_string()
    };

    format!("{} {}", ensure_sentence(&first), ensure_sentence(&second))
}

/// A diagnosis that names each failing case of a test run once (bug-3e3d08):
/// its name, where its first failure is, what that failure asserted, and how
/// many failures it had, at most [`CASES_NAMED`] of them.
fn cases_diagnosis(cases: &[crate::failure_text::FailedCase]) -> String {
    let named: Vec<String> = cases
        .iter()
        .take(CASES_NAMED)
        .map(|case| {
            let first = &case.failures[0];
            let at = first
                .location()
                .map(|location| format!(" at {location}"))
                .unwrap_or_default();
            let times = match case.failures.len() {
                1 => String::new(),
                failures => format!(", {failures} failures"),
            };
            format!(
                "{}{at}: {}{times}",
                truncate_chars(&collapse_whitespace(case.name()), CASE_PART_CHARS),
                truncate_chars(&collapse_whitespace(first.assertion()), CASE_PART_CHARS),
            )
        })
        .collect();
    let more = match cases.len().saturating_sub(CASES_NAMED) {
        0 => String::new(),
        more => format!("; and {more} more"),
    };
    let which = if cases.len() == 1 {
        "1 test case failed".to_string()
    } else {
        format!("{} test cases failed", cases.len())
    };
    format!(
        "{} The retry should make each of them pass without breaking the others.",
        ensure_sentence(&format!("{which}: {}{more}", named.join("; ")))
    )
}

fn extract_file_line(raw_error: &str) -> Option<String> {
    raw_error.lines().find_map(|line| {
        let trimmed = line.trim();
        let candidate = trimmed.strip_prefix("-->").unwrap_or(trimmed).trim();
        candidate
            .split_whitespace()
            .find(|token| is_file_line_token(token))
            .map(|token| token.trim_end_matches([':', ',']).to_string())
    })
}

/// Whether `token` is a source location, `path/file.ext:line` with an
/// optional column: Rust's `src/lib.rs:9:5` as well as C++'s
/// `layout_test.cpp:219` or Python's `test_text.py:3` (bug-3e3d08).
fn is_file_line_token(token: &str) -> bool {
    let token = token.trim_end_matches([':', ',']);
    let Some((path, tail)) = token.split_once(':') else {
        return false;
    };
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let extension = file.rsplit_once('.').map_or("", |(_, extension)| extension);
    let line = tail.split(':').next().unwrap_or_default();
    (1..=5).contains(&extension.len())
        && extension.chars().all(|ch| ch.is_ascii_alphanumeric())
        && !line.is_empty()
        && line.chars().all(|ch| ch.is_ascii_digit())
}

fn extract_headline(raw_error: &str) -> Option<String> {
    raw_error
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !crate::failure_text::is_summary_line(line)
                && (line.starts_with("error[")
                    || line.contains(": error:")
                    || line.starts_with("error:")
                    || line.contains("panicked at")
                    || line.contains("FAIL")
                    || line.contains("failed"))
        })
        .map(clean_fragment)
}

fn extract_mismatch(raw_error: &str) -> Option<String> {
    if let Some(line) = raw_error
        .lines()
        .map(str::trim)
        .find(|line| line.contains("expected ") && line.contains("found "))
    {
        return Some(truncate_chars(&focus_fragment(line), 180));
    }

    let detail = raw_error
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && (line.contains("mismatched types")
                    || line.contains("expected ")
                    || line.contains("found ")
                    || line.contains("no method named")
                    || line.contains("cannot find")
                    || line.contains("trait bound")
                    || line.contains("borrow of moved value"))
        })
        .map(focus_fragment)
        .filter(|line| !line.is_empty())
        .fold(Vec::new(), |mut kept, fragment| {
            // A fragment said twice says nothing more (bug-3e3d08).
            if kept.len() < 2 && !kept.contains(&fragment) {
                kept.push(fragment);
            }
            kept
        })
        .join(" ");

    if detail.is_empty() {
        None
    } else {
        Some(truncate_chars(&detail, 180))
    }
}

fn clean_fragment(text: &str) -> String {
    collapse_whitespace(text)
        .trim_end_matches(['.', '!', '?'])
        .to_string()
}

fn focus_fragment(text: &str) -> String {
    let cleaned = clean_fragment(text);
    const KEYWORDS: &[&str] = &[
        "expected ",
        "found ",
        "mismatched types",
        "no method named",
        "cannot find",
        "trait bound",
        "borrow of moved value",
    ];

    let start = KEYWORDS
        .iter()
        .filter_map(|keyword| cleaned.find(keyword))
        .min()
        .unwrap_or(0);

    cleaned[start..]
        .trim_start_matches(['|', '-', '^', ':', ' '])
        .to_string()
}

/// The first `limit` sentences of `text`; a last one without an end mark gets
/// a full stop.
fn leading_sentences(text: &str, limit: usize) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0;

    for (idx, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?') && is_sentence_boundary(text, idx, ch) {
            let end = idx + ch.len_utf8();
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
                if sentences.len() == limit {
                    break;
                }
            }
            start = end;
        }
    }

    if sentences.len() < limit {
        let tail = text[start..].trim();
        if !tail.is_empty() {
            sentences.push(ensure_sentence(tail));
        }
    }

    sentences.truncate(limit);
    sentences
}

/// The first `max_chars` characters of `text`, cut at the last word boundary
/// and ending with "…" when anything is cut.
fn cut_at_word(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let head: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    let words = head
        .rfind(' ')
        .map_or(head.as_str(), |end| &head[..end])
        .trim_end_matches([',', ';', ':', ' ']);
    format!("{words}…")
}

fn is_sentence_boundary(text: &str, idx: usize, ch: char) -> bool {
    let next_idx = idx + ch.len_utf8();
    let mut rest = text[next_idx..].chars().skip_while(|c| c.is_whitespace());
    match rest.next() {
        None => true,
        Some(next) => next.is_ascii_uppercase() || matches!(next, '"' | '\'' | '('),
    }
}

fn ensure_sentence(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with(['.', '!', '?']) {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::{DIAGNOSIS_CHAR_LIMIT, diagnosis_prompt, enrich_error_digest, fallback_diagnosis};
    use roko_agent::MockAgent;

    /// bug-3e3d08: with the cheap model out of balance, Undertow's phase 4.2
    /// T1#1 got "The primary error is [doctest] test cases: 5 | 3 passed | 2
    /// failed | 0 skipped. The retry should fix found != nullptr ) is NOT
    /// correct found != nullptr ) is NOT correct." Now the fallback names
    /// each failing case once, where it failed and what it asserted, and says
    /// nothing twice.
    #[test]
    fn fallback_diagnosis_repeats_no_fragment() {
        let diagnosis = fallback_diagnosis(
            crate::failure_text::tests::PHASE_4_2_LAYOUT,
            "Lay the editor out as designed",
        );

        assert!(
            diagnosis.starts_with("2 test cases failed: P4.2.01 every control"),
            "{diagnosis}"
        );
        for needed in [
            "at layout_test.cpp:109: CHECK( !b.intersects(otherBounds) )",
            "P4.2.04 every control has the design's short label, drawn under it",
            "at layout_test.cpp:219: CHECK( found != nullptr ), 4 failures",
        ] {
            assert!(diagnosis.contains(needed), "{needed} in {diagnosis}");
        }
        assert_eq!(
            diagnosis.matches("found != nullptr").count(),
            1,
            "{diagnosis}"
        );
        assert!(!diagnosis.contains("[doctest]"), "{diagnosis}");
        // A Rust mismatch said on two lines is said once.
        let twice =
            "error[E0308]: mismatched types\n  found `Option<String>`\n  found `Option<String>`\n";
        let diagnosis = fallback_diagnosis(twice, "");
        assert_eq!(
            diagnosis.matches("found `Option<String>`").count(),
            1,
            "{diagnosis}"
        );
        // A C++ compile error's location is read.
        let cpp = "src/ui/Look.cpp:89:48: error: no member named 'boneDim' in 'Theme'\n";
        let diagnosis = fallback_diagnosis(cpp, "");
        assert_eq!(
            diagnosis,
            "The primary error is src/ui/Look.cpp:89:48: error: no member named 'boneDim' in 'Theme'. \
             The retry should focus on the cited file and align the implementation with the failing \
             check."
        );
    }

    #[tokio::test]
    async fn error_enrichment_returns_two_sentence_mock_diagnosis() {
        let agent = MockAgent::reply(
            "The compile error is in crates/roko-learn/src/model_router.rs:144:17 where \
             Rust reports a mismatched types error. The function expects a String but the \
             code passes an Option<String>. Extra commentary that should be dropped.",
        );

        let diagnosis = enrich_error_digest(
            "error[E0308]: mismatched types",
            &agent,
            "Task 2J.09 implements error enrichment.",
        )
        .await;

        assert_eq!(
            diagnosis,
            "The compile error is in crates/roko-learn/src/model_router.rs:144:17 where Rust reports a mismatched types error. The function expects a String but the code passes an Option<String>."
        );
    }

    #[tokio::test]
    async fn error_enrichment_falls_back_when_agent_fails() {
        let agent = MockAgent::fail_with("upstream timeout");
        let raw_error = "\
error[E0308]: mismatched types
  --> crates/roko-learn/src/model_router.rs:144:17
   |
144 |     route(model)
   |     ----- ^^^^^ expected `String`, found `Option<String>`
   |
";

        let diagnosis = enrich_error_digest(
            raw_error,
            &agent,
            "Update the router selection path for retry handling.",
        )
        .await;

        assert_eq!(
            diagnosis,
            "The failure points to crates/roko-learn/src/model_router.rs:144:17, where error[E0308]: mismatched types. The retry should fix expected `String`, found `Option<String>`."
        );
    }

    #[tokio::test]
    async fn error_enrichment_falls_back_when_agent_output_is_empty() {
        let agent = MockAgent::reply("   ");

        let diagnosis = enrich_error_digest(
            "thread 'router' panicked at src/lib.rs:91:5: assertion failed: left == right",
            &agent,
            "",
        )
        .await;

        assert_eq!(
            diagnosis,
            "The primary error is thread 'router' panicked at src/lib.rs:91:5: assertion failed: left == right. The retry should focus on the cited file and align the implementation with the failing check."
        );
    }

    /// The diagnosis the cheap model's `reply` comes to.
    async fn diagnosed(reply: &str) -> String {
        let agent = MockAgent::reply(reply);
        enrich_error_digest("CHECK( -33.0571 >= -30 )", &agent, "Render the demos").await
    }

    /// bug-cdd885: phase 5's diagnoses were cut at 320 characters mid-word,
    /// dropping what else failed. Two sentences that fit in 600 characters
    /// stand whole; of two that don't, the first stands alone; a first
    /// sentence longer than the limit is cut at a word boundary and marked
    /// with "…". A diagnosis never ends mid-word.
    #[tokio::test]
    async fn a_long_diagnosis_ends_on_a_whole_sentence() {
        let t4 = "The failure is in `tests/spec/p5/demos_test.cpp:28`, where \
                  `spec::toDb(spec::rms(wav.left))` is expected to be at least `-30.0`, but the \
                  generated audio is only `-33.0571 dBFS RMS`. In other words, the rendered WAVs are \
                  too quiet by about 3 dB, so the signal-level check fails for \
                  `artifacts/audio/init.wav` (and also `sub-pure-foundation.wav` and \
                  `rumble-dark-room.wav`).";
        assert!(t4.chars().count() > 320, "{}", t4.len());
        assert_eq!(diagnosed(t4).await, t4);

        let first = format!(
            "The spectral check fails{}.",
            " because the preset keeps too much energy below the filter".repeat(6)
        );
        let second = format!(
            "Other checks fail{}.",
            " for the same reason in another preset file".repeat(8)
        );
        assert!(first.chars().count() + second.chars().count() > DIAGNOSIS_CHAR_LIMIT);
        let both = format!("{first} {second} A third sentence is dropped.");
        assert_eq!(diagnosed(&both).await, first);

        let long = format!(
            "The check fails{}.",
            " because a long reason continues here".repeat(20)
        );
        let diagnosis = diagnosed(&long).await;
        assert!(
            diagnosis.chars().count() <= DIAGNOSIS_CHAR_LIMIT,
            "{diagnosis}"
        );
        let cut = diagnosis.strip_suffix('…').expect("a marked cut");
        assert!(long.starts_with(cut), "{diagnosis}");
        assert!(long[cut.len()..].starts_with(' '), "{diagnosis}");
    }

    /// bug-cdd885: the prompt asks what failed against what it expected, for
    /// a test or a check as for a compiler, not for a "type mismatch".
    #[test]
    fn the_diagnosis_prompt_asks_what_failed_against_what_it_expected() {
        let prompt = diagnosis_prompt("CHECK( -33.0571 >= -30 )", "Render the demos");
        assert!(prompt.contains("test or check"), "{prompt}");
        assert!(
            prompt.contains("what it found against what it expected"),
            "{prompt}"
        );
        assert!(!prompt.contains("type mismatch"), "{prompt}");
        assert!(prompt.contains("CHECK( -33.0571 >= -30 )"), "{prompt}");
        assert!(prompt.contains("Render the demos"), "{prompt}");
    }
}
