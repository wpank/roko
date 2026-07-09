# C — TUI and Rosedust Truth Pass

Refresh target for docs 07-09: present the ratatui surface as real and wired,
then narrow the speculative "full design language" and "29 screens" framing.

Generated: 2026-04-18

---

## Headline

- Use the audit-corrected headline: the CLI/TUI topic-level surface is 58K LOC
  and already wired.
- The live TUI is organized around `F1`-`F7` tabs, views, modals, widgets, and
  effects. Do not describe it as though nothing exists until a 29-screen spec
  lands.
- Rosedust is real as a theme/palette layer. Treat the broader cross-surface
  design system as target-state language.

## Verified Anchors

| Surface | Status | Notes |
|---|---|---|
| `roko dashboard` ratatui entry | Shipping | treat it as a live primary interface |
| `F1`-`F7` tabs | Shipping | `crates/roko-cli/src/tui/tabs.rs:8-49` defines Dashboard, Plans, Agents, Git, Logs, Config, Inspect |
| TUI implementation depth | Shipping | direct pass shows `crates/roko-cli/src/tui/` is large and active; the audit headline for the broader TUI surface is 58K LOC |
| PostFX / atmosphere / effects | Shipping | keep effects as real implementation, not just art direction |
| Rosedust palette/theme layer | Shipping | real, but narrower than the full multi-surface design language |
| 29-screen inventory | Overscoped doc | rewrite around the actual tab/view/modal structure |

## Rewrite Guidance

### Keep

- `F1`-`F7` navigation as the organizing model.
- ratatui, widgets, modal flows, approval surfaces, config view, and effects.
- Inspect tab as an existing destination, even if every deep visualization
  described in the docs is not yet verified.

### Narrow

- Doc 07 should not read like a fully implemented cross-surface motion and
  typography system.
- Doc 09 should not read like 29 independent shipping screens.
- If a feature is only a design idea, move it to a future-work note instead of
  deleting the concept entirely.

### Do Not Promote To Shipping Without Proof

- command palette
- global search
- other speculative cross-cutting UI primitives beyond the visible tab/view
  structure

## Practical Writing Rule

Describe the TUI as "wired and substantial, with a narrower current shape than
the most ambitious specs" rather than as either "fully matches every spec doc"
or "still scaffold."
