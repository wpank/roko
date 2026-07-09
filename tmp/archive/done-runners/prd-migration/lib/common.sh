#!/usr/bin/env bash
# common.sh — shared utilities for the PRD migration runner.
# Sourced by run-migration.sh and helper scripts.

set -uo pipefail

# --- Paths -------------------------------------------------------------------

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${REFAC_ROOT:=/Users/will/dev/nunchi/roko/refactoring-prd}"
: "${MIGRATION_ROOT:=$ROKO_ROOT/tmp/prd-migration}"
: "${OUTPUT_ROOT:=$ROKO_ROOT/docs}"
: "${LOG_ROOT:=$MIGRATION_ROOT/logs}"
: "${CONTEXT_PACK:=$MIGRATION_ROOT/context-pack}"
: "${PROMPTS_DIR:=$MIGRATION_ROOT/prompts}"
: "${VERIFY_DIR:=$MIGRATION_ROOT/verify}"

# --- Colors ------------------------------------------------------------------

if [[ -t 1 && "${NO_COLOR:-}" == "" ]]; then
    C_RESET=$'\e[0m'
    C_BOLD=$'\e[1m'
    C_DIM=$'\e[2m'
    C_RED=$'\e[31m'
    C_GREEN=$'\e[32m'
    C_YELLOW=$'\e[33m'
    C_BLUE=$'\e[34m'
    C_MAGENTA=$'\e[35m'
    C_CYAN=$'\e[36m'
else
    C_RESET='' C_BOLD='' C_DIM='' C_RED='' C_GREEN='' C_YELLOW='' C_BLUE='' C_MAGENTA='' C_CYAN=''
fi

# --- Logging -----------------------------------------------------------------

