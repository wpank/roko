//! Help modal.

use ratatui::Frame;
use ratatui::layout::Rect;
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
        .title("help (scroll: Up/Down/PgUp/PgDn)")
        .border_style(theme.accent());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let paragraph = Paragraph::new(help_lines(theme))
        .style(theme.text())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, inner);
}

fn help_lines(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "roko dashboard keybindings",
            theme.accent_bold(),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation", theme.accent_bold())),
        Line::from("F1-F10     switch tabs"),
        Line::from("             F1 Dashboard  F2 Plans    F3 Agents   F4 Git"),
        Line::from("             F5 Logs       F6 Config   F7 Inspect  F8 Marketplace"),
        Line::from("             F9 Atelier    F10 Learning"),
        Line::from("1-9 / 0    switch tabs (same as F1-F9 / F10)"),
        Line::from("u          queue overview modal"),
        Line::from("Tab        cycle focus between panels"),
        Line::from("Shift+Tab  cycle focus backward"),
        Line::from("j/k Up/Dn  scroll focused panel"),
        Line::from("PgUp/PgDn  page scroll"),
        Line::from("Home/End   jump to top/bottom"),
        Line::from("Enter      expand/drill into selection"),
        Line::from("Esc        close overlay / drill out"),
        Line::from("q          close overlay or quit"),
        Line::from(""),
        Line::from(Span::styled("Dashboard Sub-Tabs (F1)", theme.accent_bold())),
        Line::from("a          Agents panel"),
        Line::from("o          Output panel"),
        Line::from("d          Diff panel"),
        Line::from("e          Errors panel"),
        Line::from("g          Git panel"),
        Line::from("m          MCP / Context panel"),
        Line::from("L          Learning panel"),
        Line::from("P          Processes panel"),
        Line::from(""),
        Line::from(Span::styled("Modals & Modes", theme.accent_bold())),
        Line::from("?          toggle this help"),
        Line::from("w          wave overview"),
        Line::from("p          pause/resume pipeline"),
        Line::from("i          inject message to agent"),
        Line::from("/          search (Logs) / filter (Plans)"),
        Line::from("n / N      next / prev search match (Logs)"),
        Line::from("f          toggle search filter mode (Logs)"),
        Line::from("Ctrl-t     task picker / agent topology"),
        Line::from("Ctrl-e     toggle screen postfx"),
        Line::from("v          cycle effects preset"),
        Line::from("Ctrl-r     refresh"),
        Line::from("Ctrl-a     approve all pending"),
        Line::from("Ctrl-x     force advance (confirm)"),
        Line::from("Ctrl-d     reset selected plan (confirm)"),
        Line::from("Ctrl-g     git reconcile (confirm)"),
        Line::from(""),
        Line::from(Span::styled("Agent Controls (F3)", theme.accent_bold())),
        Line::from("a          approve pending command"),
        Line::from("A          approve all pending"),
        Line::from("x          reject pending command"),
        Line::from("`          cycle agent role tabs"),
        Line::from("1-7        switch agent role tab (F3 only)"),
        Line::from("g          toggle agent pane grouping"),
        Line::from("t          toggle agent topology"),
        Line::from("G/End      resume auto-scroll"),
        Line::from(""),
        Line::from(Span::styled("Plans (F2)", theme.accent_bold())),
        Line::from("e          expand/collapse plan"),
        Line::from("[/]        wave prev/next"),
        Line::from("h/l Left/Right  drill out/in"),
        Line::from("s          soft retry failed tasks"),
        Line::from("z          diagnose selected"),
        Line::from("d          diagnose plan (confirm)"),
        Line::from("m          merge plan (confirm)"),
        Line::from("M          merge all done (confirm)"),
        Line::from("R          restart phase (confirm)"),
        Line::from("F          force advance"),
        Line::from("V / c      re-verify plan / gates only"),
        Line::from("S          repair (preserve completed)"),
        Line::from(""),
        Line::from(Span::styled("Logs (F5)", theme.accent_bold())),
        Line::from("1          toggle Info level"),
        Line::from("2          toggle Warn level"),
        Line::from("3          toggle Error level"),
        Line::from("4          toggle Debug level"),
        Line::from("a          show all log levels"),
        Line::from("/          start log search"),
        Line::from("n / N      next / prev match"),
        Line::from("f          toggle filter mode"),
    ]
}
