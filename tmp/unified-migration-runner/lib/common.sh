#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${MR_ROOT:=$ROKO_ROOT/tmp/unified-migration-runner}"
: "${LOG_ROOT:=$MR_ROOT/logs}"
: "${PROMPTS_DIR:=$MR_ROOT/prompts}"
: "${CONTEXT_DIR:=$MR_ROOT/context-pack}"
: "${WORKTREE_ROOT:=$ROKO_ROOT/.roko/worktrees}"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
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

log_info()   { printf '%s[INFO]%s  %s%-10s%s %s\n' "$C_BLUE"   "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_ok()     { printf '%s[OK]%s    %s%-10s%s %s\n' "$C_GREEN"  "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2"; }
log_warn()   { printf '%s[WARN]%s  %s%-10s%s %s\n' "$C_YELLOW" "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_err()    { printf '%s[ERR]%s   %s%-10s%s %s\n' "$C_RED"    "$C_RESET" "$C_DIM" "$1" "$C_RESET" "$2" >&2; }
log_header() { printf '\n%s=== %s ===%s\n\n' "$C_BOLD$C_MAGENTA" "$1" "$C_RESET"; }

# ---------------------------------------------------------------------------
# Batch registry — populated by manual ingest-and-audit prompt
# ---------------------------------------------------------------------------

# Canonical execution order. Each entry is "M###" where ### is a zero-padded ID.
# Batches are appended here as the manual prompt processes each docs/ folder.
ALL_BATCHES=(
  # Phase 0: Prep (01-PHASE-0-PREP.md)
  "M001" "M002" "M003" "M004"
  "M015" "M016" "M017" "M018" "M019" "M020"
  # Phase 1: Kernel (02-PHASE-1-KERNEL.md)
  "M005" "M006" "M007" "M008" "M009" "M010" "M011"
  "M012" "M013" "M014"
  "M021" "M022" "M023" "M024" "M025"
  "M026" "M027" "M028" "M029" "M030"
  "M031" "M032" "M033" "M034" "M035" "M036" "M037"
  # Phase 2: Engine (03-PHASE-2-ENGINE.md)
  "M038" "M039" "M040" "M041" "M042" "M043"
  "M044" "M045" "M046" "M047" "M048"
  "M049" "M050" "M051" "M052"
  "M053" "M054" "M055" "M056"
  "M057" "M058" "M059" "M060" "M061"
  "M062" "M063" "M064"
  # Phase 3: Economy (04-PHASE-3-ECONOMY.md)
  "M065" "M066" "M067" "M068"
  "M069" "M070" "M071"
  "M072" "M073" "M074" "M075"
  "M076" "M077" "M078" "M079" "M080" "M081"
  "M082" "M083" "M084" "M085" "M086" "M087"
  "M088" "M089" "M090" "M091"
  "M092" "M093" "M094" "M095"
  # Phase 4: Memory and Knowledge (11-memory depth docs)
  "M096" "M097"
  "M098" "M099"
  "M100" "M101" "M102"
  "M103" "M104"
  "M105" "M106"
  "M107"
  "M108" "M109"
  "M110" "M111"
  "M112"
  # Phase 5: Conductor and Affect (07-agent-runtime depth docs)
  "M116" "M117" "M118"
  "M119" "M120"
  "M121" "M122"
  "M123" "M124" "M125"
  "M126" "M127"
  "M128" "M129"
  # Phase 6: Chain and Registries (18-registries depth docs)
  "M131" "M132" "M133" "M134"
  "M135" "M136" "M137"
  "M138" "M139"
  "M140" "M141"
)

# ---------------------------------------------------------------------------
# Status helpers
# ---------------------------------------------------------------------------

success_status() {
  case "${1:-}" in
    success|success_noop|skipped) return 0 ;;
    *) return 1 ;;
  esac
}

terminal_failure_status() {
  case "${1:-}" in
    spawn_failed|verify_failed|commit_failed|timeout|blocked|merge_failed) return 0 ;;
    *) return 1 ;;
  esac
}

