# M163 — Wire Ventriloquist Defense as Verify Cell

## Objective
Wire the ventriloquist defense as a Verify Cell in `roko-chain`. The concept already appears in `agent_registry.rs` and `phase2.rs` — agent identity commitments are SHA-256 hashes of system prompts stored on-chain. Create a `VentriloquistVerify` Cell that checks incoming agent messages against the committed identity hash, preventing prompt injection attacks where an adversary impersonates an agent by replaying its messages with a different system prompt.

## Scope
- Crates: `roko-chain`
- Files:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/ventriloquist.rs` (new or extend existing)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs` (read commitment store)
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs` (re-export)
- Depth doc: `tmp/unified-depth/18-registries/07-gossip-and-privacy.md`

## Steps
1. Read existing ventriloquist references in the codebase:
   ```bash
   grep -n 'ventriloquist\|VentriloquistVerify\|identity_commitment\|prompt_hash' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs | head -15
   grep -n 'ventriloquist\|prompt_commitment' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs | head -15
   ```

2. Read the agent registry to understand how identity commitments are stored:
   ```bash
   grep -n 'pub struct\|pub fn\|commitment\|register' /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs | head -20
   ```

3. Create or extend `ventriloquist.rs` with the `VentriloquistVerify` struct:
   ```rust
   use sha2::{Sha256, Digest};

   /// Verify Cell that validates agent messages against committed identity.
   ///
   /// On agent registration, a SHA-256 hash of the system prompt is stored
   /// on-chain. This Cell verifies that any message claiming to come from
   /// that agent can prove knowledge of the original prompt.
   pub struct VentriloquistVerify {
       /// Map of agent_id → commitment (SHA-256 hash of system prompt)
       commitments: HashMap<AgentId, [u8; 32]>,
   }

   impl VentriloquistVerify {
       /// Register a new agent identity commitment.
       pub fn commit(&mut self, agent_id: AgentId, system_prompt: &str) -> [u8; 32] {
           let hash = Sha256::digest(system_prompt.as_bytes());
           let commitment: [u8; 32] = hash.into();
           self.commitments.insert(agent_id, commitment);
           commitment
       }

       /// Verify that a message's claimed identity matches the commitment.
       pub fn verify(&self, agent_id: &AgentId, system_prompt: &str) -> Result<(), VentriloquistError> {
           let expected = self.commitments.get(agent_id)
               .ok_or(VentriloquistError::UnknownAgent(agent_id.clone()))?;
           let actual: [u8; 32] = Sha256::digest(system_prompt.as_bytes()).into();
           if &actual != expected {
               return Err(VentriloquistError::IdentityMismatch {
                   agent_id: agent_id.clone(),
                   expected: *expected,
                   actual,
               });
           }
           Ok(())
       }
   }
   ```

4. Define error types:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum VentriloquistError {
       #[error("unknown agent: {0}")]
       UnknownAgent(AgentId),
       #[error("identity mismatch for agent {agent_id}: expected {expected:?}, got {actual:?}")]
       IdentityMismatch { agent_id: AgentId, expected: [u8; 32], actual: [u8; 32] },
   }
   ```

5. Add `Verify` trait implementation so it can be used as a gate cell:
   ```rust
   impl Verify for VentriloquistVerify {
       fn verify(&self, signal: &Signal) -> Result<(), VerifyError> {
           // Extract agent_id and system_prompt from signal metadata
           // Call self.verify(agent_id, system_prompt)
       }
   }
   ```

6. Wire into `lib.rs` with `pub mod ventriloquist;` (or ensure existing module is re-exported).

7. Write unit tests:
   - Commit → verify with matching prompt succeeds
   - Commit → verify with different prompt fails with IdentityMismatch
   - Verify with unknown agent fails with UnknownAgent
   - Hash stability: same prompt always produces same commitment

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- ventriloquist
```

## What NOT to do
- Do NOT implement actual on-chain storage — use in-memory HashMap (real chain storage is Phase 2+)
- Do NOT add new dependencies beyond `sha2` (already in workspace) and `thiserror`
- Do NOT modify the agent registry's existing registration flow — this is an additive Verify Cell
- Do NOT implement key rotation in this batch — single commitment per agent is sufficient
- Do NOT wire into orchestrate.rs — this is chain-internal verification
