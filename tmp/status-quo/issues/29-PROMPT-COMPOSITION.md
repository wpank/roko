# Prompt Composition Issues

## High

### Two parallel prompt-building surfaces
- Runner v2: `dispatch/prompt_builder.rs` — ad-hoc `# Role`, `# Task`, `# Files` sections.
- Legacy: `roko-compose/SystemPromptBuilder` — canonical 9-layer with cache-tier alignment.
- Runner v2 lacks: full role identity manifests, conventions layer, tool instructions layer, affect integration layer, cache-tier markers.
- Acknowledged gap at `prompt_builder.rs:714`.

### Token budget hardcoded at 64K, not model-aware
- `prompt_builder.rs:52`: `const DEFAULT_TOKEN_BUDGET: u32 = 64_000`. Used unconditionally.
- `factory.rs:85-88`: No call to `with_token_budget()`. A 200K-window model wastes context; smaller models clip silently.

### `collect_playbooks` aborts entire iteration on any I/O error
- `prompt_builder.rs:1282-1287`: `entry.ok()?` and `read_to_string().ok()?` return `None` on single file failure → all playbooks skipped. No log warning.

## Medium

### `git_command()` timeout never enforced
- `prompt_builder.rs:362-377`: `GIT_COMMAND_TIMEOUT` constant defined (3s) but never used. No `join_timeout`. Blocks Tokio worker indefinitely on slow filesystem.

### `PromptAssemblyService` scans only `src/`
- `prompt_assembly_service.rs:728-738`: Hardcoded to `workdir.join("src")`. For workspace projects (18 crates at `crates/*/src/`), directory doesn't exist → conventions layer silently skipped.

### Workspace label hardcoded
- `dispatch_helpers.rs:129,164`, `prompt_helpers.rs:90,125`: Always `"roko-cli orchestration"` regardless of actual workspace.

### Dependency outputs dropped before retry feedback
- `prompt_builder.rs:874-886`: Drop priority 7 vs retry feedback priority 5. Large prompts lose dependency context silently.
