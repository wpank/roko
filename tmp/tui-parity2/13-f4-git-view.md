# F4 Git View Audit

**Date:** 2026-09-01
**Files reviewed:**
- `crates/roko-cli/src/tui/views/git_view.rs` (790 lines)
- `crates/roko-cli/src/tui/widgets/diff_panel.rs` (89 lines)
- `crates/roko-cli/src/tui/fs_watch.rs` (294 lines)
- `crates/roko-cli/src/tui/git_watch.rs` (410 lines)
- `crates/roko-cli/src/tui/app.rs` (git bg data collection + drain)
- `crates/roko-cli/src/tui/views/dashboard_view.rs` (diff sub-tab, git summary)
- `crates/roko-cli/src/tui/input.rs` (handle_git_key)
- `crates/roko-cli/src/tui/state.rs` (git fields)

---

## 1. Branch information: shown clearly?

**Verdict: GOOD -- clear and well-structured.**

The view has a dedicated **Branch Info** panel (bottom 40% of right panel) that displays:
- Current branch name (accent bold style) or `(detached HEAD)`
- Remote URL (or `(none)`)
- Tracking branch (or `(none)`)
- Ahead/behind counts with semantic coloring (green for ahead, warning for behind)
- Total branch count and worktree count
- Modified file count

The left panel's **branch tree** (top 50%) shows all local branches sorted by committer date
with the current branch first. Each branch shows:
- Indentation based on namespace depth (e.g. `feature/x` is indented)
- `*` marker for current branch
- `[+N/-M]` ahead/behind badge (muted style)
- Selection highlighting via `theme.selection()`

The branch collection uses `git for-each-ref --sort=-committerdate` which gives a useful
recency ordering. Hierarchical depth is derived from `/` separators (`name.matches('/').count()`),
capped at 3.

**Gap:** Branch depth is flat -- all branches are rendered in a single list with indentation.
There is no collapsible tree structure; the `children: Vec<GitBranchNode>` field on
`GitBranchNode` is always empty. The `DrillIn`/`DrillOut` actions just move the cursor up/down
rather than expanding/collapsing branch groups.

---

## 2. Diff display: syntax highlighted? scrollable? side-by-side or unified?

**Verdict: PARTIAL -- exists but is on F1, not F4.**

The `diff_panel.rs` widget provides:
- **Unified diff format** (not side-by-side)
- **Color-coded lines:**
  - `diff --git` / `index` headers: accent bold
  - `@@` hunk headers: cyan bold
  - `+++` file headers: green bold (success)
  - `---` file headers: red bold (danger)
  - `+` additions: green (success)
  - `-` deletions: red (danger)
  - Context lines: default foreground
- **Scrollable** via `Paragraph::scroll()` with configurable offset
- Auto-scroll to end when no explicit scroll position

However, this diff panel is **only used on the F1 Dashboard** (in `render_sub_diff` of
`dashboard_view.rs`), not on the F4 Git tab. The F4 tab shows git `status --short` output
in the Status panel but has **no inline diff viewer**.

The diff data itself (`git_diff`) is loaded via `load_dashboard_git_diff()` in
`dashboard.rs`, which runs `git diff --cached` (staged) or `git diff HEAD` (unstaged).
This is part of the `DashboardData` snapshot loader, not the git background refresh thread.

**Gap:** The F4 Git tab has no diff display at all. The diff panel exists as a reusable widget
but is only wired into F1. Selecting a file in the status list does nothing -- there is no
per-file diff drill-down.

---

## 3. Commit log: useful format?

**Verdict: GOOD -- proper git graph with metadata.**

The **Commit Graph** panel (top 60% of right panel) shows the last 30 commits collected via:
```
git log --graph --decorate=short -30 --format=%H%x00%h%x00%an%x00%cr%x00%s
```

