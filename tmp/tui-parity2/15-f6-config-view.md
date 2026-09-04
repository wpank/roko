# F6 Config View Audit

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/config_view.rs`
**Support**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/config_meta.rs`
**Date**: 2026-09-01

---

## 1. Config tree: is the TOML structure navigable?

**Verdict: Good, with structural limitations.**

The config is rendered as a flat scrollable list, not a tree. `config_meta::all_fields()`
defines 47 editable fields organized into 11 groups via `GROUP_ORDER`:

```
Agent (7) -> Budget (3) -> Gates (3) -> Routing (8) -> Conductor (5) ->
Pipeline (12) -> Learning (4) -> PRD (1) -> Project (3) -> TUI (2) -> Server (2)
```

Each group gets a styled `-- Group Name --` header line. Fields appear below their
header. Navigation uses j/k (or arrow keys) with automatic header-skipping: the
`ConfigUp`/`ConfigDown` handlers in `app.rs:1910-1938` advance the cursor past
`ConfigItem::Header` entries so the user never lands on a non-actionable row.

**Gap**: The TOML file (`roko.toml`) has substantially more sections than the 11 groups
exposed here. `RokoConfig` has 30+ top-level sections (providers, models, profiles,
graduation, watcher, timeouts, statehub, webhooks, github, subscriptions, deploy,
perplexity, gemini, tools, chain, relay, feed_agents, runner, agents, groups,
validation, cold_storage, prompt, resources, scheduler). Only 11 of these are
represented in the editor. The remaining ~20 sections are invisible from F6, making
this view a partial representation of the actual configuration surface.

**Gap**: No tree-folding or collapsing. With 47 fields plus 5 runtime sections
appended at the bottom, the list can exceed 70+ items. There is no way to collapse a
group to reduce scrolling.

**Gap**: No jump-to-section shortcut. The user must scroll linearly through the entire
list. Other TUI tabs have features like number-key sub-view switching, but the config
view's number keys are consumed by the sub-tab bar (1:Config, 2:Providers, 3:Models).

---

## 2. Inline editing: how does edit mode work? Is it discoverable?

**Verdict: Functional but poorly discoverable.**

Three editing modes exist, determined by `ConfigFieldKind`:

1. **Bool**: Enter/Space toggles between `true`/`false`. Rendered as `[x]`/`[ ]`.
   h/l also cycle. Immediate -- no separate edit mode.

2. **Enum / Int with presets**: Enter or h/l cycles through the preset list. Wraps
   around. Rendered as `< value >` with angle-bracket chrome. No free-text entry.

3. **Str / Float / Int without presets**: Enter opens text-edit mode
   (`InputMode::ConfigEdit`). The current value is copied into
   `config_edit_buffer`, a `_` cursor indicator appends, and the field gets
   `UNDERLINED` style. The hint bar changes to `Enter:confirm  Esc:cancel`.
   Backspace deletes characters; any printable char appends.

Env-overridden fields (`ConfigSource::Env`) are silently non-editable. Pressing
Enter/h/l on them does nothing -- no feedback is given. The user sees the `env` source
tag but gets no indication that editing is blocked.

**Discoverability assessment**:
- The hint bar at the bottom (`j/k:nav  h/l:cycle  Enter:edit  Ctrl-S:save`) is the
  only on-screen affordance. It is always visible but easy to miss.
- Bool fields show checkbox chrome (`[x]`), which implies toggle-ability.
- Enum/int fields show `< value >`, which implies cycling -- reasonable convention.
- String fields show `< value >` too, but Enter opens a text buffer instead of
  cycling. This behavioral inconsistency could confuse users.
- There is no visual indicator that a field IS editable vs read-only until the user
  tries to interact with it.
- Modified fields get `BOLD` styling, which is the only visual cue that something
  has changed.

**Gap**: No undo. Once a value is changed, the only way back is to remember the
original value and type it again, or cancel without saving (but there is no
discard-all-pending action).

---

## 3. Ctrl-S save: feedback on save?

**Verdict: Good.**

`save_config_changes()` in `app.rs:3082-3108` handles three cases:

1. **No pending changes**: pushes `Notification::info("No pending changes to save")`.
2. **Save succeeds**: clears `config_pending`, invalidates the config cache, reloads
   `EffectsConfig`, sets `pending_refresh = true`, and pushes
   `Notification::info("Config saved and reloaded")`.
