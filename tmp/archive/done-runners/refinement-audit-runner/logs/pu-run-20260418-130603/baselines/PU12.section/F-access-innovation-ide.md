# F — Sonification, Accessibility, UX Innovation, IDE Integration (Docs 16, 17, 18, 20)

Parity of four "frontier innovation" chapters: sonification reframed
(450 lines), accessibility + current status (553 lines), UX innovation
proposals (**2,451 lines** — largest doc in topic 12), IDE
integration strategy (701 lines).

Docs 16 (sonification), 18 (UX innovation), 20 (IDE integration) are
pure design / proposal. Doc 17 is the topic-12 status doc — it is
more accurate than batch 10's Doc 16 but still undercounts the
shipping TUI (C.*) and `roko-serve` (B.*).

Generated: 2026-04-16.

---

## F.01 — Sonification as audio-display primitive (Doc 16 §"Reframing Sonification")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 16 (450 lines) reframes sonification — instead of "sonify any data stream", it proposes targeted audio cues for specific events (task-complete, error, approval-request, agent-idle).
**Reality**: `Grep 'sonif\|Sonification\|audio_cue\|beep' crates/ --include=*.rs` returns zero matches. No audio subsystem. Frontier.
**Fix sketch**: Doc 16 stays `Design — Phase 2+`.

---

## F.02 — Specific audio cues (task-complete / error / approval) (Doc 16 §"Specific Cues")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Small, focused audio cues for key events.
**Reality**: Follows from F.01. Frontier.

---

## F.03 — Accessibility targets: WCAG 2.1 AA (Doc 17 §"WCAG 2.1 AA Compliance")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 17 targets WCAG 2.1 AA compliance across Web Portal, TUI, CLI. Includes color contrast table (verified for Rosedust palette at §"Distinguishable"), keyboard navigation, screen reader support, reduced motion, `prefers-contrast: more` media query.
**Reality**: The Rosedust palette ships (C.06) with WCAG-AA contrast ratios verified in Doc 17. The web portal itself doesn't ship (E.01), so WCAG portal compliance is moot. TUI keyboard navigation ships (F1-F7 tabs, modal stack). Screen-reader coverage is inherent to terminal screen readers consuming stdout.
**Fix sketch**: Doc 17 §"WCAG" should scope to the shipping color palette + keyboard navigation; web-portal WCAG deferred to when portal ships.

---

## F.04 — Doc 17 "Current Implementation Status" tables (Doc 17 §"Current Status")

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 17 §"Current Implementation Status" tables the state of every interface component. Marked `Implementation: Scaffold` at top.
**Reality**: Doc 17 is topic 12's canonical status doc. Its "Scaffold" top-level banner undersells the shipping reality:
- CLI: ~20 subcommands shipping (A.*)
- `roko-serve`: 12,285 LOC of routes + SSE + WS + webhooks (B.*)
- TUI: 25,449 LOC, 7 tabs, 13 modals, 10 widgets (C.*)
- `roko-agent-server`: ~1,800 LOC sidecar with 5 features (B.04)

vs what's NOT shipping: Spectre (D.*), web portal (E.*), sonification (F.01), UX innovation (F.05-F.09), IDE integration (F.10-F.14).

Doc 17's "Scaffold" banner is misleading — topic 12 is substantially SHIPPING for CLI + HTTP + TUI + sidecar; frontier only for the visualization / portal / innovation surfaces. It also needs to stop eliding the checked-in `9090` vs `6677` default drift.
**Fix sketch**: Doc 17 should split its status table into "Shipping" (CLI + TUI + serve + sidecar) + "Partial" (portal-ready backend, CLI onboarding baseline, Rosedust palette) + "Frontier" (Spectre, portal UI, sonification, A2UI, IDE integration). The top-level "Scaffold" banner should become "Mixed".

---

## F.05 — UX innovation proposals (Doc 18 §"Proposals")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 18 is the **largest chapter in topic 12 at 2,451 lines** — a proposal document enumerating UX experiments (voice-first, gesture, multimodal, novel visualization patterns, collaborative flows).
**Reality**: `Grep 'ux_innovation\|voice_first\|gesture' crates/ --include=*.rs` returns zero matches. Pure proposal document. The shipping TUI + CLI + backend are conservative — the UX innovations in Doc 18 are all future.
**Fix sketch**: Doc 18 stays `Design — proposals under review`. Consider splitting into "high-confidence proposals" vs "speculative ideas" so prioritization is legible.

---

## F.06 — Voice-first interface proposals (Doc 18 §"Voice")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 18 includes voice-first interface proposals (speech-to-text input + TTS output + voice-tailored dialogs).
**Reality**: No voice subsystem. Frontier.

---

## F.07 — Gesture / multimodal proposals (Doc 18 §"Multimodal")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Gesture + camera-based multimodal interface proposals.
**Reality**: Frontier.

---

## F.08 — Collaborative multi-user flows (Doc 18 §"Collaboration")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Multi-user collaborative flows where multiple humans drive one agent session, or one human drives multiple agents.
**Reality**: No multi-user session surface. The HTTP control plane (B.01) is single-user-per-session today. Frontier.

---