fmt_duration() {
  local s="${1:-0}"
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

ensure_dir() {
  mkdir -p "$1"
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    log_err "bootstrap" "Missing file: $path"
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# Run / log / state path helpers
# ---------------------------------------------------------------------------

latest_run_id() {
  [[ -d "$LOG_ROOT" ]] || return 1
  find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'run-*' -exec test -f {}/manifest.env \; -print \
    | sort \
    | tail -1 \
    | sed 's|.*/||'
}

run_manifest_file()    { echo "$LOG_ROOT/$1/manifest.env"; }
run_result_file()      { echo "$LOG_ROOT/$1/$2.result"; }
run_log_file()         { echo "$LOG_ROOT/$1/$2.log"; }
run_prompts_dir()      { echo "$LOG_ROOT/$1/prompts"; }
run_prompt_snapshot()  { echo "$(run_prompts_dir "$1")/$2.prompt.md"; }
run_last_message_file(){ echo "$LOG_ROOT/$1/$2.last.txt"; }
run_failure_file()     { echo "$LOG_ROOT/$1/$2.failure.txt"; }
run_status_file()      { echo "$LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()      { echo "$LOG_ROOT/$1/backups"; }
batch_prompt_file() {
  # Prompt files are named M###-short-name.prompt.md
  # Match by batch ID prefix
  local match
  match=$(find "$PROMPTS_DIR" -maxdepth 1 -name "${1}-*.prompt.md" -o -name "${1}.prompt.md" 2>/dev/null | head -1)
  if [[ -n "$match" ]]; then
    echo "$match"
  else
    echo "$PROMPTS_DIR/$1.prompt.md"
  fi
}
tmp_target_root()      { echo "${TMPDIR:-/tmp}/roko-migration-targets"; }
batch_target_dir()     { echo "$(tmp_target_root)/$1/$2/$3-attempt-$4"; }

link_latest_run() {
  local run_id="$1"
  [[ "$run_id" == dry-run-* ]] && return 0
  ln -sfn "$LOG_ROOT/$run_id" "$LOG_ROOT/latest"
}

current_batch_value() {
  local run_id="$1"
  local key="$2"
  local file
  file="$(run_current_batch_file "$run_id")"
  [[ -f "$file" ]] || return 1
  awk -F= -v key="$key" '$1 == key { gsub(/\047/, "", $2); print $2 }' "$file"
}

current_batch_name()    { current_batch_value "$1" "BATCH"; }
current_batch_attempt() { current_batch_value "$1" "ATTEMPT"; }

worktree_dirty() {
  local worktree="$1"
  git -C "$worktree" status --porcelain=v1 -uall \
    | grep -Ev '^[ MADRCU?!]{2} (\.cargo-target/|target/)' \
    | grep -q .
}

record_status() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  local status="$4"
  local note="${5:-}"
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "$(date -Iseconds)" \
    "$batch" \
    "$attempt" \
    "$status" \
    "$note" >> "$(run_status_file "$run_id")"
}

set_current_batch() {
  local run_id="$1"
  local batch="$2"
  local attempt="$3"
  cat > "$(run_current_batch_file "$run_id")" <<EOF
BATCH='$batch'
ATTEMPT='$attempt'
UPDATED_AT='$(date -Iseconds)'
EOF
}

clear_current_batch() {
  local run_id="$1"
  rm -f "$(run_current_batch_file "$run_id")"
}

# ---------------------------------------------------------------------------
# Batch metadata — populated incrementally by ingest-and-audit prompt
# ---------------------------------------------------------------------------

batch_title() {
  case "$1" in
    # Phase 0: Prep
    M001) echo "Baseline verification snapshot" ;;
    M002) echo "Create module stubs (signal.rs, cell.rs) in roko-core" ;;
    M003) echo "Wire ExtensionChain into orchestrate.rs" ;;
    M004) echo "Wire KnowledgeAdmissionController into dispatch" ;;
    M015) echo "Wire ContextualBanditPolicy into CascadeRouter" ;;
    M016) echo "Audit ConnectorRegistry + FeedRegistry" ;;
    M017) echo "Fix token accounting in gateway.rs" ;;
    M018) echo "Parallelize batch requests in gateway.rs" ;;
    M019) echo "Fix routing context in gateway.rs" ;;
    M020) echo "Bus module stub + TopicFilter alignment" ;;
    # Phase 1: Kernel
    M005) echo "Type alias: Engram -> Signal in roko-core" ;;
    M006) echo "Type alias: Substrate -> Store in roko-core" ;;
    M007) echo "Type alias: Scorer -> ScoreProtocol in roko-core" ;;
    M008) echo "Type alias: Gate -> VerifyProtocol in roko-core + roko-gate" ;;
    M009) echo "Type alias: Router -> RouteProtocol in roko-core" ;;
    M010) echo "Type alias: Composer -> ComposeProtocol in roko-core" ;;
    M011) echo "Type alias: Policy -> ReactProtocol in roko-core" ;;
    M012) echo "Define Cell trait skeleton in roko-core" ;;
    M013) echo "Add Verdict.reward field + verify_pre method" ;;
    M014) echo "Define TypeSchema enum in roko-core" ;;
    M021) echo "Pulse struct: align fields with unified spec" ;;
    M022) echo "Bus trait + BroadcastBus: verify spec alignment" ;;
    M023) echo "Topic taxonomy constants module" ;;
    M024) echo "Wire Bus lifecycle Pulses into execution path" ;;
    M025) echo "React protocol: Pulse-based breaking change" ;;
    M026) echo "CalibrationReact Cell implementation" ;;
    M027) echo "Wire prediction Pulses for Score/Route/Compose" ;;
    M028) echo "Demurrage: add balance fields to Signal" ;;
    M029) echo "Demurrage: reinforcement kinds with novelty" ;;
    M030) echo "Demurrage: wire into Store operations + tier multipliers" ;;
    M031) echo "Heuristic Kind: define Kind::Heuristic + payload" ;;
    M032) echo "Heuristic: wire calibration from Verify verdicts" ;;
    M033) echo "EFE routing: implement + replace LinUCB in CascadeRoute" ;;
    M034) echo "Dream cycle: wire automatic trigger" ;;
    M035) echo "Observe protocol + 10 builtin Lenses" ;;
    M036) echo "Trigger protocol + Cron/Bus/FileWatch impls" ;;
    M037) echo "Connect protocol + refactor MCP connector" ;;
    # Phase 2: Engine
    M038) echo "Graph TOML schema definition" ;;
    M039) echo "Graph loader with TypeSchema validation" ;;
    M040) echo "Graph executor with Flow lifecycle" ;;
    M041) echo "Graph failure strategies (Retry/Skip/Fallback)" ;;
    M042) echo "Flow snapshot/resume" ;;
    M043) echo "Hot Graph: resident execution variant" ;;
    M044) echo "roko plan migrate CLI command" ;;
    M045) echo "Wire Graph executor into roko plan run" ;;
    M046) echo "Type-state Agent struct (Provisioning/Active/Dreaming/Terminal)" ;;
    M047) echo "Vitality model + behavioral phases" ;;
    M048) echo "Multi-slot concurrent Agent execution" ;;
    M049) echo "CognitiveWorkspace with VCG auction" ;;
    M050) echo "Section effect tracking via Beta posteriors" ;;
    M051) echo "StateHub projection types" ;;
    M052) echo "Wire StateHub into TUI/HTTP/WebSocket" ;;
    M053) echo "Five named surface protocol contracts" ;;
    M054) echo "Workbench tab in TUI" ;;
    M055) echo "Agent Inbox in TUI" ;;
    M056) echo "Autonomy Slider in TUI" ;;
    M057) echo "Rack: Graph + Macros + Slots" ;;
    M058) echo "SPI Tier 1: prompt loader" ;;
    M059) echo "SPI Tier 2: config profile deep merge" ;;
    M060) echo "SPI Tier 3: declarative tool loader" ;;
    M061) echo "SPI Tier 4: WASM Cell runtime" ;;
    M062) echo "Cell manifest + local registry" ;;
    M063) echo "roko marketplace publish/install/fork CLI" ;;
    M064) echo "Marketplace HTTP routes" ;;
    # Phase 3: Economy
    M065) echo "Extension: formalize 8 layers" ;;
    M066) echo "CaMeL IFC: define CamelTag types" ;;
    M067) echo "CaMeL IFC: tag propagation rules" ;;
    M068) echo "CaMeL Monitor: Verify Cell" ;;
    M069) echo "5-head corrigibility Verify chain" ;;
    M070) echo "RecursiveSafetyMonitor" ;;
    M071) echo "Wire corrigibility into Graph executor" ;;
    M072) echo "L4 structural change proposals" ;;
    M073) echo "L4 approval workflow via Agent Inbox" ;;
    M074) echo "Wire L4 into dream cycle" ;;
    M075) echo "Variance Inequality enforcement" ;;
    M076) echo "Finalize Solidity contracts [BLOCKED:depth]" ;;
    M077) echo "Deploy to Nunchi testnet [BLOCKED:depth]" ;;
    M078) echo "Rust clients for all registries [BLOCKED:depth]" ;;
    M079) echo "Wire passport registration into Agent startup [BLOCKED:depth]" ;;
    M080) echo "Wire knowledge publication from Memory [BLOCKED:depth]" ;;
    M081) echo "Event indexer [BLOCKED:depth]" ;;
    M082) echo "Arena types [BLOCKED:depth]" ;;
    M083) echo "7-step arena flywheel [BLOCKED:depth]" ;;
    M084) echo "Eval protocol [BLOCKED:depth]" ;;
    M085) echo "Bounty system [BLOCKED:depth]" ;;
    M086) echo "Arena + Bounty HTTP routes [BLOCKED:depth]" ;;
    M087) echo "Cross-arena transfer detection [BLOCKED:depth]" ;;
    M088) echo "Brain export format" ;;
    M089) echo "Brain export with filters" ;;
    M090) echo "Brain import with decay factor" ;;
    M091) echo "Merkle-CRDT sync" ;;
    M092) echo "Knowledge Signal broadcast via relay" ;;
    M093) echo "On-chain knowledge discovery [BLOCKED:depth]" ;;
    M094) echo "WASM compilation target [BLOCKED:depth]" ;;
    M095) echo "Agent execution tiers [BLOCKED:depth]" ;;
    # Phase 4: Memory and Knowledge
    M096) echo "Knowledge Kind mapping to Signal Kind system" ;;
    M097) echo "Neuro Store as Signal Store adapter" ;;
    M098) echo "HDC operations as Cell implementations" ;;
    M099) echo "Three-tier HDC search pipeline" ;;
    M100) echo "Distillation as Pipeline Graph (D1/D2/D3 stages)" ;;
    M101) echo "Immune Verify Pipeline and memetic fitness Score" ;;
    M102) echo "Federation Spaces and confidence Functor" ;;
    M103) echo "Dream Loop Graph structure (NREM/REM/Integration)" ;;
    M104) echo "Dream Trigger Cell scheduling" ;;
    M105) echo "NREM replay Cells (episode selection/sequencing/extraction)" ;;
    M106) echo "REM counterfactual Cells (generation/testing/gap-finding)" ;;
    M107) echo "Hypnagogia Pipeline (anti-correlation/novelty/compose/verify)" ;;
    M108) echo "Staging Store partition and SHY renormalization" ;;
    M109) echo "Threat simulation Verify Cells (FMEA/FTA/nightmare)" ;;
    M110) echo "Pheromone Pulse types on Bus" ;;
    M111) echo "Stigmergic Route Cell" ;;
    M112) echo "Calibration receipts and predict-publish-correct for knowledge" ;;
    # Phase 5: Conductor and Affect
    M116) echo "Refactor conductor watchers as Verify Cells" ;;
    M117) echo "Conductor Pipeline Graph and Route Cell" ;;
    M118) echo "Circuit breaker state-machine Cell with AIMD" ;;
    M119) echo "Diagnosis Route Cell and error categories" ;;
    M120) echo "Stuck detection Lens Cells and aggregation" ;;
    M121) echo "Self-model accuracy Lens and Yerkes-Dodson pressure" ;;
    M122) echo "Threshold adaptation with predict-publish-correct" ;;
    M123) echo "PAD as Signal metadata and PadContext type" ;;
    M124) echo "Appraisal Pipeline and ALMA temporal model" ;;
    M125) echo "Behavioral state Score Cell with hysteresis" ;;
    M126) echo "Affect Functor for Compose enrichment" ;;
    M127) echo "Somatic Store with dual k-d tree and contrarian Functor" ;;
    M128) echo "Contagion accumulator and attenuation Functor" ;;
    M129) echo "Wire affect-modulated routing into orchestrate.rs" ;;
    # Phase 6: Chain and Registries
    M131) echo "ChainConnector Cell (Connect protocol wrapper) [BLOCKED:chain]" ;;
    M132) echo "Registry Store Cells (Identity, Reputation, Validation) [BLOCKED:chain]" ;;
    M133) echo "HDC Precompile Cell (on-chain HDC operations) [BLOCKED:chain]" ;;
    M134) echo "Verifiable HDC Verify Cells (ZK/Optimistic/TEE/Binius) [BLOCKED:chain]" ;;
    M135) echo "Job marketplace Graph types (posting, matching, hiring) [BLOCKED:chain]" ;;
    M136) echo "Escrow, Settlement, and Dispute Resolution Cells [BLOCKED:chain]" ;;
    M137) echo "Reputation Score Cell with EMA + TraceRank Pipeline [BLOCKED:chain]" ;;
    M138) echo "ChainWitnessFeed Cell (Connect+Trigger+Store) [BLOCKED:chain]" ;;
    M139) echo "Triage Pipeline (4-stage Score/Observe/Compose) [BLOCKED:chain]" ;;
    M140) echo "Payment Connect Cells (x402 + State Channels) [BLOCKED:chain]" ;;
    M141) echo "ISFR Score Cell + ClearingHouse Pipeline [BLOCKED:chain]" ;;
    *) echo "Unknown batch: $1"; return 1 ;;
  esac
}

