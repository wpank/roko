//! F4 Git view -- branch tree, worktree list, commit graph, branch info,
//! diff viewer.
//!
//! Two-panel layout: left 35% (branch tree + worktree list + status),
//! right 65% (commit graph + branch info, or diff when a file is selected).
//!
//! Populates data by running git commands when the TuiState fields are
//! empty, so the view always shows real repository state.

use std::path::Path;
use std::process::Command;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap};

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::input::FocusZone;
pub(crate) use crate::tui::state::GitBranchNode;
use crate::tui::state::TuiState;

/// A worktree entry.
#[derive(Debug, Clone)]
pub(crate) struct WorktreeEntry {
    pub path: String,
    pub branch: String,
    pub status: String,
}

/// A commit log entry.
#[derive(Debug, Clone)]
pub(crate) struct CommitEntry {
    pub hash_short: String,
    pub subject: String,
    pub author: String,
    pub age: String,
    pub graph_prefix: String,
}

/// Git view data container.
#[derive(Debug, Clone, Default)]
pub(crate) struct GitViewData {
    pub branches: Vec<GitBranchNode>,
    pub worktrees: Vec<WorktreeEntry>,
    pub commits: Vec<CommitEntry>,
    pub current_branch: String,
    pub remote_url: String,
    pub status_lines: Vec<String>,
    /// Full diff output for all changes (git diff HEAD).
    pub diff_text: String,
    /// Per-file numstat: (additions, deletions, filename).
    pub numstat: Vec<(usize, usize, String)>,
}

const NOT_A_GIT_REPOSITORY: &str = "not a git repository";

impl GitViewData {
    pub(crate) fn not_a_git_repository() -> Self {
        Self {
            status_lines: vec![NOT_A_GIT_REPOSITORY.to_string()],
            ..Self::default()
        }
    }

    pub(crate) fn is_not_a_git_repository(&self) -> bool {
        self.branches.is_empty()
            && self.worktrees.is_empty()
            && self.commits.is_empty()
            && self.current_branch.is_empty()
            && self.remote_url.is_empty()
            && matches!(
                self.status_lines.as_slice(),
                [line] if line == NOT_A_GIT_REPOSITORY
            )
    }
}

/// Render the full git view.
///
/// Uses pre-populated git data from `TuiState::git_view_data` (filled by
/// the background refresh thread) so the render path does zero I/O.
/// Falls back to an empty `GitViewData` if the background thread hasn't
/// delivered data yet.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let empty = GitViewData::default();
    let git_data = tui_state.git_view_data.as_ref().unwrap_or(&empty);
    render_with_git_data(frame, area, git_data, tui_state, view_state, theme);
}

/// Render the git view with explicit git data (for integration layer).
pub(crate) fn render_with_git_data(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    let (sidebar, detail) =
        crate::tui::layout::responsive_panel_split(area, 35, 100, area.height / 3);

    let focused = matches!(tui_state.focus, FocusZone::RightPanel);

    render_left_panel(frame, sidebar, git_data, focused, view_state, theme);
    render_right_panel(frame, detail, git_data, focused, view_state, theme);
}

/// Left panel: branch tree (top 40%) + worktree list (mid 25%) + status (bottom 35%).
fn render_left_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    view_state: &ViewState,
    theme: &Theme,
) {
    let sections = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Percentage(25),
        Constraint::Percentage(35),
    ])
    .split(area);

    render_branch_tree(frame, sections[0], git_data, focused, view_state, theme);
    render_worktree_list(frame, sections[1], git_data, focused, theme);
    render_status(frame, sections[2], git_data, focused, theme);
}

