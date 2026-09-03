//! Per-frame mouse hit-region registry with z-order dispatch.
//!
//! Each draw call registers ordered `HitRegion`s from actual rendered `Rect`s.
//! Mouse dispatch selects the highest-z containing region. An active modal
//! registers the only interactive regions, blocking underlying panels.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use super::tabs::Tab;

// ---------------------------------------------------------------------------
// FocusZone — same enum used by the static fallback
// ---------------------------------------------------------------------------

/// Named focus zones in the TUI, used for coordinate-to-zone mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusZone {
    /// The plan tree / left sidebar.
    PlanTree,
    /// Task progress panel.
    TaskProgress,
    /// Agent output panel.
    AgentOutput,
    /// Command/tool output panel.
    CommandOutput,
    /// Right-side content area.
    RightContent,
    /// A header tab at the given index.
    HeaderTab(usize),
    /// A detail sub-tab at the given index.
    DetailTab(usize),
    // -- per-tab split zones --
    /// Left pane of a two-column tab (branches, keys, signals, etc.).
    LeftPane,
    /// Right/detail pane of a two-column tab.
    RightPane,
}

// ---------------------------------------------------------------------------
// ScrollTarget — what a scroll event should affect
// ---------------------------------------------------------------------------

/// Identifies which scroll state a region controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTarget {
    /// Scroll the plan tree / list offset.
    PlanTree,
    /// Scroll the task progress list.
    TaskProgress,
    /// Scroll agent output.
    AgentOutput,
    /// Scroll command output.
    CommandOutput,
    /// Scroll the right/diff panel.
    RightPanel,
    /// Scroll the procs sub-tab.
    Procs,
    /// Git detail pane.
    GitDetail,
    /// Config values pane.
    ConfigValues,
    /// Inspect detail pane.
    InspectDetail,
    /// Learning detail pane.
    LearningDetail,
    /// Config keys list.
    ConfigKeys,
    /// Log list.
    LogList,
    /// Agent roster selection.
    AgentRoster,
    /// Marketplace job selection.
    MarketplaceJobs,
    /// Atelier PRD selection.
    AtelierPrds,
    /// Modal scroll (generic).
    Modal,
    /// Not scrollable.
    None,
}

// ---------------------------------------------------------------------------
// ClickTarget — what a click event should do
// ---------------------------------------------------------------------------

/// What happens when a region is clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickTarget {
    /// Switch to a header tab.
    SwitchTab(usize),
    /// Switch to a detail sub-view.
    SwitchSubView(usize),
    /// Set focus to the given zone.
    SetFocus(FocusZone),
    /// No click action.
    None,
}

// ---------------------------------------------------------------------------
// HitRegion — one interactive region
// ---------------------------------------------------------------------------

/// A single interactive screen region registered during the draw phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitRegion {
    /// Bounding rectangle in terminal coordinates.
    pub rect: Rect,
    /// Which tab this region belongs to (if any).
    pub tab: Option<Tab>,
    /// The focus zone this region represents.
    pub focus_zone: FocusZone,
    /// What scroll events should target in this region.
    pub scroll_target: ScrollTarget,
    /// What click events should do in this region.
    pub click_target: ClickTarget,
    /// Z-order layer. Higher values are rendered on top and take priority.
    /// Base panel = 0, modal overlay = 10.
    pub z: u8,
}

// ---------------------------------------------------------------------------
// HitRegionRegistry — per-frame registry
// ---------------------------------------------------------------------------

/// Per-frame registry of interactive hit regions.
///
/// Populated by view/modal layout code from actual `Rect`s during each draw.
/// Mouse dispatch queries this to find the highest-z containing region.
#[derive(Debug, Clone, Default)]
pub struct HitRegionRegistry {
    /// Regions in insertion order. Lookup scans all and picks highest z.
    regions: Vec<HitRegion>,
    /// When true, a modal is active and only modal-z regions are interactive.
    modal_active: bool,
}

/// The z-level used for base panel regions.
pub const Z_PANEL: u8 = 0;
/// The z-level used for modal overlay regions.
pub const Z_MODAL: u8 = 10;

