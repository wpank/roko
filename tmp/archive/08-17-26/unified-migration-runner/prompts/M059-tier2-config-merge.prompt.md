# M059 — SPI Tier 2: Config Profile Deep Merge

## Objective
Implement Tier 2 of the 5-tier SPI: config profile deep merge. Profile TOML files customize agent behavior (model selection, temperature, Verify config, tool allowlists, etc.). Profiles merge with deep override semantics: workspace profile overrides user profile overrides builtin. This enables teams to share behavioral configurations without forking the codebase.

## Scope
- Crates: `roko-cli`
- Files: `crates/roko-cli/src/config/profiles.rs` (new), `crates/roko-cli/src/config/mod.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.10
- Spec ref: `tmp/unified/14-CONFIG-AND-AUTHORING.md` SS3 (5-Tier SPI)

## Steps
1. Read the current config structure:
   ```bash
   grep -rn 'pub struct.*Config\|profile\|Profile' crates/roko-cli/src/config/ --include='*.rs' | head -20
   grep -rn 'profile\|Profile' crates/roko-core/src/config/ --include='*.rs' | head -15
   cat crates/roko-core/src/config/mod.rs 2>/dev/null | head -40
   ```

2. Define the profile format:
   ```toml
   # .roko/profiles/defi-trading.toml
   [profile]
   name = "defi-trading"
   description = "Profile for DeFi trading agents"
   extends = "coding"  # optional: inherit from another profile

   [model]
   default = "claude-sonnet-4-20250514"
   temperature = 0.3
   max_tokens = 4096

   [verify]
   min_rung = 3
   required_gates = ["compile", "test"]

   [tools]
   allowed = ["shell", "fs_read", "fs_write", "net"]
   denied = ["process_kill"]
   ```

3. Implement the profile loader with deep merge in `crates/roko-cli/src/config/profiles.rs`:
   ```rust
   pub struct ProfileLoader {
       search_paths: Vec<PathBuf>,
   }

   pub struct Profile {
       pub name: String,
       pub description: String,
       pub extends: Option<String>,
       pub values: toml::Value,  // Raw TOML for deep merge
   }

   impl ProfileLoader {
       pub fn load(&self, name: &str) -> Result<Profile>;
       pub fn resolve_chain(&self, name: &str) -> Result<Vec<Profile>>;  // follow extends
       pub fn merge(&self, name: &str) -> Result<toml::Value>;           // deep-merged result
   }
   ```

4. Implement deep merge semantics:
   - Tables merge recursively (keys in override replace keys in base)
   - Arrays replace entirely (not appended)
   - Scalar values replace entirely
   - `extends` forms a chain: load base first, then overlay each extension

5. Resolution order: workspace `.roko/profiles/` > user `~/.roko/profiles/` > builtin.

6. Integrate with the existing config loading: when `agent.profile = "defi-trading"` is set in roko.toml, merge the profile values into the agent config.

7. Write tests:
   - Workspace profile overrides user profile for same key
   - Deep merge: nested table keys merge correctly
   - `extends` chain resolves and merges in order (base -> extension -> workspace)
   - Missing profile produces clear error

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- config::profiles
```

## What NOT to do
- Do NOT modify the core roko.toml schema -- profiles are overlays, not replacements
- Do NOT add profile validation against a schema -- just merge TOML values
- Do NOT implement circular `extends` detection as an error -- just limit depth to 10
- Do NOT add runtime profile switching -- profiles are loaded at agent startup
