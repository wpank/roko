# Full Self-Hosting Demo

The complete roko loop: prompt → plan → agents → gates → learn.

## The loop

```
1. Research  →  roko research topic "..."                  (optional)
2. Plan      →  roko plan generate "..."
3. Review    →  edit plans/<slug>/, then roko plan validate plans/<slug>
4. Match     →  roko job match "..." --skills X
5. Post      →  roko job create "..." --type coding_task
6. Execute   →  roko run plans/<slug>
7. Learn     →  roko learn all
8. Iterate   →  (gate failures trigger replan)
```

`roko run --plan "..."` does steps 2 and 6 in one command: it writes the plan,
shows it, and runs it once you confirm.

## Dashboard walkthrough

This is the suggested demo order for showing the full self-hosting capability:

### Act 1: Research
```
roko research topic "agent matchmaking algorithms in decentralized systems"
```
The report lands in `.roko/research/`, with citations.

### Act 2: Plan (portal)
Open the portal link `roko serve` prints and type the prompt:
```
Wire knowledge query into agent matchmaking
```
The generated plan appears with its task tree. Edit its `tasks.toml` as text
if a task needs changing; the server validates each save. The CLI equivalent is
`roko plan generate "Wire knowledge query into agent matchmaking"`.

### Act 3: Match (Nunchi dashboard chat)
```
/coding implement knowledge-weighted matchmaking
```
Accept the agent quote. Job appears in Coding tab.

### Act 4: Execute
Press Run on the plan in the portal, or run
`roko run plans/wire-knowledge-query-into-agent-matchmaking`. `roko dashboard`
shows the tasks on **F2 Plans** and the live agent output on **F3 Agents**.

### Act 5: Observe (Network tabs)
- **Agents** — Fleet roster with tiers
- **Jobs** — Active job board
- **Learning** — C-Factor, efficiency, cascade router
- **Swarm** — Network topology (if agents have heartbeats)

## Script

`demo-full-loop.sh` runs the entire flow interactively with pauses.

## What generates learning data

Each step produces data the Learning tab consumes:

| Step | Learning artifact |
|------|-------------------|
| Plan execution | Episodes in `.roko/episodes.jsonl` |
| Gate checks | Adaptive thresholds in `.roko/learn/gate-thresholds.json` |
| Model routing | Cascade router state in `.roko/learn/cascade-router.json` |
| Agent dispatch | Efficiency events in `.roko/learn/efficiency.jsonl` |
| A/B experiments | Experiment store in `.roko/learn/experiments.json` |
