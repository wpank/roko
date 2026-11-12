//! HF6, contradictory assertions (gap-ba2841): a task whose tests or acceptance criteria assert
//! one call to return two values that no one result meets cannot be done, and an agent given one
//! games it. In find-2ba86d's dogfood the request asked for a test asserting both
//! `round_to_dollars(150) == 2` and `== 3`; the spec gate advised, the run was paid for, and the
//! agent passed the test with a call counter.
//!
//! The finder is the gaming screen's ([`test_contradictions`]), run over what the spec says
//! before any attempt, in groups whose assertions must all hold at once:
//!
//! - the task's acceptance: its pinned acceptance tests (`[task.accept]` sources, read from the
//!   plan directory), its `acceptance` items, and the criteria of an acceptance section in its
//!   goal or description;
//! - each other paragraph, list and fenced code block of its goal, description and prompt, on
//!   its own: prose may give what a call returns today beside what it should return.
//!
//! A fenced code block is test code in the language its info string names; one that names none
//! is read as Python, the one language the finder reads. Prose is read for comparisons of a call
//! with a literal, quoted or not: `f(150) == 2`, `f(x) is None`, and "`f(150)` returns `2`" (or
//! "should return", "must return"); a comparison that leaves out its call, as in "`f(150) == 2`
//! and `== 3`", compares the call before it; a code span holding an assertion
//! (`` `self.assertEqual(f(150), 2)` ``) is read as one. Two comparisons of one piece of prose
//! are made at once only when nothing but joining words ([`JOINING_WORDS`]), list markers and
//! punctuation stand between them: "today; it should be", "not" or "after" may set two moments
//! or a negation. A piece that names another call (`put("k", 1)`) or a moment
//! ([`MOMENT_WORDS`]) is read as made after a change of state, so its comparisons contradict no
//! other piece's.

use std::path::{Component, Path};
use std::sync::LazyLock;

use regex::Regex;

use super::{ACCEPTANCE_HEADER, BULLET, CODE_SPAN, SpecTask, ac_id, compile_regex};
use crate::attempt_diff::{Contradiction, test_contradictions};

/// The language prose comparisons, and fenced blocks that name none, are read in.
const PROSE_LANGUAGE: &str = "python";

/// The largest pinned test read, in bytes.
const MAX_TEST_BYTES: u64 = 512 * 1024;

/// A statement that changes state, which the finder's test reading counts as one: it stands
/// where two comparisons of one piece of prose may not be made at once.
const STATE_CHANGE: &str = "_state_may_change = True";

/// Words that may join two comparisons made at once.
const JOINING_WORDS: [&str; 22] = [
    "and",
    "both",
    "also",
    "as",
    "well",
    "plus",
    "too",
    "that",
    "assert",
    "asserts",
    "asserting",
    "assertion",
    "assertions",
    "check",
    "checks",
    "checking",
    "expect",
    "expects",
    "expecting",
    "ensure",
    "ensures",
    "verify",
];

/// Words before a comparison that leaves out its call (`and == 3`).
const ELIDING_WORDS: [&str; 8] = ["and", "both", "also", "plus", "then", "but", "not", "or"];

/// Words that set a moment, a condition or a change, or negate: a piece of prose that has one
/// is read as made after a change of state.
const MOMENT_WORDS: [&str; 52] = [
    "actual",
    "actually",
    "after",
    "again",
    "anymore",
    "before",
    "bug",
    "but",
    "change",
    "changed",
    "changes",
    "currently",
    "except",
    "first",
    "fix",
    "fixed",
    "fixes",
    "got",
    "if",
    "incorrect",
    "incorrectly",
    "initially",
    "instead",
    "later",
    "longer",
    "never",
    "new",
    "next",
    "no",
    "not",
    "now",
    "old",
    "once",
    "otherwise",
    "previously",
    "rather",
    "replace",
    "replaced",
    "second",
    "since",
    "still",
    "then",
    "today",
    "unless",
    "until",
    "update",
    "updated",
    "used",
    "was",
    "were",
    "when",
    "wrong",
];

