# Provider Dispatch Gap Analysis — Index

All differences between roko and mori's provider handling, what's broken, what needs fixing.

| Doc | What | Priority |
|-----|------|----------|
| [01-DISPATCH-PATHS.md](01-DISPATCH-PATHS.md) | All 6 roko dispatch paths vs mori's 3, with entry points and line numbers | P0 |
| [02-MODEL-SELECTION.md](02-MODEL-SELECTION.md) | Every hardcoded model string, fallback chains, how mori selects models | P0 |
| [03-TOOL-OUTPUT.md](03-TOOL-OUTPUT.md) | How tool outputs flow in mori vs roko, what's fixed, what's still broken | P0 |
| [04-STREAMING-PROTOCOL.md](04-STREAMING-PROTOCOL.md) | Mori's typed ClaudeStreamEvent parsing vs roko's ad-hoc JSON pointers | P0 |
| [05-SESSION-RESUME.md](05-SESSION-RESUME.md) | Session ID capture, --resume passing, conversation continuity | P1 |
| [06-TOKEN-LIMITS-COST.md](06-TOKEN-LIMITS-COST.md) | Max tokens, role budgets, cost tracking, why chat shows $0 | P0 |
| [07-CONFIG-AUTH.md](07-CONFIG-AUTH.md) | Auth detection chain, config merge, why config is ignored at dispatch | P0 |
| [08-PER-ROLE-TOOLS.md](08-PER-ROLE-TOOLS.md) | Mori's strict per-role tool allowlists vs roko's single-path enforcement | P1 |
| [09-ERROR-HANDLING.md](09-ERROR-HANDLING.md) | Error classification, stderr handling, DOA detection, health tracking | P1 |
| [10-HARDCODED-VALUES.md](10-HARDCODED-VALUES.md) | Every hardcoded model, URL, token limit, timeout with file:line | Reference |
| [11-ACTION-PLAN.md](11-ACTION-PLAN.md) | Prioritized fix order with dependencies | Plan |