batch_deps() {
  case "$1" in
    # Phase 0
    M002) echo "M001" ;;
    M003) echo "M002" ;;
    M004) echo "M002" ;;
    M015) echo "M001" ;;
    M016) echo "M001" ;;
    M017) echo "M001" ;;
    M018) echo "M017" ;;
    M019) echo "M017" ;;
    M020) echo "M002" ;;
    # Phase 1: type aliases
    M005) echo "M002" ;;
    M006) echo "M005" ;;
    M007) echo "M005" ;;
    M008) echo "M005" ;;
    M009) echo "M005" ;;
    M010) echo "M005" ;;
    M011) echo "M005" ;;
    M012) echo "M002" ;;
    M013) echo "M008" ;;
    M014) echo "M012" ;;
    # Phase 1: Pulse/Bus
    M021) echo "M005" ;;
    M022) echo "M020 M021" ;;
    M023) echo "M022" ;;
    M024) echo "M023" ;;
    M025) echo "M011 M021" ;;
    # Phase 1: predict-publish-correct
    M026) echo "M024 M025" ;;
    M027) echo "M026" ;;
    # Phase 1: demurrage
    M028) echo "M005" ;;
    M029) echo "M028" ;;
    M030) echo "M029" ;;
    # Phase 1: heuristic
    M031) echo "M005" ;;
    M032) echo "M031 M026" ;;
    # Phase 1: EFE + dream + new protocols
    M033) echo "M009 M015" ;;
    M034) echo "M024" ;;
    M035) echo "M012 M024" ;;
    M036) echo "M012 M024" ;;
    M037) echo "M012" ;;
    # Phase 2: depends on Phase 1 completing
    M038) echo "M014" ;;
    M039) echo "M038" ;;
    M040) echo "M039" ;;
    M041) echo "M040" ;;
    M042) echo "M040" ;;
    M043) echo "M040" ;;
    M044) echo "M040" ;;
    M045) echo "M044" ;;
    M046) echo "M037" ;;
    M047) echo "M046" ;;
    M048) echo "M046" ;;
    M049) echo "M010 M030" ;;
    M050) echo "M049" ;;
    M051) echo "M035" ;;
    M052) echo "M051" ;;
    M053) echo "M051" ;;
    M054) echo "M052" ;;
    M055) echo "M052" ;;
    M056) echo "M052" ;;
    M057) echo "M038" ;;
    M058) echo "M010" ;;
    M059) echo "M058" ;;
    M060) echo "M058" ;;
    M061) echo "M060" ;;
    M062) echo "M012" ;;
    M063) echo "M062" ;;
    M064) echo "M063" ;;
    # Phase 3: depends on Phase 2 completing
    M065) echo "M046" ;;
    M066) echo "M065" ;;
    M067) echo "M066" ;;
    M068) echo "M067 M008" ;;
    M069) echo "M068" ;;
    M070) echo "M069" ;;
    M071) echo "M070 M040" ;;
    M072) echo "M040" ;;
    M073) echo "M072 M055" ;;
    M074) echo "M072 M034" ;;
    M075) echo "M072" ;;
    M076) echo "" ;;
    M077) echo "M076" ;;
    M078) echo "M076" ;;
    M079) echo "M078 M046" ;;
    M080) echo "M078 M030" ;;
    M081) echo "M078" ;;
    M082) echo "" ;;
    M083) echo "M082" ;;
    M084) echo "M082" ;;
    M085) echo "M082" ;;
    M086) echo "M083 M085" ;;
    M087) echo "M083" ;;
    M088) echo "M030" ;;
    M089) echo "M088" ;;
    M090) echo "M088" ;;
    M091) echo "M088" ;;
    M092) echo "M024 M030" ;;
    M093) echo "M078" ;;
    M094) echo "M061" ;;
    M095) echo "M046" ;;
    # Phase 4: Memory and Knowledge
    M096) echo "M005 M031" ;;
    M097) echo "M096 M006" ;;
    M098) echo "M012" ;;
    M099) echo "M098 M097" ;;
    M100) echo "M097" ;;
    M101) echo "M097" ;;
    M102) echo "M097" ;;
    M103) echo "M034" ;;
    M104) echo "M103 M036" ;;
    M105) echo "M103" ;;
    M106) echo "M105" ;;
    M107) echo "M103" ;;
    M108) echo "M097 M103" ;;
    M109) echo "M103 M101" ;;
    M110) echo "M005 M021" ;;
    M111) echo "M110" ;;
    M112) echo "M100 M026" ;;
    # Phase 5: Conductor and Affect
    M116) echo "M005 M012" ;;
    M117) echo "M116" ;;
    M118) echo "M001" ;;
    M119) echo "M001" ;;
    M120) echo "M001" ;;
    M121) echo "M005 M012" ;;
    M122) echo "M121 M116" ;;
    M123) echo "M005" ;;
    M124) echo "M123" ;;
    M125) echo "M123" ;;
    M126) echo "M124 M125" ;;
    M127) echo "M123" ;;
    M128) echo "M123 M127" ;;
    M129) echo "M125 M126 M127" ;;
    # Phase 6: Chain and Registries
    M131) echo "M012 M037 M076" ;;
    M132) echo "M131 M006" ;;
    M133) echo "M131 M012" ;;
    M134) echo "M133 M008" ;;
    M135) echo "M012 M040 M131" ;;
    M136) echo "M135 M131" ;;
    M137) echo "M132 M012 M007" ;;
    M138) echo "M131 M036 M037 M012" ;;
    M139) echo "M138 M012" ;;
    M140) echo "M131 M037 M012" ;;
    M141) echo "M140 M132" ;;
    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    M001|M002|M003|M004|M015|M016|M017|M018|M019|M020) echo "phase0" ;;
    M005|M006|M007|M008|M009|M010|M011|M012|M013|M014) echo "phase1" ;;
    M021|M022|M023|M024|M025|M026|M027|M028|M029|M030) echo "phase1" ;;
    M031|M032|M033|M034|M035|M036|M037) echo "phase1" ;;
    M038|M039|M040|M041|M042|M043|M044|M045) echo "phase2" ;;
    M046|M047|M048|M049|M050|M051|M052) echo "phase2" ;;
    M053|M054|M055|M056|M057|M058|M059|M060|M061) echo "phase2" ;;
    M062|M063|M064) echo "phase2" ;;
    M065|M066|M067|M068|M069|M070|M071) echo "phase3" ;;
    M072|M073|M074|M075|M076|M077|M078|M079|M080|M081) echo "phase3" ;;
    M082|M083|M084|M085|M086|M087|M088|M089|M090|M091) echo "phase3" ;;
    M092|M093|M094|M095) echo "phase3" ;;
    M096|M097|M098|M099|M100|M101|M102) echo "phase2" ;;
    M103|M104|M105|M106|M107|M108|M109) echo "phase2" ;;
    M110|M111|M112) echo "phase2" ;;
    M116|M117|M118|M119|M120|M121|M122) echo "phase2" ;;
    M123|M124|M125|M126|M127|M128|M129) echo "phase2" ;;
    M131|M132|M133|M134|M135|M136|M137|M138|M139|M140|M141) echo "phase3" ;;
    *) echo "misc" ;;
  esac
}