3. **Save fails**: pushes `Notification::error("Save failed: {error}")`.

The notification system (`modals/notification.rs`) renders toast popups at the
bottom-right corner with level-appropriate styling (info=5s, error=10s TTL). This
provides clear feedback.

**Good**: Two unit tests (`config_save_reloads_config_immediately` and
`config_save_reloads_screen_postfx_immediately`) verify save+reload behavior.

**Good**: The Save button itself is context-aware. When pending changes exist, it
shows `[ Apply & Save * ]` with bold accent styling. When no changes exist, it
shows `[ Apply & Save ]` in muted style.

**Gap**: Ctrl-S and the Save button both route to the same `save_config_changes()`,
but neither is available while in text-edit mode. From `handle_config_edit_key()`,
only Enter (commit) and Esc (cancel) are handled. The user must first commit/cancel
the current edit, then press Ctrl-S. This is a minor friction point.

**Gap**: No dirty-state warning on tab switch. If the user has pending edits and
switches to another tab (F1-F10), the pending edits persist in `config_pending`
but there is no warning. They survive tab switching, which is acceptable, but the
user might forget they have unsaved changes.

---

## 4. Config validation: shown inline?

**Verdict: Missing.**

There is **no validation** at any point in the editing pipeline:

- `ConfigFieldKind::Int { min, max, .. }` declares bounds, but they are never
  checked during editing or commit. A user can type any string into an int field.
- `ConfigFieldKind::Float { min, max }` similarly declares but never enforces bounds.
- `coerce_to_toml()` in `config_meta.rs:757-769` attempts type coercion (e.g.,
  parse string as i64 for Int fields) but silently falls back to storing the raw
  string if parsing fails. This means invalid values like `"abc"` for an Int field
  will be written to `roko.toml` as a TOML string, likely causing a parse error on
  next config load.
- Enum fields use cycling (h/l/Enter) through a fixed list, so they cannot receive
  invalid values. This is the only field kind with implicit validation.
- Bool fields toggle, so they also cannot receive invalid values.
- The `save_pending_edits` function writes blindly. If the TOML file is malformed
  after the write, the next `config_items()` cache rebuild will get `None` from
  the parse and show empty values.

**Gap**: No min/max enforcement on Int or Float fields during text editing.
**Gap**: No type-validation feedback (e.g., "not a number") when committing edits.
**Gap**: No config-schema validation after save (e.g., running `RokoConfig::from_toml`
and checking for errors).

---

## 5. Provider config section: useful?

**Verdict: Useful sub-view, data-dependent.**

The F6 tab has three sub-views selectable via number keys 1/2/3:

### Sub-view 1 (default): Config Editor
The main editable config view. Contains no provider-specific fields. The
`Agent.default_backend` enum is the closest thing (claude, codex, cursor, openai,
ollama, perplexity), but individual provider configuration (API keys, endpoints,
rate limits) from `RokoConfig.providers: IndexMap<String, ProviderConfig>` is
entirely absent from the editor.

### Sub-view 2: Provider Health (`render_provider_health`)
Aggregates efficiency events by inferred provider name. Displays a table:
`provider | status | latency | success`. Provider names are inferred from model
strings using `infer_provider()` (heuristic: contains "claude" -> anthropic,
"gpt" -> openai, etc.). Health status uses three buckets: healthy (>=90%),
degraded (>=70%), unhealthy (<70%).

**Gap**: Provider inference is fragile. Model strings like "claude-sonnet-4-6" map
correctly, but custom model names or provider aliases will fall through to
`trimmed.split('/').next()`, which may produce confusing labels.

**Gap**: No provider configuration is editable here. The `providers` and `models`
maps in `RokoConfig` are not represented in `all_fields()` at all. Users cannot
add/edit/remove provider entries from the TUI.

### Sub-view 3: Model Comparison (`render_model_comparison`)
Table of models from the cascade router: `model | tier | cost | gate% | tries`.
Tier is inferred heuristically (haiku/mini -> "fast", opus/pro -> "deep", else
"std"). Best cost and best gate rate get BOLD highlighting.

**Good**: Cost is computed from actual efficiency events, not configured. Gate
rate comes from the cascade router's confidence stats. This is real operational
data.

