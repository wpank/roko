#!/usr/bin/env bash
# run-migration.sh — orchestrates the PRD migration overnight.
#
# For each of the 22 topics in lib/common.sh:ALL_TOPICS, it:
#   1. Checks if the topic is already complete (resumability).
#   2. Spawns a fresh Claude Opus agent with the topic's prompt file.
#   3. Runs verification checks after the agent finishes.
#   4. Logs everything under logs/<run_id>/.
#
# Agents run in parallel up to --parallel N (default 3).
#
# Usage:
#   run-migration.sh                         # run all remaining topics, default parallelism
#   run-migration.sh --parallel 5            # use 5 parallel agents
#   run-migration.sh --only 00-architecture  # run a single topic
#   run-migration.sh --only "02,05,08"       # run specific topics (comma-separated)
#   run-migration.sh --dry-run               # show what would run, don't spawn agents
#   run-migration.sh --force                 # re-run even if output exists
#   run-migration.sh --verify-only           # skip spawning, just verify existing output
#   run-migration.sh --list                  # list topics and their completion state
#
# Environment variables:
#   ROKO_MIGRATION_MODEL         model (default: claude-opus-4-6)
#   ROKO_MIGRATION_TIMEOUT       per-topic timeout in seconds (default: 2700)
#   ROKO_MIGRATION_PARALLEL      default parallelism (default: 3)
#   ROKO_MIGRATION_CLAUDE_FLAGS  extra flags to pass to the claude CLI

set -uo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"
# shellcheck source=lib/spawn.sh
source "$SCRIPT_DIR/lib/spawn.sh"
# shellcheck source=lib/verify.sh
source "$SCRIPT_DIR/lib/verify.sh"

# --- Defaults ----------------------------------------------------------------

: "${ROKO_MIGRATION_PARALLEL:=3}"

PARALLEL=$ROKO_MIGRATION_PARALLEL
DRY_RUN=0
FORCE=0
VERIFY_ONLY=0
LIST_ONLY=0
SELECTED_TOPICS=()

# --- Arg parsing -------------------------------------------------------------

print_usage() {
    sed -n '/^#/,/^$/p' "$0" | sed 's/^# \{0,1\}//' | head -40
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --parallel) PARALLEL="$2"; shift 2 ;;
        --parallel=*) PARALLEL="${1#*=}"; shift ;;
        --only) IFS=',' read -r -a SELECTED_TOPICS <<<"$2"; shift 2 ;;
        --only=*) IFS=',' read -r -a SELECTED_TOPICS <<<"${1#*=}"; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
        --force) FORCE=1; shift ;;
        --verify-only) VERIFY_ONLY=1; shift ;;
        --list) LIST_ONLY=1; shift ;;
        -h|--help) print_usage; exit 0 ;;
        *) log_err "cli" "Unknown argument: $1"; print_usage; exit 1 ;;
    esac
done

# --- Topic selection ---------------------------------------------------------

select_topics() {
    if [[ ${#SELECTED_TOPICS[@]} -eq 0 ]]; then
        printf '%s\n' "${ALL_TOPICS[@]}"
        return
    fi

    local raw
    for raw in "${SELECTED_TOPICS[@]}"; do
        # Accept both "00-architecture" and "00" forms.
        local match=""
        local candidate
        for candidate in "${ALL_TOPICS[@]}"; do
            if [[ "$candidate" == "$raw" ]] || [[ "$candidate" == "$raw-"* ]]; then
                match="$candidate"
                break
            fi
        done
        if [[ -n "$match" ]]; then
            echo "$match"
        else
            log_err "cli" "Unknown topic: $raw"
            exit 1
        fi
    done
}

# --- List mode ---------------------------------------------------------------

list_topics() {
    printf '%s%-18s %-12s %s%s\n' "$C_BOLD" "TOPIC" "STATUS" "OUTPUT" "$C_RESET"
    local topic
    for topic in "${ALL_TOPICS[@]}"; do
        local status
        if is_topic_complete "$topic"; then
            status="${C_GREEN}DONE${C_RESET}"
        else
            status="${C_DIM}pending${C_RESET}"
        fi
        local out
        out=$(topic_output_dir "$topic")
        printf '%-18s %-20s %s\n' "$topic" "$status" "$out"
    done
}

# --- Worker function (for parallel execution) --------------------------------

# process_topic <topic> <run_id>
#   Spawns agent, verifies output, writes result to logs/<run_id>/<topic>.result.
process_topic() {
    local topic="$1"
    local run_id="$2"

    local result_file="$LOG_ROOT/$run_id/${topic}.result"

    # --verify-only takes precedence: always re-verify existing output.
    if (( VERIFY_ONLY == 1 )); then
        local verify_rc=0
        verify_topic "$topic" || verify_rc=$?
        case "$verify_rc" in
            0) echo "verified" > "$result_file" ;;
            2) echo "success_warnings" > "$result_file" ;;
            *) echo "verify_failed" > "$result_file"; return 1 ;;
        esac
        return 0
    fi

    if (( FORCE == 0 )) && is_topic_complete "$topic"; then
        log_info "$topic" "Already complete — skipping (use --force to re-run)"
        echo "skipped" > "$result_file"
        return 0
    fi

    if (( DRY_RUN == 1 )); then
        spawn_topic_dry_run "$topic" "$run_id"
        echo "dry_run" > "$result_file"
        return 0
    fi

    # Real spawn.
    if ! spawn_topic "$topic" "$run_id"; then
        echo "spawn_failed" > "$result_file"
        return 1
    fi

    # Verification.
    local verify_rc=0
    verify_topic "$topic" || verify_rc=$?
    case "$verify_rc" in
        0) echo "success" > "$result_file" ;;
        2) echo "success_warnings" > "$result_file" ;;
        *) echo "verify_failed" > "$result_file"; return 1 ;;
    esac

    return 0
}

