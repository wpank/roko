# Dogfood: Resource Management — 2026-04-26

## Incident: OOM from zombie processes

### What happened
Running `roko plan run` spawned multiple claude-cli agents for enrichment. Some timed out
(120s limit), some crashed with exit signals. The parent roko process retried them multiple
times. Combined with Cursor (89GB!), the system ran out of memory.

### Process counts before cleanup
- claude: 44 processes
- roko: 9 processes
- codex: 25 processes
- cargo: 27 processes

### Root causes

1. **Enrichment retry loop spawns too many agents** — Failed enrichment steps are retried
   without killing the previous timed-out process. Each retry spawns a new claude process.
   With 13 enrichment steps × retries = dozens of zombie processes.

2. **No process cleanup on timeout** — When `timeout 120s claude ...` fires, the claude
   process may not actually die (it spawns child processes that survive the parent kill).

3. **No resource budget** — No limit on total concurrent agents. The enrichment pipeline
   runs steps in parallel without constraining how many claude processes exist simultaneously.

### Fixes needed

1. **Kill process tree on timeout** — Use `timeout --kill-after=10` and kill the entire
   process group, not just the parent PID.

2. **Limit concurrent agents** — Add `max_concurrent_agents = 2` to executor config.
   The enrichment pipeline should queue, not stampede.

3. **Track child PIDs** — The agent dispatcher should track spawned PIDs and kill them
   on shutdown/timeout. Currently PIDs are fire-and-forget.

4. **Skip enrichment for authored plans** — Add `[meta] skip_enrichment = true` support.
   The 13-step enrichment pipeline is overkill for plans with complete task definitions.
