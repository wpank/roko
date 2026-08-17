# M055 — Agent Inbox in TUI

## Objective
Implement the Agent Inbox surface as a TUI tab. The Inbox provides ambient notification with three urgency levels (Critical, Urgent, Notice), notification aggregation (grouping related items), and lifecycle management (created -> read -> acted -> archived). Agent errors appear as Critical notifications. Verify failures as Urgent. Informational updates as Notice.

## Scope
- Crates: `roko-cli`
- Files: `crates/roko-cli/src/tui/` (new tab file for inbox)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.8
- Spec ref: `tmp/unified/16-SURFACES.md` SS4 (Agent Inbox)

## Steps
1. Check for existing notification or inbox code:
   ```bash
   grep -rn 'Notification\|Inbox\|notification\|inbox\|Urgency' crates/roko-cli/src/tui/ --include='*.rs' | head -10
   grep -rn 'Notification\|Inbox' crates/roko-core/src/ --include='*.rs' | head -10
   ```

2. Use the Notification types defined in M053 (from `roko-core/src/surfaces.rs`).

3. Implement the Inbox tab:
   - **Three-section layout**: Critical (red, top), Urgent (yellow, middle), Notice (dim, bottom)
   - Each section shows notifications sorted by time (newest first)
   - Aggregation: group notifications by source (e.g., "Agent coding-agent: 3 errors") with expand/collapse
   - Unread count badge in tab title: `Inbox (5)`

4. Implement notification lifecycle in the tab:
   - Selecting a notification marks it as Read
   - `a` key: act on notification (dispatch SurfaceEvent)
   - `d` key: archive notification
   - `f` key: filter by source/urgency

5. Implement a notification store that accumulates notifications:
   ```rust
   pub struct InboxStore {
       notifications: Vec<Notification>,
       max_stored: usize,
   }

   impl InboxStore {
       pub fn push(&mut self, notification: Notification);
       pub fn by_urgency(&self, urgency: Urgency) -> Vec<&Notification>;
       pub fn unread_count(&self) -> usize;
       pub fn mark_read(&mut self, id: &str);
       pub fn archive(&mut self, id: &str);
   }
   ```

6. Subscribe to Bus topics that generate notifications:
   - `agent.*.error` -> Critical
   - `verify.*.failed` -> Urgent
   - `flow.*.completed` -> Notice
   - `structural.proposal` -> Urgent (L4 proposals from M072)

7. Write tests:
   - Agent error Pulse creates Critical notification
   - Unread count increments on new notification, decrements on mark_read
   - Archive removes notification from active list
   - Aggregation groups by source

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- tui::inbox
```

## What NOT to do
- Do NOT persist notifications to disk -- they are session-scoped
- Do NOT implement push notifications to external systems (email, Slack) -- that is a Connector concern
- Do NOT couple notification generation to specific error types -- use Bus topic patterns
- Do NOT block the TUI render loop waiting for notifications -- use async channels
