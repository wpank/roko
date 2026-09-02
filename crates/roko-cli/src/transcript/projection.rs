//! Transcript query and projection API.
//!
//! [`TranscriptProjection`] provides cursor-based iteration and filtering
//! over a sequence of [`TranscriptBlock`]s. Both inline and TUI renderers
//! consume this API without directly managing the block vector.

use std::collections::HashMap;

use super::block::{ToolBlockStatus, TranscriptBlock};
use super::fold::{FoldRule, FoldState};

// ─── BlockFilter ────────────────────────────────────────────────────────

/// Predicate for filtering transcript blocks.
#[derive(Debug, Clone, Default)]
pub struct BlockFilter {
    /// Only include blocks of these types (empty = all types).
    pub block_types: Vec<&'static str>,
    /// Only include tool calls with these statuses.
    pub tool_statuses: Vec<ToolBlockStatus>,
    /// Only include blocks containing this text (case-insensitive).
    pub text_search: Option<String>,
    /// Only include blocks from this agent.
    pub agent_id: Option<String>,
}

impl BlockFilter {
    /// Check if a block passes this filter.
    #[must_use]
    pub fn matches(&self, block: &TranscriptBlock) -> bool {
        // Type filter
        if !self.block_types.is_empty() && !self.block_types.contains(&block.block_type()) {
            return false;
        }
        // Tool status filter
        if !self.tool_statuses.is_empty() {
            if let TranscriptBlock::ToolCall { status, .. } = block {
                if !self.tool_statuses.contains(status) {
                    return false;
                }
            } else {
                // Non-tool blocks don't match a tool_status filter
                return false;
            }
        }
        // Text search
        if let Some(ref needle) = self.text_search {
            if !block.contains_text(needle) {
                return false;
            }
        }
        // Agent filter (only applicable to subagent blocks)
        if let Some(ref agent) = self.agent_id {
            if let TranscriptBlock::SubagentBlock { agent_id, .. } = block {
                if agent_id != agent {
                    return false;
                }
            }
        }
        true
    }
}

// ─── BlockQuery ─────────────────────────────────────────────────────────

/// A structured query combining filter, pagination, and sort direction.
#[derive(Debug, Clone)]
pub struct BlockQuery {
    /// Filter predicate.
    pub filter: BlockFilter,
    /// Cursor position (0-indexed block offset).
    pub cursor: usize,
    /// Maximum number of blocks to return.
    pub limit: usize,
}

impl Default for BlockQuery {
    fn default() -> Self {
        Self {
            filter: BlockFilter::default(),
            cursor: 0,
            limit: 100,
        }
    }
}

// ─── TranscriptProjection ───────────────────────────────────────────────

/// A queryable projection over transcript blocks.
///
/// Owns the block vector, fold states, and fold rules. Renderers
/// iterate via [`query`](Self::query) or use the cursor-based
/// [`page`](Self::page) method.
pub struct TranscriptProjection {
    blocks: Vec<TranscriptBlock>,
    fold_states: HashMap<usize, FoldState>,
    fold_rule: FoldRule,
}

impl TranscriptProjection {
    /// Create a new projection from a block vector.
    pub fn new(blocks: Vec<TranscriptBlock>) -> Self {
        let fold_rule = FoldRule::default();
        let mut fold_states = HashMap::new();
        for (i, block) in blocks.iter().enumerate() {
            let state = fold_rule.initial_state(block);
            if state != FoldState::Expanded {
                fold_states.insert(i, state);
            }
        }
        Self {
            blocks,
            fold_states,
            fold_rule,
        }
    }

    /// Create a projection with custom fold rules.
    pub fn with_fold_rule(blocks: Vec<TranscriptBlock>, rule: FoldRule) -> Self {
        let mut fold_states = HashMap::new();
        for (i, block) in blocks.iter().enumerate() {
            let state = rule.initial_state(block);
            if state != FoldState::Expanded {
                fold_states.insert(i, state);
            }
        }
        Self {
            blocks,
            fold_states,
            fold_rule: rule,
        }
    }

    /// Total number of blocks (unfiltered).
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Whether the projection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Get the fold state for block at `index`.
    #[must_use]
    pub fn fold_state(&self, index: usize) -> FoldState {
        self.fold_states
            .get(&index)
            .copied()
            .unwrap_or(FoldState::Expanded)
    }

    /// Toggle fold state for block at `index`.
    pub fn toggle_fold(&mut self, index: usize) {
        let current = self.fold_state(index);
        let next = FoldRule::toggle(current);
        if next == FoldState::Expanded {
            self.fold_states.remove(&index);
        } else {
            self.fold_states.insert(index, next);
        }
    }

    /// Access the underlying blocks.
    #[must_use]
    pub fn blocks(&self) -> &[TranscriptBlock] {
        &self.blocks
    }

    /// Get a block by index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&TranscriptBlock> {
        self.blocks.get(index)
    }

    /// Query blocks with a filter, returning matching (index, block) pairs.
    pub fn query(&self, filter: &BlockFilter) -> Vec<(usize, &TranscriptBlock)> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| filter.matches(block))
            .collect()
    }

    /// Paginated query: returns blocks starting from `cursor`, up to `limit`.
    pub fn page(&self, query: &BlockQuery) -> Vec<(usize, &TranscriptBlock)> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| query.filter.matches(block))
            .skip(query.cursor)
            .take(query.limit)
            .collect()
    }

    /// Count blocks matching a filter.
    #[must_use]
    pub fn count(&self, filter: &BlockFilter) -> usize {
        self.blocks.iter().filter(|b| filter.matches(b)).count()
    }

    /// Search all blocks for text, returning matching indices.
    pub fn search(&self, needle: &str) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.contains_text(needle))
            .map(|(i, _)| i)
            .collect()
    }

    /// Get all tool call blocks.
    pub fn tool_calls(&self) -> Vec<(usize, &TranscriptBlock)> {
        self.query(&BlockFilter {
            block_types: vec!["tool_call"],
            ..Default::default()
        })
    }

    /// Get tool calls with a specific status.
    pub fn tool_calls_by_status(&self, status: ToolBlockStatus) -> Vec<(usize, &TranscriptBlock)> {
        self.query(&BlockFilter {
            tool_statuses: vec![status],
            ..Default::default()
        })
    }

    /// Append a new block (for live streaming).
    pub fn push(&mut self, block: TranscriptBlock) {
        let index = self.blocks.len();
        let state = self.fold_rule.initial_state(&block);
        if state != FoldState::Expanded {
            self.fold_states.insert(index, state);
        }
        self.blocks.push(block);
    }

    /// Replace the last block if it matches the same type (for streaming accumulation).
    pub fn replace_last_if(&mut self, block: TranscriptBlock, same_type: &str) -> bool {
        if let Some(last) = self.blocks.last() {
            if last.block_type() == same_type {
                let index = self.blocks.len() - 1;
                let state = self.fold_rule.initial_state(&block);
                if state != FoldState::Expanded {
                    self.fold_states.insert(index, state);
                } else {
                    self.fold_states.remove(&index);
                }
                self.blocks[index] = block;
                return true;
            }
        }
        false
    }
}