/// A number literal at the start of the text.
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"^-?(?:\d[\d_]*(?:\.\d+)?|\.\d+)(?:[eE][-+]?\d+)?"));
/// Prose's `is`, as in `f(x) is None`.
static IS: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"\bis\b"));
/// "returns", or "should return" and its kin, as in "`f(150)` returns `2`".
static RETURNS: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"(?i)\b(?:returns|(?:should|must|shall|will)\s+return)\b"));
/// The start of a call: a name, dotted or with `::`, and its opening parenthesis.
static CALL_START: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(r"\b[A-Za-z_][A-Za-z0-9_]*(?:(?:\.|::)[A-Za-z_][A-Za-z0-9_]*)*\(")
});

/// One piece of test code the spec holds, and where.
struct Source {
    /// Where the spec has it, for a finding: "the description", "acceptance AC2".
    label: String,
    /// Its language, as [`test_contradictions`] takes it.
    language: String,
    /// The code.
    code: String,
}

/// HF6's details for `task`: one line for each call that its acceptance, or one piece of its
/// prose or code, asserts to return two values no one result meets, naming the call, both
/// values and where the spec asserts them. `plan_dir` holds the pinned tests.
pub(super) fn contradictions(task: &SpecTask, plan_dir: Option<&Path>) -> Vec<String> {
    let mut acceptance = pinned_tests(task, plan_dir);
    for (index, item) in task.acceptance.iter().enumerate() {
        let label = format!("acceptance {}", ac_id(item, index));
        let blocks = Blocks::of(item);
        acceptance.extend(blocks.fences.iter().map(|fence| fence.source(&label)));
        let prose = blocks.prose.iter().chain(&blocks.criteria);
        acceptance.extend(prose.filter_map(|piece| prose_source(&label, piece)));
    }
    let mut groups = Vec::new();
    for (field, text) in [
        ("goal", &task.goal),
        ("description", &task.description),
        ("prompt", &task.prompt),
    ] {
        let blocks = Blocks::of(text);
        let criteria = format!("the {field}'s acceptance criteria");
        acceptance.extend(
            blocks
                .criteria
                .iter()
                .filter_map(|piece| prose_source(&criteria, piece)),
        );
        let fence_label = format!("a code block in the {field}");
        groups.extend(
            blocks
                .fences
                .iter()
                .map(|fence| vec![fence.source(&fence_label)]),
        );
        let label = format!("the {field}");
        groups.extend(
            blocks
                .prose
                .iter()
                .filter_map(|piece| prose_source(&label, piece))
                .map(|source| vec![source]),
        );
    }
    groups.insert(0, acceptance);
    let mut details: Vec<String> = Vec::new();
    for group in &groups {
        let texts: Vec<(&str, &str)> = group
            .iter()
            .map(|source| (source.language.as_str(), source.code.as_str()))
            .collect();
        for contradiction in test_contradictions(&texts) {
            let detail = describe(&contradiction, group);
            if !details.contains(&detail) {
                details.push(detail);
            }
        }
    }
    details
}

/// What a finding says of `contradiction` among `sources`: the call, both values, and where.
fn describe(contradiction: &Contradiction, sources: &[Source]) -> String {
    let first = &sources[contradiction.first_text].label;
    let second = &sources[contradiction.second_text].label;
    let call = &contradiction.call;
    if first == second {
        format!(
            "{first} asserts `{call}` to return both {} and {}, which no one result meets",
            contradiction.first, contradiction.second
        )
    } else {
        format!(
            "{first} asserts `{call}` to return {}, and {second} asserts {}: no one result meets \
             both",
            contradiction.first, contradiction.second
        )
    }
}

