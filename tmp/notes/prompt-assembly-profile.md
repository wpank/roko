# Prompt Assembly Profiling

## SystemPromptBuilder (9 layers)
1. Base persona
2. Role-specific instructions
3. Tool descriptions
4. Context window (recent signals)
5. Task description
6. Playbook examples
7. Safety constraints
8. Output format
9. Meta-instructions

## Profile results
- Layer 1-2: <1ms (static strings)
- Layer 3: ~5ms (tool enumeration)
- Layer 4: ~15ms (signal query + format)
- Layer 5: <1ms
- Layer 6: ~20ms (playbook DB query)
- Layer 7: <1ms
- Layer 8-9: <1ms
- Total: ~42ms avg

## Bottleneck: layers 4 and 6 (DB queries)
- Cache playbook results per-role (TTL 60s)
- Pre-format recent signals on write (not read)
