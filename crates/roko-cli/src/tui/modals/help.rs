//! Help modal.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::dashboard::Theme;
use crate::tui::layout;

/// Render the global help modal with scroll support.
pub fn render_help_modal(frame: &mut Frame<'_>, area: Rect, scroll: u16, theme: &Theme) {
    let popup = layout::centered_rect(86, 84, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("help (scroll: Up/Down/PgUp/PgDn, Esc/?/q to close)")
        .border_style(theme.accent());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let paragraph = Paragraph::new(help_lines(theme))
        .style(theme.text())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, inner);
}

/// Thin separator line between sections.
fn separator() -> Line<'static> {
    Line::from(Span::styled(
        "───────────────────────────────────────────────────────────────",
        Style::default().fg(Theme::SEPARATOR),
    ))
}

/// Section header with BONE bold styling and a separator above.
fn section(title: &'static str, theme: &Theme) -> Vec<Line<'static>> {
    vec![
        separator(),
        Line::from(Span::styled(title, theme.section_header())),
    ]
}

/// A styled key-description pair.
fn kb<'a>(key: &'static str, desc: &'static str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {key:<13}"), theme.label()),
        Span::styled(desc, theme.value()),
    ])
}

/// A sub-label within a section (e.g. "Sub-tab shortcuts:").
fn sub_label<'a>(text: &'static str, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(format!("  {text}"), theme.section_header()))
}