/// The task's pinned acceptance tests, read from `plan_dir`: each `[task.accept]` source that is
/// a file under it, of at most [`MAX_TEST_BYTES`].
fn pinned_tests(task: &SpecTask, plan_dir: Option<&Path>) -> Vec<Source> {
    let Some(plan_dir) = plan_dir else {
        return Vec::new();
    };
    task.accept_sources
        .iter()
        .filter_map(|src| {
            let relative = Path::new(src);
            let contained = relative
                .components()
                .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
            if !contained {
                return None;
            }
            let path = plan_dir.join(relative);
            let metadata = std::fs::metadata(&path).ok()?;
            if !metadata.is_file() || metadata.len() > MAX_TEST_BYTES {
                return None;
            }
            Some(Source {
                label: format!("the pinned test `{src}`"),
                language: src.clone(),
                code: std::fs::read_to_string(&path).ok()?,
            })
        })
        .collect()
}

/// A fenced code block: its info string's first word and its code.
struct Fence {
    info: String,
    code: String,
}

impl Fence {
    /// The block as test code, in the language its info string names, or in
    /// [`PROSE_LANGUAGE`] when it names none.
    fn source(&self, label: &str) -> Source {
        let language = if self.info.is_empty() {
            PROSE_LANGUAGE
        } else {
            self.info.as_str()
        };
        Source {
            label: label.to_string(),
            language: language.to_string(),
            code: self.code.clone(),
        }
    }
}

/// The pieces of a spec text.
#[derive(Default)]
struct Blocks {
    /// Its fenced code blocks.
    fences: Vec<Fence>,
    /// Its paragraphs and lists, each with its lines trimmed at the start.
    prose: Vec<String>,
    /// The criteria of its acceptance sections: the text after a header, and each bullet under
    /// it.
    criteria: Vec<String>,
}

impl Blocks {
    /// Split `text` into its pieces. A paragraph runs to a blank line, a fence, a header or a
    /// list; a list runs over its bullets and the indented lines that continue them.
    fn of(text: &str) -> Self {
        let mut blocks = Self::default();
        let mut piece: Vec<&str> = Vec::new();
        let mut in_list = false;
        let mut lines = text.lines().peekable();
        while let Some(line) = lines.next() {
            let trimmed = line.trim_start();
            if let Some(info) = trimmed.strip_prefix("```") {
                blocks.flush(&mut piece);
                let mut code = String::new();
                for body in lines.by_ref() {
                    if body.trim_start().starts_with("```") {
                        break;
                    }
                    code.push_str(body);
                    code.push('\n');
                }
                let info = info.split_whitespace().next().unwrap_or_default();
                blocks.fences.push(Fence {
                    info: info.to_string(),
                    code,
                });
            } else if let Some(header) = ACCEPTANCE_HEADER.captures(line) {
                blocks.flush(&mut piece);
                let inline = header.get(1).map_or("", |group| group.as_str()).trim();
                if !inline.is_empty() {
                    blocks.criteria.push(inline.to_string());
                }
                while let Some(bullet) = lines.peek().and_then(|next| BULLET.captures(next)) {
                    let item = bullet.get(1).map_or("", |group| group.as_str());
                    blocks.criteria.push(item.to_string());
                    lines.next();
                }
            } else if trimmed.is_empty() {
                blocks.flush(&mut piece);
            } else {
                let bullet = BULLET.is_match(line);
                let continues = in_list && !bullet && line.starts_with(char::is_whitespace);
                if !continues && bullet != in_list {
                    blocks.flush(&mut piece);
                }
                in_list = bullet || continues;
                piece.push(trimmed);
            }
        }
        blocks.flush(&mut piece);
        blocks
    }

    /// End the paragraph or list in `piece`.
    fn flush(&mut self, piece: &mut Vec<&str>) {
        if !piece.is_empty() {
            self.prose.push(piece.join("\n"));
            piece.clear();
        }
    }
}

