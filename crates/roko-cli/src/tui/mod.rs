//! TUI/dashboard support for the CLI.
//!
//! The dashboard snapshot and scaffold types stay in place for text-mode and
//! API consumers. The interactive terminal shell lives in `app`, `event`,
//! `pages`, and `widgets`. The Mori-style modal+focus+tab system is in
//! `input`, `state`, and `tabs`.
#![deny(unused_imports)]

pub mod ansi;
pub mod app;
pub mod approval_ipc;
pub mod atmosphere;
pub mod config_meta;
mod cursors;
pub mod dashboard;
pub mod dashboard_gen;
pub mod display_utils;
pub mod effects_config;
pub mod empty_state;
pub mod event;
pub mod fs_watch;
pub(crate) mod git_watch;
pub mod hit_test;
pub mod icons;
pub mod input;
mod jsonl_cursor;
pub mod jsonl_tailer;
pub mod layout;
pub mod modals;
pub mod pages;
pub mod postfx;
pub mod postfx_pipeline;
#[cfg(feature = "tui-png")]
pub mod png_renderer;
pub mod scroll;
#[cfg(feature = "tui-png")]
pub mod screenshot_diff;
pub mod segment;
pub mod smoothing;
pub mod snapshot;
pub mod state;
pub mod tabs;
pub mod task_outputs;
pub mod theme;
pub mod util;
pub mod verdicts;
pub mod views;
#[cfg(feature = "tui-png")]
pub mod visual_assessment;
pub mod widgets;
pub mod ws_client;

pub use app::App;
pub use approval_ipc::{ApprovalChannel, ApprovalRequest};
pub use atmosphere::Atmosphere;
pub use dashboard::{DashboardData, DashboardScaffold, DashboardSummary};
pub use dashboard_gen::{DashboardGenerationState, DurableDashboardGenerationCounter};
pub use effects_config::EffectsConfig;
pub use event::{Event, EventHandler, FrameStats, RenderDirty, TickPolicy};
pub use hit_test::HitZones;
pub use input::{ConfirmAction, FocusZone, InputMode, TuiAction};
pub use layout::{centered_rect, responsive_outer_margin};
pub use modals::{ModalState, Notification, NotificationLevel, render_modals};
pub use pages::{Page, PageId, PageRegistry, PageScaffold, WidgetScaffold};
pub use scroll::ScrollAccel;
pub use state::TuiState;
pub use tabs::Tab;
pub use task_outputs::TaskOutputCursors;
pub use theme::Theme;