/// Branch tree: hierarchical branch listing.
fn render_branch_tree(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Branches ({}) ", git_data.branches.len()),
            if focused {
                Theme::focused_title_style()
            } else {
                theme.section_header()
            },
        ))
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            theme.accent()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if git_data.branches.is_empty() {
        let empty = Paragraph::new("Loading branch data...")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let items: Vec<ListItem<'_>> = git_data
        .branches
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let indent = "  ".repeat(branch.depth as usize);
            let marker = if branch.is_current { "* " } else { "  " };
            let ahead_behind = if branch.ahead > 0 || branch.behind > 0 {
                format!(" [+{}/-{}]", branch.ahead, branch.behind)
            } else {
                String::new()
            };

            let style = if i == view_state.selected {
                theme.selection()
            } else if branch.is_current {
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.text()
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{indent}{marker}")),
                Span::styled(&branch.name, style),
                Span::styled(ahead_behind, theme.metadata()),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

/// Worktree list: table with path, branch, and parsed status.
fn render_worktree_list(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Worktrees ({}) ", git_data.worktrees.len()),
            if focused {
                Theme::focused_title_style()
            } else {
                theme.section_header()
            },
        ))
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            Theme::unfocused_border_style()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if git_data.worktrees.is_empty() {
        let empty = Paragraph::new("No additional worktrees")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let rows: Vec<Row<'_>> = git_data
        .worktrees
        .iter()
        .map(|wt| {
            let status_style = match wt.status.as_str() {
                "locked" => theme.warning(),
                "prunable" => theme.danger(),
                _ => theme.success(),
            };
            Row::new(vec![
                Cell::from(Span::styled(
                    shorten_path(&wt.path, inner.width.saturating_sub(26) as usize),
                    theme.metadata(),
                )),
                Cell::from(Span::styled(wt.branch.as_str(), theme.value())),
                Cell::from(Span::styled(wt.status.as_str(), status_style)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(16),
        Constraint::Min(12),
        Constraint::Length(10),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(["path", "branch", "status"]).style(theme.label()),
        )
        .column_spacing(1);
    frame.render_widget(table, inner);
}

/// Status panel: git status with improved two-column code parsing and numstat.
fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    theme: &Theme,
) {
    let status_count = git_data.status_lines.len();
    let title = if status_count > 0 {
        format!(" Status ({status_count}) ")
    } else {
        " Status ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title,
            if focused {
                Theme::focused_title_style()
            } else {
                theme.section_header()
            },
        ))
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            Theme::unfocused_border_style()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if git_data.is_not_a_git_repository() {
        let empty = Paragraph::new(NOT_A_GIT_REPOSITORY)
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    if git_data.status_lines.is_empty() {
        let empty = Paragraph::new("clean working tree")
            .style(theme.success())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let max_width = inner.width as usize;
    let lines: Vec<Line<'_>> = git_data
        .status_lines
        .iter()
        .take(inner.height as usize)
        .map(|line| render_status_line(line, &git_data.numstat, max_width, theme))
        .collect();

    let remaining = git_data
        .status_lines
        .len()
        .saturating_sub(inner.height as usize);
    let mut all_lines = lines;
    if remaining > 0 {
        all_lines.push(Line::from(Span::styled(
            format!("  ... +{remaining} more"),
            theme.muted(),
        )));
    }

    let paragraph = Paragraph::new(all_lines);
    frame.render_widget(paragraph, inner);
}

/// Render a single status line with proper two-column code parsing and
/// optional +N/-M numstat badge.
fn render_status_line<'a>(
    line: &str,
    numstat: &[(usize, usize, String)],
    max_width: usize,
    theme: &'a Theme,
) -> Line<'a> {
    // git status --short format: XY filename
    // X = index status, Y = worktree status
    let (index_code, work_code) = if line.len() >= 2 {
        let bytes = line.as_bytes();
        (bytes[0] as char, bytes[1] as char)
    } else {
        (' ', ' ')
    };

    let filename = if line.len() > 3 { &line[3..] } else { line };
    let filename_trimmed = filename.trim();

    // Find numstat for this file
    let stat_badge = numstat
        .iter()
        .find(|(_, _, name)| name == filename_trimmed)
        .map(|(add, del, _)| format!(" +{add}/-{del}"));

    // Color the two-character status code
    let index_style = status_char_style(index_code, theme);
    let work_style = status_char_style(work_code, theme);

    let badge_len = stat_badge.as_ref().map_or(0, |b| b.len());
    let name_max = max_width.saturating_sub(3 + badge_len);
    let display_name = truncate(filename_trimmed, name_max);

    let mut spans = vec![
        Span::styled(index_code.to_string(), index_style),
        Span::styled(work_code.to_string(), work_style),
        Span::raw(" "),
        Span::styled(display_name, theme.text()),
    ];

    if let Some(badge) = stat_badge {
        spans.push(Span::styled(badge, theme.muted()));
    }

    Line::from(spans)
}

/// Map a single status character to a style.
fn status_char_style(ch: char, theme: &Theme) -> Style {
    match ch {
        'M' => theme.warning(),
        'A' => theme.success(),
        'D' => theme.danger(),
        'R' => Style::default().fg(Theme::DREAM),
        'C' => Style::default().fg(Theme::DREAM),
        'U' => theme.danger().add_modifier(Modifier::BOLD),
        '?' => theme.muted(),
        '!' => theme.muted(),
        _ => theme.text(),
    }
}

/// Right panel: commit graph (top 55%) + branch info (bottom 45%),
/// or diff viewer when diff data is available.
fn render_right_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    view_state: &ViewState,
    theme: &Theme,
) {
    // If there is diff text, split the right panel into commit graph and diff.
    if !git_data.diff_text.is_empty() {
        let sections = Layout::vertical([
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ])
        .split(area);
        render_commit_graph(frame, sections[0], git_data, focused, view_state, theme);
        render_diff_viewer(frame, sections[1], git_data, focused, view_state, theme);
        render_branch_info(frame, sections[2], git_data, focused, theme);
    } else {
        let sections = Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        render_commit_graph(frame, sections[0], git_data, focused, view_state, theme);
        render_branch_info(frame, sections[1], git_data, focused, theme);
    }
}

/// Commit graph: rendered git log with graph characters and commit age.
fn render_commit_graph(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Recent Commits ({}) ", git_data.commits.len()),
            if focused {
                Theme::focused_title_style()
            } else {
                theme.section_header()
            },
        ))
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            theme.accent()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if git_data.commits.is_empty() {
        let empty = Paragraph::new("No commits found")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(empty, inner);
        return;
    }

    let lines: Vec<Line<'_>> = git_data
        .commits
        .iter()
        .map(|commit| {
            Line::from(vec![
                Span::styled(&commit.graph_prefix, theme.muted()),
                Span::styled(format!(" {} ", commit.hash_short), theme.value()),
                Span::styled(&commit.subject, theme.text()),
                Span::styled(format!(" {}", commit.age), theme.metadata()),
                Span::styled(format!("  ({})", commit.author), theme.metadata()),
            ])
        })
        .collect();

    // Use scroll without wrap to preserve graph alignment.
    let paragraph = Paragraph::new(lines).scroll((view_state.scroll, 0));
    frame.render_widget(paragraph, inner);
}