batch_verify_commands() {
  case "$1" in
    M001)
      cat <<'EOF'
test -f tmp/unified-migration-runner/baseline.json
cargo check --workspace
EOF
      ;;
    M002)
      cat <<'EOF'
test -f crates/roko-core/src/signal.rs
test -f crates/roko-core/src/cell.rs
cargo check -p roko-core
EOF
      ;;
    M003)
      cat <<'EOF'
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    M004)
      cat <<'EOF'
cargo check -p roko-neuro -p roko-cli
cargo clippy -p roko-neuro -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    M005)
      cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
grep -q 'pub use.*Signal' crates/roko-core/src/signal.rs
EOF
      ;;
    M006)
      cat <<'EOF'
cargo check -p roko-core -p roko-fs
cargo clippy -p roko-core -p roko-fs --no-deps -- -D warnings
EOF
      ;;
    M007|M009|M010|M011)
      cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
EOF
      ;;
    M008)
      cat <<'EOF'
cargo check -p roko-core -p roko-gate
cargo clippy -p roko-core -p roko-gate --no-deps -- -D warnings
EOF
      ;;
    M012)
      cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
EOF
      ;;
    M013)
      cat <<'EOF'
cargo check -p roko-core -p roko-gate -p roko-cli
cargo clippy -p roko-core -p roko-gate --no-deps -- -D warnings
EOF
      ;;
    M014)
      cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