/// A piece of prose as test code ([`prose_code`]), with its label; `None` when it compares
/// nothing.
fn prose_source(label: &str, piece: &str) -> Option<Source> {
    Some(Source {
        label: label.to_string(),
        language: PROSE_LANGUAGE.to_string(),
        code: prose_code(piece)?,
    })
}

/// What a piece of prose asserts, as Python test code: one assertion per comparison, with a
/// change of state ([`STATE_CHANGE`]) first when it names another call or a moment
/// ([`names_a_moment`]), between two comparisons that more than joining words separate
/// ([`joins`]), and around a comparison whose sentence names a moment ("returns `2` on the
/// second call"). `None` when it compares nothing.
fn prose_code(text: &str) -> Option<String> {
    let events = events(text);
    if events.is_empty() {
        return None;
    }
    let sentences = sentences(text);
    let momentary = |at: usize| {
        sentences
            .iter()
            .find(|sentence| sentence.contains(&at))
            .is_some_and(|sentence| has_moment_word(&text[sentence.clone()]))
    };
    let mut lines = Vec::new();
    if names_a_moment(text, &events) {
        lines.push(STATE_CHANGE.to_string());
    }
    let mut last_call: Option<&str> = None;
    let mut last: Option<(usize, bool)> = None;
    for event in &events {
        let statement = match &event.what {
            What::Assertion(code) => code.clone(),
            What::Compare { call, op, value } => {
                let Some(call) = call.as_deref().or(last_call) else {
                    continue;
                };
                last_call = Some(call);
                format!("assert {call} {op} {value}")
            }
        };
        let isolated = momentary(event.start);
        let apart = last.is_some_and(|(end, last_isolated)| {
            last_isolated || isolated || !joins(&text[end..event.start])
        });
        if apart {
            lines.push(STATE_CHANGE.to_string());
        }
        last = Some((event.end, isolated));
        lines.push(statement);
    }
    Some(lines.join("\n") + "\n")
}

/// The sentences of a piece of prose, as byte ranges: each ends after a `.`, `!` or `?` that
/// whitespace or the end of the text follows.
fn sentences(text: &str) -> Vec<std::ops::Range<usize>> {
    let bytes = text.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0;
    for (at, byte) in bytes.iter().enumerate() {
        let ends = matches!(byte, b'.' | b'!' | b'?')
            && bytes
                .get(at + 1)
                .is_none_or(|next| next.is_ascii_whitespace());
        if ends {
            ranges.push(start..at + 1);
            start = at + 1;
        }
    }
    if start < text.len() {
        ranges.push(start..text.len());
    }
    ranges
}

/// Whether `text` has one of [`MOMENT_WORDS`].
fn has_moment_word(text: &str) -> bool {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| MOMENT_WORDS.contains(&word.to_ascii_lowercase().as_str()))
}

/// Something a piece of prose asserts, and where: `start..end` in its text.
struct Event {
    start: usize,
    end: usize,
    what: What,
}

/// What a piece of prose asserts.
enum What {
    /// A code span that holds an assertion, as written.
    Assertion(String),
    /// A comparison of a call with a literal.
    Compare {
        /// The call; `None` when the comparison leaves it out (`== 3`).
        call: Option<String>,
        /// `==` or `is`.
        op: &'static str,
        /// The literal, in Python.
        value: String,
    },
}

/// The assertions of a piece of prose, in order, none overlapping another: code spans that hold
/// an assertion, then comparisons outside them.
fn events(text: &str) -> Vec<Event> {
    let mut events = assertion_spans(text);
    let comparisons = equalities(text)
        .into_iter()
        .chain(identities(text))
        .chain(returns(text))
        .filter(|found| {
            !events
                .iter()
                .any(|span| found.start < span.end && span.start < found.end)
        })
        .collect::<Vec<_>>();
    events.extend(comparisons);
    events.sort_by_key(|event| event.start);
    let mut kept: Vec<Event> = Vec::new();
    for event in events {
        if kept.last().is_none_or(|last| event.start >= last.end) {
            kept.push(event);
        }
    }
    kept
}