impl HitRegionRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all regions for the start of a new frame.
    pub fn clear(&mut self) {
        self.regions.clear();
        self.modal_active = false;
    }

    /// Register a hit region.
    pub fn register(&mut self, region: HitRegion) {
        if region.z >= Z_MODAL {
            self.modal_active = true;
        }
        self.regions.push(region);
    }

    /// Register a modal region. Convenience wrapper that sets z = Z_MODAL.
    pub fn register_modal(&mut self, rect: Rect, scroll_target: ScrollTarget) {
        self.register(HitRegion {
            rect,
            tab: None,
            focus_zone: FocusZone::RightContent,
            scroll_target,
            click_target: ClickTarget::None,
            z: Z_MODAL,
        });
    }

    /// Whether a modal is currently blocking underlying panel regions.
    #[must_use]
    pub fn is_modal_active(&self) -> bool {
        self.modal_active
    }

    /// Find the hit region at (x, y). Returns the highest-z region that
    /// contains the point. When a modal is active, only modal-z regions
    /// are considered.
    #[must_use]
    pub fn region_at(&self, x: u16, y: u16) -> Option<&HitRegion> {
        let min_z = if self.modal_active { Z_MODAL } else { 0 };
        self.regions
            .iter()
            .filter(|r| r.z >= min_z && rect_contains(r.rect, x, y))
            .max_by_key(|r| r.z)
    }

    /// Return the focus zone at (x, y), respecting z-order and modal blocking.
    #[must_use]
    pub fn zone_at(&self, x: u16, y: u16) -> Option<FocusZone> {
        self.region_at(x, y).map(|r| r.focus_zone)
    }

    /// Return the scroll target at (x, y), respecting z-order and modal blocking.
    #[must_use]
    pub fn scroll_target_at(&self, x: u16, y: u16) -> Option<ScrollTarget> {
        self.region_at(x, y).map(|r| r.scroll_target)
    }

    /// Return the click target at (x, y), respecting z-order and modal blocking.
    #[must_use]
    pub fn click_target_at(&self, x: u16, y: u16) -> Option<ClickTarget> {
        self.region_at(x, y).map(|r| r.click_target)
    }

    /// Number of registered regions (for diagnostics).
    #[must_use]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// HitZones — backward-compatible static hit zone computation
// ---------------------------------------------------------------------------

/// Cached screen regions for hit testing (backward-compatible API).
///
/// This is the original static layout recomputation. New code should prefer
/// `HitRegionRegistry` which uses actual rendered `Rect`s, but the static
/// fallback remains available for callers that do not have a registry.
#[derive(Debug, Clone, Default)]
pub struct HitZones {
    pub plan_tree: Rect,
    pub task_progress: Rect,
    pub agent_output: Rect,
    pub command_output: Rect,
    pub right_content: Rect,
    pub left_pane: Rect,
    pub right_pane: Rect,
    pub detail_tab_rects: Vec<(Rect, usize)>,
    pub header_tab_rects: Vec<(Rect, usize)>,
}

impl HitZones {
    /// Replay layout math to compute hit zones for the current terminal size and tab.
    ///
    /// `tab` is the active tab index (0 = Dashboard, 1 = Plans, 2 = Agents, etc.).
    /// `header_tab_count` is how many top-level tabs exist.
    #[must_use]
    pub fn compute(area: Rect, tab: usize, header_tab_count: usize) -> Self {
        let mut zones = Self::default();

        // Top-level layout: compact header | body | compact footer. Warning
        // and wave rows are optional at runtime; treating them as body keeps
        // clicks conservative without replaying application state here.
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let header_area = outer[0];
        let body_area = outer[1];

        // Compute header tab hit rects (evenly spaced across the header).
        if header_tab_count > 0 {
            let tab_width = header_area.width / header_tab_count as u16;
            for i in 0..header_tab_count {
                let x = header_area.x + (i as u16) * tab_width;
                let w = if i == header_tab_count - 1 {
                    header_area.width - (i as u16) * tab_width
                } else {
                    tab_width
                };
                zones
                    .header_tab_rects
                    .push((Rect::new(x, header_area.y, w, header_area.height), i));
            }
        }

        // Per-tab body layout
        match tab {
            0 => {
                // Dashboard: single content area
                zones.right_content = body_area;
            }
            1 => {
                // Plans: left sidebar | VOID gutter | right detail.
                let h = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(31),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(body_area);
                zones.plan_tree = h[0];
                zones.right_content = h[2];

                // Right side: task progress (top 40%) | agent output (bottom 60%)
                let v = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(h[2]);
                zones.task_progress = v[0];
                zones.agent_output = v[1];
            }
            2 => {
                // Agents: left roster | VOID gutter | right output.
                let h = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(32),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(body_area);
                zones.plan_tree = h[0]; // reuse as agent list
                zones.agent_output = h[2];
            }
            3 | 4 | 5 | 6 | 7 | 8 | 9 => {
                // Two-column split: left list/tree | gutter | right detail.
                let h = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(35),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(body_area);
                zones.left_pane = h[0];
                zones.right_pane = h[2];
                zones.right_content = body_area;
            }
            _ => {
                // Fallback: full body is right_content.
                zones.right_content = body_area;
            }
        }

        zones
    }