EOF
      ;;
    # Phase 0: remaining prep
    M015)
      cat <<'EOF'
cargo check -p roko-learn -p roko-cli
cargo clippy -p roko-learn --no-deps -- -D warnings
EOF
      ;;
    M016)
      cat <<'EOF'
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
EOF
      ;;
    M017|M018|M019)
      cat <<'EOF'
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    M020)
      cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
EOF
      ;;
    # Phase 1: Pulse/Bus
    M021|M022|M023)
      cat <<'EOF'
cargo check -p roko-core -p roko-runtime
cargo clippy -p roko-core -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
EOF
      ;;
    M024)
      cat <<'EOF'
cargo check -p roko-cli -p roko-orchestrator
cargo clippy -p roko-cli -p roko-orchestrator --no-deps -- -D warnings
EOF
      ;;
    M025)
      cat <<'EOF'
cargo check -p roko-core -p roko-daimon -p roko-conductor
cargo clippy -p roko-core --no-deps -- -D warnings
EOF
      ;;
    # Phase 1: calibration
    M026|M027)
      cat <<'EOF'
cargo check -p roko-learn -p roko-core
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn --lib --no-run
EOF
      ;;
    # Phase 1: demurrage
    M028|M029|M030)
      cat <<'EOF'
cargo check -p roko-core -p roko-neuro -p roko-fs
cargo clippy -p roko-core -p roko-neuro --no-deps -- -D warnings
EOF
      ;;
    # Phase 1: heuristic
    M031|M032)
      cat <<'EOF'
cargo check -p roko-core -p roko-learn -p roko-compose
cargo clippy -p roko-core -p roko-learn --no-deps -- -D warnings
EOF
      ;;
    # Phase 1: EFE + dream
    M033)
      cat <<'EOF'
cargo check -p roko-learn -p roko-conductor
cargo clippy -p roko-learn --no-deps -- -D warnings
EOF
      ;;
    M034)
      cat <<'EOF'
cargo check -p roko-dreams -p roko-cli
cargo clippy -p roko-dreams --no-deps -- -D warnings
EOF
      ;;
    # Phase 1: new protocols
    M035)
      cat <<'EOF'
cargo check -p roko-core -p roko-conductor
cargo clippy -p roko-core -p roko-conductor --no-deps -- -D warnings
EOF
      ;;
    M036)
      cat <<'EOF'
cargo check -p roko-core -p roko-runtime
cargo clippy -p roko-core -p roko-runtime --no-deps -- -D warnings
EOF
      ;;
    M037)
      cat <<'EOF'
cargo check -p roko-core -p roko-agent
cargo clippy -p roko-core -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    # Phase 2+3: default to workspace check for wider-impact batches
    M038|M039|M040|M041|M042|M043|M044|M045)
      cat <<'EOF'
cargo check -p roko-orchestrator -p roko-cli
cargo clippy -p roko-orchestrator --no-deps -- -D warnings
EOF
      ;;
    M046|M047|M048)
      cat <<'EOF'
cargo check -p roko-agent -p roko-core
cargo clippy -p roko-agent --no-deps -- -D warnings
EOF
      ;;
    M049|M050)
      cat <<'EOF'
cargo check -p roko-compose -p roko-core
cargo clippy -p roko-compose --no-deps -- -D warnings
EOF
      ;;
    M051|M052|M053|M054|M055|M056)
      cat <<'EOF'
cargo check -p roko-cli -p roko-serve -p roko-conductor
cargo clippy -p roko-cli -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    M057|M058|M059|M060|M061)
      cat <<'EOF'
cargo check -p roko-orchestrator -p roko-compose -p roko-core
cargo clippy -p roko-orchestrator -p roko-compose --no-deps -- -D warnings
EOF
      ;;
    M062|M063|M064)
      cat <<'EOF'
cargo check -p roko-core -p roko-cli -p roko-serve
cargo clippy -p roko-core -p roko-cli -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    M065|M066|M067|M068)
      cat <<'EOF'
cargo check -p roko-agent -p roko-core
cargo clippy -p roko-agent -p roko-core --no-deps -- -D warnings
EOF
      ;;
    M069|M070|M071)
      cat <<'EOF'
cargo check -p roko-gate -p roko-orchestrator
cargo clippy -p roko-gate -p roko-orchestrator --no-deps -- -D warnings
EOF
      ;;
    M072|M073|M074|M075)
      cat <<'EOF'
