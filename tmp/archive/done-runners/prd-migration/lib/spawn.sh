#!/usr/bin/env bash
# spawn.sh — spawns a Claude agent for one topic.
# Sourced by run-migration.sh.

# shellcheck source=common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# --- Parameters --------------------------------------------------------------

# Default model (Opus 4.6 with 1M context). Override via env var.
: "${ROKO_MIGRATION_MODEL:=claude-opus-4-6}"
# Per-topic timeout in seconds (default 45 min).
: "${ROKO_MIGRATION_TIMEOUT:=2700}"
# Per-topic budget cap in USD (default $15 — generous; most topics will cost $3-8).
: "${ROKO_MIGRATION_BUDGET_USD:=15}"
# Extra flags for the claude CLI.
: "${ROKO_MIGRATION_CLAUDE_FLAGS:=}"

# --- Main spawn function -----------------------------------------------------

# spawn_topic <topic> <run_id>
#   - Reads $PROMPTS_DIR/<topic>.prompt.md
#   - Pipes it to `claude -p` with Opus
#   - Allows the agent to use Read, Write, Edit, Bash, Glob, Grep tools
#   - Writes transcript to $LOG_ROOT/<run_id>/<topic>.log
#   - Returns 0 on success, 1 on failure
spawn_topic() {
    local topic="$1"
    local run_id="$2"

    local prompt_file
    prompt_file=$(topic_prompt_file "$topic")
    local log_file
    log_file=$(topic_log_file "$topic" "$run_id")
    local output_dir
    output_dir=$(topic_output_dir "$topic")

    if [[ ! -f "$prompt_file" ]]; then
        log_err "$topic" "Prompt file missing: $prompt_file"
        return 1
    fi

    ensure_dir "$(dirname "$log_file")"
    ensure_dir "$output_dir"

    log_info "$topic" "Spawning Opus agent (timeout $(fmt_duration "$ROKO_MIGRATION_TIMEOUT"))..."
    local start_ts
    start_ts=$(date +%s)

    # Build the full command as ONE array. This avoids all the edge cases of:
    # (a) IFS=$'\n\t' in run-migration.sh preventing space-based word-splitting
    #     of unquoted string expansions like `$timeout_cmd`
    # (b) empty-array expansion with set -u on older bash
    # (c) macOS lacking `timeout` (falls through to gtimeout or no timeout at all)
    #
    # `env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT` is required so we can spawn
    # Claude from inside another Claude Code session. Without this, the nested-
    # session guard rejects the invocation.
    local -a cmd=()

    # Optionally prefix with timeout.
    if command -v timeout >/dev/null 2>&1; then
        cmd+=(timeout "${ROKO_MIGRATION_TIMEOUT}s")
    elif command -v gtimeout >/dev/null 2>&1; then
        cmd+=(gtimeout "${ROKO_MIGRATION_TIMEOUT}s")
    fi

    # Core claude invocation.
    # --output-format stream-json: each tool call, partial message, and final
    #   result is emitted as a JSON event on its own line. This gives real-time
    #   visibility into what the agent is doing (otherwise text mode only prints
    #   the final result after the entire agent run is complete).
    # --verbose: required when using --output-format stream-json with --print.
    # --include-partial-messages: emit partial text chunks as they arrive so
    #   the agent's in-progress writing is visible.
    # --disallowedTools: CRITICAL for preventing cost explosions.
    #   - Task: prevents agents from spawning sub-agents via the Task tool,
    #     which would create nested Claude sessions, multiply cost, and bypass
    #     our per-topic budget cap.
    #   - TodoWrite: optional — keeps output focused on actual work.
    #   - WebFetch / WebSearch: the migration is strictly local, no web.
    cmd+=(
        env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT
        claude
        --print
        --verbose
        --output-format stream-json
        --include-partial-messages
        --model "$ROKO_MIGRATION_MODEL"
        --permission-mode bypassPermissions
        --max-budget-usd "$ROKO_MIGRATION_BUDGET_USD"
        --disallowedTools Task TodoWrite WebFetch WebSearch
        --add-dir "$ROKO_ROOT"
        --add-dir "$REFAC_ROOT"
        --add-dir "/Users/will/dev/nunchi/roko/bardo-backup"
    )

    # Optional extra flags from ROKO_MIGRATION_CLAUDE_FLAGS env var.
    if [[ -n "${ROKO_MIGRATION_CLAUDE_FLAGS:-}" ]]; then
        # Use read to split on default IFS (space/tab/newline), unaffected by
        # the calling shell's modified IFS.
        local -a extra_flags=()
        IFS=$' \t\n' read -r -a extra_flags <<<"$ROKO_MIGRATION_CLAUDE_FLAGS"
        cmd+=("${extra_flags[@]}")
    fi

    # Run the agent. All stdout+stderr goes to the log file.
    # The prompt file is piped to stdin (-p/--print mode reads from stdin).
    local exit_code=0
    {
        echo "=== Migration run: $run_id ==="
        echo "=== Topic: $topic ==="
        echo "=== Started: $(date -Iseconds) ==="
        echo "=== Model: $ROKO_MIGRATION_MODEL ==="
        echo "=== Budget cap: \$${ROKO_MIGRATION_BUDGET_USD} ==="
        echo "=== Prompt: $prompt_file ==="
        echo "=== Output: $output_dir ==="
        echo "=== Command: ${cmd[*]} ==="
        echo
    } > "$log_file"

    "${cmd[@]}" \
        < "$prompt_file" \
        >> "$log_file" 2>&1 || exit_code=$?

    local end_ts
    end_ts=$(date +%s)
    local duration=$((end_ts - start_ts))

    {
        echo
        echo "=== Finished: $(date -Iseconds) ==="
        echo "=== Duration: $(fmt_duration "$duration") ==="
        echo "=== Exit code: $exit_code ==="
    } >> "$log_file"

    if [[ $exit_code -eq 0 ]]; then
        log_ok "$topic" "Agent completed in $(fmt_duration "$duration")"
        return 0
    elif [[ $exit_code -eq 124 ]]; then
        log_err "$topic" "TIMEOUT after $(fmt_duration "$ROKO_MIGRATION_TIMEOUT")"
        return 1
    else
        log_err "$topic" "Agent exited with code $exit_code (see log: $log_file)"
        return 1
    fi
}