Each commit line renders:
- Graph prefix characters (`*`, `|`, `/`, `\`, space) in muted style
- Short hash in warning/yellow color
- Subject in default text color
- Author name in muted/parenthesized style

The graph prefix is parsed via `split_graph_line()` which correctly identifies graph-drawing
characters and separates them from the commit data. Records are NUL-delimited to handle
subjects containing pipes and tabs (verified by unit test).

**Gap:** The commit age (`cr` = relative date) is collected but **never rendered** in the
commit graph panel. Each `CommitEntry` has an `age` field that is populated but not displayed
in the `render_commit_graph` function. Only the top-level summary uses it.

**Gap:** No ref decorations visible. Although `--decorate=short` is passed to git log, the
decoration output is not captured in the format string (the format uses `%s` for subject,
not `%d` for decorations). Branch/tag labels on commits are invisible.

**Gap:** Scrolling uses `Wrap { trim: false }` on the `Paragraph`, which means long subjects
can wrap across lines and push subsequent commits down, making the graph characters misalign.

---

## 4. Worktree information: visible?

**Verdict: GOOD -- dedicated panel with structured data.**

The **Worktree List** panel (middle 25% of left panel) shows a table with:
- Path (truncated to 24 chars)
- Branch name (stripped of `refs/heads/` prefix; shows `(bare)` or `(detached)`)
- Status (currently hardcoded to `"active"`)

Data is collected via `git worktree list --porcelain` and parsed line-by-line.

**Gap:** The status column always shows `"active"` -- there is no detection of locked,
prunable, or dirty worktree states. The porcelain format emits `locked` and `prunable` lines
that are not parsed.

**Gap:** Path truncation to 24 characters can make long worktree paths unreadable. There is
no tooltip or full-path display on selection.

---

## 5. File change list: clear additions/deletions/modifications?

**Verdict: PARTIAL -- status codes are color-coded but minimal.**

The **Status** panel (bottom 25% of left panel) renders `git status --short` lines with:
- `M` or ` M` (modified): warning/yellow
- `A` or `??` (added/untracked): green/success
- `D` (deleted): red/danger
- Other lines: default text

Lines are truncated to 40 characters. A `... +N more` overflow indicator appears when lines
exceed the panel height. Output is capped at 50 lines from `collect_status()`.

**Gap:** The status display is just raw `git status --short` output. There is no structured
file tree, no grouping by directory, no file icons, and no way to select a file and see its
diff. The two-letter status codes (`MM`, `AM`, `R `, etc.) are not fully decoded -- only the
first character is checked, so renamed files (`R`) and copied files (`C`) get default styling.

**Gap:** Staged vs unstaged distinction is not visible. `git status --short` uses the
two-column format (index/worktree) but only the first character is checked for coloring.
A file that is `M ` (staged modification) and ` M` (unstaged modification) would get the
same treatment.

---

## 6. Does the diff update in real-time as agents make changes?

**Verdict: YES for branch/commit/status data; NO for diffs.**

The F4 Git tab uses a dedicated **git watcher** (`git_watch.rs`) that is separate from the
`.roko/` filesystem watcher. The git watcher:

1. **Discovers the git admin directory** -- handles both normal repos (`.git/` directory) and
   linked worktrees (`.git` file with `gitdir:` pointer, plus `commondir` resolution).
2. **Watches via `notify`** -- watches `.git/` recursively, plus `refs/` and the common dir
   for worktrees. Falls back to 500ms metadata polling if notify fails.
3. **Debounces** -- 500ms debounce window coalesces rapid changes.
4. **Triggers background collection** -- when the watcher fires, the app spawns a
   `tui-git-collect` thread that runs all git subcommands and sends results back via a
   `sync_channel(1)`. Only one background job runs at a time (guarded by `git_bg_rx`).
5. **Generation counter** -- monotonic `git_bg_generation` prevents stale results from
   overwriting newer data.

So commits, branches, worktrees, and status lines update automatically as agents work.

**Gap:** The diff data used by the F1 dashboard's diff panel is loaded via the `DashboardData`
snapshot path, not the git watcher. It refreshes on the `.roko/` filesystem watcher cadence
(200ms debounce), not git-event driven. The F4 tab has no diff display at all.

---

## 7. Is there a file tree showing changed files?

**Verdict: NO.**

There is no file tree widget. Changed files are shown as a flat list of `git status --short`
lines in the Status panel. There is no tree structure, no directory grouping, and no
expand/collapse behavior for directories.

---

## 8. How does it handle large diffs?

**Verdict: PARTIALLY -- but the gap is academic since F4 has no diff.**

The `diff_panel.rs` widget handles large diffs by:
- Scrolling via `Paragraph::scroll()` with bounded offset
- Auto-scroll to end by default (latest changes visible first)
- Max scroll clamped to `total_lines - visible_height`

The `collect_status()` function caps at 50 status lines. Commit history is capped at 30
entries. There is an overflow indicator (`... +N more`) in the status panel.

**Gap:** No truncation or lazy loading for very large diffs in the diff panel widget. The
entire diff string is loaded into memory and converted to `Vec<Line>` upfront. For a
multi-megabyte diff, this could cause high memory usage and slow rendering.

**Gap:** The commit graph uses `Paragraph` with `Wrap { trim: false }` which re-wraps
content on every frame. For 30 commits with graph characters, this is fine, but the wrapping
can misalign the graph visualization.

---

## 9. Compare with `git diff --stat` output quality

**Verdict: BELOW `git diff --stat`.**

`git diff --stat` provides:
- Per-file insertion/deletion counts with `+`/`-` histogram bars
- Total summary line ("N files changed, N insertions, N deletions")
- Binary file indicators
- Rename detection with similarity percentage

The F4 Git tab provides:
- Raw `git status --short` lines (no insertion/deletion counts)
- File count in the Branch Info panel ("modified: N files")
- No histogram visualization
- No rename detection display

The status panel is closer to `git status --short` than `git diff --stat`. The summary in
Branch Info is closer to `git status` summary format.

---

## 10. Proposed improvements

### Priority 1: Wire diff panel into F4

The `diff_panel.rs` widget already exists and is reusable. Add a diff sub-panel to F4 that
shows the diff for the selected file from the status list.

- Add a file selection cursor to the Status panel
- On selection, run `git diff -- <file>` in the background thread
- Render the per-file diff using the existing `render_diff_panel` widget
- Replace the fixed 50/25/25 left panel split with an adaptive layout when a diff is active

### Priority 2: Add `--stat` summary

Add a `git diff --stat` line to each status entry or a separate summary section:
- Run `git diff --numstat` to get per-file insertion/deletion counts
- Display `+N/-M` badges next to each file in the status list
- Add `+`/`-` histogram bars (similar to terminal `git diff --stat`)
- Add a total summary line at the bottom

### Priority 3: File tree sidebar with directory grouping

Replace the flat status list with a collapsible directory tree:
- Group changed files by directory
- Show directory-level aggregate counts
- Use tree-drawing characters for visual hierarchy
- Support expand/collapse with Enter/arrow keys

### Priority 4: Syntax-highlighted diff with `+`/`-` coloring

The existing `diff_panel.rs` already does `+`/`-` coloring. Enhancements:
- Add line numbers in the gutter (left margin)
- Add inline word-level highlighting (highlight the changed characters within a line,
  not just the whole line)
- Consider side-by-side mode as an alternative layout (toggled with a key)
- Add a minimap scrollbar for large diffs

### Priority 5: Show commit age and ref decorations

- Add the `age` field to the commit graph rendering (currently collected but not shown)
- Add `%d` to the git log format string and render branch/tag decorations in distinct colors
- Consider coloring the graph prefix characters by branch

### Priority 6: Commit graph improvements

- Use `Paragraph` without `Wrap` (use `Line` truncation instead) to prevent graph misalignment
- Add a commit detail panel: selecting a commit shows its full message, author, date, and
  stat summary
- Consider showing the graph as an actual DAG visualization for complex merge histories

### Priority 7: Richer worktree and branch data

- Parse `locked` and `prunable` from worktree porcelain output
- Show dirty/clean state per worktree (requires `git -C <path> status`)
- Make the branch tree actually collapsible by namespace (e.g., collapse all `feature/*`)
- Show remote branches as a separate section (currently only local branches via `refs/heads/`)

---

## Architecture summary

```
+------------------+     +------------------+     +------------------+
| git_watch.rs     |     | app.rs           |     | git_view.rs      |
| watches .git/    |---->| drain_bg_channels|---->| render()         |
| debounce 500ms   |     | spawn bg thread  |     | two-panel layout |
| notify + poll    |     | generation guard  |     | pure render, 0IO |
+------------------+     +------------------+     +------------------+
                                |
                                v
                          +------------------+
                          | collect_git_data |
                          | 5 git subprocesses|
                          | branch/commit/   |
                          | worktree/status  |
                          +------------------+

Separate path (F1 Dashboard only):
+------------------+     +------------------+
| dashboard.rs     |     | diff_panel.rs    |
| load_git_diff()  |---->| render_diff_panel|
| git diff HEAD    |     | +/- coloring     |
+------------------+     +------------------+
```

**Key observation:** The git view and the diff panel live in completely separate data paths.
The git view has excellent real-time refresh via the dedicated git watcher, but the diff
panel is only wired to F1 Dashboard. The biggest single improvement would be connecting the
diff panel to F4 with per-file selection.

---

## Test coverage

- `git_view.rs`: 1 unit test (`parses_nul_delimited_commit_subject_with_tabs_and_pipes`) --
  covers the NUL-delimited commit parser edge case
- `git_watch.rs`: 1 unit test (`non_git_path_returns_disabled_handle`) -- covers the disabled
  handle path
- `fs_watch.rs`: 1 unit test (`watch_roko_dir_emits_refresh_within_500ms`) -- covers the poll
  fallback
- `app.rs`: 2 integration tests covering git cursor drill and view state mapping
- No rendering snapshot tests for the git view panels

---

## Implementation Status (2026-09-02 swarm)

F4 Git view improvements (task #14): diff display, branch info, worktree indicators.