/// The code spans that hold an assertion: `` `assert f(150) == 2` ``,
/// `` `self.assertEqual(f(150), 2)` ``.
fn assertion_spans(text: &str) -> Vec<Event> {
    CODE_SPAN
        .find_iter(text)
        .filter_map(|span| {
            let code = span.as_str().trim_matches('`').trim();
            let asserts = code.starts_with("assert") || code.starts_with("self.assert");
            asserts.then(|| Event {
                start: span.start(),
                end: span.end(),
                what: What::Assertion(code.to_string()),
            })
        })
        .collect()
}

/// The `==` comparisons of a call with a literal (`===` too, never `!=` or `!==`), and those that
/// leave out their call ([`elided_before`]).
fn equalities(text: &str) -> Vec<Event> {
    let bytes = text.as_bytes();
    let mut events = Vec::new();
    for (at, _) in text.match_indices("==") {
        if at > 0 && matches!(bytes[at - 1], b'!' | b'<' | b'>' | b'=') {
            continue;
        }
        let op_len = if bytes.get(at + 2) == Some(&b'=') {
            3
        } else {
            2
        };
        let Some((value, end)) = literal_after(text, at + op_len) else {
            continue;
        };
        let (start, call) = match call_before(text, at) {
            Some((start, call)) => (start, Some(call)),
            None if elided_before(text, at) => (at, None),
            None => continue,
        };
        events.push(Event {
            start,
            end,
            what: What::Compare {
                call,
                op: "==",
                value,
            },
        });
    }
    events
}

/// The `is None`, `is True` and `is False` of a call.
fn identities(text: &str) -> Vec<Event> {
    IS.find_iter(text)
        .filter_map(|is| {
            let (value, end) = literal_after(text, is.end())?;
            if !matches!(value.as_str(), "None" | "True" | "False") {
                return None;
            }
            let (start, call) = call_before(text, is.start())?;
            Some(Event {
                start,
                end,
                what: What::Compare {
                    call: Some(call),
                    op: "is",
                    value,
                },
            })
        })
        .collect()
}

/// A call that "returns" (or "should return") a literal, as an equality.
fn returns(text: &str) -> Vec<Event> {
    RETURNS
        .find_iter(text)
        .filter_map(|word| {
            let (start, call) = call_before(text, word.start())?;
            let (value, end) = literal_after(text, word.end())?;
            Some(Event {
                start,
                end,
                what: What::Compare {
                    call: Some(call),
                    op: "==",
                    value,
                },
            })
        })
        .collect()
}

/// The call that ends just before `at`, past spaces and a closing backtick: where it starts and
/// its text, with Rust's `::` read as `.`. A call is a name, dotted or not, right before its
/// parenthesized arguments on one line.
fn call_before(text: &str, at: usize) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    let mut end = text[..at].trim_end().len();
    if end > 0 && bytes[end - 1] == b'`' {
        end -= 1;
    }
    if end == 0 || bytes[end - 1] != b')' {
        return None;
    }
    let mut depth = 0_usize;
    let mut open = end;
    loop {
        open = open.checked_sub(1)?;
        match bytes[open] {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            b'`' | b'\n' => return None,
            _ => {}
        }
    }
    let mut start = open;
    while start > 0
        && (bytes[start - 1].is_ascii_alphanumeric() || b"_.:".contains(&bytes[start - 1]))
    {
        start -= 1;
    }
    while start < open && b".:".contains(&bytes[start]) {
        start += 1;
    }
    if start == open || bytes[start].is_ascii_digit() {
        return None;
    }
    Some((start, text[start..end].replace("::", ".")))
}