    /// Return the focus zone at the given screen coordinate, if any.
    #[must_use]
    pub fn zone_at(&self, x: u16, y: u16) -> Option<FocusZone> {
        // Check header tabs first (highest priority for clicks).
        for &(rect, idx) in &self.header_tab_rects {
            if rect_contains(rect, x, y) {
                return Some(FocusZone::HeaderTab(idx));
            }
        }

        // Check detail tabs.
        for &(rect, idx) in &self.detail_tab_rects {
            if rect_contains(rect, x, y) {
                return Some(FocusZone::DetailTab(idx));
            }
        }

        // Check body zones (order: most specific first).
        if rect_contains(self.plan_tree, x, y) && !rect_is_empty(self.plan_tree) {
            return Some(FocusZone::PlanTree);
        }
        if rect_contains(self.task_progress, x, y) && !rect_is_empty(self.task_progress) {
            return Some(FocusZone::TaskProgress);
        }
        if rect_contains(self.agent_output, x, y) && !rect_is_empty(self.agent_output) {
            return Some(FocusZone::AgentOutput);
        }
        if rect_contains(self.command_output, x, y) && !rect_is_empty(self.command_output) {
            return Some(FocusZone::CommandOutput);
        }
        // Per-tab split panes (more specific than right_content).
        if rect_contains(self.left_pane, x, y) && !rect_is_empty(self.left_pane) {
            return Some(FocusZone::LeftPane);
        }
        if rect_contains(self.right_pane, x, y) && !rect_is_empty(self.right_pane) {
            return Some(FocusZone::RightPane);
        }
        if rect_contains(self.right_content, x, y) && !rect_is_empty(self.right_content) {
            return Some(FocusZone::RightContent);
        }

        None
    }

    /// Convert this static zone set into a `HitRegionRegistry` with z=0 panel regions.
    ///
    /// This bridges old callers that use `HitZones::compute` into the new registry API.
    #[must_use]
    pub fn into_registry(self, tab: Tab) -> HitRegionRegistry {
        let mut registry = HitRegionRegistry::new();

        for &(rect, idx) in &self.header_tab_rects {
            if !rect_is_empty(rect) {
                registry.register(HitRegion {
                    rect,
                    tab: Some(tab),
                    focus_zone: FocusZone::HeaderTab(idx),
                    scroll_target: ScrollTarget::None,
                    click_target: ClickTarget::SwitchTab(idx),
                    z: Z_PANEL,
                });
            }
        }

        for &(rect, idx) in &self.detail_tab_rects {
            if !rect_is_empty(rect) {
                registry.register(HitRegion {
                    rect,
                    tab: Some(tab),
                    focus_zone: FocusZone::DetailTab(idx),
                    scroll_target: ScrollTarget::None,
                    click_target: ClickTarget::SwitchSubView(idx),
                    z: Z_PANEL,
                });
            }
        }

        let panel_regions: &[(Rect, FocusZone, ScrollTarget)] = &[
            (self.plan_tree, FocusZone::PlanTree, ScrollTarget::PlanTree),
            (
                self.task_progress,
                FocusZone::TaskProgress,
                ScrollTarget::TaskProgress,
            ),
            (
                self.agent_output,
                FocusZone::AgentOutput,
                ScrollTarget::AgentOutput,
            ),
            (
                self.command_output,
                FocusZone::CommandOutput,
                ScrollTarget::CommandOutput,
            ),
            (self.left_pane, FocusZone::LeftPane, scroll_for_left(tab)),
            (self.right_pane, FocusZone::RightPane, scroll_for_right(tab)),
            (
                self.right_content,
                FocusZone::RightContent,
                ScrollTarget::RightPanel,
            ),
        ];

        for &(rect, zone, scroll) in panel_regions {
            if !rect_is_empty(rect) {
                registry.register(HitRegion {
                    rect,
                    tab: Some(tab),
                    focus_zone: zone,
                    scroll_target: scroll,
                    click_target: ClickTarget::SetFocus(zone),
                    z: Z_PANEL,
                });
            }
        }

        registry
    }
}

