//! Input buffer, completion dropdown, history search, and command palette.

use super::types::SLASH_COMMANDS;

// ---------------------------------------------------------------------------
// Fuzzy matching
// ---------------------------------------------------------------------------

/// Fuzzy-match `query` against `target` using character-subsequence matching.
///
/// Returns `Some((score, matched_indices))` if all characters in `query` appear
/// in order within `target`, or `None` if not.
///
/// Scoring:
/// - +15 for a match at index 0
/// - +10 for a match at a word boundary (after `/`, `-`, `_`, or space)
/// - +5 for consecutive matches
/// - -1 per gap between matched characters
pub(crate) fn fuzzy_match(query: &str, target: &str) -> Option<(i32, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }

    let query_lower: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let target_chars: Vec<char> = target.chars().collect();
    let target_lower: Vec<char> = target.chars().flat_map(|c| c.to_lowercase()).collect();

    let mut matched_indices = Vec::with_capacity(query_lower.len());
    let mut score: i32 = 0;
    let mut qi = 0;
    let mut last_match: Option<usize> = None;

    for (ti, &tc) in target_lower.iter().enumerate() {
        if qi < query_lower.len() && tc == query_lower[qi] {
            // Scoring bonuses
            if ti == 0 {
                score += 15;
            } else if matches!(
                target_chars.get(ti.wrapping_sub(1)),
                Some('/' | '-' | '_' | ' ')
            ) {
                score += 10;
            }

            if let Some(prev) = last_match {
                if ti == prev + 1 {
                    score += 5; // consecutive
                } else {
                    score -= (ti - prev - 1) as i32; // gap penalty
                }
            }

            matched_indices.push(ti);
            last_match = Some(ti);
            qi += 1;
        }
    }

    if qi == query_lower.len() {
        Some((score, matched_indices))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Completion state
// ---------------------------------------------------------------------------

/// A single match in the completion dropdown.
#[derive(Debug, Clone)]
pub(crate) struct CompletionMatch {
    pub command: &'static str,
    pub description: &'static str,
    pub score: i32,
    pub matched_indices: Vec<usize>,
}

/// State for the slash-command completion dropdown.
#[derive(Debug)]
pub(crate) struct CompletionState {
    /// Whether the dropdown is currently visible.
    pub visible: bool,
    /// Sorted matches (best first).
    pub matches: Vec<CompletionMatch>,
    /// Currently selected index within `matches`.
    pub selected: usize,
    /// The query that produced the current matches (without leading `/`).
    pub query: String,
}

impl CompletionState {
    pub fn new() -> Self {
        Self {
            visible: false,
            matches: Vec::new(),
            selected: 0,
            query: String::new(),
        }
    }

    /// Recompute matches from the current buffer. Shows all commands when query
    /// is just `/`, fuzzy-filters otherwise.
    pub fn update(&mut self, buffer: &str) {
        let trimmed = buffer.trim_start();
        if !trimmed.starts_with('/') {
            self.dismiss();
            return;
        }

        self.query = trimmed.to_string();
        let filter = &trimmed[1..]; // text after the `/`

        let mut matches: Vec<CompletionMatch> = if filter.is_empty() {
            // Show all commands when user types just `/`
            SLASH_COMMANDS
                .iter()
                .map(|&(cmd, desc)| CompletionMatch {
                    command: cmd,
                    description: desc,
                    score: 0,
                    matched_indices: Vec::new(),
                })
                .collect()
        } else {
            SLASH_COMMANDS
                .iter()
                .filter_map(|&(cmd, desc)| {
                    // Match against the command without the leading `/`
                    let cmd_body = &cmd[1..];
                    fuzzy_match(filter, cmd_body).map(|(score, indices)| CompletionMatch {
                        command: cmd,
                        description: desc,
                        score,
                        // Offset indices by 1 to account for the leading `/`
                        matched_indices: indices.into_iter().map(|i| i + 1).collect(),
                    })
                })
                .collect()
        };

        matches.sort_by(|a, b| b.score.cmp(&a.score));

        self.visible = !matches.is_empty();
        self.matches = matches;
        // Keep selected in bounds
        if self.selected >= self.matches.len() {
            self.selected = 0;
        }
    }

    /// Move selection to the next match (wraps around).
    pub fn select_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + 1) % self.matches.len();
        }
    }

    /// Move selection to the previous match (wraps around).
    pub fn select_prev(&mut self) {
        if !self.matches.is_empty() {
            if self.selected == 0 {
                self.selected = self.matches.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    /// Accept the currently selected match. Returns the command string to populate
    /// into the buffer, or `None` if nothing is selected.
    pub fn accept(&mut self) -> Option<String> {
        if !self.visible || self.matches.is_empty() {
            return None;
        }
        let cmd = self.matches[self.selected].command.to_string();
        self.dismiss();
        Some(cmd)
    }

    /// Hide the dropdown and clear state.
    pub fn dismiss(&mut self) {
        self.visible = false;
        self.matches.clear();
        self.selected = 0;
        self.query.clear();
    }
}

/// Reverse history search state (Ctrl+R).
#[derive(Debug)]
pub(crate) struct HistorySearch {
    /// Whether the search is active.
    pub active: bool,
    /// Current search query.
    pub query: String,
    /// Index into history matches (0 = most recent match).
    pub match_idx: usize,
    /// Cached matched entries (indices into history, most-recent-first).
    pub matches: Vec<usize>,
}

impl HistorySearch {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            match_idx: 0,
            matches: Vec::new(),
        }
    }

    /// Update matches against the given history.
    pub fn update(&mut self, history: &[String]) {
        self.matches.clear();
        if self.query.is_empty() {
            return;
        }
        let q = self.query.to_lowercase();
        for (i, entry) in history.iter().enumerate().rev() {
            if entry.to_lowercase().contains(&q) {
                self.matches.push(i);
            }
        }
        // Clamp match_idx
        if !self.matches.is_empty() {
            self.match_idx = self.match_idx.min(self.matches.len() - 1);
        } else {
            self.match_idx = 0;
        }
    }

    /// Get the currently selected history entry, if any.
    pub fn current_match<'a>(&self, history: &'a [String]) -> Option<&'a str> {
        self.matches
            .get(self.match_idx)
            .and_then(|&idx| history.get(idx))
            .map(|s| s.as_str())
    }

    /// Move to the next (older) match.
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.match_idx = (self.match_idx + 1) % self.matches.len();
        }
    }

    /// Accept the current match and deactivate.
    pub fn accept(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let idx = self.matches[self.match_idx];
        self.deactivate();
        Some(idx)
    }

    /// Deactivate without accepting.
    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.match_idx = 0;
        self.matches.clear();
    }
}