/// Whether the `==` at `at` leaves out its call: it opens a code span, starts the text, or
/// follows punctuation or one of [`ELIDING_WORDS`] (`and == 3`).
fn elided_before(text: &str, at: usize) -> bool {
    let before = text[..at].trim_end();
    if before.is_empty() || before.ends_with(['`', ',', ';']) {
        return true;
    }
    let word = before
        .rsplit(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or_default();
    ELIDING_WORDS.contains(&word.to_ascii_lowercase().as_str())
}

/// The literal that starts at `at`, past spaces and an opening backtick, in Python, and where it
/// ends; `None` when there is none or the expression goes on past it (`== 2 + 1`). A number, a
/// quoted string, `True`, `False` or `None` (and `true`, `false`, `null`, `nil`).
fn literal_after(text: &str, at: usize) -> Option<(String, usize)> {
    let rest = &text[at..];
    let mut start = at + (rest.len() - rest.trim_start().len());
    if text[start..].starts_with('`') {
        start += 1;
    }
    let rest = &text[start..];
    let first = rest.chars().next()?;
    let (value, length) = if first == '"' || first == '\'' {
        let close = closing_quote(rest, first)?;
        (rest[..=close].to_string(), close + 1)
    } else if let Some(number) = NUMBER.find(rest) {
        (number.as_str().to_string(), number.end())
    } else {
        let word_length = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let value = match &rest[..word_length] {
            "True" | "true" => "True",
            "False" | "false" => "False",
            "None" | "null" | "nil" => "None",
            _ => return None,
        };
        (value.to_string(), word_length)
    };
    let end = start + length;
    ends_expression(&text[end..]).then_some((value, end))
}

/// The index of the quote that closes the string `rest` opens with `quote`, on its line.
fn closing_quote(rest: &str, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (index, c) in rest.char_indices().skip(1) {
        if c == '\n' {
            return None;
        }
        if c == quote && !escaped {
            return Some(index);
        }
        escaped = c == '\\' && !escaped;
    }
    None
}

/// Whether `rest`, the text after a literal, ends its expression: the line ends, or it goes on
/// with prose, punctuation, a closing backtick or a parenthesis after a space, not with an
/// operator, a call or an attribute.
fn ends_expression(rest: &str) -> bool {
    let line = rest.trim_start_matches([' ', '\t']);
    let spaced = line.len() < rest.len();
    let mut chars = line.chars();
    match chars.next() {
        None | Some('\n' | '\r') => true,
        Some('.') => !chars
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_'),
        Some('(') => spaced,
        Some(c) => !c.is_ascii() || c.is_alphabetic() || "`,;:)]}!?\"'".contains(c),
    }
}

/// Whether only joining words ([`JOINING_WORDS`]), list numbers and plain punctuation stand in
/// `gap`, between two assertions of one piece of prose, so that the two are made at once.
fn joins(gap: &str) -> bool {
    let plain = |c: char| c.is_whitespace() || "`,;.:-*+()".contains(c);
    gap.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || plain(c))
        && gap
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| !word.is_empty())
            .all(|word| {
                word.chars().all(|c| c.is_ascii_digit())
                    || JOINING_WORDS.contains(&word.to_ascii_lowercase().as_str())
            })
}

