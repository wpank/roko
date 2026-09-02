//! Help modal.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
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

/// Section header with underline styling.
fn section(title: &'static str, theme: &Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            title,
            theme.accent_bold().add_modifier(Modifier::UNDERLINED),
        )),
    ]
}

fn help_lines(theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(160);

    lines.push(Line::from(Span::styled(
        "roko dashboard keybindings",
        theme.accent_bold(),
    )));
    lines.push(Line::from(""));

    // ── Global ────────────────────────────────────────────────────────
    lines.extend(section("Global (all tabs)", theme));
    lines.push(Line::from("Ctrl-c       quit immediately"));
    lines.push(Line::from("q            quit (close modal first if open)"));
    lines.push(Line::from("?            toggle this help modal"));
    lines.push(Line::from("F1-F10       switch tabs"));
    lines.push(Line::from("               F1 Dashboard  F2 Plans    F3 Agents   F4 Git"));
    lines.push(Line::from("               F5 Logs       F6 Config   F7 Inspect  F8 Marketplace"));
    lines.push(Line::from("               F9 Atelier    F10 Learning"));
    lines.push(Line::from("1-9 / 0      switch tabs (except on Agents/Logs/Plans)"));
    lines.push(Line::from("Alt+1-9      switch sub-view within current tab"));
    lines.push(Line::from("Tab          cycle focus forward between panels"));
    lines.push(Line::from("Shift+Tab    cycle focus backward"));
    lines.push(Line::from("n            dismiss oldest notification toast"));
    lines.push(Line::from("v            verify / re-verify selected plan"));
    lines.push(Line::from("u            queue overview modal"));
    lines.push(Line::from("Ctrl-n       notification history modal"));
    lines.push(Line::from("Ctrl-r       refresh data"));
    lines.push(Line::from("Ctrl-a       approve all pending requests"));
    lines.push(Line::from("Ctrl-t       agent topology panel"));
    lines.push(Line::from("Ctrl-e       toggle screen post-processing effects"));
    lines.push(Line::from("Ctrl-x       force-advance (confirm)"));
    lines.push(Line::from("Ctrl-d       reset selected plan (confirm)"));
    lines.push(Line::from("Ctrl-g       git reconcile (confirm)"));
    lines.push(Line::from(""));

    // ── Dashboard (F1) ────────────────────────────────────────────────
    lines.extend(section("Dashboard (F1)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll focused panel"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("h/l Left/Rt  drill out/in"));
    lines.push(Line::from("Shift+L/R    wave prev/next"));
    lines.push(Line::from("Enter        open plan detail modal"));
    lines.push(Line::from("Esc          close plan detail"));
    lines.push(Line::from("w            wave overview modal"));
    lines.push(Line::from("p            pause/resume pipeline"));
    lines.push(Line::from("i            inject directive to agent"));
    lines.push(Line::from("y            approve pending command"));
    lines.push(Line::from("`            cycle agent role tabs"));
    lines.push(Line::from("  Sub-tab shortcuts:"));
    lines.push(Line::from("a            Agents panel"));
    lines.push(Line::from("o            Output panel"));
    lines.push(Line::from("d            Diff panel"));
    lines.push(Line::from("e            Errors panel"));
    lines.push(Line::from("g            Git panel"));
    lines.push(Line::from("m            MCP / Context panel"));
    lines.push(Line::from("L            Learning panel"));
    lines.push(Line::from("P            Processes panel"));
    lines.push(Line::from(""));

    // ── Plans (F2) ───────────────────────────────────────────────────
    lines.extend(section("Plans (F2)", theme));
    lines.push(Line::from("j/k Up/Dn    select plan up/down"));
    lines.push(Line::from("1-9          jump to plan by index"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("h/l Left/Rt  drill out/in"));
    lines.push(Line::from("e            expand/collapse plan"));
    lines.push(Line::from("Enter        open plan detail modal"));
    lines.push(Line::from("Esc          close plan detail"));
    lines.push(Line::from("[  /  ]      wave prev / wave next"));
    lines.push(Line::from("Shift+L/R    wave prev/next (alt)"));
    lines.push(Line::from("w            wave overview modal"));
    lines.push(Line::from("o / q        queue overview modal"));
    lines.push(Line::from("t            task picker modal"));
    lines.push(Line::from("/            filter plan tree"));
    lines.push(Line::from("  Recovery:"));
    lines.push(Line::from("s            soft-retry failed tasks"));
    lines.push(Line::from("z            diagnose selected task"));
    lines.push(Line::from("d            diagnose plan (confirm)"));
    lines.push(Line::from("S            repair with error context"));
    lines.push(Line::from("c            re-verify gates only"));
    lines.push(Line::from("V            re-verify plan"));
    lines.push(Line::from("F            force-advance"));
    lines.push(Line::from("R            restart plan (confirm)"));
    lines.push(Line::from("m            merge plan (confirm)"));
    lines.push(Line::from("M            merge all completed (confirm)"));
    lines.push(Line::from(""));

    // ── Agents (F3) ──────────────────────────────────────────────────
    lines.extend(section("Agents (F3)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll agent output / select agent"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("G            resume auto-scroll (tail)"));
    lines.push(Line::from("`            cycle agent role tabs"));
    lines.push(Line::from("1-7          switch agent role tab directly"));
    lines.push(Line::from("a            approve pending command"));
    lines.push(Line::from("A            approve all pending"));
    lines.push(Line::from("x            reject pending command"));
    lines.push(Line::from("i            inject directive to agent"));
    lines.push(Line::from("g            toggle agent pane grouping"));
    lines.push(Line::from("t            toggle agent topology"));
    lines.push(Line::from(""));

    // ── Git (F4) ─────────────────────────────────────────────────────
    lines.extend(section("Git (F4)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll branch/commit list"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("h/l Left/Rt  drill out/in"));
    lines.push(Line::from("Enter        expand/collapse"));
    lines.push(Line::from(""));

    // ── Logs (F5) ────────────────────────────────────────────────────
    lines.extend(section("Logs (F5)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll log list"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home         jump to top"));
    lines.push(Line::from("End / G      jump to bottom (resume auto-tail)"));
    lines.push(Line::from("1            toggle Info level"));
    lines.push(Line::from("2            toggle Warn level"));
    lines.push(Line::from("3            toggle Error level"));
    lines.push(Line::from("4            toggle Debug level"));
    lines.push(Line::from("a            show all log levels"));
    lines.push(Line::from("/            start log search"));
    lines.push(Line::from("n / N        next / prev search match"));
    lines.push(Line::from("f            toggle search filter mode"));
    lines.push(Line::from(""));

    // ── Config (F6) ──────────────────────────────────────────────────
    lines.extend(section("Config (F6)", theme));
    lines.push(Line::from("j/k Up/Dn    navigate config keys"));
    lines.push(Line::from("h/l Left/Rt  cycle option value left/right"));
    lines.push(Line::from("Enter/Space  toggle or edit value"));
    lines.push(Line::from("Ctrl-s       save config changes"));
    lines.push(Line::from("  Edit mode (when editing a text value):"));
    lines.push(Line::from("Enter        commit edit"));
    lines.push(Line::from("Esc          cancel edit"));
    lines.push(Line::from(""));

    // ── Inspect (F7) ─────────────────────────────────────────────────
    lines.extend(section("Inspect (F7)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll signal tree"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("h/l Left/Rt  drill out/in"));
    lines.push(Line::from("Enter        expand/collapse"));
    lines.push(Line::from(""));

    // ── Marketplace (F8) ─────────────────────────────────────────────
    lines.extend(section("Marketplace (F8)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll job list"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("Enter        expand job detail"));
    lines.push(Line::from("n            new job form"));
    lines.push(Line::from("r            refresh"));
    lines.push(Line::from("Ctrl-s       submit job form"));
    lines.push(Line::from(""));

    // ── Atelier (F9) ─────────────────────────────────────────────────
    lines.extend(section("Atelier (F9)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll artifact list"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("Enter        expand artifact detail"));
    lines.push(Line::from("r            refresh"));
    lines.push(Line::from(""));

    // ── Learning (F10) ───────────────────────────────────────────────
    lines.extend(section("Learning (F10)", theme));
    lines.push(Line::from("j/k Up/Dn    scroll metrics list"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from("r            refresh"));
    lines.push(Line::from(""));

    // ── Modals ───────────────────────────────────────────────────────
    lines.extend(section("Modal Controls", theme));
    lines.push(Line::from("  All modals:"));
    lines.push(Line::from("Esc / q      close modal"));
    lines.push(Line::from("j/k Up/Dn    scroll content"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Help modal:"));
    lines.push(Line::from("PgUp/PgDn    page scroll"));
    lines.push(Line::from("Home/End     jump to top/bottom"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Plan detail:"));
    lines.push(Line::from("Esc          close"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Task picker:"));
    lines.push(Line::from("Enter        open task detail"));
    lines.push(Line::from("Esc          close"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Task detail:"));
    lines.push(Line::from("Tab          switch detail sub-tab"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Approval dialog:"));
    lines.push(Line::from("y / Enter    approve"));
    lines.push(Line::from("n / Esc      reject"));
    lines.push(Line::from("A / Ctrl-a   approve all"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Confirm dialog:"));
    lines.push(Line::from("y / Enter    confirm"));
    lines.push(Line::from("n / Esc      cancel"));
    lines.push(Line::from(""));
    lines.push(Line::from("  Batch review:"));
    lines.push(Line::from("a            accept batch"));
    lines.push(Line::from("r            reject batch"));
    lines.push(Line::from(""));

    // ── Text Input ───────────────────────────────────────────────────
    lines.extend(section("Text Input Modes", theme));
    lines.push(Line::from("  Inject / Filter / Log Search / Plan Filter:"));
    lines.push(Line::from("Enter        submit / accept"));
    lines.push(Line::from("Esc          cancel"));
    lines.push(Line::from("Backspace    delete character"));

    lines
}