cargo check -p roko-learn -p roko-dreams -p roko-serve
cargo clippy -p roko-learn -p roko-dreams --no-deps -- -D warnings
EOF
      ;;
    M076|M077|M078|M079|M080|M081)
      cat <<'EOF'
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
EOF
      ;;
    M082|M083|M084|M085|M086|M087)
      cat <<'EOF'
cargo check -p roko-learn -p roko-serve
cargo clippy -p roko-learn -p roko-serve --no-deps -- -D warnings
EOF
      ;;
    M088|M089|M090|M091)
      cat <<'EOF'
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
EOF
      ;;
    M092|M093)
      cat <<'EOF'
cargo check -p roko-runtime -p roko-neuro
cargo clippy -p roko-runtime --no-deps -- -D warnings
EOF
      ;;
    # Phase 4: Memory and Knowledge
    M096|M097|M100|M101|M102|M108|M112)
      cat <<'EOF'
cargo check -p roko-neuro -p roko-core
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro --lib --no-run
EOF
      ;;
    M098)
      cat <<'EOF'
cargo check -p roko-primitives
cargo clippy -p roko-primitives --no-deps -- -D warnings
cargo test -p roko-primitives --lib --no-run
EOF
      ;;
    M099)
      cat <<'EOF'
cargo check -p roko-neuro
cargo clippy -p roko-neuro --no-deps -- -D warnings
cargo test -p roko-neuro --lib --no-run
EOF
      ;;
    M103|M104|M105|M106|M107|M109)
      cat <<'EOF'
cargo check -p roko-dreams
cargo clippy -p roko-dreams --no-deps -- -D warnings
cargo test -p roko-dreams --lib --no-run
EOF
      ;;
    M110)
      cat <<'EOF'
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
EOF
      ;;
    M111)
      cat <<'EOF'
cargo check -p roko-learn -p roko-core
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn --lib --no-run
EOF
      ;;
    # Phase 5: Conductor and Affect
    M116|M117|M118|M119|M120|M121|M122)
      cat <<'EOF'
cargo check -p roko-conductor -p roko-core
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor --lib --no-run
EOF
      ;;
    M123)
      cat <<'EOF'
cargo check -p roko-core -p roko-daimon
cargo clippy -p roko-core -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
EOF
      ;;
    M124|M125|M127|M128)
      cat <<'EOF'
cargo check -p roko-daimon
cargo clippy -p roko-daimon --no-deps -- -D warnings
cargo test -p roko-daimon --lib --no-run
EOF
      ;;
    M126|M129)
      cat <<'EOF'
cargo check -p roko-daimon -p roko-cli
cargo clippy -p roko-daimon -p roko-cli --no-deps -- -D warnings
EOF
      ;;
    # Phase 6: Chain and Registries
    M131|M132|M133|M134|M135|M136|M137|M138|M139|M140|M141)
      cat <<'EOF'
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
EOF
      ;;
    *)
      cat <<'EOF'
cargo check --workspace
EOF
      ;;
  esac
}