# log_<level> <topic> <message>
log_info()   { printf '%s[INFO]%s  %s%-18s%s %s\n' "$C_BLUE"   "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_ok()     { printf '%s[OK]%s    %s%-18s%s %s\n' "$C_GREEN"  "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_warn()   { printf '%s[WARN]%s  %s%-18s%s %s\n' "$C_YELLOW" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_err()    { printf '%s[ERR]%s   %s%-18s%s %s\n' "$C_RED"    "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_header() { printf '\n%s=== %s ===%s\n\n' "$C_BOLD$C_MAGENTA" "$1" "$C_RESET"; }

# --- Topic list --------------------------------------------------------------

# All 22 topics in order. Format: "NN-slug"
#
# If ROKO_MIGRATION_TEST_MODE=1, only a single test-smoke topic is used. This is
# how verify/test-runner-e2e.sh exercises the full run-migration.sh pipeline
# (arg parsing, preflight, subshell, backgrounding, spawn_topic, verify_topic)
# with a tiny prompt that doesn't cost much.
if [[ "${ROKO_MIGRATION_TEST_MODE:-0}" == "1" ]]; then
    ALL_TOPICS=("test-smoke")
else
    ALL_TOPICS=(
        "00-architecture"
        "01-orchestration"
        "02-agents"
        "03-composition"
        "04-verification"
        "05-learning"
        "06-neuro"
        "07-conductor"
        "08-chain"
        "09-daimon"
        "10-dreams"
        "11-safety"
        "12-interfaces"
        "13-coordination"
        "14-identity-economy"
        "15-code-intelligence"
        "16-heartbeat"
        "17-lifecycle"
        "18-tools"
        "19-deployment"
        "20-technical-analysis"
        "21-references"
    )
fi

# --- Helpers -----------------------------------------------------------------

# is_topic_complete <topic> → 0 if the topic's INDEX.md exists and is non-empty.
is_topic_complete() {
    local topic="$1"
    local index="$OUTPUT_ROOT/$topic/INDEX.md"
    [[ -s "$index" ]]
}

# topic_output_dir <topic> → echoes the output directory
topic_output_dir() {
    echo "$OUTPUT_ROOT/$1"
}

# topic_prompt_file <topic> → echoes the prompt file path
topic_prompt_file() {
    echo "$PROMPTS_DIR/${1}.prompt.md"
}

# topic_log_file <topic> <run_id> → echoes the log file path
topic_log_file() {
    echo "$LOG_ROOT/$2/${1}.log"
}

# topic_json_file <topic> <run_id> → echoes the JSON transcript path
topic_json_file() {
    echo "$LOG_ROOT/$2/${1}.json"
}

# Ensure a directory exists
ensure_dir() {
    mkdir -p "$1"
}

# Assert a required file exists, else exit with error
require_file() {
    local f="$1"
    if [[ ! -f "$f" ]]; then
        log_err "bootstrap" "Required file not found: $f"
        exit 1
    fi
}

# Pretty duration formatter: seconds → "1h 23m 45s"
fmt_duration() {
    local s="$1"
    local h=$((s / 3600))
    local m=$(((s % 3600) / 60))
    local sec=$((s % 60))
    if (( h > 0 )); then
        printf '%dh %dm %ds' "$h" "$m" "$sec"
    elif (( m > 0 )); then
        printf '%dm %ds' "$m" "$sec"
    else
        printf '%ds' "$sec"
    fi
}

# Check that the `claude` binary is available
check_claude_cli() {
    if ! command -v claude >/dev/null 2>&1; then
        log_err "bootstrap" "claude CLI not found in PATH. Install Claude Code CLI first."
        log_err "bootstrap" "  npm install -g @anthropic-ai/claude-code"
        exit 1
    fi
}

# preflight_check — comprehensive pre-run health check.
# Run this before any spawn to catch config / environment problems early.
preflight_check() {
    local errors=0

    log_header "PREFLIGHT CHECKS"

    # 1. claude CLI available
    if command -v claude >/dev/null 2>&1; then
        local claude_path
        claude_path=$(command -v claude)
        local claude_version
        claude_version=$(env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT claude --version 2>&1 | head -1)
        log_ok "preflight" "claude CLI: $claude_path ($claude_version)"
    else
        log_err "preflight" "claude CLI not found. Install: npm install -g @anthropic-ai/claude-code"
        errors=$((errors + 1))
    fi

    # 2. CLAUDECODE can be unset (critical for nested sessions)
    if [[ -n "${CLAUDECODE:-}" ]]; then
        log_warn "preflight" "CLAUDECODE is set ('${CLAUDECODE}'). The runner unsets it automatically via env -u."
    else
        log_ok "preflight" "CLAUDECODE not set"
    fi

    # 3. Required directories
    local dirs=(
        "$ROKO_ROOT"
        "$REFAC_ROOT"
        "/Users/will/dev/nunchi/roko/bardo-backup"
        "$MIGRATION_ROOT"
        "$CONTEXT_PACK"
        "$PROMPTS_DIR"
    )
    for d in "${dirs[@]}"; do
        if [[ -d "$d" ]]; then
            log_ok "preflight" "dir: $d"
        else
            log_err "preflight" "MISSING dir: $d"
            errors=$((errors + 1))
        fi
    done

    # 4. Context-pack files (must be 7 + README)
    local required_context=(
        "00-ALWAYS-READ-FIRST.md"
        "01-naming-map.md"
        "02-reframe-rules.md"
        "03-concepts-lifecycle.md"
        "04-writing-rules.md"
        "05-source-files.md"
        "06-output-structure.md"
    )
    local missing_context=0
    for f in "${required_context[@]}"; do
        if [[ ! -s "$CONTEXT_PACK/$f" ]]; then
            log_err "preflight" "MISSING context-pack file: $f"
            missing_context=$((missing_context + 1))
            errors=$((errors + 1))
        fi
    done
    if (( missing_context == 0 )); then
        log_ok "preflight" "context-pack: all 7 files present"
    fi

    # 5. Prompt files (one per topic)
    local missing_prompts=0
    local topic
    for topic in "${ALL_TOPICS[@]}"; do
        local pfile="$PROMPTS_DIR/${topic}.prompt.md"
        if [[ ! -s "$pfile" ]]; then
            log_err "preflight" "MISSING prompt: ${topic}.prompt.md"
            missing_prompts=$((missing_prompts + 1))
            errors=$((errors + 1))
        fi
    done
    if (( missing_prompts == 0 )); then
        log_ok "preflight" "prompts: all ${#ALL_TOPICS[@]} files present"
    fi

    # 6. Key source files exist (spot check — if these are missing, something is badly wrong)
    local key_sources=(
        "$REFAC_ROOT/00-overview.md"
        "$REFAC_ROOT/01-synapse-architecture.md"
        "$REFAC_ROOT/08-translation-guide.md"
        "$REFAC_ROOT/09-innovations.md"
        "$MIGRATION_ROOT/SOURCE-INDEX.md"
        "$ROKO_ROOT/crates/roko-core/src/lib.rs"
    )
    local missing_sources=0
    for f in "${key_sources[@]}"; do
        if [[ ! -s "$f" ]]; then
            log_err "preflight" "MISSING source: $f"
            missing_sources=$((missing_sources + 1))
            errors=$((errors + 1))
        fi
    done
    if (( missing_sources == 0 )); then
        log_ok "preflight" "key source files: all present"
    fi

    # 7. Required binaries for verification
    local required_bins=(grep wc find stat)
    local missing_bins=0
    for b in "${required_bins[@]}"; do
        if ! command -v "$b" >/dev/null 2>&1; then
            log_err "preflight" "MISSING binary: $b (needed for verification)"
            missing_bins=$((missing_bins + 1))
            errors=$((errors + 1))
        fi
    done
    if (( missing_bins == 0 )); then
        log_ok "preflight" "binaries: grep, wc, find, stat all available"
    fi

    if (( errors > 0 )); then
        log_err "preflight" "$errors errors — refusing to start migration run"
        return 1
    fi

    log_ok "preflight" "all checks passed — ready to run"
    return 0
}