# spawn_topic_dry_run <topic> <run_id>
#   Prints what would be executed without actually spawning the agent.
spawn_topic_dry_run() {
    local topic="$1"
    local run_id="$2"

    local prompt_file
    prompt_file=$(topic_prompt_file "$topic")
    local output_dir
    output_dir=$(topic_output_dir "$topic")
    local log_file
    log_file=$(topic_log_file "$topic" "$run_id")

    log_info "$topic" "[DRY RUN] Would execute:"
    printf '  env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT claude --print \\\n'
    printf '    --model %s \\\n' "$ROKO_MIGRATION_MODEL"
    printf '    --permission-mode bypassPermissions \\\n'
    printf '    --max-budget-usd %s \\\n' "$ROKO_MIGRATION_BUDGET_USD"
    printf '    --add-dir %s \\\n' "$ROKO_ROOT"
    printf '    --add-dir %s \\\n' "$REFAC_ROOT"
    printf '    --add-dir /Users/will/dev/nunchi/roko/bardo-backup \\\n'
    printf '    < %s \\\n' "$prompt_file"
    printf '    > %s\n' "$log_file"
    printf '  output dir: %s\n' "$output_dir"

    if [[ ! -f "$prompt_file" ]]; then
        log_warn "$topic" "  !! prompt file does not exist"
        return 1
    fi

    local prompt_lines
    prompt_lines=$(wc -l < "$prompt_file" | tr -d ' ')
    log_info "$topic" "  prompt: $prompt_lines lines"

    return 0
}