**Gap**: No link between model comparison data and model configuration. Users
see performance data but cannot act on it (e.g., promote a model, adjust routing
weights) from this view.

---

## 6. Secret management: safely displayed?

**Verdict: Not applicable -- secrets are excluded.**

The `all_fields()` registry does not include any secret fields (API keys,
tokens, passwords). The `RokoConfig` schema has `ProviderConfig` with potential
API key fields, but these are not in the 47 editable fields.

Env var overrides (`ENV_OVERRIDES` in `config_meta.rs:97-108`) cover operational
settings only (ROKO_MODEL, ROKO_BACKEND, etc.), not secret values.

**This is correct behavior.** Secrets should not be displayed or editable in a
TUI that may be screen-shared or shoulder-surfed.

**Gap**: There is no indication in the UI that secrets exist but are managed
elsewhere. A field or note like "API keys: managed via `roko config set-secret`"
would help discoverability.

---

## 7. Config sections: logically grouped?

**Verdict: Good grouping, incomplete coverage.**

The 11 groups follow a logical progression:

1. **Agent** (7 fields): Core agent settings (model, backend, effort, context,
   bare mode, fallback, timeout). Good -- these are the most commonly adjusted.
2. **Budget** (3 fields): Cost controls. Good placement after Agent.
3. **Gates** (3 fields): Validation controls. Related to execution quality.
4. **Routing** (8 fields): Model selection algorithm and weights. Heavy section
   but logically coherent.
5. **Conductor** (5 fields): Parallelism and execution mode. Clear.
6. **Pipeline** (12 fields): Per-complexity-tier strategist/reviewer/iteration
   settings. The largest section. Four tiers x 3 fields each.
7. **Learning** (4 fields): Auto-refresh and knowledge injection toggles.
8. **PRD** (1 field): `auto_plan` toggle. Feels thin.
9. **Project** (3 fields): Name, root, base branch. Reasonable.
10. **TUI** (2 fields): Refresh rate and PostFX toggle.
11. **Server** (2 fields): Bind address and port.

After the editable fields, four runtime sections are appended as read-only:
- Runtime: Efficiency (6 rows: cost, events, avg time, tokens, pass rate)
- Runtime: Cascade Route (model slug stats)
- Runtime: Verify Thresholds (per-rung adaptive thresholds with trend icons)
- Runtime: Verify Results (per-gate pass rates)
- Runtime: Experiments (active A/B experiments)

**Gap**: The Pipeline section is very long (12 fields) with repetitive structure.
This is a case where tree-folding per complexity tier would help significantly.

**Gap**: Major config areas entirely absent from the editor: `providers`, `models`,
`profiles`, `timeouts`, `webhooks`, `github`, `subscriptions`, `deploy`, `chain`,
`relay`, `tools`, `runner`, `prompt`, `resources`, `cold_storage`, `scheduler`,
`statehub`, `graduation`, `watcher`, `validation`, `gemini`, `perplexity`,
`feed_agents`. That is roughly 25 config sections with no TUI representation.

---

## 8. Search/filter in config

**Verdict: Missing.**

There is no search or filter capability in the config view. The `ViewState` struct
has a `search_query: String` field, but it is unused by `config_view.rs`. The Logs
tab has `LogSearch` mode with regex matching, and Plans has `PlanFilter`, but Config
has neither.

With 47+ editable fields plus runtime sections, finding a specific field requires
linear scrolling. No `/` search, no group jump, no type-to-filter.

---

## 9. Default vs custom values: visually distinct?

**Verdict: Good -- three-level source distinction.**

Each field displays a right-aligned source tag with distinct styling:

| Source | Tag | Style |
|--------|-----|-------|
| `File` (explicitly set in roko.toml) | `file` | `theme.accent()` |
| `Env` (environment variable override) | `env` | `theme.warning()` |
| `Default` (not explicitly set) | `default` | `theme.muted()` |

The `determine_source()` function (`config_meta.rs:613-628`) resolves source
correctly: env overrides win, then file values differing from defaults mark as
`File`, otherwise `Default`.

Pending (unsaved) edits are treated as `ConfigSource::File` in the display
(`config_meta.rs:673-674`), and the field label gets `BOLD` styling.

