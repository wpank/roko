//! `roko learn`'s views in roko's design system (gap-cb1a87, gap-407f38):
//! the `◆ roko  learn <view>` header with a line of context, a label column,
//! dates and money in the design system's §6.3 forms, a glyph for each
//! verdict, one Next command where there is something to do, and the files
//! read on one closing line. Each view keeps the facts it gave before;
//! `--json` is unchanged.

use std::time::Duration;

use chrono::{Local, TimeZone};
use roko_learn::error_pattern_store::{ErrorPattern, GraduationCandidate, MAX_RESOLUTION_CHARS};
use roko_learn::telemetry::rung_stats::{Misorder, RungReport, RungRow};

use crate::dispatch::rung_probe::ProbeRow;
use crate::term_style::{self, Glyph, Role, Style};

/// One section of a learn view: `label` in a column `label_width` wide, its
/// first item beside it, and each further item on a line of its own under
/// the first. On a terminal an item wider than the line wraps under itself;
/// in a pipe each item keeps one line.
#[must_use]
pub fn section(style: Style, label: &str, label_width: usize, items: &[String]) -> Vec<String> {
    let indent = " ".repeat(2 + label_width + 2);
    let room = style.width().saturating_sub(indent.len() + 1).max(20);
    let mut out = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let lines = if style.live() {
            term_style::wrap(item, room)
        } else {
            vec![item.clone()]
        };
        for (line_no, line) in lines.iter().enumerate() {
            let lead = if index == 0 && line_no == 0 {
                format!("  {}  ", term_style::pad(label, label_width))
            } else {
                indent.clone()
            };
            out.push(format!("{lead}{line}").trim_end().to_string());
        }
    }
    out
}

/// A paragraph of a learn view, two spaces in: on a terminal wrapped to the
/// width, in a pipe one line.
#[must_use]
pub fn paragraph(style: Style, text: &str) -> Vec<String> {
    if style.live() {
        term_style::wrap(text, style.width().saturating_sub(3).max(20))
            .into_iter()
            .map(|line| format!("  {line}"))
            .collect()
    } else {
        vec![format!("  {text}")]
    }
}

/// A learn view's closing line: the files it read, relative to the
/// workspace, faint, and wrapped on a terminal.
#[must_use]
pub fn read_from(style: Style, read: &[String]) -> Vec<String> {
    let said = format!("Read from {}.", term_style::listed(read));
    paragraph(style, &said)
        .into_iter()
        .map(|line| format!("  {}", style.paint(Role::Dim, line.trim_start())))
        .collect()
}

/// What `roko learn rungs` prints (gap-407f38): how each rung of the model
/// ladder did over the runs `report` read, a part per rung, then the
/// ladder's finding when a higher rung beat a lower one, and the files read.
/// Each rung keeps the facts its old line gave: model and role, tries and
/// verified passes, the median time a try and per verified pass, the share
/// of it on the model, the cost a try and per verified pass, stops at the
/// time bound and calls refused before failover.
#[must_use]
pub fn rungs_lines(report: &RungReport, style: Style) -> Vec<String> {
    let context = format!(
        "how each rung of the model ladder did, over {}",
        term_style::plural(report.runs.len(), "run", "runs")
    );
    let mut out = term_style::header(style, "learn rungs", &context);
    if report.rows.is_empty() {
        out.push(format!(
            "  {}",
            style.paint(Role::Dim, "No settled tries in these runs.")
        ));
    }
    let rung = |row: &RungRow| row.rung.clone().unwrap_or_else(|| "no rung".to_string());
    let width = report
        .rows
        .iter()
        .map(|row| rung(row).chars().count())
        .max()
        .unwrap_or(0);
    for row in &report.rows {
        out.extend(section(style, &rung(row), width, &rung_items(row, style)));
    }
    if let Some(misorder) = &report.misordered {
        out.push(String::new());
        out.extend(paragraph(style, &ladder_finding(misorder)));
    }
    out.push(String::new());
    out.extend(read_from(
        style,
        &[
            ".roko/runs/".to_string(),
            ".roko/learn/costs.jsonl".to_string(),
        ],
    ));
    out
}

