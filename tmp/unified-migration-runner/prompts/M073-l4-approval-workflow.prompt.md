# M073 — L4 Approval Workflow via Inbox

## Objective
Wire L4 structural proposals into the Agent Inbox (M055) as Urgent notifications. When a proposal is submitted, it appears in the Inbox with evidence and a diff. The human reviews, approves or rejects. Approved proposals are applied to the workspace. Rejected proposals are archived with the rejection reason. This provides the human-in-the-loop control required for structural self-evolution.

## Scope
- Crates: `roko-serve`, `roko-cli`
- Files: `crates/roko-serve/src/routes/approvals.rs` (new), `crates/roko-serve/src/routes/mod.rs`, `crates/roko-cli/src/tui/` (inbox integration)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.3
- Spec ref: `tmp/unified/10-LEARNING-LOOPS.md` SS5.2

## Steps
1. Read the Inbox implementation from M055 and proposal store from M072:
   ```bash
   grep -rn 'InboxStore\|Notification\|notification' crates/roko-cli/src/tui/ --include='*.rs' | head -10
   grep -rn 'ProposalStore\|StructuralProposal' crates/roko-learn/src/ --include='*.rs' | head -10
   ```

2. Subscribe to `structural.proposal.submitted` Bus topic to generate Inbox notifications:
   ```rust
   // When a proposal is submitted:
   let notification = Notification {
       urgency: Urgency::Urgent,
       title: format!("Structural proposal: {}", proposal.kind_name()),
       body: format!("{}\n\nEvidence: {} signals", proposal.description, proposal.evidence.len()),
       source: format!("L4:{}", proposal.author),
       ..
   };
   ```

3. Implement HTTP approval routes in `crates/roko-serve/src/routes/approvals.rs`:
   ```rust
   // GET  /api/proposals               -> Vec<StructuralProposal> (pending)
   // GET  /api/proposals/{id}          -> StructuralProposal (detail + diff)
   // POST /api/proposals/{id}/approve  -> apply and return result
   // POST /api/proposals/{id}/reject   -> archive with reason
   // GET  /api/proposals/history       -> Vec<StructuralProposal> (all statuses)
   ```

4. Implement the approval action:
   - Approve: call `ProposalStore::approve()`, then `ProposalStore::apply()` which executes the diff
   - For `ModifyGraph`: update the Graph TOML file
   - For `AddCell`: create the Cell manifest in `.roko/cells/`
   - For `ChangeConfig`: update the relevant config file
   - For `UpdateVerifyPipeline`: modify the Verify configuration

5. Add TUI keybinding: when viewing a structural proposal notification in the Inbox, `a` approves and `r` rejects (with reason prompt).

6. Write tests:
   - Proposal submission creates Urgent notification in Inbox
   - Approve via HTTP applies the change
   - Reject via HTTP archives with reason
   - Applied proposal's diff is reflected in workspace files

## Verification
```bash
cargo check -p roko-serve
cargo check -p roko-cli
cargo clippy -p roko-serve --no-deps -- -D warnings
cargo test -p roko-serve -- approvals
```

## What NOT to do
- Do NOT auto-approve proposals based on confidence threshold -- all require explicit human action
- Do NOT apply proposals that modify protected safety components (checked by RecursiveSafetyMonitor from M070)
- Do NOT implement undo for applied proposals -- that is a separate concern
- Do NOT add batch approval -- each proposal is reviewed individually
