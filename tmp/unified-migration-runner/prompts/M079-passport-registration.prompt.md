# M079 — Wire Passport into Agent Startup

**[BLOCKED:depth]** -- This item depends on M078 (Rust chain clients) and M076/M077 (deployed contracts).

## Objective
Wire passport registration into the Agent startup lifecycle. When an Agent transitions to Active state (M046), if no on-chain passport exists for this Agent, register one via the PassportClient. Store the passport token ID in Agent state. Subsequent runs reuse the existing passport. This gives every participating Agent a persistent on-chain identity.

## Scope
- Crates: `roko-agent`
- Files: `crates/roko-agent/src/passport.rs` (new), `crates/roko-agent/src/lifecycle.rs` (modify activation)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.4

## Steps
1. Read the Agent lifecycle code from M046:
   ```bash
   grep -rn 'activate\|Provisioning.*Active' crates/roko-agent/src/lifecycle.rs | head -10
   ```

2. Read the PassportClient from M078:
   ```bash
   grep -rn 'PassportClient\|register_passport' crates/roko-chain/src/ --include='*.rs' | head -10
   ```

3. Implement passport management in `crates/roko-agent/src/passport.rs`:
   ```rust
   pub struct PassportManager {
       client: Arc<PassportClient>,
       cache_path: PathBuf,
   }

   impl PassportManager {
       pub async fn ensure_passport(&self, agent: &AgentConfig) -> Result<PassportInfo> {
           // 1. Check local cache for existing passport token ID
           // 2. If cached: verify on-chain it still exists
           // 3. If not cached or invalid: register new passport
           // 4. Cache the token ID locally
           // 5. Return passport info
       }
   }
   ```

4. Wire into Agent activation:
   ```rust
   impl Agent<Provisioning> {
       pub async fn activate(self) -> Result<Agent<Active>> {
           // ... existing activation logic ...
           if let Some(chain_config) = &self.config.chain {
               let passport = passport_manager.ensure_passport(&self.config).await?;
               // Store passport.token_id in Agent state
           }
           // ... continue activation ...
       }
   }
   ```

5. Make passport registration optional (only when chain config is present in roko.toml).

6. Write tests:
   - New Agent registers passport on first activation
   - Second activation reuses cached passport
   - Missing chain config skips passport registration gracefully

## Verification
```bash
cargo check -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo test -p roko-agent -- passport
```

## What NOT to do
- Do NOT make passport registration mandatory -- it is opt-in via chain config
- Do NOT block Agent activation on chain errors -- fall back to local-only mode
- Do NOT implement passport updates here -- registration only
- Do NOT proceed without M078 chain clients