# --- Parallel execution loop -------------------------------------------------

run_topics_parallel() {
    local run_id="$1"
    shift
    local -a topics=("$@")

    log_header "RUNNING ${#topics[@]} TOPICS (parallel=$PARALLEL, run_id=$run_id)"
    ensure_dir "$LOG_ROOT/$run_id"

    local -a pids=()
    local -a running_topics=()
    local topic

    for topic in "${topics[@]}"; do
        # Wait for a slot.
        while (( ${#pids[@]} >= PARALLEL )); do
            local new_pids=()
            local new_topics=()
            local i
            for i in "${!pids[@]}"; do
                if kill -0 "${pids[i]}" 2>/dev/null; then
                    new_pids+=("${pids[i]}")
                    new_topics+=("${running_topics[i]}")
                fi
            done
            pids=("${new_pids[@]}")
            running_topics=("${new_topics[@]}")
            if (( ${#pids[@]} >= PARALLEL )); then
                sleep 2
            fi
        done

        # Spawn.
        (process_topic "$topic" "$run_id") &
        pids+=("$!")
        running_topics+=("$topic")
        log_info "$topic" "Launched (pid=$!, slot=${#pids[@]}/$PARALLEL)"
    done

    # Wait for all stragglers.
    log_info "runner" "Waiting for ${#pids[@]} remaining agents..."
    wait
    log_ok "runner" "All agents finished"
}

# --- Summary -----------------------------------------------------------------

print_summary() {
    local run_id="$1"
    shift
    local -a topics=("$@")

    log_header "RUN SUMMARY: $run_id"

    local success=0 warnings=0 failed=0 skipped=0 dry=0
    local topic
    for topic in "${topics[@]}"; do
        local result_file="$LOG_ROOT/$run_id/${topic}.result"
        local result="unknown"
        [[ -f "$result_file" ]] && result=$(cat "$result_file")

        case "$result" in
            success) printf '  %s✓%s  %-22s passed\n' "$C_GREEN" "$C_RESET" "$topic"; success=$((success + 1)) ;;
            success_warnings) printf '  %s⚠%s  %-22s passed with warnings\n' "$C_YELLOW" "$C_RESET" "$topic"; warnings=$((warnings + 1)) ;;
            skipped) printf '  %s-%s  %-22s skipped (already done)\n' "$C_DIM" "$C_RESET" "$topic"; skipped=$((skipped + 1)) ;;
            verified) printf '  %s✓%s  %-22s verified\n' "$C_GREEN" "$C_RESET" "$topic"; success=$((success + 1)) ;;
            dry_run) printf '  %s·%s  %-22s dry run\n' "$C_CYAN" "$C_RESET" "$topic"; dry=$((dry + 1)) ;;
            spawn_failed) printf '  %s✗%s  %-22s agent failed\n' "$C_RED" "$C_RESET" "$topic"; failed=$((failed + 1)) ;;
            verify_failed) printf '  %s✗%s  %-22s verification failed\n' "$C_RED" "$C_RESET" "$topic"; failed=$((failed + 1)) ;;
            *) printf '  %s?%s  %-22s %s\n' "$C_MAGENTA" "$C_RESET" "$topic" "$result"; failed=$((failed + 1)) ;;
        esac
    done

    printf '\n'
    printf '  %sSUCCESS:%s %d  %sWARN:%s %d  %sFAILED:%s %d  %sSKIPPED:%s %d  %sDRY:%s %d\n' \
        "$C_GREEN" "$C_RESET" "$success" \
        "$C_YELLOW" "$C_RESET" "$warnings" \
        "$C_RED" "$C_RESET" "$failed" \
        "$C_DIM" "$C_RESET" "$skipped" \
        "$C_CYAN" "$C_RESET" "$dry"
    printf '\n'
    printf '  Logs: %s\n' "$LOG_ROOT/$run_id/"
    printf '  Output: %s\n' "$OUTPUT_ROOT/"
    printf '\n'

    if (( failed > 0 )); then
        log_err "runner" "Run had $failed failures. Re-run with --only <topic> to retry."
        return 1
    fi
    return 0
}

