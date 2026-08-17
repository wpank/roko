# M151 — Create Plugin Manifest Loader

## Objective
Create a manifest loader for Tier 1-3 plugins in `roko-cli`. Scan `plugins/` directories for `manifest.toml` files, parse manifest metadata (id, version, tier, kind, capabilities, permissions), validate tier-appropriate capabilities, and register with the tool registry. Wire into `roko config plugins list` so discovered plugins are visible.

## Scope
- Crates: `roko-cli`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plugin.rs` (new file)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` (wire module + CLI command)
- Depth doc: `tmp/unified-depth/13-builtin-catalog/` (plugin tiers)

## Steps
1. Check if plugin infrastructure already exists:
   ```bash
   grep -rn 'plugin\|Plugin\|manifest' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -15
   grep -rn 'plugin\|Plugin' /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/ --include='*.rs' | head -10
   ```

2. Check how `roko config plugins list` is currently defined:
   ```bash
   grep -rn 'plugins\|PluginCmd\|plugin.*list' /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/ --include='*.rs' | head -10
   ```

3. Define plugin manifest schema in `plugin.rs`:
   ```rust
   /// Plugin manifest parsed from `manifest.toml`.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PluginManifest {
       pub id: String,
       pub name: String,
       pub version: String,
       pub tier: PluginTier,
       pub kind: PluginKind,
       pub capabilities: Vec<String>,
       pub permissions: Vec<PluginPermission>,
       pub entry_point: Option<String>,
       pub description: Option<String>,
   }

   /// Plugin tier determines capability limits.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum PluginTier {
       /// Read-only, no side effects.
       Tier1,
       /// Can write to workspace, scoped I/O.
       Tier2,
       /// Full access, requires explicit approval.
       Tier3,
   }

   /// Plugin kind determines execution model.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "kebab-case")]
   pub enum PluginKind {
       Tool,
       Transformer,
       Watcher,
       Gate,
   }

   /// Permission grants for the plugin.
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "kebab-case")]
   pub enum PluginPermission {
       ReadFs,
       WriteFs,
       Network,
       Subprocess,
       Secrets,
   }
   ```

4. Implement manifest scanner:
   ```rust
   /// Scan plugin directories for manifest.toml files.
   ///
   /// Searches: .roko/plugins/, plugins/, ~/.config/roko/plugins/
   pub fn discover_plugins(workspace_root: &Path) -> Result<Vec<DiscoveredPlugin>> {
       let search_dirs = [
           workspace_root.join(".roko/plugins"),
           workspace_root.join("plugins"),
           dirs::config_dir().map(|d| d.join("roko/plugins")).unwrap_or_default(),
       ];
       // For each dir, find manifest.toml files and parse them
   }
   ```

5. Implement tier validation:
   ```rust
   /// Validate that plugin capabilities are appropriate for its tier.
   ///
   /// Tier 1: read_fs only
   /// Tier 2: read_fs, write_fs, subprocess
   /// Tier 3: all permissions
   pub fn validate_tier_capabilities(manifest: &PluginManifest) -> Result<(), Vec<String>> {
       let allowed = match manifest.tier {
           PluginTier::Tier1 => vec![PluginPermission::ReadFs],
           PluginTier::Tier2 => vec![PluginPermission::ReadFs, PluginPermission::WriteFs, PluginPermission::Subprocess],
           PluginTier::Tier3 => vec![/* all */],
       };
       // Check each permission against allowed set
   }
   ```

6. Wire into `roko config plugins list`:
   - Call `discover_plugins(workspace_root)`
   - Validate each manifest
   - Print table: id, name, version, tier, kind, status (valid/invalid)

7. Write tests:
   - Valid Tier 1 manifest passes validation
   - Tier 1 manifest requesting `WriteFs` fails validation
   - Scanner finds manifests in nested directories
   - Invalid TOML produces clear error message

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- plugin
```

## What NOT to do
- Do NOT implement plugin execution — only discovery and validation
- Do NOT auto-load plugins on startup — explicit registration only
- Do NOT add wasm/dynamic library loading — that is future work
- Do NOT modify roko-core for this — plugin types live in roko-cli
- Do NOT create example plugins — only the loader infrastructure