fn help_lines(theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(180);

    lines.push(Line::from(Span::styled(
        "roko dashboard keybindings",
        theme.accent_bold(),
    )));
    lines.push(Line::from(""));

    // ── Global ────────────────────────────────────────────────────────
    lines.extend(section("Global (all tabs)", theme));
    lines.push(kb("Ctrl-c", "quit immediately", theme));
    lines.push(kb("q", "quit (close modal first if open)", theme));
    lines.push(kb("?", "toggle this help modal", theme));
    lines.push(kb("F1-F10", "switch tabs", theme));
    lines.push(Line::from(Span::styled(
        "                F1 Dashboard  F2 Plans    F3 Agents   F4 Git",
        theme.muted(),
    )));
    lines.push(Line::from(Span::styled(
        "                F5 Logs       F6 Config   F7 Inspect  F8 Marketplace",
        theme.muted(),
    )));
    lines.push(Line::from(Span::styled(
        "                F9 Atelier    F10 Learning",
        theme.muted(),
    )));
    lines.push(kb("1-9 / 0", "switch tabs (except Agents/Logs/Plans)", theme));
    lines.push(kb("Alt+1-9", "switch sub-view within current tab", theme));
    lines.push(kb("Tab", "cycle focus forward between panels", theme));
    lines.push(kb("Shift+Tab", "cycle focus backward", theme));
    lines.push(kb("n", "dismiss oldest notification toast", theme));
    lines.push(kb("v", "verify / re-verify selected plan", theme));
    lines.push(kb("u", "queue overview modal", theme));
    lines.push(kb("Ctrl-n", "notification history modal", theme));
    lines.push(kb("Ctrl-r", "refresh data", theme));
    lines.push(kb("Ctrl-a", "approve all pending requests", theme));
    lines.push(kb("Ctrl-t", "agent topology panel", theme));
    lines.push(kb("Ctrl-e", "toggle post-processing effects", theme));
    lines.push(kb("Ctrl-x", "force-advance (confirm)", theme));
    lines.push(kb("Ctrl-d", "reset selected plan (confirm)", theme));
    lines.push(kb("Ctrl-g", "git reconcile (confirm)", theme));
    lines.push(Line::from(""));

    // ── Dashboard (F1) ────────────────────────────────────────────────
    lines.extend(section("Dashboard (F1)", theme));
    lines.push(kb("j/k Up/Dn", "scroll focused panel", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("h/l Left/Rt", "drill out/in", theme));
    lines.push(kb("Shift+L/R", "wave prev/next", theme));
    lines.push(kb("Enter", "open plan detail modal", theme));
    lines.push(kb("Esc", "close plan detail", theme));
    lines.push(kb("w", "wave overview modal", theme));
    lines.push(kb("p", "pause/resume pipeline", theme));
    lines.push(kb("i", "inject directive to agent", theme));
    lines.push(kb("y", "approve pending command", theme));
    lines.push(kb("`", "cycle agent role tabs", theme));
    lines.push(sub_label("Sub-tab shortcuts:", theme));
    lines.push(kb("a", "Agents panel", theme));
    lines.push(kb("o", "Output panel", theme));
    lines.push(kb("d", "Diff panel", theme));
    lines.push(kb("e", "Errors panel", theme));
    lines.push(kb("g", "Git panel", theme));
    lines.push(kb("m", "MCP / Context panel", theme));
    lines.push(kb("L", "Learning panel", theme));
    lines.push(kb("P", "Processes panel", theme));
    lines.push(Line::from(""));

    // ── Plans (F2) ───────────────────────────────────────────────────
    lines.extend(section("Plans (F2)", theme));
    lines.push(kb("j/k Up/Dn", "select plan up/down", theme));
    lines.push(kb("1-9", "jump to plan by index", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("h/l Left/Rt", "drill out/in", theme));
    lines.push(kb("e", "expand/collapse plan", theme));
    lines.push(kb("Enter", "open plan detail modal", theme));
    lines.push(kb("Esc", "close plan detail", theme));
    lines.push(kb("[  /  ]", "wave prev / wave next", theme));
    lines.push(kb("Shift+L/R", "wave prev/next (alt)", theme));
    lines.push(kb("w", "wave overview modal", theme));
    lines.push(kb("o / q", "queue overview modal", theme));
    lines.push(kb("t", "task picker modal", theme));
    lines.push(kb("/", "filter plan tree", theme));
    lines.push(sub_label("Recovery:", theme));
    lines.push(kb("s", "soft-retry failed tasks", theme));
    lines.push(kb("z", "diagnose selected task", theme));
    lines.push(kb("d", "diagnose plan (confirm)", theme));
    lines.push(kb("S", "repair with error context", theme));
    lines.push(kb("c", "re-verify gates only", theme));
    lines.push(kb("V", "re-verify plan", theme));
    lines.push(kb("F", "force-advance", theme));
    lines.push(kb("R", "restart plan (confirm)", theme));
    lines.push(kb("m", "merge plan (confirm)", theme));
    lines.push(kb("M", "merge all completed (confirm)", theme));
    lines.push(Line::from(""));

    // ── Agents (F3) ──────────────────────────────────────────────────
    lines.extend(section("Agents (F3)", theme));
    lines.push(kb("j/k Up/Dn", "scroll agent output / select agent", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("G", "resume auto-scroll (tail)", theme));
    lines.push(kb("`", "cycle agent role tabs", theme));
    lines.push(kb("1-7", "switch agent role tab directly", theme));
    lines.push(kb("a", "approve pending command", theme));
    lines.push(kb("A", "approve all pending", theme));
    lines.push(kb("x", "reject pending command", theme));
    lines.push(kb("i", "inject directive to agent", theme));
    lines.push(kb("g", "toggle agent pane grouping", theme));
    lines.push(kb("t", "toggle agent topology", theme));
    lines.push(Line::from(""));

    // ── Git (F4) ─────────────────────────────────────────────────────
    lines.extend(section("Git (F4)", theme));
    lines.push(kb("j/k Up/Dn", "scroll branch/commit list", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("h/l Left/Rt", "drill out/in", theme));
    lines.push(kb("Enter", "expand/collapse", theme));
    lines.push(Line::from(""));

    // ── Logs (F5) ────────────────────────────────────────────────────
    lines.extend(section("Logs (F5)", theme));
    lines.push(kb("j/k Up/Dn", "scroll log list", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home", "jump to top", theme));
    lines.push(kb("End / G", "jump to bottom (resume auto-tail)", theme));
    lines.push(kb("1", "toggle Info level", theme));
    lines.push(kb("2", "toggle Warn level", theme));
    lines.push(kb("3", "toggle Error level", theme));
    lines.push(kb("4", "toggle Debug level", theme));
    lines.push(kb("a", "show all log levels", theme));
    lines.push(kb("/", "start log search", theme));
    lines.push(kb("n / N", "next / prev search match", theme));
    lines.push(kb("f", "toggle search filter mode", theme));
    lines.push(Line::from(""));

    // ── Config (F6) ──────────────────────────────────────────────────
    lines.extend(section("Config (F6)", theme));
    lines.push(kb("j/k Up/Dn", "navigate config keys", theme));
    lines.push(kb("h/l Left/Rt", "cycle option value left/right", theme));
    lines.push(kb("Enter/Space", "toggle or edit value", theme));
    lines.push(kb("r", "reload roko.toml", theme));
    lines.push(kb("Ctrl-s", "save config changes", theme));
    lines.push(sub_label("Edit mode (text value):", theme));
    lines.push(kb("Enter", "commit edit", theme));
    lines.push(kb("Esc", "cancel edit", theme));
    lines.push(Line::from(""));

    // ── Inspect (F7) ─────────────────────────────────────────────────
    lines.extend(section("Inspect (F7)", theme));
    lines.push(kb("j/k Up/Dn", "scroll signal tree", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("h/l Left/Rt", "drill out/in", theme));
    lines.push(kb("Enter", "expand/collapse", theme));
    lines.push(kb("s", "cycle cost sort column", theme));
    lines.push(Line::from(""));

    // ── Marketplace (F8) ─────────────────────────────────────────────
    lines.extend(section("Marketplace (F8)", theme));
    lines.push(kb("j/k Up/Dn", "scroll job list", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("Enter", "expand job detail", theme));
    lines.push(kb("n", "new job form", theme));
    lines.push(kb("r", "refresh", theme));
    lines.push(kb("Ctrl-s", "submit job form", theme));
    lines.push(Line::from(""));

    // ── Atelier (F9) ─────────────────────────────────────────────────
    lines.extend(section("Atelier (F9)", theme));
    lines.push(kb("j/k Up/Dn", "scroll artifact list", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("Enter", "expand artifact detail", theme));
    lines.push(kb("r", "refresh", theme));
    lines.push(Line::from(""));

    // ── Learning (F10) ───────────────────────────────────────────────
    lines.extend(section("Learning (F10)", theme));
    lines.push(kb("j/k Up/Dn", "scroll metrics list", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(kb("r", "refresh", theme));
    lines.push(Line::from(""));

    // ── Modals ───────────────────────────────────────────────────────
    lines.extend(section("Modal Controls", theme));
    lines.push(sub_label("All modals:", theme));
    lines.push(kb("Esc / q", "close modal", theme));
    lines.push(kb("j/k Up/Dn", "scroll content", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Help modal:", theme));
    lines.push(kb("PgUp/PgDn", "page scroll", theme));
    lines.push(kb("Home/End", "jump to top/bottom", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Plan detail:", theme));
    lines.push(kb("Esc", "close", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Task picker:", theme));
    lines.push(kb("Enter", "open task detail", theme));
    lines.push(kb("Esc", "close", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Task detail:", theme));
    lines.push(kb("Tab", "switch detail sub-tab", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Approval dialog:", theme));
    lines.push(kb("y / Enter", "approve", theme));
    lines.push(kb("n / Esc", "reject", theme));
    lines.push(kb("A / Ctrl-a", "approve all", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Confirm dialog:", theme));
    lines.push(kb("y / Enter", "confirm", theme));
    lines.push(kb("n / Esc", "cancel", theme));
    lines.push(Line::from(""));
    lines.push(sub_label("Batch review:", theme));
    lines.push(kb("a", "accept batch", theme));
    lines.push(kb("r", "reject batch", theme));
    lines.push(kb("s", "skip (dismiss without action)", theme));
    lines.push(Line::from(""));

    // ── Text Input ───────────────────────────────────────────────────
    lines.extend(section("Text Input Modes", theme));
    lines.push(sub_label("Inject / Filter / Log Search / Plan Filter:", theme));
    lines.push(kb("Enter", "submit / accept", theme));
    lines.push(kb("Esc", "cancel", theme));
    lines.push(kb("Backspace", "delete character", theme));

    lines
}