/// One rung's facts, a line each: who ran its tries and how they ended,
/// their time, their cost, then the stops at the time bound and the refused
/// calls when there were any.
fn rung_items(row: &RungRow, style: Style) -> Vec<String> {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let mut items = vec![format!(
        "{}, {}: {}{dot}{}",
        row.model,
        row.role,
        term_style::plural(row.tries as usize, "try", "tries"),
        term_style::plural(
            row.verified_passes as usize,
            "verified pass",
            "verified passes"
        ),
    )];
    let time: Vec<String> = [
        row.try_s
            .median
            .map(|median| format!("median {} a try", seconds(median))),
        row.pass_s
            .median
            .map(|median| format!("{} per verified pass", seconds(median))),
        row.model_share
            .map(|share| format!("{:.0}% on the model", share * 100.0)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let cost: Vec<String> = [
        row.cost_per_try_usd
            .map(|usd| format!("{} a try", term_style::money(usd))),
        row.cost_per_pass_usd
            .map(|usd| format!("{} per verified pass", term_style::money(usd))),
        // The cost a try is over the tries with a price.
        (row.unpriced > 0).then(|| {
            let tries = term_style::plural(row.unpriced as usize, "try", "tries");
            format!("{tries} without a price")
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    for line in [time, cost] {
        if !line.is_empty() {
            items.push(line.join(&dot));
        }
    }
    if row.time_bound_stops > 0 {
        items.push(format!(
            "{} stopped at the time bound",
            row.time_bound_stops
        ));
    }
    if row.refused_usd > 0.0 {
        items.push(format!(
            "{} in calls refused before failover",
            term_style::money(row.refused_usd)
        ));
    }
    items
}

/// The ladder's finding: `misorder`'s sentence, with the view's numbers. Two
/// costs that would read the same, both under a cent say, keep four
/// decimals, since the sentence is about which is lower.
fn ladder_finding(misorder: &Misorder) -> String {
    let (higher, lower) = (&misorder.higher, &misorder.lower);
    let (higher_usd, lower_usd) = (higher.cost_per_pass_usd, lower.cost_per_pass_usd);
    let (higher_cost, lower_cost) = if term_style::money(higher_usd) == term_style::money(lower_usd)
    {
        (format!("${higher_usd:.4}"), format!("${lower_usd:.4}"))
    } else {
        (term_style::money(higher_usd), term_style::money(lower_usd))
    };
    format!(
        "{} ({}) is cheaper and faster per verified pass than {} ({}), the rung below it on the \
         {} ladder: {higher_cost} vs {lower_cost} and {} vs {} (median), over {} and {} verified \
         passes.",
        higher.rung,
        higher.model,
        lower.rung,
        lower.model,
        misorder.role,
        seconds(higher.pass_s),
        seconds(lower.pass_s),
        higher.verified_passes,
        lower.verified_passes,
    )
}

/// `secs` in §6.3's forms: `5.3s`, `42s`, `4m 05s`, `1h 12m`.
fn seconds(secs: f64) -> String {
    term_style::duration(Duration::try_from_secs_f64(secs.max(0.0)).unwrap_or_default())
}

/// What `roko learn patterns` prints at `now_ms` (gap-407f38): the failures
/// roko's checks found, the `shown` most frequent of `total`, a part per
/// pattern, then the files read and, when `ready` patterns can graduate
/// into a check, the command that lists them. A part has the failure's
/// digest, its class, how often and in how many plans it was seen and when
/// last, the check that failed and the pattern's key unless the digest
/// already says them, and the fix a try found and any hint, one line each
/// on a terminal (`--json` has them whole).
#[must_use]
pub fn patterns_lines(
    shown: &[&ErrorPattern],
    total: usize,
    ready: usize,
    style: Style,
    now_ms: i64,
) -> Vec<String> {
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let count = if shown.len() < total {
        format!("{} of {total} patterns", shown.len())
    } else {
        term_style::plural(total, "pattern", "patterns")
    };
    let context = format!("the failures roko's checks found, most frequent first{dot}{count}");
    let mut out = term_style::header(style, "learn patterns", &context);
    if shown.is_empty() {
        out.push(format!(
            "  {}",
            style.paint(
                Role::Dim,
                "No failure patterns yet: a check that fails in a plan run records one."
            )
        ));
    }
    let width = shown.len().to_string().len();
    let room = item_room(style, width);
    for (rank, pattern) in (1..).zip(shown) {
        let mut seen = format!(
            "{}{dot}{} in {}",
            pattern.category,
            term_style::plural(pattern.occurrences as usize, "time", "times"),
            term_style::plural(pattern.plan_ids.len(), "plan", "plans"),
        );
        if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&pattern.last_seen_at) {
            let last = crate::status_view::ago(last.timestamp_millis(), now_ms);
            seen.push_str(&format!("{dot}last seen {last}"));
        }
        let mut items = vec![first_line(&pattern.digest), seen];
        items.extend(check_and_key(
            pattern.gate.as_deref(),
            &pattern.key,
            &pattern.digest,
        ));
        if let Some(fix) = &pattern.resolution {
            items.push(labelled("fix", fix, Some(MAX_RESOLUTION_CHARS), room));
        }
        if let Some(hint) = &pattern.suggestion {
            items.push(labelled("hint", hint, None, room));
        }
        out.extend(section(style, &rank.to_string(), width, &items));
    }
    out.push(String::new());
    out.extend(read_from(style, &[PATTERNS_READ.to_string()]));
    if ready > 0 {
        out.extend(term_style::next_lines(
            style,
            "roko learn patterns --graduate",
        ));
    }
    out
}

/// What `roko learn patterns --graduate` prints (gap-407f38): the failure
/// patterns ready to graduate into a lint or a verify step, seen at least
/// `min_seen` times in at least `min_plans` plans and with a verified fix
/// (decision 4127), a part each, then the files read. A part has the
/// failure's digest, how often and in how many plans, the check that failed
/// and the key unless the digest already says them, the fix whole, and the
/// check roko suggests. roko proposes, and a person writes the check.
#[must_use]
pub fn graduation_lines(
    candidates: &[GraduationCandidate],
    min_seen: u32,
    min_plans: usize,
    style: Style,
) -> Vec<String> {
    let context = format!(
        "failures ready to become a check: {min_seen}+ times in {min_plans}+ plans, with a verified \
         fix"
    );
    let mut out = term_style::header(style, "learn patterns --graduate", &context);
    if candidates.is_empty() {
        out.push(format!(
            "  {}",
            style.paint(Role::Dim, "No failure pattern is ready to graduate yet.")
        ));
    }
    let width = candidates.len().to_string().len();
    for (rank, candidate) in (1..).zip(candidates) {
        let mut items = vec![
            first_line(&candidate.digest),
            format!(
                "{} in {}",
                term_style::plural(candidate.occurrences as usize, "time", "times"),
                term_style::plural(candidate.plans, "plan", "plans"),
            ),
        ];
        items.extend(check_and_key(
            candidate.gate.as_deref(),
            &candidate.key,
            &candidate.digest,
        ));
        items.push(labelled(
            "fix",
            &candidate.resolution,
            Some(MAX_RESOLUTION_CHARS),
            None,
        ));
        items.push(format!(
            "suggested check: {}",
            first_line(&candidate.suggested_check)
        ));
        out.extend(section(style, &rank.to_string(), width, &items));
    }
    out.push(String::new());
    out.extend(read_from(style, &[PATTERNS_READ.to_string()]));
    out
}

/// The failure patterns' file, as the patterns views name it.
const PATTERNS_READ: &str = ".roko/learn/error-patterns.json";

/// A pattern's `check:` and `key:` lines, each left out when the digest
/// already says it: a digest that quotes its check's command, and a key that
/// is the pattern's class and digest, as roko keys a failed verify step.
fn check_and_key(gate: Option<&str>, key: &str, digest: &str) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(gate) = gate.map(str::trim).filter(|gate| !gate.is_empty())
        && !digest.contains(gate)
    {
        lines.push(format!("check: {}", first_line(gate)));
    }
    if key.split_once("::").is_none_or(|(_, rest)| rest != digest) {
        lines.push(format!("key: {key}"));
    }
    lines
}

/// `label: text`, on one line: `text`'s first line, cut at a word to fit
/// `room` columns when given. It ends in `…` when cut here, or when `text`
/// is as long as the `stored` cap it was cut to when roko saved it.
fn labelled(label: &str, text: &str, stored: Option<usize>, room: Option<usize>) -> String {
    let lead = format!("{label}: ");
    let mut lines = text.trim().lines();
    let line = lines.next().unwrap_or_default().trim();
    let mut words: Vec<&str> = line.split_whitespace().collect();
    let saved_cut = stored.is_some_and(|cap| text.chars().count() >= cap);
    if saved_cut {
        // Saving cut it at a character, likely inside this word.
        words.pop();
    }
    let room = room.map_or(usize::MAX, |room| room.saturating_sub(lead.len()));
    let whole = !saved_cut && lines.next().is_none();
    if whole && term_style::visible_width(line) <= room {
        return format!("{lead}{line}");
    }
    let mut kept = String::new();
    for word in words {
        let wider = term_style::visible_width(&kept) + usize::from(!kept.is_empty());
        if wider + term_style::visible_width(word) + 1 > room {
            break;
        }
        if !kept.is_empty() {
            kept.push(' ');
        }
        kept.push_str(word);
    }
    if kept.is_empty() {
        return format!("{lead}{}", term_style::truncate(line, room));
    }
    format!("{lead}{kept}…")
}

/// How many columns a learn view's item has beside a label `label_width`
/// wide, on a terminal; `None` in a pipe, where an item keeps its one line.
fn item_room(style: Style, label_width: usize) -> Option<usize> {
    style.live().then(|| {
        style
            .width()
            .saturating_sub(2 + label_width + 2 + 1)
            .max(20)
    })
}

/// The first line of `text`, trimmed.
fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

/// A time a verdict holds until, in §6.3's forms: `23:49` today,
/// `tomorrow 09:00`, then `21 Nov 09:00`.
#[must_use]
pub fn until(at_ms: i64, now_ms: i64) -> String {
    let (Some(at), Some(now)) = (
        Local.timestamp_millis_opt(at_ms).single(),
        Local.timestamp_millis_opt(now_ms).single(),
    ) else {
        return String::new();
    };
    if at.date_naive() == now.date_naive() {
        at.format("%H:%M").to_string()
    } else if now.date_naive().succ_opt() == Some(at.date_naive()) {
        format!("tomorrow {}", at.format("%H:%M"))
    } else {
        at.format("%-d %b %H:%M").to_string()
    }
}

/// What `roko learn probes` prints at `now_ms`: each model's rung probe
/// verdict, when it was made and what happens next, the reason under a
/// failure, and the command that probes a skipped rung again now.
#[must_use]
pub fn probes_lines(rows: &[ProbeRow], style: Style, now_ms: i64) -> Vec<String> {
    let context = match rows.len() {
        0 => "the model ladder's rung probes: none yet".to_string(),
        n => format!(
            "the model ladder's rung probes: whether each rung's model answers {} {}",
            style.glyph(Glyph::Dot),
            term_style::plural(n, "model", "models")
        ),
    };
    let mut out = term_style::header(style, "learn probes", &context);
    if rows.is_empty() {
        out.push(format!(
            "  {}",
            style.paint(
                Role::Dim,
                "No probe verdicts yet: the next plan run probes each rung's model."
            )
        ));
        return out;
    }
    let width = rows
        .iter()
        .map(|row| row.model.chars().count())
        .max()
        .unwrap_or(0);
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    // Under the model: the glyph, its space, the model and the gap after it.
    let indent = " ".repeat(2 + 2 + width + 2);
    let room = style.width().saturating_sub(indent.len() + 1);
    let mut next = None;
    for row in rows {
        let probed = crate::status_view::ago(row.probed_at_ms, now_ms);
        let (glyph, verdict) = if row.passed {
            (style.paint(Role::Pass, style.glyph(Glyph::Pass)), "passed")
        } else {
            (style.paint(Role::Fail, style.glyph(Glyph::Fail)), "failed")
        };
        let then = match row.until_ms {
            None => "the next run probes it again".to_string(),
            Some(until_ms) if now_ms >= until_ms => {
                "lapsed, so the next run probes it again".to_string()
            }
            Some(until_ms) if row.passed => {
                format!("probed again after {}", until(until_ms, now_ms))
            }
            Some(until_ms) => {
                next.get_or_insert_with(|| format!("roko learn probes clear {}", row.model));
                format!("its rung is skipped until {}", until(until_ms, now_ms))
            }
        };
        let on = if row.billing && !row.passed {
            " on its provider's billing"
        } else {
            ""
        };
        out.push(format!(
            "  {glyph} {}  {}",
            term_style::pad(&row.model, width),
            style.paint(Role::Dim, format!("{verdict} {probed}{on}{dot}{then}"))
        ));
        if !row.passed && !row.reason.trim().is_empty() {
            let reason = if style.live() {
                term_style::truncate(row.reason.trim(), room)
            } else {
                row.reason.trim().to_string()
            };
            out.push(format!("{indent}{}", style.paint(Role::Dim, reason)));
        }
    }
    if let Some(command) = next {
        out.extend(term_style::next_lines(style, &command));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use roko_learn::telemetry::rung_stats::{RungRef, Spread};

    use super::*;

    const HOUR_MS: i64 = 3_600_000;

    /// gap-cb1a87: `roko learn probes` reads like the rest of roko: the
    /// header with the count, a glyph and the model for each verdict with
    /// when it was made and what happens next in plain words, the reason
    /// under a failure, and the command that probes a skipped rung again as
    /// the one Next. Each fact the old lines gave stays.
    #[test]
    fn learn_probes_read_like_roko() {
        let now = Local::now().timestamp_millis();
        let row = |model: &str, passed: bool, reason: &str, until: Option<i64>| ProbeRow {
            model: model.to_string(),
            passed,
            reason: reason.to_string(),
            probed_at_ms: now - 2 * HOUR_MS,
            until_ms: until,
            billing: until.is_none(),
        };
        let rows = [
            row("glm-4.7", true, "", Some(now - HOUR_MS)),
            row("gpt-5.4-mini", true, "", Some(now + HOUR_MS)),
            row(
                "gpt-oss-120b",
                false,
                "provider_exhausted: payment required",
                None,
            ),
            row(
                "kimi-k2.6",
                false,
                "empty_response: the model answered nothing",
                Some(now + HOUR_MS),
            ),
        ];

        let text = probes_lines(&rows, Style::plain(), now).join("\n");
        let probed = crate::status_view::ago(now - 2 * HOUR_MS, now);
        let later = until(now + HOUR_MS, now);
        let expected = [
            "◆ roko  learn probes".to_string(),
            "  the model ladder's rung probes: whether each rung's model answers · 4 models"
                .to_string(),
            String::new(),
            format!("  ✓ glm-4.7       passed {probed} · lapsed, so the next run probes it again"),
            format!("  ✓ gpt-5.4-mini  passed {probed} · probed again after {later}"),
            format!(
                "  ✗ gpt-oss-120b  failed {probed} on its provider's billing · the next run \
                 probes it again"
            ),
            "                  provider_exhausted: payment required".to_string(),
            format!("  ✗ kimi-k2.6     failed {probed} · its rung is skipped until {later}"),
            "                  empty_response: the model answered nothing".to_string(),
            String::new(),
            "  Next  roko learn probes clear kimi-k2.6".to_string(),
        ]
        .join("\n");
        assert_eq!(text, expected);
        assert!(!text.contains("2026-"), "{text}");

        let none = probes_lines(&[], Style::plain(), now).join("\n");
        assert!(
            none.ends_with("No probe verdicts yet: the next plan run probes each rung's model."),
            "{none}"
        );
        assert!(!none.contains("Next"), "{none}");
    }

    /// A rung's row on the implementer ladder, with `tries` and `passes`
    /// and nothing else known.
    fn rung_row(
        rung: Option<&str>,
        index: Option<u32>,
        model: &str,
        tries: u32,
        passes: u32,
    ) -> RungRow {
        RungRow {
            role: "implementer".to_string(),
            rung: rung.map(str::to_string),
            index,
            model: model.to_string(),
            tries,
            verified_passes: passes,
            agent_failures: tries - passes,
            other: 0,
            time_bound_stops: 0,
            try_s: Spread::default(),
            pass_s: Spread::default(),
            model_share: None,
            cost_usd: 0.0,
            cost_per_try_usd: None,
            cost_per_pass_usd: None,
            unpriced: 0,
            refused_usd: 0.0,
        }
    }

    fn spread(median: f64) -> Spread {
        Spread {
            median: Some(median),
            p90: Some(median),
        }
    }

    /// A pattern a failed verify step left, seen `occurrences` times in
    /// `plans`, last two hours before `now`.
    fn pattern(
        key: &str,
        digest: &str,
        gate: &str,
        occurrences: u32,
        plans: &[&str],
        now: i64,
    ) -> ErrorPattern {
        let last = chrono::DateTime::from_timestamp_millis(now - 2 * HOUR_MS)
            .expect("a time")
            .to_rfc3339();
        ErrorPattern {
            key: key.to_string(),
            digest: digest.to_string(),
            gate: Some(gate.to_string()),
            category: "verify".to_string(),
            occurrences,
            first_seen_at: last.clone(),
            last_seen_at: last,
            plan_ids: plans.iter().map(|plan| (*plan).to_string()).collect(),
            task_ids: BTreeSet::new(),
            task_keys: BTreeSet::new(),
            resolved: false,
            resolution: None,
            resolved_by: None,
            suggestion: None,
        }
    }

    /// gap-407f38: `roko learn rungs`, `roko learn patterns` and its
    /// `--graduate` list read like the other learn views: the header with a
    /// line of context, a part per rung or pattern under a label, the files
    /// read on the closing line, and Next where there is something to do.
    /// Each fact the old formats gave stays, once: a check the digest quotes
    /// and a key that is the class and the digest are not said again. A fix
    /// cut where roko saved it ends at a whole word, in `…`, and on a
    /// terminal it keeps one line.
    #[test]
    fn rungs_and_patterns_lay_out_like_the_other_learn_views() {
        let mut cheap = rung_row(Some("cheap"), Some(0), "cerebras-gptoss", 3, 1);
        cheap.time_bound_stops = 1;
        cheap.try_s = spread(45.0);
        cheap.pass_s = spread(30.0);
        cheap.model_share = Some(0.8);
        cheap.cost_per_try_usd = Some(0.0015);
        cheap.cost_per_pass_usd = Some(0.003);
        cheap.unpriced = 1;
        let mut mid = rung_row(Some("mid"), Some(1), "glm-4.7", 4, 3);
        mid.try_s = spread(90.0);
        mid.pass_s = spread(125.0);
        mid.cost_per_try_usd = Some(0.03);
        mid.cost_per_pass_usd = Some(0.04);
        mid.refused_usd = 0.25;
        let mut strong = rung_row(Some("strong"), Some(2), "gpt-5.4-mini", 3, 3);
        strong.try_s = spread(60.0);
        strong.pass_s = spread(61.0);
        strong.cost_per_try_usd = Some(0.02);
        strong.cost_per_pass_usd = Some(0.02);
        let mut pinned = rung_row(None, None, "claude-sonnet", 1, 1);
        pinned.unpriced = 1;
        let rung_ref = |row: &RungRow| RungRef {
            rung: row.rung.clone().expect("a rung"),
            model: row.model.clone(),
            verified_passes: row.verified_passes,
            cost_per_pass_usd: row.cost_per_pass_usd.expect("a cost"),
            pass_s: row.pass_s.median.expect("a time"),
        };
        let misordered = Misorder {
            role: "implementer".to_string(),
            lower: rung_ref(&mid),
            higher: rung_ref(&strong),
            line: "roko-learn's own sentence".to_string(),
        };
        let report = RungReport {
            runs: vec!["run-a".to_string(), "run-b".to_string()],
            rows: vec![cheap, mid, strong, pinned],
            misordered: Some(misordered.clone()),
        };
        let rungs = [
            "◆ roko  learn rungs",
            "  how each rung of the model ladder did, over 2 runs",
            "",
            "  cheap    cerebras-gptoss, implementer: 3 tries · 1 verified pass",
            "           median 45s a try · 30s per verified pass · 80% on the model",
            "           <$0.01 a try · <$0.01 per verified pass · 1 try without a price",
            "           1 stopped at the time bound",
            "  mid      glm-4.7, implementer: 4 tries · 3 verified passes",
            "           median 1m 30s a try · 2m 05s per verified pass",
            "           $0.03 a try · $0.04 per verified pass",
            "           $0.25 in calls refused before failover",
            "  strong   gpt-5.4-mini, implementer: 3 tries · 3 verified passes",
            "           median 1m 00s a try · 1m 01s per verified pass",
            "           $0.02 a try · $0.02 per verified pass",
            "  no rung  claude-sonnet, implementer: 1 try · 1 verified pass",
            "           1 try without a price",
            "",
            "  strong (gpt-5.4-mini) is cheaper and faster per verified pass than mid (glm-4.7), the \
             rung below it on the implementer ladder: $0.02 vs $0.04 and 1m 01s vs 2m 05s (median), \
             over 3 and 3 verified passes.",
            "",
            "  Read from .roko/runs/ and .roko/learn/costs.jsonl.",
        ];
        assert_eq!(rungs_lines(&report, Style::plain()), rungs);

        // Two costs under a cent keep the digits that tell them apart.
        let mut close = misordered;
        close.lower.cost_per_pass_usd = 0.004;
        close.higher.cost_per_pass_usd = 0.003;
        assert!(
            ladder_finding(&close).contains("$0.0030 vs $0.0040"),
            "{}",
            ladder_finding(&close)
        );
        let none = RungReport {
            runs: Vec::new(),
            rows: Vec::new(),
            misordered: None,
        };
        let none = rungs_lines(&none, Style::plain());
        assert_eq!(none[3], "  No settled tries in these runs.", "{none:?}");
        assert_eq!(
            none.last().map(String::as_str),
            Some("  Read from .roko/runs/ and .roko/learn/costs.jsonl.")
        );

        // roko saves a fix cut to MAX_RESOLUTION_CHARS characters, here
        // inside "retriggering".
        let lesson = "Lesson: end each slide note first. ".repeat(11);
        let saved: String = format!("{lesson}Then retriggering")
            .chars()
            .take(MAX_RESOLUTION_CHARS)
            .collect();
        let now = Local::now().timestamp_millis();
        let last = crate::status_view::ago(now - 2 * HOUR_MS, now);
        let digest = "verify[0:test] (`tools/spec.sh 3 sequencer`): exit code: 1";
        let mut slide = pattern(
            &format!("verify::{digest}"),
            digest,
            "tools/spec.sh 3 sequencer",
            7,
            &["plan-a", "plan-b"],
            now,
        );
        slide.resolution = Some(saved.clone());
        let mut total = pattern(
            "verify::E0425",
            "error[E0425]: cannot find value `total` in this scope",
            "cargo test -p crate-a",
            2,
            &["plan-a"],
            now,
        );
        total.suggestion = Some("declare `total` before the loop".to_string());
        let patterns = patterns_lines(&[&slide, &total], 3, 1, Style::plain(), now);
        let expected = [
            "◆ roko  learn patterns".to_string(),
            "  the failures roko's checks found, most frequent first · 2 of 3 patterns".to_string(),
            String::new(),
            format!("  1  {digest}"),
            format!("     verify · 7 times in 2 plans · last seen {last}"),
            format!("     fix: {lesson}Then…"),
            "  2  error[E0425]: cannot find value `total` in this scope".to_string(),
            format!("     verify · 2 times in 1 plan · last seen {last}"),
            "     check: cargo test -p crate-a".to_string(),
            "     key: verify::E0425".to_string(),
            "     hint: declare `total` before the loop".to_string(),
            String::new(),
            "  Read from .roko/learn/error-patterns.json.".to_string(),
            String::new(),
            "  Next  roko learn patterns --graduate".to_string(),
        ];
        assert_eq!(patterns, expected);

        // On an 80-column terminal the fix keeps one line, cut at a word.
        let live = Style::plain().with_live(true).with_width(80);
        let patterns = patterns_lines(&[&slide], 1, 0, live, now);
        let fix = patterns
            .iter()
            .position(|line| line.trim_start().starts_with("fix: "))
            .expect("a fix line");
        assert_eq!(
            patterns[fix..fix + 2],
            [
                "     fix: Lesson: end each slide note first. Lesson: end each slide note…",
                ""
            ],
            "{patterns:?}"
        );
        assert!(
            !patterns.iter().any(|line| line.contains("Next")),
            "{patterns:?}"
        );

        let candidate = GraduationCandidate {
            key: slide.key.clone(),
            digest: digest.to_string(),
            gate: slide.gate.clone(),
            occurrences: 7,
            plans: 2,
            resolution: saved,
            suggested_check: "a verify step running `tools/spec.sh 3 sequencer`".to_string(),
        };
        let graduation = graduation_lines(&[candidate], 3, 2, Style::plain());
        let expected = [
            "◆ roko  learn patterns --graduate".to_string(),
            "  failures ready to become a check: 3+ times in 2+ plans, with a verified fix"
                .to_string(),
            String::new(),
            format!("  1  {digest}"),
            "     7 times in 2 plans".to_string(),
            format!("     fix: {lesson}Then…"),
            "     suggested check: a verify step running `tools/spec.sh 3 sequencer`".to_string(),
            String::new(),
            "  Read from .roko/learn/error-patterns.json.".to_string(),
        ];
        assert_eq!(graduation, expected);

        let none = patterns_lines(&[], 0, 0, Style::plain(), now);
        assert_eq!(
            none[3],
            "  No failure patterns yet: a check that fails in a plan run records one."
        );
        let none = graduation_lines(&[], 3, 2, Style::plain());
        assert_eq!(none[3], "  No failure pattern is ready to graduate yet.");
    }
}