batch_crates() {
  case "$1" in
    # Phase 0
    M001) echo "roko-core" ;;
    M002) echo "roko-core" ;;
    M003) echo "roko-cli" ;;
    M004) echo "roko-neuro roko-cli" ;;
    M015) echo "roko-learn roko-cli" ;;
    M016) echo "roko-runtime" ;;
    M017) echo "roko-serve" ;;
    M018) echo "roko-serve" ;;
    M019) echo "roko-serve" ;;
    M020) echo "roko-core" ;;
    # Phase 1: type aliases
    M005) echo "roko-core" ;;
    M006) echo "roko-core roko-fs" ;;
    M007) echo "roko-core" ;;
    M008) echo "roko-core roko-gate" ;;
    M009) echo "roko-core" ;;
    M010) echo "roko-core" ;;
    M011) echo "roko-core" ;;
    M012) echo "roko-core" ;;
    M013) echo "roko-core roko-gate roko-cli" ;;
    M014) echo "roko-core" ;;
    # Phase 1: Pulse/Bus
    M021) echo "roko-core roko-runtime" ;;
    M022) echo "roko-core roko-runtime" ;;
    M023) echo "roko-core roko-runtime" ;;
    M024) echo "roko-cli roko-orchestrator" ;;
    M025) echo "roko-core roko-daimon roko-conductor" ;;
    # Phase 1: calibration
    M026) echo "roko-learn roko-core" ;;
    M027) echo "roko-learn roko-core" ;;
    # Phase 1: demurrage
    M028) echo "roko-core roko-neuro roko-fs" ;;
    M029) echo "roko-core roko-neuro" ;;
    M030) echo "roko-core roko-neuro roko-fs" ;;
    # Phase 1: heuristic
    M031) echo "roko-core roko-learn roko-compose" ;;
    M032) echo "roko-core roko-learn" ;;
    # Phase 1: EFE + dream + protocols
    M033) echo "roko-learn roko-conductor" ;;
    M034) echo "roko-dreams roko-cli" ;;
    M035) echo "roko-core roko-conductor" ;;
    M036) echo "roko-core roko-runtime" ;;
    M037) echo "roko-core roko-agent" ;;
    # Phase 2: Graph engine
    M038) echo "roko-orchestrator roko-cli" ;;
    M039) echo "roko-orchestrator roko-cli" ;;
    M040) echo "roko-orchestrator roko-cli" ;;
    M041) echo "roko-orchestrator" ;;
    M042) echo "roko-orchestrator" ;;
    M043) echo "roko-orchestrator" ;;
    M044) echo "roko-orchestrator roko-cli" ;;
    M045) echo "roko-orchestrator roko-cli" ;;
    # Phase 2: Agent runtime
    M046) echo "roko-agent roko-core" ;;
    M047) echo "roko-agent" ;;
    M048) echo "roko-agent" ;;
    # Phase 2: CognitiveWorkspace
    M049) echo "roko-compose roko-core" ;;
    M050) echo "roko-compose roko-core" ;;
    # Phase 2: StateHub + Surfaces
    M051) echo "roko-cli roko-serve roko-conductor" ;;
    M052) echo "roko-cli roko-serve roko-conductor" ;;
    M053) echo "roko-cli roko-serve roko-conductor" ;;
    M054) echo "roko-cli roko-serve" ;;
    M055) echo "roko-cli roko-serve" ;;
    M056) echo "roko-cli roko-serve" ;;
    # Phase 2: Rack/SPI/Marketplace
    M057) echo "roko-orchestrator roko-compose roko-core" ;;
    M058) echo "roko-orchestrator roko-compose" ;;
    M059) echo "roko-orchestrator roko-compose" ;;
    M060) echo "roko-orchestrator roko-compose" ;;
    M061) echo "roko-orchestrator roko-compose" ;;
    M062) echo "roko-core roko-cli roko-serve" ;;
    M063) echo "roko-core roko-cli roko-serve" ;;
    M064) echo "roko-core roko-cli roko-serve" ;;
    # Phase 3: Extension/CaMeL/Corrigibility
    M065) echo "roko-agent roko-core" ;;
    M066) echo "roko-agent roko-core" ;;
    M067) echo "roko-agent roko-core" ;;
    M068) echo "roko-agent roko-core" ;;
    M069) echo "roko-gate roko-orchestrator" ;;
    M070) echo "roko-gate roko-orchestrator" ;;
    M071) echo "roko-gate roko-orchestrator" ;;
    # Phase 3: L4
    M072) echo "roko-learn roko-dreams roko-serve" ;;
    M073) echo "roko-learn roko-dreams roko-serve" ;;
    M074) echo "roko-learn roko-dreams" ;;
    M075) echo "roko-learn roko-dreams" ;;
    # Phase 3: On-chain
    M076|M077|M078|M079|M080|M081) echo "roko-chain" ;;
    # Phase 3: Arena
    M082|M083|M084|M085|M086|M087) echo "roko-learn roko-serve" ;;
    # Phase 3: Brain
    M088|M089|M090|M091) echo "roko-neuro" ;;
    # Phase 3: Knowledge + deploy
    M092|M093) echo "roko-runtime roko-neuro" ;;
    M094) echo "roko-orchestrator roko-compose" ;;
    M095) echo "roko-agent roko-core" ;;
    # Phase 5: Conductor and Affect
    M116) echo "roko-conductor roko-core" ;;
    M117) echo "roko-conductor" ;;
    M118) echo "roko-conductor" ;;
    M119) echo "roko-conductor" ;;
    M120) echo "roko-conductor" ;;
    M121) echo "roko-conductor" ;;
    M122) echo "roko-conductor roko-cli" ;;
    M123) echo "roko-core roko-daimon" ;;
    M124) echo "roko-daimon" ;;
    M125) echo "roko-daimon roko-core" ;;
    M126) echo "roko-daimon roko-cli" ;;
    M127) echo "roko-daimon" ;;
    M128) echo "roko-daimon" ;;
    M129) echo "roko-cli roko-daimon" ;;
    # Phase 4: Memory and Knowledge
    M096) echo "roko-neuro roko-core" ;;
    M097) echo "roko-neuro roko-core roko-fs" ;;
    M098) echo "roko-primitives" ;;
    M099) echo "roko-neuro" ;;
    M100) echo "roko-neuro roko-learn" ;;
    M101) echo "roko-neuro" ;;
    M102) echo "roko-neuro roko-core" ;;
    M103) echo "roko-dreams" ;;
    M104) echo "roko-dreams roko-cli" ;;
    M105) echo "roko-dreams" ;;
    M106) echo "roko-dreams" ;;
    M107) echo "roko-dreams" ;;
    M108) echo "roko-dreams roko-neuro" ;;
    M109) echo "roko-dreams" ;;
    M110) echo "roko-core" ;;
    M111) echo "roko-learn roko-core" ;;
    M112) echo "roko-neuro roko-learn" ;;
    # Phase 6: Chain and Registries
    M131) echo "roko-chain roko-core" ;;
    M132) echo "roko-chain roko-core" ;;
    M133) echo "roko-chain roko-primitives" ;;
    M134) echo "roko-chain" ;;
    M135) echo "roko-chain" ;;
    M136) echo "roko-chain" ;;
    M137) echo "roko-chain" ;;
    M138) echo "roko-chain" ;;
    M139) echo "roko-chain" ;;
    M140) echo "roko-chain" ;;
    M141) echo "roko-chain" ;;
    *) echo "roko-core" ;;
  esac
}