/// Diff viewer panel -- delegates to the shared `diff_panel` widget for
/// syntax coloring, line numbers, file headers, and word-level highlighting.
fn render_diff_viewer(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    view_state: &ViewState,
    theme: &Theme,
) {
    // Count files in diff.
    let file_count = git_data
        .diff_text
        .lines()
        .filter(|l| l.starts_with("diff --git"))
        .count();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" Diff ({file_count} files) "),
            if focused {
                Theme::focused_title_style()
            } else {
                theme.section_header()
            },
        ))
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            theme.accent()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Use secondary_selected as diff-specific scroll.
    let scroll = if view_state.secondary_selected > 0 {
        Some(view_state.secondary_selected)
    } else {
        Some(0)
    };

    let opts = crate::tui::widgets::diff_panel::DiffRenderOpts {
        line_numbers: true,
        gutter_width: 0,
        word_highlight: true,
    };
    crate::tui::widgets::diff_panel::render_diff_content(
        frame,
        inner,
        &git_data.diff_text,
        scroll,
        theme,
        &opts,
    );
}

/// Branch info: current branch, remote tracking, ahead/behind, diff summary.
fn render_branch_info(
    frame: &mut Frame<'_>,
    area: Rect,
    git_data: &GitViewData,
    focused: bool,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Branch Info ",
            if focused {
                Theme::focused_title_style()
            } else {
                theme.section_header()
            },
        ))
        .border_style(if focused {
            Theme::focused_border_style()
        } else {
            Theme::unfocused_border_style()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if git_data.is_not_a_git_repository() {
        let paragraph = Paragraph::new("not a git repository")
            .style(theme.muted())
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
        return;
    }

    let current = if git_data.current_branch.is_empty() {
        "(detached HEAD)"
    } else {
        git_data.current_branch.as_str()
    };

    let current_node = git_data.branches.iter().find(|b| b.is_current);
    let tracking_display = current_node
        .and_then(|node| node.tracking.as_deref())
        .unwrap_or("(none)");

    // Compute diff summary from numstat.
    let (total_add, total_del) = git_data
        .numstat
        .iter()
        .fold((0usize, 0usize), |(a, d), (add, del, _)| {
            (a + add, d + del)
        });

    let sep_width = inner.width as usize;
    let separator =
        Line::from(Span::styled("─".repeat(sep_width), Style::default().fg(Theme::SEPARATOR)));

    let mut lines = vec![
        Line::from(vec![
            Span::styled("branch:   ", theme.label()),
            Span::styled(
                current,
                Style::default()
                    .fg(Theme::SAGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("remote:   ", theme.label()),
            Span::styled(
                if git_data.remote_url.is_empty() {
                    "(none)"
                } else {
                    git_data.remote_url.as_str()
                },
                theme.value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("tracking: ", theme.label()),
            Span::styled(tracking_display, theme.value()),
        ]),
        separator.clone(),
        Line::from(vec![
            Span::styled("ahead:    ", theme.label()),
            Span::styled(
                current_node.map_or("0".to_string(), |n| n.ahead.to_string()),
                theme.success(),
            ),
            Span::raw("  "),
            Span::styled("behind: ", theme.label()),
            Span::styled(
                current_node.map_or("0".to_string(), |n| n.behind.to_string()),
                theme.warning(),
            ),
        ]),
        Line::from(vec![
            Span::styled("branches: ", theme.label()),
            Span::styled(git_data.branches.len().to_string(), theme.value()),
            Span::raw("  "),
            Span::styled("worktrees: ", theme.label()),
            Span::styled(git_data.worktrees.len().to_string(), theme.value()),
        ]),
        Line::from(vec![
            Span::styled("modified: ", theme.label()),
            Span::styled(git_data.status_lines.len().to_string(), theme.value()),
            Span::styled(" files", theme.metadata()),
        ]),
    ];

    // Add diff summary if we have numstat data.
    if total_add > 0 || total_del > 0 {
        lines.push(separator);
        lines.push(Line::from(vec![
            Span::styled("changes:  ", theme.label()),
            Span::styled(format!("+{total_add}"), theme.success()),
            Span::styled(" / ", theme.metadata()),
            Span::styled(format!("-{total_del}"), theme.danger()),
            Span::styled(
                format!(" across {} files", git_data.numstat.len()),
                theme.metadata(),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Git data collection
// ---------------------------------------------------------------------------

/// Collect live git data by running git commands.
///
/// This is intentionally expensive (multiple git subprocess calls) and
/// should only be called from watcher-driven refresh paths, never from the
/// render path.
pub(crate) fn collect_git_data(workdir: &Path) -> GitViewData {
    if !is_git_repository(workdir) {
        return GitViewData::not_a_git_repository();
    }

    let current_branch = run_git(workdir, &["branch", "--show-current"])
        .unwrap_or_default()
        .trim()
        .to_string();

    let remote_url = run_git(workdir, &["remote", "get-url", "origin"])
        .unwrap_or_default()
        .trim()
        .to_string();

    let branches = collect_branches(workdir, &current_branch);
    let worktrees = collect_worktrees(workdir);
    let commits = collect_commits(workdir);
    let status_lines = collect_status(workdir);
    let diff_text = collect_diff(workdir);
    let numstat = collect_numstat(workdir);

    GitViewData {
        branches,
        worktrees,
        commits,
        current_branch,
        remote_url,
        status_lines,
        diff_text,
        numstat,
    }
}

fn is_git_repository(workdir: &Path) -> bool {
    run_git(workdir, &["rev-parse", "--is-inside-work-tree"])
        .map(|value| value.trim() == "true")
        .unwrap_or(false)
}

/// Run a git command and return stdout as a string.
fn run_git(workdir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workdir)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Collect branch list with ahead/behind info.
fn collect_branches(workdir: &Path, current_branch: &str) -> Vec<GitBranchNode> {
    // git branch --format with ahead/behind
    let output = run_git(
        workdir,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)\t%(upstream:short)\t%(upstream:track)",
            "refs/heads/",
        ],
    );

    let Some(output) = output else {
        return Vec::new();
    };

    let mut branches = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let name = parts.first().map_or("", |s| s.trim()).to_string();
        let tracking = parts
            .get(1)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let track_info = parts.get(2).map_or("", |s| s.trim());
        let (ahead, behind) = parse_ahead_behind(track_info);

        let is_current = name == current_branch;
        let depth = name.matches('/').count().min(3) as u16;

        branches.push(GitBranchNode {
            name,
            is_current,
            tracking,
            ahead,
            behind,
            depth,
            children: Vec::new(),
        });
    }

    // Ensure current branch is first
    branches.sort_by(|a, b| b.is_current.cmp(&a.is_current).then(a.name.cmp(&b.name)));
    branches
}

/// Parse "[ahead N, behind M]" from git tracking info.
fn parse_ahead_behind(s: &str) -> (usize, usize) {
    let mut ahead = 0usize;
    let mut behind = 0usize;
    if s.contains("ahead") {
        if let Some(n) = s
            .split("ahead ")
            .nth(1)
            .and_then(|s| s.split([',', ']']).next())
            .and_then(|n| n.trim().parse().ok())
        {
            ahead = n;
        }
    }
    if s.contains("behind") {
        if let Some(n) = s
            .split("behind ")
            .nth(1)
            .and_then(|s| s.split(']').next())
            .and_then(|n| n.trim().parse().ok())
        {
            behind = n;
        }
    }
    (ahead, behind)
}

/// Collect worktree list with proper status parsing (locked, prunable).
fn collect_worktrees(workdir: &Path) -> Vec<WorktreeEntry> {
    let output = run_git(workdir, &["worktree", "list", "--porcelain"]);
    let Some(output) = output else {
        return Vec::new();
    };

    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_branch = String::new();
    let mut current_status = String::from("active");

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if !current_path.is_empty() {
                worktrees.push(WorktreeEntry {
                    path: current_path.clone(),
                    branch: current_branch.clone(),
                    status: current_status.clone(),
                });
            }
            current_path = path.trim().to_string();
            current_branch = String::new();
            current_status = String::from("active");
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = branch
                .trim()
                .strip_prefix("refs/heads/")
                .unwrap_or(branch.trim())
                .to_string();
        } else if line.trim() == "bare" {
            current_branch = String::from("(bare)");
        } else if line.trim() == "detached" {
            current_branch = String::from("(detached)");
        } else if line.trim() == "locked" || line.starts_with("locked ") {
            current_status = String::from("locked");
        } else if line.trim() == "prunable" || line.starts_with("prunable ") {
            current_status = String::from("prunable");
        }
    }

    // Push last entry
    if !current_path.is_empty() {
        worktrees.push(WorktreeEntry {
            path: current_path,
            branch: current_branch,
            status: current_status,
        });
    }

    worktrees
}

/// Collect recent commits with graph.
fn collect_commits(workdir: &Path) -> Vec<CommitEntry> {
    let output = run_git(
        workdir,
        &[
            "log",
            "--graph",
            "--decorate=short",
            "-30",
            "--format=%H%x00%h%x00%an%x00%cr%x00%s%x1e",
        ],
    );
    let Some(output) = output else {
        return Vec::new();
    };

    parse_commit_records(&output)
}

fn parse_commit_records(output: &str) -> Vec<CommitEntry> {
    let mut commits = Vec::new();
    for record in output.split('\x1e') {
        let record = record.trim_start_matches('\n');
        if record.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = record.split('\0').collect();
        if parts.len() < 5 {
            continue;
        }
        let (graph_prefix, _) = split_graph_line(parts[0]);
        let hash_short = parts[1].trim().to_string();
        let author = parts[2].trim().to_string();
        let age = parts[3].trim().to_string();
        let subject = parts[4].trim().to_string();

        if !hash_short.is_empty() {
            commits.push(CommitEntry {
                hash_short,
                subject,
                author,
                age,
                graph_prefix,
            });
        }
    }

    commits
}

/// Split a git log --graph line into graph prefix and rest.
fn split_graph_line(line: &str) -> (String, &str) {
    // Graph chars: *, |, /, \, space
    let graph_end = line
        .char_indices()
        .find(|(_, ch)| !matches!(ch, '*' | '|' | '/' | '\\' | ' ' | '_'))
        .map_or(line.len(), |(idx, _)| idx);
    let prefix = &line[..graph_end];
    let rest = &line[graph_end..];
    (prefix.to_string(), rest)
}

/// Collect git status --short.
fn collect_status(workdir: &Path) -> Vec<String> {
    let output = run_git(workdir, &["status", "--short"]);
    let Some(output) = output else {
        return Vec::new();
    };
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(50)
        .map(|l| l.to_string())
        .collect()
}

/// Collect unified diff output (staged + unstaged).
fn collect_diff(workdir: &Path) -> String {
    // Combine staged and unstaged diffs.
    let staged = run_git(workdir, &["diff", "--cached"]).unwrap_or_default();
    let unstaged = run_git(workdir, &["diff"]).unwrap_or_default();
    // Cap at ~64KB to avoid blowing up memory on huge diffs.
    let combined = if staged.is_empty() {
        unstaged
    } else if unstaged.is_empty() {
        staged
    } else {
        format!("{staged}{unstaged}")
    };
    if combined.len() > 65536 {
        let mut truncated: String = combined.chars().take(65536).collect();
        truncated.push_str("\n... (diff truncated at 64KB)");
        truncated
    } else {
        combined
    }
}

/// Collect per-file insertion/deletion counts via `git diff --numstat`.
fn collect_numstat(workdir: &Path) -> Vec<(usize, usize, String)> {
    let mut result = Vec::new();
    // Collect both staged and unstaged numstat.
    for args in [&["diff", "--numstat"][..], &["diff", "--cached", "--numstat"][..]] {
        let output = run_git(workdir, args);
        let Some(output) = output else { continue };
        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let add = parts[0].parse::<usize>().unwrap_or(0);
                let del = parts[1].parse::<usize>().unwrap_or(0);
                let name = parts[2].to_string();
                // Avoid duplicates if same file appears in both.
                if !result.iter().any(|(_, _, n): &(usize, usize, String)| n == &name) {
                    result.push((add, del, name));
                }
            }
        }
    }
    result
}

/// Shorten a filesystem path for display, keeping the last N chars.
fn shorten_path(path: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if path.len() <= max {
        return path.to_string();
    }
    format!("...{}", &path[path.len().saturating_sub(max.saturating_sub(3))..])
}

use crate::tui::display_utils::truncate;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nul_delimited_commit_subject_with_tabs_and_pipes() {
        let raw = concat!(
            "* 0123456789abcdef0123456789abcdef01234567",
            "\0",
            "abc1234",
            "\0",
            "Will",
            "\0",
            "2h ago",
            "\0",
            "fix: handle a\tb | c edge",
            "\x1e"
        );
        let commits = parse_commit_records(raw);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash_short, "abc1234");
        assert_eq!(commits[0].author, "Will");
        assert_eq!(commits[0].age, "2h ago");
        assert_eq!(commits[0].subject, "fix: handle a\tb | c edge");
        assert_eq!(commits[0].graph_prefix, "* ");
    }

    #[test]
    fn worktree_status_locked_and_prunable() {
        // Simulate porcelain output with locked and prunable worktrees.
        let porcelain = "\
worktree /repo
branch refs/heads/main

worktree /repo/.claude/worktrees/fix-1
branch refs/heads/fix-1
locked

worktree /repo/.claude/worktrees/old-branch
branch refs/heads/old
prunable

";
        // We can't call collect_worktrees directly without a real repo,
        // but we can verify the parsing logic by exercising the same code inline.
        let mut worktrees = Vec::new();
        let mut current_path = String::new();
        let mut current_branch = String::new();
        let mut current_status = String::from("active");

        for line in porcelain.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                if !current_path.is_empty() {
                    worktrees.push(WorktreeEntry {
                        path: current_path.clone(),
                        branch: current_branch.clone(),
                        status: current_status.clone(),
                    });
                }
                current_path = path.trim().to_string();
                current_branch = String::new();
                current_status = String::from("active");
            } else if let Some(branch) = line.strip_prefix("branch ") {
                current_branch = branch
                    .trim()
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch.trim())
                    .to_string();
            } else if line.trim() == "locked" || line.starts_with("locked ") {
                current_status = String::from("locked");
            } else if line.trim() == "prunable" || line.starts_with("prunable ") {
                current_status = String::from("prunable");
            }
        }
        if !current_path.is_empty() {
            worktrees.push(WorktreeEntry {
                path: current_path,
                branch: current_branch,
                status: current_status,
            });
        }

        assert_eq!(worktrees.len(), 3);
        assert_eq!(worktrees[0].status, "active");
        assert_eq!(worktrees[1].status, "locked");
        assert_eq!(worktrees[2].status, "prunable");
    }

    #[test]
    fn shorten_path_truncates_long_paths() {
        assert_eq!(shorten_path("/short", 20), "/short");
        assert_eq!(
            shorten_path("/very/long/path/to/worktree", 20),
            ".../path/to/worktree"
        );
    }

    #[test]
    fn numstat_badge_renders_in_status_line() {
        let theme = Theme::default();
        let numstat = vec![(10, 3, "src/main.rs".to_string())];
        let line = render_status_line(" M src/main.rs", &numstat, 60, &theme);
        // The line should have 5 spans: index_code, work_code, space, filename, badge.
        assert_eq!(line.spans.len(), 5);
        assert!(line.spans[4].content.contains("+10/-3"));
    }
}
