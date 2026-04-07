# Episode Logger Design Notes

## Data model
Each episode captures one agent invocation:
- task_id, agent_name, role
- prompt (system + user)
- response (raw text + tool calls)
- duration_ms, token_count
- gate_results (pass/fail per rung)
- cost_usd

## Storage
- Append-only JSONL at `.roko/episodes.jsonl`
- One line per episode
- HDC fingerprint computed from prompt+response
- Used for playbook extraction and efficiency tracking

## Integration point
orchestrate.rs `dispatch_agent_with()` → log episode after response