batch_phase_ref() {
  case "$1" in
    M001) echo "01-PHASE-0-PREP.md §0.4" ;; M002) echo "01-PHASE-0-PREP.md §0.3" ;;
    M003) echo "01-PHASE-0-PREP.md §0.1" ;; M004) echo "01-PHASE-0-PREP.md §0.1" ;;
    M015) echo "01-PHASE-0-PREP.md §0.1" ;; M016) echo "01-PHASE-0-PREP.md §0.1" ;;
    M017) echo "01-PHASE-0-PREP.md §0.2" ;; M018) echo "01-PHASE-0-PREP.md §0.2" ;;
    M019) echo "01-PHASE-0-PREP.md §0.2" ;; M020) echo "01-PHASE-0-PREP.md §0.3" ;;
    M005|M006|M007|M008|M009|M010|M011|M013) echo "02-PHASE-1-KERNEL.md §1.1" ;;
    M012) echo "02-PHASE-1-KERNEL.md §1.4" ;; M014) echo "02-PHASE-1-KERNEL.md §1.13" ;;
    M021|M022|M023|M024) echo "02-PHASE-1-KERNEL.md §1.2" ;;
    M025) echo "02-PHASE-1-KERNEL.md §1.3" ;;
    M026|M027) echo "02-PHASE-1-KERNEL.md §1.5" ;;
    M028|M029|M030) echo "02-PHASE-1-KERNEL.md §1.6" ;;
    M031|M032) echo "02-PHASE-1-KERNEL.md §1.7" ;;
    M033) echo "02-PHASE-1-KERNEL.md §1.8" ;; M034) echo "02-PHASE-1-KERNEL.md §1.9" ;;
    M035) echo "02-PHASE-1-KERNEL.md §1.10" ;; M036) echo "02-PHASE-1-KERNEL.md §1.11" ;;
    M037) echo "02-PHASE-1-KERNEL.md §1.12" ;;
    M038|M039) echo "03-PHASE-2-ENGINE.md §2.1" ;;
    M040|M041|M042) echo "03-PHASE-2-ENGINE.md §2.2" ;;
    M043) echo "03-PHASE-2-ENGINE.md §2.3" ;; M044|M045) echo "03-PHASE-2-ENGINE.md §2.4" ;;
    M046|M047|M048) echo "03-PHASE-2-ENGINE.md §2.5" ;;
    M049|M050) echo "03-PHASE-2-ENGINE.md §2.6" ;;
    M051|M052) echo "03-PHASE-2-ENGINE.md §2.7" ;;
    M053|M054|M055|M056) echo "03-PHASE-2-ENGINE.md §2.8" ;;
    M057) echo "03-PHASE-2-ENGINE.md §2.9" ;;
    M058|M059|M060|M061) echo "03-PHASE-2-ENGINE.md §2.10" ;;
    M062|M063|M064) echo "03-PHASE-2-ENGINE.md §2.11" ;;
    M065|M066|M067|M068) echo "04-PHASE-3-ECONOMY.md §3.1" ;;
    M069|M070|M071) echo "04-PHASE-3-ECONOMY.md §3.2" ;;
    M072|M073|M074|M075) echo "04-PHASE-3-ECONOMY.md §3.3" ;;
    M076|M077|M078|M079|M080|M081) echo "04-PHASE-3-ECONOMY.md §3.4" ;;
    M082|M083|M084|M085|M086|M087) echo "04-PHASE-3-ECONOMY.md §3.5" ;;
    M088|M089|M090|M091) echo "04-PHASE-3-ECONOMY.md §3.6" ;;
    M092|M093) echo "04-PHASE-3-ECONOMY.md §3.7" ;;
    M094|M095) echo "04-PHASE-3-ECONOMY.md §3.8" ;;
    # Phase 5: Conductor and Affect (depth docs)
    M116|M117) echo "07-agent-runtime/14-conductor-as-verify-pipeline.md" ;;
    M118) echo "07-agent-runtime/15-circuit-breaker-and-interventions.md" ;;
    M119) echo "07-agent-runtime/16-diagnosis-and-stuck-detection.md" ;;
    M120) echo "07-agent-runtime/16-diagnosis-and-stuck-detection.md" ;;
    M121) echo "07-agent-runtime/17-adaptive-supervision-loop.md" ;;
    M122) echo "07-agent-runtime/17-adaptive-supervision-loop.md" ;;
    M123|M124) echo "07-agent-runtime/18-affect-as-functor.md" ;;
    M125) echo "07-agent-runtime/19-behavioral-states-and-routing.md" ;;
    M126) echo "07-agent-runtime/18-affect-as-functor.md" ;;
    M127) echo "07-agent-runtime/20-somatic-landscape.md" ;;
    M128) echo "07-agent-runtime/21-collective-contagion.md" ;;
    M129) echo "07-agent-runtime/19-behavioral-states-and-routing.md" ;;
    # Phase 4: Memory and Knowledge (depth docs)
    M096|M097) echo "11-memory/01-knowledge-as-signal.md" ;;
    M098|M099) echo "11-memory/02-hdc-algebra-and-retrieval.md" ;;
    M100) echo "11-memory/03-knowledge-lifecycle-loop.md" ;;
    M101) echo "11-memory/04-antiknowledge-and-immunity.md" ;;
    M102) echo "11-memory/05-cross-domain-transfer.md" ;;
    M103|M104) echo "11-memory/06-dream-cycle-as-loop.md" ;;
    M105|M106) echo "11-memory/07-replay-and-counterfactual-cells.md" ;;
    M107) echo "11-memory/08-hypnagogia-and-creativity.md" ;;
    M108) echo "11-memory/09-consolidation-and-staging.md" ;;
    M109) echo "11-memory/10-threat-simulation-and-nightmares.md" ;;
    M110|M111) echo "11-memory/11-stigmergy-as-bus.md" ;;
    M112) echo "11-memory/03-knowledge-lifecycle-loop.md" ;;
    # Phase 6: Chain and Registries (depth docs)
    M131|M132) echo "18-registries/01-chain-as-domain-plugin.md" ;;
    M133|M134) echo "18-registries/02-hdc-on-chain-and-verification.md" ;;
    M135|M136) echo "18-registries/03-job-market-and-hiring.md" ;;
    M137) echo "18-registries/04-reputation-and-peer-scoring.md" ;;
    M138|M139) echo "18-registries/05-chain-witness-and-triage.md" ;;
    M140|M141) echo "18-registries/06-payments-and-settlement.md" ;;
    *) echo "" ;;
  esac
}

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------

preflight_check() {
  local errors=0
  log_header "PREFLIGHT"

  if command -v claude >/dev/null 2>&1; then
    log_ok "preflight" "claude CLI: $(command -v claude)"
  else
    log_err "preflight" "claude CLI not found"
    errors=$((errors + 1))
  fi

  if git -C "$ROKO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    log_ok "preflight" "git repo detected"
  else
    log_err "preflight" "ROKO_ROOT is not a git repo: $ROKO_ROOT"
    errors=$((errors + 1))
  fi

  ensure_dir "$LOG_ROOT"
  ensure_dir "$WORKTREE_ROOT"

  if [[ ${#ALL_BATCHES[@]} -eq 0 ]]; then
    log_warn "preflight" "ALL_BATCHES is empty — run ingest-and-audit prompt to populate"
  else
    local batch missing=0
    for batch in "${ALL_BATCHES[@]}"; do
      if [[ ! -f "$(batch_prompt_file "$batch")" ]]; then
        log_err "preflight" "Missing prompt: $(batch_prompt_file "$batch")"
        missing=$((missing + 1))
      fi
    done
    if (( missing > 0 )); then
      log_err "preflight" "$missing batch prompt file(s) missing"
      errors=$((errors + 1))
    else
      log_ok "preflight" "All ${#ALL_BATCHES[@]} batch prompts found"
    fi
  fi

  local dirty_count
  dirty_count=$(git -C "$ROKO_ROOT" status --porcelain | wc -l | tr -d ' ')
  if (( dirty_count > 0 )); then
    log_warn "preflight" "main repo has $dirty_count uncommitted change(s); worktree starts from committed HEAD"
  else
    log_ok "preflight" "main repo is clean"
  fi

  return "$errors"
}