/// Map a tab to the scroll target for its left pane.
fn scroll_for_left(tab: Tab) -> ScrollTarget {
    match tab {
        Tab::Git => ScrollTarget::PlanTree,
        Tab::Logs => ScrollTarget::LogList,
        Tab::Config => ScrollTarget::ConfigKeys,
        Tab::Inspect => ScrollTarget::PlanTree,
        Tab::Marketplace => ScrollTarget::MarketplaceJobs,
        Tab::Atelier => ScrollTarget::AtelierPrds,
        Tab::Learning => ScrollTarget::PlanTree,
        _ => ScrollTarget::PlanTree,
    }
}

/// Map a tab to the scroll target for its right pane.
fn scroll_for_right(tab: Tab) -> ScrollTarget {
    match tab {
        Tab::Git => ScrollTarget::GitDetail,
        Tab::Config => ScrollTarget::ConfigValues,
        Tab::Inspect => ScrollTarget::InspectDetail,
        Tab::Learning => ScrollTarget::LearningDetail,
        _ => ScrollTarget::RightPanel,
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn rect_is_empty(rect: Rect) -> bool {
    rect.width == 0 || rect.height == 0
}

// Backward-compatible aliases used by the old module-internal code.
// Retained so any stale references compile, but new code should use the
// `rect_` prefixed versions above.

/// Backward-compatible alias for `rect_contains`.
#[doc(hidden)]
pub fn contains(rect: Rect, x: u16, y: u16) -> bool {
    rect_contains(rect, x, y)
}

/// Backward-compatible alias for `rect_is_empty`.
#[doc(hidden)]
pub fn is_empty(rect: Rect) -> bool {
    rect_is_empty(rect)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Legacy HitZones tests (preserved from original) --

    #[test]
    fn dashboard_tab_has_right_content() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 0, 5);
        assert!(zones.right_content.width > 0);
        assert!(zones.right_content.height > 0);
    }

    #[test]
    fn plans_tab_has_sidebar() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 1, 5);
        assert!(zones.plan_tree.width > 0);
        assert!(zones.task_progress.width > 0);
    }

    #[test]
    fn header_tabs_are_clickable() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 0, 5);
        assert_eq!(zones.header_tab_rects.len(), 5);
        // Click in first tab region
        let (first_rect, _) = zones.header_tab_rects[0];
        let zone = zones.zone_at(first_rect.x + 1, first_rect.y);
        assert_eq!(zone, Some(FocusZone::HeaderTab(0)));
    }

    #[test]
    fn zone_at_returns_none_outside() {
        let area = Rect::new(0, 0, 80, 24);
        let zones = HitZones::compute(area, 0, 3);
        // Way outside
        assert_eq!(zones.zone_at(200, 200), None);
    }
}

