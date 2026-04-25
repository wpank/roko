# 05 — Tab Autocompletion, Autosuggestions & Fuzzy Search

**Status**: implementing
**Scope**: `crates/roko-cli/src/chat_inline.rs` only

## Problem

The `roko chat` inline REPL has basic prefix-match tab completion for slash commands
but lacks:

1. **Visual dropdown** — no visible menu showing matching commands
2. **Fuzzy search** — typing `/mo` matches `/model` but `/mdl` does not
3. **History autosuggestions** — no ghost text from previous inputs

## Solution

### Fuzzy match

Character-subsequence matching with weighted scoring:
- +15 for match at index 0
- +10 for match at word boundary (after `/`, `-`, `_`, space)
- +5 for consecutive character matches
- -1 per gap between matched characters
- Case-insensitive

### Completion dropdown

A `CompletionState` struct replaces the old `tab_matches`/`tab_index`/`tab_prefix` fields.
Rendered as an overlay between the spacer and input line, max 8 rows. Selected row uses
`theme.selection()`, matched characters highlighted with BONE+Bold.

### Ghost suggestions

When the cursor is at end-of-buffer and the buffer doesn't start with `/`, the most
recent history entry matching the current buffer prefix is shown as ghost text in
`TEXT_GHOST` color. Accepted with Right or End arrow.

## Key bindings

| Key | Dropdown visible | Dropdown hidden |
|-----|-----------------|-----------------|
| `/` typed | opens dropdown | opens dropdown |
| Any char | refreshes | — |
| Backspace | refreshes | — |
| Tab | select next | opens dropdown |
| Shift+Tab | select prev | — |
| Up | select prev | history up |
| Down | select next | history down |
| Enter | accept → buffer | submit |
| Escape | dismiss | — |
| Right (at end) | — | accept ghost |
| End (at end) | — | accept ghost |
