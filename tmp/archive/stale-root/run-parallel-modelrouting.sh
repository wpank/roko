#!/usr/bin/env bash
# run-parallel-modelrouting.sh — Run modelrouting tasks in parallel using git worktrees
#
# Creates isolated worktrees so multiple agents can work simultaneously without
# interfering with each other's builds or file changes.
#
# Each stream gets:
#   - Its own git worktree (full repo copy)
#   - Its own branch (for safe commits)
#   - Its own cargo target dir (no build conflicts)
#   - A copy of the shared state file (knows what's already done)
#   - Assigned docs to work on
#
# After completion, merge branches back with:
#   git merge mr-stream-1 mr-stream-2 mr-stream-3 mr-stream-4
#
# Usage:
#   bash tmp/run-parallel-modelrouting.sh           # launch all streams
#   bash tmp/run-parallel-modelrouting.sh --status   # check progress across all streams
#   bash tmp/run-parallel-modelrouting.sh --stop     # stop all streams
#
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CURRENT_BRANCH=$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD)
STATE_FILE="$REPO_ROOT/.roko/modelrouting-state/state.json"
PARALLEL_LOG_DIR="$REPO_ROOT/tmp/logs/modelrouting-parallel"
PIDS_FILE="$PARALLEL_LOG_DIR/pids"

# ─── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
BLUE='\033[0;34m'; MAGENTA='\033[0;35m'; CYAN='\033[0;36m'
BOLD='\033[1m'; NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $(date +%H:%M:%S) $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $(date +%H:%M:%S) $*"; }
log_err()   { echo -e "${RED}[ERR]${NC}   $(date +%H:%M:%S) $*"; }
log_phase() { echo -e "\n${BOLD}${MAGENTA}═══ $* ═══${NC}\n"; }

mkdir -p "$PARALLEL_LOG_DIR"

# ─── Stream definitions ──────────────────────────────────────────────────────
#
# Grouped by crate affinity to minimize merge conflicts:
#
# Stream 1: Doc 08 (finish), 09, 10, 12 — primarily roko-learn
# Stream 2: Doc 20, 21              — new backends (roko-agent, separate files)
# Stream 3: Doc 13, 14, 15          — integration + operational
# Stream 4: Doc 16, 17, 18          — hardening + cleanup
#
# Agents: mix Claude and Codex for diversity

# Stream 1 (Codex): learning + integration docs — roko-learn + roko-agent internals
STREAM_1_NAME="mr-stream-alpha"
STREAM_1_DOCS="08 09 10 12 13 14 15"
STREAM_1_AGENT="codex"
STREAM_1_MODEL="gpt-5.4"
STREAM_1_EXTRA="--reasoning high"

# Stream 2 (Codex): backends + hardening — new providers + operational
STREAM_2_NAME="mr-stream-beta"
STREAM_2_DOCS="20 21 16 17 18"
STREAM_2_AGENT="codex"
STREAM_2_MODEL="gpt-5.4"
STREAM_2_EXTRA="--reasoning high"

ALL_STREAMS="1 2"

# ─── Functions ───────────────────────────────────────────────────────────────

create_worktree() {
  local stream_name="$1" stream_num="$2"
  local wt_path="$REPO_ROOT/../roko-${stream_name}"
  local branch_name="${stream_name}"

  if [[ -d "$wt_path" ]]; then
    log_info "Worktree already exists: $wt_path"
  else
    log_info "Creating worktree: $wt_path (branch: $branch_name)"
    git -C "$REPO_ROOT" worktree add "$wt_path" -b "$branch_name" "$CURRENT_BRANCH" 2>&1 || {
      # Branch might already exist
      git -C "$REPO_ROOT" worktree add "$wt_path" "$branch_name" 2>&1 || {
        log_err "Failed to create worktree for $stream_name"
        return 1
      }
    }
  fi

  # Copy gitignored files that the script needs:
  # - tmp/run-modelrouting.sh (the runner script)
  # - tmp/implementation-plans/ (the task specs)
  # - .roko/modelrouting-state/ (knows what's already done)
  log_info "Syncing gitignored files to $stream_name..."
  rsync -a --delete "$REPO_ROOT/tmp/run-modelrouting.sh" "$wt_path/tmp/run-modelrouting.sh"
  rsync -a --delete "$REPO_ROOT/tmp/implementation-plans/" "$wt_path/tmp/implementation-plans/"
  local wt_state_dir="$wt_path/.roko/modelrouting-state"
  mkdir -p "$wt_state_dir"
  mkdir -p "$wt_path/tmp/logs/modelrouting"
  if [[ -f "$STATE_FILE" ]]; then
    cp "$STATE_FILE" "$wt_state_dir/state.json"
    log_info "Copied state file to $stream_name"
  fi

  log_ok "Worktree ready: $wt_path"
}