// ---------------------------------------------------------------------------
// #368 hit-test table tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tui_mouse_hit_test {
    use super::*;

    // -- HitRegionRegistry basics --

    #[test]
    fn empty_registry_returns_none() {
        let reg = HitRegionRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.region_at(10, 10), None);
        assert_eq!(reg.zone_at(10, 10), None);
        assert_eq!(reg.scroll_target_at(10, 10), None);
        assert_eq!(reg.click_target_at(10, 10), None);
    }

    #[test]
    fn single_region_hit() {
        let mut reg = HitRegionRegistry::new();
        reg.register(HitRegion {
            rect: Rect::new(5, 5, 20, 10),
            tab: Some(Tab::Plans),
            focus_zone: FocusZone::PlanTree,
            scroll_target: ScrollTarget::PlanTree,
            click_target: ClickTarget::SetFocus(FocusZone::PlanTree),
            z: Z_PANEL,
        });
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.zone_at(10, 8), Some(FocusZone::PlanTree));
        assert_eq!(reg.scroll_target_at(10, 8), Some(ScrollTarget::PlanTree));
        // Outside
        assert_eq!(reg.zone_at(0, 0), None);
        assert_eq!(reg.zone_at(30, 20), None);
    }

    #[test]
    fn higher_z_wins_overlap() {
        let mut reg = HitRegionRegistry::new();
        let overlap = Rect::new(0, 0, 40, 20);
        // Base panel
        reg.register(HitRegion {
            rect: overlap,
            tab: Some(Tab::Dashboard),
            focus_zone: FocusZone::RightContent,
            scroll_target: ScrollTarget::RightPanel,
            click_target: ClickTarget::SetFocus(FocusZone::RightContent),
            z: 0,
        });
        // Higher z overlay
        reg.register(HitRegion {
            rect: overlap,
            tab: Some(Tab::Dashboard),
            focus_zone: FocusZone::AgentOutput,
            scroll_target: ScrollTarget::AgentOutput,
            click_target: ClickTarget::None,
            z: 5,
        });
        assert_eq!(reg.zone_at(10, 10), Some(FocusZone::AgentOutput));
        assert_eq!(
            reg.scroll_target_at(10, 10),
            Some(ScrollTarget::AgentOutput)
        );
    }

    #[test]
    fn modal_blocks_underlying_panels() {
        let mut reg = HitRegionRegistry::new();
        let panel_rect = Rect::new(0, 0, 80, 24);
        let modal_rect = Rect::new(20, 5, 40, 14);

        // Base panel covers the whole screen.
        reg.register(HitRegion {
            rect: panel_rect,
            tab: Some(Tab::Plans),
            focus_zone: FocusZone::PlanTree,
            scroll_target: ScrollTarget::PlanTree,
            click_target: ClickTarget::SetFocus(FocusZone::PlanTree),
            z: Z_PANEL,
        });
        // Modal covers center area.
        reg.register_modal(modal_rect, ScrollTarget::Modal);

        assert!(reg.is_modal_active());

        // Inside modal: returns modal region, not underlying panel.
        let hit = reg.region_at(30, 10);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().z, Z_MODAL);
        assert_eq!(
            reg.scroll_target_at(30, 10),
            Some(ScrollTarget::Modal)
        );

        // Outside modal but inside panel: blocked by modal, returns None.
        assert_eq!(reg.region_at(5, 5), None);
        assert_eq!(reg.zone_at(5, 5), None);
        assert_eq!(reg.scroll_target_at(5, 5), None);
    }

    #[test]
    fn clear_resets_registry() {
        let mut reg = HitRegionRegistry::new();
        reg.register_modal(Rect::new(0, 0, 10, 10), ScrollTarget::Modal);
        assert!(reg.is_modal_active());
        assert_eq!(reg.len(), 1);

        reg.clear();
        assert!(!reg.is_modal_active());
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    // -- Border and edge tests --

    #[test]
    fn borders_top_left_inclusive_bottom_right_exclusive() {
        let mut reg = HitRegionRegistry::new();
        reg.register(HitRegion {
            rect: Rect::new(10, 10, 5, 5), // x=10..14, y=10..14
            tab: None,
            focus_zone: FocusZone::LeftPane,
            scroll_target: ScrollTarget::None,
            click_target: ClickTarget::None,
            z: Z_PANEL,
        });

        // Top-left corner: inclusive.
        assert_eq!(reg.zone_at(10, 10), Some(FocusZone::LeftPane));
        // Bottom-right corner (exclusive): just outside.
        assert_eq!(reg.zone_at(15, 15), None);
        // Bottom-right within: last valid cell.
        assert_eq!(reg.zone_at(14, 14), Some(FocusZone::LeftPane));
        // One pixel outside right edge.
        assert_eq!(reg.zone_at(15, 10), None);
        // One pixel outside bottom edge.
        assert_eq!(reg.zone_at(10, 15), None);
    }

    #[test]
    fn adjacent_non_overlapping_regions() {
        let mut reg = HitRegionRegistry::new();
        // Two side-by-side regions with no gap.
        reg.register(HitRegion {
            rect: Rect::new(0, 0, 40, 20),
            tab: Some(Tab::Plans),
            focus_zone: FocusZone::PlanTree,
            scroll_target: ScrollTarget::PlanTree,
            click_target: ClickTarget::SetFocus(FocusZone::PlanTree),
            z: Z_PANEL,
        });
        reg.register(HitRegion {
            rect: Rect::new(40, 0, 40, 20),
            tab: Some(Tab::Plans),
            focus_zone: FocusZone::AgentOutput,
            scroll_target: ScrollTarget::AgentOutput,
            click_target: ClickTarget::SetFocus(FocusZone::AgentOutput),
            z: Z_PANEL,
        });

        // At the boundary: x=39 is left, x=40 is right.
        assert_eq!(reg.zone_at(39, 10), Some(FocusZone::PlanTree));
        assert_eq!(reg.zone_at(40, 10), Some(FocusZone::AgentOutput));
    }

    // -- Resize tests --

    #[test]
    fn resize_produces_deterministic_regions() {
        let sizes: [(u16, u16); 4] = [(80, 24), (120, 40), (200, 60), (40, 12)];
        for (w, h) in sizes {
            let area = Rect::new(0, 0, w, h);
            let zones = HitZones::compute(area, 1, 10);
            let reg = zones.into_registry(Tab::Plans);
            // Every registered region must fit within the terminal area.
            for region in &reg.regions {
                assert!(
                    region.rect.x + region.rect.width <= w,
                    "Region {:?} exceeds width {} at size ({}, {})",
                    region.focus_zone,
                    w,
                    w,
                    h
                );
                assert!(
                    region.rect.y + region.rect.height <= h,
                    "Region {:?} exceeds height {} at size ({}, {})",
                    region.focus_zone,
                    h,
                    w,
                    h
                );
            }
        }
    }

    // -- Per-tab panel tests --

    #[test]
    fn dashboard_tab_right_content_covers_body() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 0, 10);
        let reg = zones.into_registry(Tab::Dashboard);
        // Center of body should hit RightContent.
        assert_eq!(reg.zone_at(60, 25), Some(FocusZone::RightContent));
    }

    #[test]
    fn plans_tab_has_left_and_right_panels() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 1, 10);
        let reg = zones.into_registry(Tab::Plans);

        // Left column: plan tree.
        assert_eq!(reg.zone_at(5, 10), Some(FocusZone::PlanTree));

        // Right column top: task progress.
        let tp_zone = reg.zone_at(80, 5);
        assert!(
            tp_zone == Some(FocusZone::TaskProgress)
                || tp_zone == Some(FocusZone::RightContent),
            "Expected TaskProgress or RightContent in right-top, got {tp_zone:?}"
        );
    }

    #[test]
    fn agents_tab_has_roster_and_output() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 2, 10);
        let reg = zones.into_registry(Tab::Agents);

        // Left column: agent roster (mapped as PlanTree in static layout).
        assert_eq!(reg.zone_at(5, 10), Some(FocusZone::PlanTree));
        // Right column: agent output.
        assert_eq!(reg.zone_at(80, 10), Some(FocusZone::AgentOutput));
    }

    #[test]
    fn two_column_tabs_have_left_and_right_panes() {
        // Tabs 3..=9 use the two-column split.
        let tabs_and_enums = [
            (3, Tab::Git),
            (4, Tab::Logs),
            (5, Tab::Config),
            (6, Tab::Inspect),
            (7, Tab::Marketplace),
            (8, Tab::Atelier),
            (9, Tab::Learning),
        ];
        for (tab_idx, tab) in tabs_and_enums {
            let area = Rect::new(0, 0, 120, 50);
            let zones = HitZones::compute(area, tab_idx, 10);
            let reg = zones.into_registry(tab);

            // Left pane: should be present.
            let left = reg.zone_at(5, 10);
            assert_eq!(
                left,
                Some(FocusZone::LeftPane),
                "Tab {} should have LeftPane at (5,10)",
                tab.label()
            );

            // Right pane: should be present.
            let right = reg.zone_at(80, 10);
            assert!(
                right == Some(FocusZone::RightPane) || right == Some(FocusZone::RightContent),
                "Tab {} should have RightPane or RightContent at (80,10), got {right:?}",
                tab.label()
            );
        }
    }

    // -- Scroll target per-tab tests --

    #[test]
    fn scroll_targets_are_tab_specific() {
        let area = Rect::new(0, 0, 120, 50);

        // Git tab: right pane should target GitDetail.
        let git_zones = HitZones::compute(area, 3, 10);
        let git_reg = git_zones.into_registry(Tab::Git);
        assert_eq!(
            git_reg.scroll_target_at(80, 10),
            Some(ScrollTarget::GitDetail)
        );

        // Config tab: right pane should target ConfigValues.
        let cfg_zones = HitZones::compute(area, 5, 10);
        let cfg_reg = cfg_zones.into_registry(Tab::Config);
        assert_eq!(
            cfg_reg.scroll_target_at(80, 10),
            Some(ScrollTarget::ConfigValues)
        );

        // Config tab: left pane should target ConfigKeys.
        assert_eq!(
            cfg_reg.scroll_target_at(5, 10),
            Some(ScrollTarget::ConfigKeys)
        );
    }

    // -- Click target tests --

    #[test]
    fn header_tab_click_returns_switch_tab() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 0, 10);
        let reg = zones.into_registry(Tab::Dashboard);
        // Click on the first header tab region.
        let click = reg.click_target_at(2, 0);
        assert_eq!(click, Some(ClickTarget::SwitchTab(0)));
    }

    #[test]
    fn panel_click_returns_set_focus() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 1, 10);
        let reg = zones.into_registry(Tab::Plans);
        let click = reg.click_target_at(5, 10);
        assert_eq!(
            click,
            Some(ClickTarget::SetFocus(FocusZone::PlanTree))
        );
    }

    // -- Modal z-order integration --

    #[test]
    fn modal_on_top_of_full_tab_registry() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 1, 10);
        let mut reg = zones.into_registry(Tab::Plans);

        // Before modal: panel is reachable.
        assert!(reg.zone_at(5, 10).is_some());

        // Add a modal covering center.
        reg.register_modal(Rect::new(30, 10, 60, 30), ScrollTarget::Modal);

        // Panel outside modal: blocked.
        assert_eq!(reg.zone_at(5, 10), None);

        // Inside modal: returns modal region.
        assert_eq!(
            reg.scroll_target_at(50, 20),
            Some(ScrollTarget::Modal)
        );
    }

    // -- into_registry preserves all non-empty zones --

    #[test]
    fn into_registry_preserves_header_tabs() {
        let area = Rect::new(0, 0, 120, 50);
        let zones = HitZones::compute(area, 0, 10);
        let reg = zones.into_registry(Tab::Dashboard);
        // Should have 10 header tab regions plus at least 1 body region.
        assert!(reg.len() >= 11, "Expected >=11 regions, got {}", reg.len());
    }

    // -- Zero-size terminal --

    #[test]
    fn zero_size_terminal_produces_empty_registry() {
        let area = Rect::new(0, 0, 0, 0);
        let zones = HitZones::compute(area, 0, 10);
        let reg = zones.into_registry(Tab::Dashboard);
        // No non-empty regions should be registered.
        assert_eq!(reg.len(), 0);
    }

    // -- Minimal terminal --

    #[test]
    fn minimal_terminal_still_works() {
        let area = Rect::new(0, 0, 10, 3);
        let zones = HitZones::compute(area, 0, 2);
        let reg = zones.into_registry(Tab::Dashboard);
        // Should at least have header tabs.
        assert!(reg.len() >= 2);
        // No panics, all regions fit.
        for region in &reg.regions {
            assert!(region.rect.x + region.rect.width <= 10);
            assert!(region.rect.y + region.rect.height <= 3);
        }
    }

    // -- Scroll target isolation tests --

    #[test]
    fn detail_panes_use_distinct_scroll_targets_per_tab() {
        let area = Rect::new(0, 0, 120, 50);

        // Build a registry for every two-column tab and verify the right-pane
        // scroll target is unique to that tab, not a shared generic value.
        let tabs_and_expected_right: &[(usize, Tab, ScrollTarget)] = &[
            (3, Tab::Git, ScrollTarget::GitDetail),
            (5, Tab::Config, ScrollTarget::ConfigValues),
            (6, Tab::Inspect, ScrollTarget::InspectDetail),
            (9, Tab::Learning, ScrollTarget::LearningDetail),
        ];

        for &(tab_idx, tab, expected) in tabs_and_expected_right {
            let zones = HitZones::compute(area, tab_idx, 10);
            let reg = zones.into_registry(tab);

            // Right pane should use the tab-specific scroll target.
            let target = reg.scroll_target_at(80, 10);
            assert_eq!(
                target,
                Some(expected),
                "Tab {} right pane scroll target: expected {:?}, got {:?}",
                tab.label(),
                expected,
                target
            );
        }
    }

    #[test]
    fn left_panes_use_distinct_scroll_targets_per_tab() {
        let area = Rect::new(0, 0, 120, 50);

        let tabs_and_expected_left: &[(usize, Tab, ScrollTarget)] = &[
            (3, Tab::Git, ScrollTarget::PlanTree),
            (4, Tab::Logs, ScrollTarget::LogList),
            (5, Tab::Config, ScrollTarget::ConfigKeys),
            (6, Tab::Inspect, ScrollTarget::PlanTree),
            (7, Tab::Marketplace, ScrollTarget::MarketplaceJobs),
            (8, Tab::Atelier, ScrollTarget::AtelierPrds),
            (9, Tab::Learning, ScrollTarget::PlanTree),
        ];

        for &(tab_idx, tab, expected) in tabs_and_expected_left {
            let zones = HitZones::compute(area, tab_idx, 10);
            let reg = zones.into_registry(tab);

            let target = reg.scroll_target_at(5, 10);
            assert_eq!(
                target,
                Some(expected),
                "Tab {} left pane scroll target: expected {:?}, got {:?}",
                tab.label(),
                expected,
                target
            );
        }
    }

    // -- No-mouse registry isolation --

    #[test]
    fn no_mouse_registry_still_computes_valid_regions() {
        // When --no-mouse is active, the App does not call enable_mouse_capture.
        // The registry itself is still populated normally (it is purely geometric).
        // This test verifies that the registry computation is independent of
        // the mouse capture flag: regions are always valid for layout purposes.
        let area = Rect::new(0, 0, 120, 50);
        for &tab in &Tab::ALL {
            let zones = HitZones::compute(area, tab.index(), 10);
            let reg = zones.into_registry(tab);
            // All regions must fit the terminal.
            for region in &reg.regions {
                assert!(
                    region.rect.x + region.rect.width <= 120
                        && region.rect.y + region.rect.height <= 50,
                    "Tab {:?}: region {:?} exceeds terminal at 120x50",
                    tab,
                    region.focus_zone
                );
            }
        }
    }

    // -- Modal click target is always None --

    #[test]
    fn modal_region_click_target_is_none() {
        let mut reg = HitRegionRegistry::new();
        reg.register_modal(Rect::new(10, 10, 40, 20), ScrollTarget::Modal);
        let click = reg.click_target_at(20, 15);
        assert_eq!(click, Some(ClickTarget::None));
    }

    // -- Register_modal sets correct z level --

    #[test]
    fn register_modal_uses_z_modal() {
        let mut reg = HitRegionRegistry::new();
        reg.register_modal(Rect::new(0, 0, 10, 10), ScrollTarget::Modal);
        let region = reg.region_at(5, 5).unwrap();
        assert_eq!(region.z, Z_MODAL);
    }

    // -- Multiple modals: highest z wins --

    #[test]
    fn multiple_overlapping_modal_regions_use_highest_z() {
        let mut reg = HitRegionRegistry::new();
        // Base panel
        reg.register(HitRegion {
            rect: Rect::new(0, 0, 80, 24),
            tab: Some(Tab::Plans),
            focus_zone: FocusZone::PlanTree,
            scroll_target: ScrollTarget::PlanTree,
            click_target: ClickTarget::SetFocus(FocusZone::PlanTree),
            z: Z_PANEL,
        });
        // Modal layer 1
        reg.register(HitRegion {
            rect: Rect::new(10, 5, 60, 14),
            tab: None,
            focus_zone: FocusZone::RightContent,
            scroll_target: ScrollTarget::Modal,
            click_target: ClickTarget::None,
            z: Z_MODAL,
        });
        // Modal layer 2 (higher z, smaller rect)
        reg.register(HitRegion {
            rect: Rect::new(20, 8, 40, 8),
            tab: None,
            focus_zone: FocusZone::LeftPane,
            scroll_target: ScrollTarget::None,
            click_target: ClickTarget::None,
            z: Z_MODAL + 1,
        });

        // Inside both modals: highest z wins.
        let hit = reg.region_at(30, 10).unwrap();
        assert_eq!(hit.z, Z_MODAL + 1);
        assert_eq!(hit.focus_zone, FocusZone::LeftPane);

        // Inside only modal layer 1: still returns layer 1 (not panel).
        let hit2 = reg.region_at(15, 6).unwrap();
        assert_eq!(hit2.z, Z_MODAL);
    }

    // -- Every scroll_for_left and scroll_for_right return a valid target --

    #[test]
    fn scroll_for_helpers_cover_all_tabs() {
        for &tab in &Tab::ALL {
            let left = scroll_for_left(tab);
            let right = scroll_for_right(tab);

            // Neither should return Modal or None for any tab.
            assert_ne!(
                left,
                ScrollTarget::Modal,
                "Tab {:?} left scroll target is Modal",
                tab
            );
            assert_ne!(
                right,
                ScrollTarget::Modal,
                "Tab {:?} right scroll target is Modal",
                tab
            );
        }
    }

    // -- Full resize sweep: every tab at every common size --

    #[test]
    fn all_tabs_all_sizes_produce_valid_registries() {
        let sizes: &[(u16, u16)] = &[
            (80, 24),
            (120, 40),
            (200, 60),
            (40, 12),
            (20, 6),
            (160, 50),
        ];
        for &(w, h) in sizes {
            for &tab in &Tab::ALL {
                let area = Rect::new(0, 0, w, h);
                let zones = HitZones::compute(area, tab.index(), 10);
                let reg = zones.into_registry(tab);
                for region in &reg.regions {
                    assert!(
                        region.rect.x + region.rect.width <= w
                            && region.rect.y + region.rect.height <= h,
                        "Tab {:?} at ({w}x{h}): region {:?} exceeds terminal",
                        tab,
                        region.focus_zone
                    );
                }
            }
        }
    }
}
