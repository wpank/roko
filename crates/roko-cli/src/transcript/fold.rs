//! Fold state and rules for transcript blocks.
//!
//! Controls how tool results and subagent blocks are displayed: collapsed,
//! expanded, or auto-folded based on configurable rules.

use serde::{Deserialize, Serialize};

use super::block::TranscriptBlock;

// ─── FoldState ──────────────────────────────────────────────────────────

/// Collapse/expand state for a renderable transcript block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldState {
    /// Explicitly collapsed by the user.
    Collapsed,
    /// Explicitly expanded by the user.
    Expanded,
    /// Automatically folded by a rule (e.g. large output).
    AutoFolded,
}

impl FoldState {
    /// Whether the block should be rendered in collapsed form.
    #[must_use]
    pub const fn is_collapsed(self) -> bool {
        matches!(self, Self::Collapsed | Self::AutoFolded)
    }
}

impl Default for FoldState {
    fn default() -> Self {
        Self::Expanded
    }
}

// ─── FoldRule ───────────────────────────────────────────────────────────

/// Configurable rules for automatic fold decisions.
#[derive(Debug, Clone)]
pub struct FoldRule {
    /// Results larger than this many bytes are auto-folded.
    pub auto_fold_bytes: usize,
    /// Errors are always expanded regardless of size.
    pub errors_always_expanded: bool,
}

impl Default for FoldRule {
    fn default() -> Self {
        Self {
            auto_fold_bytes: 2048,
            errors_always_expanded: true,
        }
    }
}

impl FoldRule {
    /// Compute the initial fold state for a transcript block.
    #[must_use]
    pub fn initial_state(&self, block: &TranscriptBlock) -> FoldState {
        match block {
            TranscriptBlock::ToolCall {
                status,
                result_preview,
                error,
                ..
            } => {
                // Errors always expanded
                if self.errors_always_expanded && status.is_error() {
                    return FoldState::Expanded;
                }
                if self.errors_always_expanded && error.is_some() {
                    return FoldState::Expanded;
                }
                // Large results auto-fold
                if let Some(preview) = result_preview {
                    if preview.len() > self.auto_fold_bytes {
                        return FoldState::AutoFolded;
                    }
                }
                FoldState::Expanded
            }
            TranscriptBlock::SubagentBlock { children, .. } => {
                // Subagent blocks with many children auto-fold
                if children.len() > 5 {
                    FoldState::AutoFolded
                } else {
                    FoldState::Expanded
                }
            }
            TranscriptBlock::SystemMessage { level, .. } => {
                // Errors always expanded
                if self.errors_always_expanded && matches!(level, super::block::MessageLevel::Error)
                {
                    FoldState::Expanded
                } else {
                    FoldState::Expanded
                }
            }
            // Other block types are always expanded
            _ => FoldState::Expanded,
        }
    }

    /// Apply a user toggle: if currently collapsed -> expanded, if expanded -> collapsed.
    /// User toggles override auto-fold and persist until the next explicit toggle.
    #[must_use]
    pub fn toggle(current: FoldState) -> FoldState {
        match current {
            FoldState::Collapsed | FoldState::AutoFolded => FoldState::Expanded,
            FoldState::Expanded => FoldState::Collapsed,
        }
    }
}
