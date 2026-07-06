# V2-Depth Research Prompt Ledger

> Status-quo audit · re-verified 2026-07-08 · git HEAD `5852c93c05a4` · scope: the 14 `docs/v2-depth/RESEARCH-PROMPT*.md` files (2,720 lines total).

The 14 `RESEARCH-PROMPT*.md` files are **strategic source material, not specification and not implementation truth.** Each is literally a prompt authored to be pasted into Claude Desktop with deep-research enabled ("Copy everything below the `---` line into Claude Desktop…" is the second line of every file). They contain pitch framing, market/category positioning, competitor intelligence, demo direction, and convergence questions. They should inform product narrative and prioritization, but **a claim in one of these files is not true about the codebase unless it is tied to code, a proof command, or a dated external source.** They sit inside `docs/v2-depth/` next to real spec sections, which is exactly why they need fencing: a reader skimming the directory can mistake investor-deck copy for architecture.

## Verified fencing status (this pass)

**The banner checklist item is NOT done.** Re-grep of all 14 files finds **zero** occurrences of any `STRATEGIC SOURCE` / `NOT CURRENT IMPLEMENTATION TRUTH` banner. The only two hits for "aspirational" (`RESEARCH-PROMPT-11.md:31`, `RESEARCH-PROMPT-12.md:29`) are incidental prose about naming strategy ("Casado rewards descriptive structural names, never aspirational ones"), **not** fencing banners. So today nothing at the file level distinguishes these prompts from the spec sections around them — the fence exists only here in the status-quo pack, not in the docs themselves. This is the single highest-value cleanup: adding a one-line banner to each of the 14 files is mechanical and closes the confusion surface.

## Inventory

| File | Focus | Status-pack treatment | Key drift risk |
|---|---|---|---|
| `RESEARCH-PROMPT.md` | First deep-research synthesis prompt. | Strategy input. | Oldest; pre-integration assumptions. |
| `RESEARCH-PROMPT-2.md` | Round 2, post-integration. | Strategy input; verify integration claims against code. | "Post-integration" wording implies shipped state that must be re-checked. |
| `RESEARCH-PROMPT-3.md` | Round 3, frontier capabilities. | Research/future, not roadmap proof. | Frontier ≠ shipped. |
| `RESEARCH-PROMPT-4.md` | Round 5, from spec to reality. | Useful for convergence framing. | "Spec to reality" language is the exact seam to fence. |
| `RESEARCH-PROMPT-5.md` | Round 6, Series A intelligence. | Investor narrative; fence claims. | Series-A numbers need source/date. |
| `RESEARCH-PROMPT-6.md` | Round 7, execution intelligence. | Execution thesis, not default-engine truth. | Do NOT cite as evidence that default `plan run` is Graph — it's RunnerV2 (see `18`, `37`). |
| `RESEARCH-PROMPT-7.md` | Round 8, business model, growth, developer UX. | Product-market source only. | GTM claims, not features. |
| `RESEARCH-PROMPT-8.md` | Round 9, narrative/category/untapped mechanisms. | Narrative source only. | "Untapped mechanisms" may name things with no code. |
| `RESEARCH-PROMPT-9.md` | Round 9b, deck words, numbers, visuals. | Deck source; all numbers need source/date checks. | Every quantitative claim is unverified. |
| `RESEARCH-PROMPT-10.md` | Category definition for agent infrastructure. | Category source; map to current surfaces before use. | Category framing, not surface inventory. |
| `RESEARCH-PROMPT-11.md` | Beachhead, demo, convergence proof for ACP; names category **"Agent Coordination Plane"**. | High value for ACP/demo roadmap; verify against `51-ACP.md`, `73`, `70`. | Names competitors (Capsule, Nava, t54, Sycamore, /dev/agents) with funding figures — all need dated sources. |
| `RESEARCH-PROMPT-12.md` | Aubakirova memo, deck copy, remaining gaps. | Strategy/memo source. | "Remaining gaps" list may be stale vs `.roko/GAPS.md`. |
| `RESEARCH-PROMPT-13.md` | May 6 a16z pitch prep. | Time-bound pitch source. | Dated event; do not import as current. |
| `RESEARCH-PROMPT-14.md` | Actual deck, memo, demo commands. | Demo-command archaeology; commands need current CLI proof. | "Actual demo commands" must be smoke-tested against current CLI/schema before reuse. |

## Current relevance → current-state anchors

| Theme | Current-state anchor (status-quo pack) | Caution |
|---|---|---|
| Agent coordination plane | `51-ACP.md`, `70-RELAY-PROTOCOL-FREEZE.md`, `59-API-ROUTE-LEDGER.md` | ACP exists (`roko-acp/src/lib.rs`, JSON-RPC 2.0 over stdio), relay is real (`apps/agent-relay`), but permissions/capability truth and relay route shape are still gaps. |
| Execution intelligence | `37-RUNNER-V2-AND-GRAPH.md`, `73-EXAMPLES-PLANS-GRAPHS.md`, `18-V2-DEPTH-COVERAGE.md` | **RunnerV2 is the `#[default]` engine** (`roko-cli/src/main.rs:1301`); Graph is opt-in and dry-runs (7 PassthroughCell stubs). Do NOT repeat any command implying default `plan run` uses Graph. |
| Business/category narrative | `01-EXECUTIVE-SUMMARY.md`, `12-ROADMAP.md` | Narrative should follow current implementation facts, not aspirational v2 labels. |
| Demo commands | `73-EXAMPLES-PLANS-GRAPHS.md`, `77-OPERATIONS-DEPLOY-RUNBOOK.md` | Commands must be smoke-tested; older graph/demo examples include stale schema. |
| Market numbers & competitor claims | External dated sources, not this repo. | RP-11's competitor funding figures (Sycamore $65M, /dev/agents $56M, etc.) are unverified — reverify before any investor/customer doc. |

## Checklist

- [ ] **[P1]** Add a banner to **every** research prompt (all 14 currently lack one): `> STRATEGIC SOURCE — NOT CURRENT IMPLEMENTATION TRUTH. Pitch/market/demo seed for deep research; verify every claim against code, a proof command, or a dated external source before reuse.` — verify: `grep -Lc 'STRATEGIC SOURCE' docs/v2-depth/RESEARCH-PROMPT*.md` returns empty.
- [ ] **[P1]** Extract demo commands (esp. RP-14 "actual demo commands") into a dated command ledger and run each against current CLI `--help`/schema; flag any that assume Graph as default engine.
- [ ] **[P2]** Map ACP claims to `roko-acp`, relay, serve routes, permissions, and image/MCP capability proof (`51`, `70`).
- [ ] **[P2]** Map category/business/competitor claims (RP-9/10/11/12) to a source/date field before using them in decks or README copy.
- [ ] **[P2]** Convert still-relevant convergence prompts into issues only after they cite `tmp/status-quo` proof gates.
- [ ] **[P3]** Consider relocating the 14 prompts out of `docs/v2-depth/` (e.g. `docs/strategy/` or `tmp/`) so the spec directory contains only spec — the co-location is the root cause of the fencing risk.
- [ ] Do not import market numbers, deck claims, or "actual demo" language into maintained docs without revalidation.