/// Command palette state (Ctrl+K).
///
/// Fuzzy-searchable overlay that gives access to all slash commands and extra
/// actions from anywhere in the input.
#[derive(Debug)]
pub(crate) struct CommandPalette {
    /// Whether the palette is visible.
    pub active: bool,
    /// User's search query.
    pub query: String,
    /// Filtered matches.
    pub matches: Vec<CompletionMatch>,
    /// Currently selected index.
    pub selected: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        }
    }

    /// Open the palette, showing all commands.
    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
        self.selected = 0;
        self.refresh();
    }

    /// Refresh matches based on current query.
    pub fn refresh(&mut self) {
        if self.query.is_empty() {
            // Show all commands
            self.matches = SLASH_COMMANDS
                .iter()
                .map(|&(cmd, desc)| CompletionMatch {
                    command: cmd,
                    description: desc,
                    score: 0,
                    matched_indices: Vec::new(),
                })
                .collect();
        } else {
            let mut matches: Vec<CompletionMatch> = SLASH_COMMANDS
                .iter()
                .filter_map(|&(cmd, desc)| {
                    // Search both command name and description
                    let cmd_match = fuzzy_match(&self.query, &cmd[1..]);
                    let desc_match = fuzzy_match(&self.query, desc);
                    let best = match (cmd_match, desc_match) {
                        (Some((s1, i1)), Some((s2, _))) => {
                            if s1 >= s2 {
                                Some((s1, i1))
                            } else {
                                Some((s2, Vec::new()))
                            }
                        }
                        (Some(m), None) | (None, Some(m)) => Some(m),
                        (None, None) => None,
                    };
                    best.map(|(score, indices)| CompletionMatch {
                        command: cmd,
                        description: desc,
                        score,
                        matched_indices: indices,
                    })
                })
                .collect();
            matches.sort_by(|a, b| b.score.cmp(&a.score));
            self.matches = matches;
        }
        // Clamp selected
        if !self.matches.is_empty() {
            self.selected = self.selected.min(self.matches.len() - 1);
        } else {
            self.selected = 0;
        }
    }

    /// Type a character into the search query.
    pub fn type_char(&mut self, ch: char) {
        self.query.push(ch);
        self.refresh();
    }

    /// Backspace in the search query.
    pub fn backspace(&mut self) {
        self.query.pop();
        self.refresh();
    }

    pub fn select_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + 1) % self.matches.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + self.matches.len() - 1) % self.matches.len();
        }
    }

    /// Accept the selected command, returning it.
    pub fn accept(&mut self) -> Option<String> {
        if self.matches.is_empty() {
            self.dismiss();
            return None;
        }
        let cmd = self.matches[self.selected].command.to_string();
        self.dismiss();
        Some(cmd)
    }

    pub fn dismiss(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.selected = 0;
    }
}