**Good**: This is a clear three-level visual hierarchy. Default fields are
visually de-emphasized (muted), file-sourced fields use the accent color, and
env-overridden fields use warning color to indicate they cannot be edited.

**Gap**: There is no visual indicator showing what the *default* value IS for a
non-default field. The user can see that a value is "file"-sourced (i.e.,
explicitly set), but cannot see what it would be if they reset it to default.
No "reset to default" action exists.

---

## 10. Proposals

### P1: Config diff (current vs default)

Add a toggle or sub-view (could be sub_tab 4 in the F6 region bar) showing only
fields whose values differ from defaults. Implementation: `build_flat_items()`
already resolves `default_str` via `resolve_default()`. A filtered variant could
skip items where `value == default_str`. Additionally, show the default value
inline (e.g., `Default: 30000` in the description line for modified fields).

### P2: Validation indicators

Enforce `min`/`max` bounds from `ConfigFieldKind::Int` and `Float` during
`ConfigCommitEdit`. On commit, check the parsed value against bounds. If out of
range, show an inline error description (red text) and reject the commit (stay in
edit mode). For text fields, at minimum validate that Int/Float values parse to
their declared type before accepting.

The `coerce_to_toml()` fallback path that stores unparseable int values as TOML
strings is a latent bug that should be caught here.

### P3: Provider health inline

Bring key provider health indicators (healthy/degraded/unhealthy status) from the
Provider Health sub-view into the main config editor as read-only runtime fields
under the Agent or Routing sections. This would give the user at-a-glance health
context when adjusting model selection or backend settings.

### P4: Search/filter

Add `/` to enter a filter mode that narrows the field list to items matching a
substring query against field label, key, or description. The infrastructure
already exists: `InputMode` has filter variants, `ViewState.search_query` exists
but is unused. Pattern after the `LogSearch` implementation in the Logs tab.

### P5: Section collapsing

Allow Enter on a section header to toggle collapse/expand of that group's fields.
Track collapsed groups in a `HashSet<String>` on `TuiState` (parallel to
`collapsed_waves` for the Plans tab). This would make the Pipeline section's 12
fields much more manageable.

### P6: Reset-to-default action

Add a keybinding (e.g., `d` for "default" or `Backspace` on a non-editing field)
that reverts the selected field to its default value. The default is already
computed by `resolve_default()`. Insert the default into `config_pending` so it
becomes a pending change that can be saved or discarded.

### P7: Env-override feedback

When a user attempts to edit an env-overridden field, show a notification like
"Overridden by $ROKO_MODEL -- unset the env var to edit here" instead of silently
ignoring the input. The env var name is available from `ENV_OVERRIDES`.

### P8: Missing section coverage

Expand `all_fields()` to cover at least the most commonly adjusted missing
sections: `timeouts` (HTTP/subprocess timeouts), `github` (repo, auto-PR
settings), `prompt` (composition strategy), and `resources` (disk thresholds).
These are frequently tweaked during development and currently require manual
TOML editing.

---

## Summary

| # | Criterion | Rating | Notes |
|---|-----------|--------|-------|
| 1 | Config tree navigable | GOOD- | Flat list with group headers; no tree-folding, no jump-to-section |
| 2 | Inline editing discoverable | FAIR | Three edit modes work, hint bar present, but type-behavior inconsistency and no undo |
| 3 | Ctrl-S save feedback | GOOD | Toast notifications for success/failure/no-changes; tests exist |
| 4 | Config validation | MISSING | No min/max enforcement, no type validation, no schema check |
| 5 | Provider config section | FAIR | Health + model comparison sub-views exist; no provider config editing |
| 6 | Secret management | GOOD | Correctly excluded from display; no discoverability note |
| 7 | Config sections grouped | GOOD- | 11 groups, logical order; ~25 config sections entirely absent |
| 8 | Search/filter | MISSING | No capability; linear scrolling only |
| 9 | Default vs custom distinct | GOOD | Three-level source tags with distinct styles; no show-default or reset |
| 10 | Proposals | -- | 8 proposals: diff view, validation, inline health, search, collapse, reset, env feedback, coverage |

---

## Implementation Status (2026-09-02 swarm)

F6 Config view improvements (task #15): config editor layout, provider health, MCP panel.