launch_stream() {
  local stream_num="$1"
  local name_var="STREAM_${stream_num}_NAME"
  local docs_var="STREAM_${stream_num}_DOCS"
  local agent_var="STREAM_${stream_num}_AGENT"
  local model_var="STREAM_${stream_num}_MODEL"
  local extra_var="STREAM_${stream_num}_EXTRA"

  local name="${!name_var}"
  local docs="${!docs_var}"
  local agent="${!agent_var}"
  local model="${!model_var}"
  local extra="${!extra_var}"

  local wt_path="$REPO_ROOT/../roko-${name}"
  local log_file="$PARALLEL_LOG_DIR/${name}.log"

  log_phase "Stream $stream_num: $name"
  log_info "Docs:  $docs"
  log_info "Agent: $agent (model: $model)"
  log_info "Path:  $wt_path"
  log_info "Log:   $log_file"

  # Build the command — run each doc sequentially within the stream
  # Use continuous mode so failures get retried
  # SKIP_GATE2=true skips the slow workspace test compile gate
  local cmd="cd '$wt_path' && export SKIP_GATE2=true"
  for doc in $docs; do
    cmd+=" && bash tmp/run-modelrouting.sh --doc $doc --agent $agent --model $model $extra --commit --continuous --retries 3"
  done

  # Launch in background with nohup to survive terminal disconnect
  nohup bash -c "$cmd" > "$log_file" 2>&1 &
  local pid=$!
  echo "$pid $name" >> "$PIDS_FILE"
  log_ok "Launched stream $stream_num (PID: $pid)"
}

check_status() {
  log_phase "Parallel Stream Status"

  for stream_num in $ALL_STREAMS; do
    local name_var="STREAM_${stream_num}_NAME"
    local docs_var="STREAM_${stream_num}_DOCS"
    local name="${!name_var}"
    local docs="${!docs_var}"
    local wt_path="$REPO_ROOT/../roko-${name}"
    local wt_state="$wt_path/.roko/modelrouting-state/state.json"

    echo -e "${BOLD}Stream $stream_num: $name${NC} (docs: $docs)"

    # Check if process is running
    local pid=""
    if [[ -f "$PIDS_FILE" ]]; then
      pid=$(grep "$name" "$PIDS_FILE" | tail -1 | awk '{print $1}')
    fi
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      echo -e "  Status: ${GREEN}running${NC} (PID: $pid)"
    else
      echo -e "  Status: ${YELLOW}not running${NC}"
    fi

    # Check task progress from worktree state
    if [[ -f "$wt_state" ]]; then
      local done_count total_count
      done_count=$(python3 -c "
import json
with open('$wt_state') as f: s = json.load(f)
print(sum(1 for v in s.values() if isinstance(v,dict) and v.get('status')=='done'))
" 2>/dev/null || echo "?")
      echo -e "  Done tasks: $done_count"
    fi

    # Show last progress entry
    local progress_file="$wt_path/tmp/logs/modelrouting/progress.log"
    if [[ -f "$progress_file" ]]; then
      local last_entry
      last_entry=$(tail -1 "$progress_file")
      echo -e "  Last: $last_entry"
    fi
    echo ""
  done

  # Also show main repo state
  echo -e "${BOLD}Main repo (original run):${NC}"
  if [[ -f "$STATE_FILE" ]]; then
    python3 -c "
import json
with open('$STATE_FILE') as f: s = json.load(f)
done = sum(1 for v in s.values() if isinstance(v,dict) and v.get('status')=='done')
total = sum(1 for v in s.values() if isinstance(v,dict) and 'status' in v and not v.get('status','').startswith('doc'))
print(f'  Done: {done} tasks')
" 2>/dev/null
  fi
}

stop_all() {
  log_phase "Stopping all streams"
  if [[ ! -f "$PIDS_FILE" ]]; then
    log_info "No PID file found"
    return
  fi

  while IFS= read -r line; do
    local pid name
    pid=$(echo "$line" | awk '{print $1}')
    name=$(echo "$line" | awk '{print $2}')
    if kill -0 "$pid" 2>/dev/null; then
      log_info "Stopping $name (PID: $pid)"
      kill -TERM "$pid" 2>/dev/null
      # Also kill child processes (the actual agent)
      pkill -TERM -P "$pid" 2>/dev/null || true
    else
      log_info "$name already stopped"
    fi
  done < "$PIDS_FILE"

  rm -f "$PIDS_FILE"
  log_ok "All streams stopped"
}

# ─── CLI ─────────────────────────────────────────────────────────────────────

case "${1:-run}" in
  --status|-s)
    check_status
    exit 0
    ;;
  --stop)
    stop_all
    exit 0
    ;;
  --merge)
    log_phase "Merge Guide"
    echo "After all streams complete, merge their branches into $CURRENT_BRANCH:"
    echo ""
    echo "  cd $REPO_ROOT"
    for stream_num in $ALL_STREAMS; do
      _nv="STREAM_${stream_num}_NAME"
      echo "  git merge ${!_nv} --no-edit"
    done
    echo ""
    echo "If there are conflicts, resolve them and commit."
    echo "Worktrees will be preserved for inspection."
    exit 0
    ;;
  run|"")
    ;;
  *)
    echo "Usage: bash tmp/run-parallel-modelrouting.sh [--status|--stop|--merge]"
    exit 1
    ;;