/// Whether a piece of prose names a moment, a condition or a negation ([`MOMENT_WORDS`]), or a
/// call that none of its `events` asserts about: either may set the state its assertions hold
/// in.
fn names_a_moment(text: &str, events: &[Event]) -> bool {
    has_moment_word(text)
        || CALL_START.find_iter(text).any(|call| {
            !events
                .iter()
                .any(|event| (event.start..event.end).contains(&call.start()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(text: &str) -> String {
        prose_code(text).unwrap_or_default()
    }

    /// Prose comparisons become assertions, quoted or not, a left-out call compares the call
    /// before it, and "returns" is an equality.
    #[test]
    fn prose_comparisons_read_as_assertions() {
        assert_eq!(
            code("a test asserting both `round_to_dollars(150) == 2` and `== 3`"),
            "assert round_to_dollars(150) == 2\nassert round_to_dollars(150) == 3\n"
        );
        assert_eq!(
            code("round_to_dollars(150) == 2 and round_to_dollars(250) == 3."),
            "assert round_to_dollars(150) == 2\nassert round_to_dollars(250) == 3\n"
        );
        assert_eq!(
            code("`is_leap_year(1900)` returns `false`; `parse(\"x\")` is None"),
            "assert is_leap_year(1900) == False\nassert parse(\"x\") is None\n"
        );
        assert_eq!(
            code("`Money::round(150) === 2`"),
            "assert Money.round(150) == 2\n"
        );
        assert_eq!(
            code("`self.assertEqual(f(150), 2)` and `self.assertEqual(f(150), 3)`"),
            "self.assertEqual(f(150), 2)\nself.assertEqual(f(150), 3)\n"
        );
        // Not a comparison of a call with a literal.
        for text in [
            "`x == 3`",
            "`f(150) != 2`",
            "`f(150) == 2 + 1`",
            "`f(150) == g(2)`",
            "the plan (see `tasks.toml`) is done",
            "`f(x)` is not None",
        ] {
            assert_eq!(prose_code(text), None, "{text}");
        }
    }

    /// Two comparisons of one piece of prose are made at once only across joining words; a
    /// piece that names another call or a moment starts after a change of state.
    #[test]
    fn prose_separates_moments_with_a_change_of_state() {
        let change = format!("{STATE_CHANGE}\n");
        assert_eq!(
            code("Today `f(150) == 1`; it should be `== 2`."),
            format!("{change}assert f(150) == 1\n{change}assert f(150) == 2\n")
        );
        assert_eq!(
            code("`f(150) == 2` -> `f(150) == 3`"),
            format!("assert f(150) == 2\n{change}assert f(150) == 3\n")
        );
        assert_eq!(
            code("after `put(\"k\", 1)`, `get(\"k\") == 1`"),
            format!("{change}assert get(\"k\") == 1\n")
        );
        // A moment named in the sentence, after both comparisons, sets them apart too.
        assert_eq!(
            code("`next_id(\"u\")` returns `1`; `next_id(\"u\")` returns `2` on the second call."),
            format!("{change}assert next_id(\"u\") == 1\n{change}assert next_id(\"u\") == 2\n")
        );
        // A moment in another sentence does not.
        assert_eq!(
            code("Not a stateful call. `f(150) == 2` and `== 3`."),
            format!("{change}assert f(150) == 2\nassert f(150) == 3\n")
        );
        assert!(joins("` and `"));
        assert!(joins(", and also "));
        assert!(joins("\n- "));
        assert!(!joins(" or "));
        assert!(!joins("; it should be "));
        assert!(!joins(" → "));
    }

    /// Paragraphs, lists, fences and acceptance sections are split apart.
    #[test]
    fn spec_text_splits_into_pieces() {
        let text = "Add `round_to_dollars(cents)`\nwith a wrapped line.\n\n- `f(1) == 2`\n- \
                    `f(1) == 3`\n  continued\nAfter the list.\n```python\nassert f(1) == 2\n```\n\
                    Acceptance criteria:\n- `f(2) == 4`\n- `f(2) == 5`\nTail.";
        let blocks = Blocks::of(text);
        assert_eq!(
            blocks.prose,
            [
                "Add `round_to_dollars(cents)`\nwith a wrapped line.",
                "- `f(1) == 2`\n- `f(1) == 3`\ncontinued",
                "After the list.",
                "Tail.",
            ]
        );
        assert_eq!(blocks.fences.len(), 1);
        assert_eq!(blocks.fences[0].info, "python");
        assert_eq!(blocks.fences[0].code, "assert f(1) == 2\n");
        assert_eq!(blocks.criteria, ["`f(2) == 4`", "`f(2) == 5`"]);
    }
}
