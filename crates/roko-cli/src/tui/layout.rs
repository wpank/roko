//! Layout helper functions for the TUI.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Return a centered rectangle using percentage-based constraints.
#[must_use]
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

/// Apply a 1-cell outer margin when the terminal is large enough (>=120w x >=50h).
/// Returns the inner area unchanged if the terminal is too small.
#[must_use]
pub fn responsive_outer_margin(area: Rect) -> Rect {
    if area.width >= 120 && area.height >= 50 {
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    } else {
        area
    }
}

/// Split an area horizontally (left/right) by percentage for the left pane.
#[must_use]
pub fn split_horizontal(area: Rect, left_pct: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(100 - left_pct),
        ])
        .split(area);
    (chunks[0], chunks[1])
}

/// Split an area vertically (top/bottom) by percentage for the top pane.
#[must_use]
pub fn split_vertical(area: Rect, top_pct: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(top_pct),
            Constraint::Percentage(100 - top_pct),
        ])
        .split(area);
    (chunks[0], chunks[1])
}

/// Minimum terminal width the TUI is designed for.
/// Below this, layout calculations may produce degenerate results.
pub const MIN_VIABLE_WIDTH: u16 = 60;

/// Minimum terminal height the TUI is designed for.
pub const MIN_VIABLE_HEIGHT: u16 = 10;

/// Returns `true` if the terminal is too small for useful rendering.
#[must_use]
pub fn is_terminal_too_small(area: Rect) -> bool {
    area.width < MIN_VIABLE_WIDTH || area.height < MIN_VIABLE_HEIGHT
}

/// Responsive master-detail panel split.
///
/// Above `stack_threshold` columns, returns a horizontal split with the
/// given sidebar percentage and a 1-cell gutter. At or below the threshold,
/// returns a vertical stack with the sidebar taking `stacked_rows` rows.
///
/// Returns `(sidebar_area, detail_area)`.
#[must_use]
pub fn responsive_panel_split(
    area: Rect,
    sidebar_pct: u16,
    stack_threshold: u16,
    stacked_rows: u16,
) -> (Rect, Rect) {
    if area.width > stack_threshold {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(sidebar_pct),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        (chunks[0], chunks[2])
    } else {
        let clamped = stacked_rows
            .clamp(4, 9)
            .min(area.height.saturating_sub(5).max(1));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(clamped),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        (chunks[0], chunks[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_fits_inside() {
        let area = Rect::new(0, 0, 100, 50);
        let r = centered_rect(50, 50, area);
        assert!(r.x >= area.x);
        assert!(r.y >= area.y);
        assert!(r.right() <= area.right());
        assert!(r.bottom() <= area.bottom());
    }

    #[test]
    fn responsive_margin_applied_for_large_terminal() {
        let area = Rect::new(0, 0, 120, 50);
        let inner = responsive_outer_margin(area);
        assert_eq!(inner.x, 1);
        assert_eq!(inner.y, 1);
        assert_eq!(inner.width, 118);
        assert_eq!(inner.height, 48);
    }

    #[test]
    fn responsive_margin_skipped_for_small_terminal() {
        let area = Rect::new(0, 0, 80, 24);
        let inner = responsive_outer_margin(area);
        assert_eq!(inner, area);
    }

    #[test]
    fn split_horizontal_sums_to_whole() {
        let area = Rect::new(0, 0, 100, 50);
        let (left, right) = split_horizontal(area, 30);
        assert!(left.width + right.width <= area.width);
        assert_eq!(left.y, right.y);
    }

    #[test]
    fn split_vertical_sums_to_whole() {
        let area = Rect::new(0, 0, 100, 50);
        let (top, bottom) = split_vertical(area, 40);
        assert!(top.height + bottom.height <= area.height);
        assert_eq!(top.x, bottom.x);
    }

    #[test]
    fn is_terminal_too_small_detects_narrow() {
        assert!(is_terminal_too_small(Rect::new(0, 0, 50, 24)));
        assert!(!is_terminal_too_small(Rect::new(0, 0, 80, 24)));
    }

    #[test]
    fn is_terminal_too_small_detects_short() {
        assert!(is_terminal_too_small(Rect::new(0, 0, 80, 8)));
        assert!(!is_terminal_too_small(Rect::new(0, 0, 80, 24)));
    }

    #[test]
    fn responsive_panel_split_horizontal_above_threshold() {
        let area = Rect::new(0, 0, 120, 40);
        let (sidebar, detail) = responsive_panel_split(area, 30, 100, 6);
        // Horizontal: sidebar on left, detail on right
        assert!(sidebar.width > 0);
        assert!(detail.width > 0);
        assert_eq!(sidebar.y, detail.y);
    }

    #[test]
    fn responsive_panel_split_stacks_below_threshold() {
        let area = Rect::new(0, 0, 80, 30);
        let (sidebar, detail) = responsive_panel_split(area, 30, 100, 6);
        // Stacked: sidebar on top, detail below
        assert_eq!(sidebar.x, detail.x);
        assert!(sidebar.bottom() <= detail.y + 1);
    }
}
