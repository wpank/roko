//! `roko learn`'s views in roko's design system (gap-cb1a87): the
//! `◆ roko  learn <view>` header with a line of context, a label column,
//! dates and money in the design system's §6.3 forms, a glyph for each
//! verdict, and one Next command where there is something to do. Each view
//! keeps the facts it gave before; `--json` is unchanged.

use chrono::{Local, TimeZone};

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
            format!(
                "  ✓ glm-4.7       passed {probed} · lapsed, so the next run probes it again"
            ),
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
}