esac

# ─── Pre-flight ──────────────────────────────────────────────────────────────

log_phase "Parallel Model Routing Runner"
log_info "Base branch: $CURRENT_BRANCH"
log_info "Streams: 4"
log_info "State file: $STATE_FILE"
echo ""

# Verify tools
command -v claude &>/dev/null || { log_err "claude CLI not found"; exit 1; }
command -v codex &>/dev/null || { log_err "codex CLI not found"; exit 1; }

# Clean old PIDs
rm -f "$PIDS_FILE"

# ─── Create worktrees ───────────────────────────────────────────────────────

log_phase "Creating worktrees"
for stream_num in $ALL_STREAMS; do
  _nv="STREAM_${stream_num}_NAME"
  create_worktree "${!_nv}" "$stream_num"
done

# ─── Pre-build in each worktree (parallel) ───────────────────────────────────
# Kick off cargo check in each worktree to warm up the build cache.
# This avoids all 4 streams competing for a cold build simultaneously.

log_phase "Warming up build caches (parallel)"
_build_pids=()
for stream_num in $ALL_STREAMS; do
  _nv="STREAM_${stream_num}_NAME"
  _name="${!_nv}"
  _wt_path="$REPO_ROOT/../roko-${_name}"
  log_info "Building $_name..."
  (cd "$_wt_path" && cargo check --workspace >/dev/null 2>&1) &
  _build_pids+=($!)
done

# Wait for all builds
_build_failed=false
for pid in "${_build_pids[@]}"; do
  if ! wait "$pid"; then
    _build_failed=true
  fi
done
if [[ "$_build_failed" == "true" ]]; then
  log_err "Some worktrees failed to build — check manually"
else
  log_ok "All worktrees compiled"
fi

# ─── Launch streams ──────────────────────────────────────────────────────────

log_phase "Launching streams"
for stream_num in $ALL_STREAMS; do
  launch_stream "$stream_num"
done

echo ""
log_ok "All 4 streams launched!"
echo ""
echo -e "${BOLD}Monitor:${NC}"
echo "  bash tmp/run-parallel-modelrouting.sh --status"
echo ""
echo -e "${BOLD}Watch a stream:${NC}"
echo "  tail -f $PARALLEL_LOG_DIR/mr-stream-learn.log"
echo "  tail -f $PARALLEL_LOG_DIR/mr-stream-backends.log"
echo "  tail -f $PARALLEL_LOG_DIR/mr-stream-integration.log"
echo "  tail -f $PARALLEL_LOG_DIR/mr-stream-hardening.log"
echo ""
echo -e "${BOLD}Watch all progress:${NC}"
echo "  watch -n 30 'bash tmp/run-parallel-modelrouting.sh --status'"
echo ""
echo -e "${BOLD}Stop all:${NC}"
echo "  bash tmp/run-parallel-modelrouting.sh --stop"
echo ""
echo -e "${BOLD}After completion, merge:${NC}"
echo "  bash tmp/run-parallel-modelrouting.sh --merge"
