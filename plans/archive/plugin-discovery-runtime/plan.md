# plugin-discovery-runtime

## Problem

`roko config plugins install` writes to `.roko/plugins/<name>/plugin.toml`. The runner's
`load_extensions_with_disabled()` probes only `.roko/extensions/` and `plugins/` (line 347
of `extension_loader.rs`). Plugins in `.roko/plugins/` are silently ignored at plan
execution time despite appearing in `roko config plugins list`.

## Fix

Two surgical changes:

1. **T1** — Add `RokoLayout::plugins_dir()` (returns `.roko/plugins/`) to
   `crates/roko-fs/src/layout.rs`. ~5 LOC. Follows the `extensions_dir()` pattern at line 199.

2. **T2** — Extend the `plugin_dirs` array in `load_extensions_with_disabled` from 2 to 3
   entries by appending `layout.plugins_dir()`. Add one unit test that verifies the new path
   is scanned. ~30 LOC.

`RokoLayout` is already constructed at line 345 and `discover_plugins()` already handles
non-existent directories by returning `Ok(vec![])`, so the fix is fail-open by default.

## Not in scope

- `roko serve` scan path (already includes `.roko/plugins/` via a separate site)
- `roko config plugins list` (already includes `.roko/plugins/`)
- Plugin execution (triggers, tools) — this plan only closes the discovery gap