# --- Main --------------------------------------------------------------------

main() {
    # List mode short-circuit.
    if (( LIST_ONLY == 1 )); then
        list_topics
        exit 0
    fi

    # Preflight checks — always run (also in dry-run mode so users get early warning).
    # Skip only in verify-only mode since there we don't need claude.
    if (( VERIFY_ONLY == 0 )); then
        if ! preflight_check; then
            exit 1
        fi
        echo
    fi

    require_file "$MIGRATION_ROOT/SOURCE-INDEX.md"
    require_file "$MIGRATION_ROOT/README.md"

    # Select topics to run.
    local -a topics
    mapfile -t topics < <(select_topics)

    if (( ${#topics[@]} == 0 )); then
        log_err "cli" "No topics selected"
        exit 1
    fi

    # Create a run ID for this invocation.
    local run_id
    run_id="run-$(date +%Y%m%d-%H%M%S)"
    ensure_dir "$LOG_ROOT/$run_id"

    # Write a manifest.
    {
        echo "run_id: $run_id"
        echo "started: $(date -Iseconds)"
        echo "model: $ROKO_MIGRATION_MODEL"
        echo "parallel: $PARALLEL"
        echo "dry_run: $DRY_RUN"
        echo "force: $FORCE"
        echo "verify_only: $VERIFY_ONLY"
        echo "topics:"
        printf '  - %s\n' "${topics[@]}"
    } > "$LOG_ROOT/$run_id/manifest.yaml"

    log_info "runner" "Run manifest: $LOG_ROOT/$run_id/manifest.yaml"

    # Run.
    local start_ts
    start_ts=$(date +%s)
    run_topics_parallel "$run_id" "${topics[@]}"
    local end_ts
    end_ts=$(date +%s)
    local duration=$((end_ts - start_ts))

    # Summary.
    print_summary "$run_id" "${topics[@]}"
    local summary_rc=$?

    log_info "runner" "Total duration: $(fmt_duration "$duration")"

    # Generate master INDEX.md from topic INDEX.md files (best-effort).
    if (( DRY_RUN == 0 )); then
        generate_master_index "$run_id"
    fi

    exit $summary_rc
}

# --- Master index generation -------------------------------------------------

generate_master_index() {
    local run_id="$1"
    local master="$OUTPUT_ROOT/INDEX.md"
    log_info "runner" "Generating master index at $master"

    ensure_dir "$OUTPUT_ROOT"

    {
        echo "# Roko PRD — Master Index"
        echo
        echo "> Generated by the PRD migration runner on $(date -Iseconds)."
        echo "> Run ID: \`$run_id\`"
        echo
        echo "This is the top-level index for the Roko PRD documentation. Each topic below"
        echo "has its own folder with multiple sub-docs. For naming conventions, reframe rules,"
        echo "and the authoritative source of truth, see \`tmp/prd-migration/README.md\` and"
        echo "\`/Users/will/dev/nunchi/roko/refactoring-prd/\`."
        echo
        echo "## Topics"
        echo
        local topic
        for topic in "${ALL_TOPICS[@]}"; do
            local dir="$OUTPUT_ROOT/$topic"
            local index="$dir/INDEX.md"
            if [[ -s "$index" ]]; then
                # Try to extract the first heading as the title.
                local title
                title=$(head -20 "$index" | grep -m1 '^# ' | sed 's/^# //' || echo "$topic")
                echo "- [\`$topic/\`]($topic/INDEX.md) — $title"
            else
                echo "- \`$topic/\` — (not generated)"
            fi
        done
        echo
        echo "## How this was built"
        echo
        echo "- Each topic folder contains multiple sub-docs broken down for easy navigation and maintenance."
        echo "- Content is generated from three source layers:"
        echo "  1. \`/Users/will/dev/nunchi/roko/refactoring-prd/\` — canonical new-architecture spec"
        echo "  2. \`/Users/will/dev/nunchi/roko/bardo-backup/prd/\` and \`bardo-backup/tmp/\` — legacy research and PRDs"
        echo "  3. \`/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/\` — active work items"
        echo "- Every academic citation from the sources is preserved."
        echo "- See \`tmp/prd-migration/SOURCE-INDEX.md\` for the full source mapping per topic."
    } > "$master"

    log_ok "runner" "Master index written"
}

main "$@"