## F.09 — Novel visualization patterns (Doc 18 §"Visualization")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 18's visualization proposals include chord diagrams for agent communication, Sankey diagrams for plan progression, force-directed Engram DAG layout.
**Reality**: Shipping TUI widgets (C.04) are text-based. These are web-portal-adjacent visualizations depending on E.01 portal.

---

## F.10 — IDE integration 5-approach evaluation (Doc 20 §"Five Approaches")

**Status**: NOT DONE (decision document only)
**Severity**: LOW
**Doc claim**: Doc 20 (701 lines) §"The five approaches" evaluates: (1) MCP server, (2) ACP agent, (3) VS Code chat participant, (4) VS Code full extension, (5) VS Code fork. Recommends phased approach: ACP-first, MCP as universal adapter, VS Code extension when warranted, never fork. Doc banner: `Status: Proposed`.
**Reality**: Doc 20 is an architecture decision document with `Status: Proposed`. No IDE extension / plugin ships. `Grep 'ide_extension\|vscode_extension\|acp_agent' crates/ --include=*.rs` returns zero matches. `roko-mcp-code` crate ships as an MCP server (CLAUDE.md: "Code-intelligence MCP server | New in PR #13") — which partially covers approach #1, but it's not specifically an IDE integration.
**Fix sketch**: Doc 20 is appropriately marked "Proposed". It's a planning doc, not a shipping gap. Cross-link `roko-mcp-code` as the shipping MCP-server foundation.

---

## F.11 — ACP agent (`roko acp`) (Doc 20 §"ACP Agent")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `roko acp` would spawn Roko as an ACP-protocol agent for Zed / JetBrains / Neovim / Emacs.
**Reality**: `Grep 'acp\|roko acp\|ACP protocol' crates/ --include=*.rs` returns only references to ACP in `docs/02-agents/`. No shipping ACP mode. Frontier.

---

## F.12 — MCP server (cross-ref `roko-mcp-code`) (Doc 20 §"MCP Server")

**Status**: DONE (partial via code-intelligence)
**Severity**: —
**Doc claim**: MCP server approach exposes Roko tools via MCP protocol to VS Code Copilot / Cursor / Continue.
**Reality**: `crates/roko-mcp-code/` ships as a code-intelligence MCP server (CLAUDE.md "roko-mcp-code | crates/roko-mcp-code/ | Code-intelligence MCP server | New in PR #13"). This is approach #1 from Doc 20, shipping. Additional MCP integrations at `roko-mcp-github/`, `roko-mcp-slack/`, `roko-mcp-scripts/`, `roko-mcp-stdio/` (CLAUDE.md "Partial; see `tmp/ux-followup/05-partially-wired-subsystems.md`").
**Fix sketch**: Doc 20 should cite `roko-mcp-code` as shipping coverage of approach #1. The broader MCP-server family (GitHub / Slack / scripts / stdio) is partial per CLAUDE.md.

---

## F.13 — VS Code chat participant (Doc 20 §"VS Code Chat Participant")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `@roko` chat participant in VS Code.
**Reality**: No extension ships. Frontier.

---

## F.14 — VS Code full extension / fork (Doc 20 §"Full Extension", §"Fork")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Full VS Code extension vs fork. Doc recommends "never fork".
**Reality**: Neither ships. The decision document says "never fork" — so the NOT DONE status for the fork path is intentional + aligned with the recommendation.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 1 (F.12 MCP server shipping via `roko-mcp-code`) |
| PARTIAL | 2 (F.03 WCAG palette + keyboard nav, F.04 Doc 17 status-table undercounts) |
| NOT DONE | 11 (F.01 sonification, F.02 audio cues, F.05-F.09 UX innovation proposals, F.06 voice, F.07 gesture, F.08 collab, F.09 visualization patterns, F.10 ACP, F.11 ACP agent, F.13 VS Code chat, F.14 full extension) |

Section F is mostly frontier/proposal content. The one clear
shipping item is `roko-mcp-code` covering MCP-server approach from
Doc 20.

The most actionable item in section F is **Doc 17 §"Current
Status"** — it is topic 12's status doc and needs to reflect the
shipping reality (TUI + CLI + serve + sidecar) rather than the
"Scaffold" top-level banner.

## Agent Execution Notes

### F.04 — Regenerate Doc 17 status table

Doc 17 undersells. Its status table should cite the shipping LOC
counts: CLI ~20 subcommands, roko-serve 12,285 LOC, TUI 25,449 LOC,
agent-server ~1,800 LOC. The "Scaffold" top-level banner should be
"Mixed" (shipping + partial + frontier). It should also include the
current port/default inconsistency instead of implying one settled
control-plane address.

### F.05 — Split Doc 18 proposals

Doc 18 is 2,451 lines — too large to process as a single "proposals"
document. Split into "near-term proposals" vs "speculative" so
readers can prioritize.

### F.10-F.14 — IDE integration status

Doc 20 is a well-written decision document. The only update needed
is to cite `roko-mcp-code` as shipping coverage of approach #1.

Acceptance criteria:

- Doc 17 status table reflects shipping CLI / serve / TUI / sidecar,
- Doc 17 does not silently collapse the `9090` vs `6677` split,
- Doc 18 split into near-term + speculative,
- Doc 20 cites `roko-mcp-code` + MCP family as partial approach #1.