/// Input buffer state.
#[derive(Debug)]
pub(crate) struct InputState {
    /// Current input text.
    pub buffer: String,
    /// Cursor position (byte offset).
    pub cursor: usize,
    /// Command history.
    pub history: Vec<String>,
    /// History navigation index (None = current input).
    pub history_idx: Option<usize>,
    /// Saved current input when navigating history.
    pub saved_input: String,
    /// Slash-command completion dropdown state.
    pub completion: CompletionState,
    /// Reverse history search (Ctrl+R).
    pub search: HistorySearch,
    /// Command palette (Ctrl+K).
    pub palette: CommandPalette,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_idx: None,
            saved_input: String::new(),
            completion: CompletionState::new(),
            search: HistorySearch::new(),
            palette: CommandPalette::new(),
        }
    }

    /// Returns the ghost suggestion suffix if available.
    ///
    /// Active when: buffer is non-empty, cursor is at end, buffer does NOT
    /// start with `/`, and a history entry matches the current buffer as prefix.
    /// Searches most-recent-first.
    pub fn ghost_suggestion(&self) -> Option<&str> {
        if self.buffer.is_empty()
            || self.cursor != self.buffer.len()
            || self.buffer.starts_with('/')
            || self.completion.visible
        {
            return None;
        }

        for entry in self.history.iter().rev() {
            if entry.len() > self.buffer.len() && entry.starts_with(&self.buffer) {
                return Some(&entry[self.buffer.len()..]);
            }
        }
        None
    }

    /// Accept the current ghost suggestion, appending it to the buffer.
    /// Returns true if a suggestion was accepted.
    pub fn accept_ghost(&mut self) -> bool {
        if let Some(suffix) = self.ghost_suggestion().map(|s| s.to_string()) {
            self.buffer.push_str(&suffix);
            self.cursor = self.buffer.len();
            true
        } else {
            false
        }
    }

    /// Insert a newline at the cursor position.
    pub fn insert_newline(&mut self) {
        self.buffer.insert(self.cursor, '\n');
        self.cursor += 1;
    }

    /// Number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.buffer.lines().count().max(1)
    }

    /// Get the current line index and column offset for the cursor.
    pub fn cursor_line_col(&self) -> (usize, usize) {
        let before = &self.buffer[..self.cursor];
        let line = before.matches('\n').count();
        let col = before
            .rfind('\n')
            .map(|i| self.cursor - i - 1)
            .unwrap_or(self.cursor);
        (line, col)
    }

    pub fn insert(&mut self, ch: char) {
        self.buffer.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.buffer[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.buffer.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.buffer.len() {
            let next = self.buffer[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.buffer.len());
            self.buffer.drain(self.cursor..next);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.buffer[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            self.cursor = self.buffer[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.buffer.len());
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.buffer.len();
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            None => {
                self.saved_input = self.buffer.clone();
                self.history.len() - 1
            }
            Some(0) => return,
            Some(i) => i - 1,
        };
        self.history_idx = Some(idx);
        self.buffer = self.history[idx].clone();
        self.cursor = self.buffer.len();
    }

    pub fn history_down(&mut self) {
        let idx = match self.history_idx {
            None => return,
            Some(i) => i + 1,
        };
        if idx >= self.history.len() {
            self.history_idx = None;
            self.buffer = std::mem::take(&mut self.saved_input);
        } else {
            self.history_idx = Some(idx);
            self.buffer = self.history[idx].clone();
        }
        self.cursor = self.buffer.len();
    }

    pub fn submit(&mut self) -> String {
        let text = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        self.history_idx = None;
        if !text.trim().is_empty() {
            self.history.push(text.clone());
        }
        text
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.trim().is_empty()
    }
}
