#!/usr/bin/env bash

set -uo pipefail

: "${ROKO_ROOT:=/Users/will/dev/nunchi/roko/roko}"
: "${REF_ROOT:=$ROKO_ROOT/tmp/refinements-runner}"
: "${LOG_ROOT:=$REF_ROOT/logs}"
: "${PROMPTS_DIR:=$REF_ROOT/prompts}"
: "${CONTEXT_DIR:=$REF_ROOT/context-pack}"
: "${WORKTREE_ROOT:=$ROKO_ROOT/.roko/worktrees}"
: "${REFINEMENTS_DIR:=$ROKO_ROOT/tmp/refinements}"
: "${DOCS_DIR:=$ROKO_ROOT/docs}"

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

# Execution order — one batch per refinement (REF01..REF35)
# Phase A (foundation reframing, docs/00-architecture heavy):
#   REF01..REF09.
# Phase B (learning / intelligence, new subdirs):
#   REF10..REF16.
# Phase C (moat / modularity):
#   REF17..REF21.
# Phase D (UX):
#   REF22..REF30.
# Phase E (integrators):
#   REF31..REF35.
ALL_BATCHES=(
  "REF01" "REF02" "REF03" "REF04" "REF05"
  "REF06" "REF07" "REF08" "REF09"
  "REF10" "REF11" "REF12" "REF13" "REF14" "REF15" "REF16"
  "REF17" "REF18" "REF19" "REF20" "REF21"
  "REF22" "REF23" "REF24" "REF25"
  "REF26" "REF27" "REF28" "REF29" "REF30"
  "REF31" "REF32" "REF33" "REF34" "REF35"
)

success_status() {
  case "${1:-}" in
    success|success_noop|skipped) return 0 ;;
    *) return 1 ;;
  esac
}

terminal_failure_status() {
  case "${1:-}" in
    spawn_failed|verify_failed|commit_failed|timeout|blocked) return 0 ;;
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

ensure_dir() { mkdir -p "$1"; }

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    log_err "bootstrap" "Missing file: $path"
    exit 1
  fi
}

latest_run_id() {
  [[ -d "$LOG_ROOT" ]] || return 1
  local id
  id="$(find "$LOG_ROOT" -maxdepth 1 -mindepth 1 -type d -name 'run-*' \
        -exec test -f {}/manifest.env \; -print 2>/dev/null \
      | sort \
      | tail -1 \
      | sed 's|.*/||')"
  if [[ -z "$id" ]]; then
    return 1
  fi
  printf '%s\n' "$id"
}

run_manifest_file()   { echo "$LOG_ROOT/$1/manifest.env"; }
run_result_file()     { echo "$LOG_ROOT/$1/$2.result"; }
run_log_file()        { echo "$LOG_ROOT/$1/$2.log"; }
run_prompts_dir()     { echo "$LOG_ROOT/$1/prompts"; }
run_prompt_snapshot() { echo "$(run_prompts_dir "$1")/$2.prompt.md"; }
run_last_message_file(){ echo "$LOG_ROOT/$1/$2.last.txt"; }
run_failure_file()    { echo "$LOG_ROOT/$1/$2.failure.txt"; }
run_status_file()     { echo "$LOG_ROOT/$1/status.tsv"; }
run_current_batch_file(){ echo "$LOG_ROOT/$1/current-batch.env"; }
run_backups_dir()     { echo "$LOG_ROOT/$1/backups"; }
batch_prompt_file()   { echo "$PROMPTS_DIR/$1.prompt.md"; }

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
# Batch metadata
# ---------------------------------------------------------------------------

# Each batch maps 1:1 to a refinement doc at tmp/refinements/NN-*.md and
# updates the docs/** files that the refinement implies should change.
# The full refinement text is injected into the prompt so the agent has
# the canonical source.

batch_title() {
  case "$1" in
    REF01) echo "Critique 'one-noun, six-verbs' across docs/00-architecture" ;;
    REF02) echo "Introduce Pulse (ephemeral medium) across architecture chapter" ;;
    REF03) echo "Promote Bus to kernel fabric across architecture + subsystem docs" ;;
    REF04) echo "Generalize six operators over Datum (Engram|Pulse)" ;;
    REF05) echo "Retell universal cognitive loop as 7 steps with broadcast" ;;
    REF06) echo "Land refactoring-plan phases as a dedicated architecture sub-chapter" ;;
    REF07) echo "Naming decisions: Pulse/Bus/Topic/Datum across glossary + docs" ;;
    REF08) echo "Code sketches: add appendix + wire types into relevant chapters" ;;
    REF09) echo "Phase-2 implications: update chain/dreams/mesh/coordination chapters" ;;

    REF10) echo "Self-learning cybernetic loops: update learning + heartbeat chapters" ;;
    REF11) echo "HDC as first-class substrate field across neuro + architecture" ;;
    REF12) echo "Knowledge demurrage supersedes decay field across decay chapter" ;;
    REF13) echo "c-factor continuous measurement across coordination + architecture" ;;
    REF14) echo "Heuristics + falsifiers as new learning sub-chapter" ;;
    REF15) echo "Exponential scaling loops across autocatalytic + technical-analysis" ;;
    REF16) echo "Research-to-runtime pipeline across references + learning" ;;

    REF17) echo "Plugin/extension five-tier SPI across tools + interfaces" ;;
    REF18) echo "Competitive moat structural components across technical-analysis" ;;
    REF19) echo "Net-new innovations catalog across design-principles + frontier" ;;
    REF20) echo "Modularity + dep graph across crate-map + architecture" ;;
    REF21) echo "From-scratch rewrite candidates across implementation-readiness" ;;

    REF22) echo "Developer UX: four-layer Rust SDK across interfaces" ;;
    REF23) echo "User UX: four surfaces + unified verb set across interfaces" ;;
    REF24) echo "Deployment UX: five shapes across deployment chapter" ;;
    REF25) echo "Domain-specific agents: six profiles across agents chapter" ;;

    REF26) echo "StateHub rearchitecture across interfaces + architecture" ;;
    REF27) echo "Realtime event surface (WS/SSE/gRPC) across interfaces" ;;
    REF28) echo "CLI parity with Claude-Code muscle memory across interfaces" ;;
    REF29) echo "Web UI five-page architecture across interfaces" ;;
    REF30) echo "Rich UX primitives (ten) across interfaces" ;;

    REF31) echo "Synergy integration map as architecture integrator chapter" ;;
    REF32) echo "Safety/sandbox/provenance spine across safety chapter" ;;
    REF33) echo "Observability + telemetry across deployment + architecture" ;;
    REF34) echo "Glossary consolidation across naming-and-glossary" ;;
    REF35) echo "Consolidated roadmap as architecture backmatter" ;;

    *) return 1 ;;
  esac
}

batch_refinement_file() {
  # Source refinement file that drives this batch.
  case "$1" in
    REF01) echo "$REFINEMENTS_DIR/01-critique-one-noun.md" ;;
    REF02) echo "$REFINEMENTS_DIR/02-engram-vs-pulse.md" ;;
    REF03) echo "$REFINEMENTS_DIR/03-bus-as-first-class.md" ;;
    REF04) echo "$REFINEMENTS_DIR/04-operators-generalized.md" ;;
    REF05) echo "$REFINEMENTS_DIR/05-loop-retold.md" ;;
    REF06) echo "$REFINEMENTS_DIR/06-refactoring-plan.md" ;;
    REF07) echo "$REFINEMENTS_DIR/07-naming.md" ;;
    REF08) echo "$REFINEMENTS_DIR/08-code-sketches.md" ;;
    REF09) echo "$REFINEMENTS_DIR/09-phase-2-implications.md" ;;
    REF10) echo "$REFINEMENTS_DIR/10-self-learning-cybernetic-loops.md" ;;
    REF11) echo "$REFINEMENTS_DIR/11-hyperdimensional-substrate.md" ;;
    REF12) echo "$REFINEMENTS_DIR/12-knowledge-demurrage.md" ;;
    REF13) echo "$REFINEMENTS_DIR/13-collective-intelligence-c-factor.md" ;;
    REF14) echo "$REFINEMENTS_DIR/14-worldview-validation.md" ;;
    REF15) echo "$REFINEMENTS_DIR/15-exponential-scaling.md" ;;
    REF16) echo "$REFINEMENTS_DIR/16-research-to-runtime.md" ;;
    REF17) echo "$REFINEMENTS_DIR/17-plugin-extension-architecture.md" ;;
    REF18) echo "$REFINEMENTS_DIR/18-competitive-moat.md" ;;
    REF19) echo "$REFINEMENTS_DIR/19-net-new-innovations.md" ;;
    REF20) echo "$REFINEMENTS_DIR/20-modularity-composability.md" ;;
    REF21) echo "$REFINEMENTS_DIR/21-from-scratch-redesigns.md" ;;
    REF22) echo "$REFINEMENTS_DIR/22-developer-ux-rust.md" ;;
    REF23) echo "$REFINEMENTS_DIR/23-user-ux-running-agents.md" ;;
    REF24) echo "$REFINEMENTS_DIR/24-deployment-ux.md" ;;
    REF25) echo "$REFINEMENTS_DIR/25-domain-specific-agents.md" ;;
    REF26) echo "$REFINEMENTS_DIR/26-statehub-rearchitecture.md" ;;
    REF27) echo "$REFINEMENTS_DIR/27-realtime-event-surface.md" ;;
    REF28) echo "$REFINEMENTS_DIR/28-cli-parity-familiar-workflows.md" ;;
    REF29) echo "$REFINEMENTS_DIR/29-web-ui-architecture.md" ;;
    REF30) echo "$REFINEMENTS_DIR/30-rich-ux-primitives.md" ;;
    REF31) echo "$REFINEMENTS_DIR/31-synergy-integration-map.md" ;;
    REF32) echo "$REFINEMENTS_DIR/32-safety-sandbox-provenance.md" ;;
    REF33) echo "$REFINEMENTS_DIR/33-observability-telemetry.md" ;;
    REF34) echo "$REFINEMENTS_DIR/34-glossary.md" ;;
    REF35) echo "$REFINEMENTS_DIR/35-consolidated-roadmap.md" ;;
    *) return 1 ;;
  esac
}

batch_target_docs() {
  # Space-separated list of candidate docs/ files the refinement implies
  # should change. The prompt lists these; the agent is free to skip any
  # that on inspection don't actually need to change, and add others.
  case "$1" in
    REF01) echo "docs/00-architecture/INDEX.md docs/00-architecture/06-synapse-traits.md docs/00-architecture/23-architectural-analysis-improvements.md docs/INDEX.md" ;;
    REF02) echo "docs/00-architecture/02-engram-data-type.md docs/00-architecture/INDEX.md docs/00-architecture/01-naming-and-glossary.md" ;;
    REF03) echo "docs/00-architecture/07-substrate-trait.md docs/00-architecture/12-five-layer-taxonomy.md docs/00-architecture/24-cross-section-integration-map.md docs/00-architecture/INDEX.md" ;;
    REF04) echo "docs/00-architecture/06-synapse-traits.md docs/00-architecture/08-scorer-gate-router-composer-policy.md docs/00-architecture/23-architectural-analysis-improvements.md" ;;
    REF05) echo "docs/00-architecture/09-universal-cognitive-loop.md docs/16-heartbeat/00-coala-9-step-pipeline.md docs/16-heartbeat/01-universal-loop-mapping.md docs/00-architecture/13-cognitive-cross-cuts.md" ;;
    REF06) echo "docs/00-architecture/INDEX.md docs/00-architecture/31-implementation-readiness-audit.md" ;;
    REF07) echo "docs/00-architecture/01-naming-and-glossary.md docs/00-architecture/INDEX.md" ;;
    REF08) echo "docs/00-architecture/06-synapse-traits.md docs/00-architecture/07-substrate-trait.md docs/00-architecture/08-scorer-gate-router-composer-policy.md" ;;
    REF09) echo "docs/08-chain/ docs/10-dreams/ docs/13-coordination/ docs/16-heartbeat/ docs/00-architecture/24-cross-section-integration-map.md" ;;

    REF10) echo "docs/05-learning/ docs/00-architecture/11-dual-process-and-active-inference.md docs/00-architecture/16-autocatalytic-and-cybernetics.md docs/16-heartbeat/11-active-inference-state-space.md" ;;
    REF11) echo "docs/06-neuro/ docs/00-architecture/02-engram-data-type.md docs/00-architecture/07-substrate-trait.md docs/00-architecture/27-temporal-knowledge-topology.md" ;;
    REF12) echo "docs/00-architecture/04-decay-variants.md docs/00-architecture/18-decay-tier-matrix.md docs/00-architecture/25-attention-as-currency.md docs/06-neuro/ docs/05-learning/" ;;
    REF13) echo "docs/00-architecture/14-c-factor-collective-intelligence.md docs/13-coordination/11-collective-intelligence-metrics.md docs/13-coordination/ docs/00-architecture/INDEX.md" ;;
    REF14) echo "docs/05-learning/ docs/00-architecture/INDEX.md docs/06-neuro/" ;;
    REF15) echo "docs/00-architecture/16-autocatalytic-and-cybernetics.md docs/00-architecture/30-cross-pollination-innovations.md docs/20-technical-analysis/ docs/13-coordination/10-exponential-flywheel.md" ;;
    REF16) echo "docs/21-references/ docs/05-learning/ docs/00-architecture/INDEX.md" ;;

    REF17) echo "docs/18-tools/ docs/12-interfaces/ docs/00-architecture/15-crate-map.md" ;;
    REF18) echo "docs/20-technical-analysis/ docs/00-architecture/17-design-principles-and-frontier-summary.md docs/00-architecture/30-cross-pollination-innovations.md" ;;
    REF19) echo "docs/00-architecture/17-design-principles-and-frontier-summary.md docs/00-architecture/30-cross-pollination-innovations.md docs/20-technical-analysis/" ;;
    REF20) echo "docs/00-architecture/15-crate-map.md docs/00-architecture/12-five-layer-taxonomy.md docs/00-architecture/23-architectural-analysis-improvements.md" ;;
    REF21) echo "docs/00-architecture/31-implementation-readiness-audit.md docs/00-architecture/23-architectural-analysis-improvements.md" ;;

    REF22) echo "docs/12-interfaces/ docs/02-agents/ docs/00-architecture/INDEX.md" ;;
    REF23) echo "docs/12-interfaces/ docs/00-architecture/INDEX.md" ;;
    REF24) echo "docs/19-deployment/ docs/12-interfaces/" ;;
    REF25) echo "docs/02-agents/ docs/12-interfaces/ docs/18-tools/" ;;

    REF26) echo "docs/12-interfaces/ docs/00-architecture/24-cross-section-integration-map.md" ;;
    REF27) echo "docs/12-interfaces/ docs/19-deployment/" ;;
    REF28) echo "docs/12-interfaces/" ;;
    REF29) echo "docs/12-interfaces/" ;;
    REF30) echo "docs/12-interfaces/" ;;

    REF31) echo "docs/00-architecture/24-cross-section-integration-map.md docs/00-architecture/INDEX.md docs/00-architecture/17-design-principles-and-frontier-summary.md" ;;
    REF32) echo "docs/11-safety/ docs/00-architecture/05-provenance-and-attestation.md docs/00-architecture/26-cognitive-immune-system.md" ;;
    REF33) echo "docs/19-deployment/ docs/00-architecture/21-performance-numerical-stability.md docs/00-architecture/32-comprehensive-test-strategy.md" ;;
    REF34) echo "docs/00-architecture/01-naming-and-glossary.md docs/INDEX.md docs/00-architecture/INDEX.md" ;;
    REF35) echo "docs/00-architecture/31-implementation-readiness-audit.md docs/00-architecture/INDEX.md docs/INDEX.md" ;;
    *) echo "" ;;
  esac
}

batch_deps() {
  # Dependency DAG — refinements that build on earlier ones must wait.
  case "$1" in
    REF02) echo "REF01" ;;
    REF03) echo "REF02" ;;
    REF04) echo "REF02 REF03" ;;
    REF05) echo "REF04" ;;
    REF06) echo "" ;;
    REF07) echo "REF02 REF03" ;;
    REF08) echo "REF04" ;;
    REF09) echo "REF03" ;;

    REF10) echo "REF03" ;;
    REF11) echo "REF02" ;;
    REF12) echo "REF02 REF11" ;;
    REF13) echo "REF10 REF11" ;;
    REF14) echo "REF12" ;;
    REF15) echo "REF12 REF14" ;;
    REF16) echo "REF14" ;;

    REF17) echo "REF03" ;;
    REF18) echo "REF15 REF17" ;;
    REF19) echo "REF11 REF12 REF13 REF14" ;;
    REF20) echo "REF03 REF11" ;;
    REF21) echo "REF06" ;;

    REF22) echo "REF04 REF20" ;;
    REF23) echo "REF04" ;;
    REF24) echo "REF03" ;;
    REF25) echo "REF17 REF23" ;;

    REF26) echo "REF03 REF23" ;;
    REF27) echo "REF26" ;;
    REF28) echo "REF23" ;;
    REF29) echo "REF26 REF27" ;;
    REF30) echo "REF26 REF23" ;;

    REF31) echo "REF11 REF12 REF13 REF14 REF17" ;;
    REF32) echo "REF17 REF25" ;;
    REF33) echo "REF26 REF27 REF24" ;;
    REF34) echo "REF07" ;;
    REF35) echo "REF31" ;;

    *) echo "" ;;
  esac
}

batch_group() {
  case "$1" in
    REF01|REF02|REF03|REF04|REF05|REF06|REF07|REF08|REF09) echo "foundation" ;;
    REF10|REF11|REF12|REF13|REF14|REF15|REF16) echo "learning" ;;
    REF17|REF18|REF19|REF20|REF21) echo "moat" ;;
    REF22|REF23|REF24|REF25) echo "ux-core" ;;
    REF26|REF27|REF28|REF29|REF30) echo "ux-surface" ;;
    REF31|REF32|REF33|REF34|REF35) echo "integrator" ;;
    *) echo "misc" ;;
  esac
}

# Retired terms that should NOT appear in updated docs.
# Used by the terminology check in verify.sh.
# Case-insensitive extended-regex patterns; word boundaries required so
# "Mori" doesn't match "memoRIes" etc.
#
# Exceptions: these terms are allowed in sections that explicitly
# document them as retired ("retired", "formerly", "deprecated",
# "historical", "legacy", "archive", "backup", "predecessor",
# "successor", "bardo-backup", "### Old"). See is_safe_retired_context
# in lib/verify.sh for the full list.
RETIRED_TERMS=(
  'Signal is the same as Engram'
  'In Rust code it is `Signal`'
  'Signal is a Rust alias for Engram'
  '\bBardo\b'
  '\bGolem\b'
  '\bMori\b'
  '\bGrimoire\b'
  '\bStyx\b'
  '\bClade\b'
  '\bmortal\b'
  '\bdeath as\b'
  '\breincarnation\b'
)

# Terms that SHOULD appear somewhere in the updated docs after the
# batch lands, if the refinement introduced them. The verify step
# checks the touched set for at least one mention.
batch_required_terms() {
  case "$1" in
    REF02) echo "Pulse" ;;
    REF03) echo "Bus trait|Bus fabric|Bus primitive|Bus kernel|kernel Bus" ;;
    REF04) echo "Datum|two mediums|two fabrics" ;;
    REF05) echo "seven.?step|SENSE|BROADCAST" ;;
    REF07) echo "Pulse|Topic|Datum|Bus" ;;
    REF09) echo "ChainBus|two.?fabric" ;;
    REF10) echo "prediction.?error|active inference" ;;
    REF11) echo "HDC|fingerprint" ;;
    REF12) echo "demurrage|balance" ;;
    REF13) echo "c.?factor" ;;
    REF14) echo "heuristic|falsifier|worldview" ;;
    REF15) echo "compounding|superlinear|exponential" ;;
    REF16) echo "replication ledger|claim|falsifier" ;;
    REF17) echo "plugin|five.?tier|SPI" ;;
    REF18) echo "moat" ;;
    REF20) echo "roko-bus|roko-hdc|roko-spi|dep graph" ;;
    REF23) echo "verb set|unified.*verb|four surfaces|CLI.*TUI.*Chat.*Web" ;;
    REF24) echo "laptop|single.?server|container|clustered|edge" ;;
    REF25) echo "domain profile|TypedContext|Custody" ;;
    REF26) echo "StateHub|projection" ;;
    REF27) echo "WebSocket|SSE|subscribe.*channel" ;;
    REF30) echo "footnote|reasoning stream|uncertainty" ;;
    REF31) echo "synergy|matrix" ;;
    REF32) echo "custody|sandbox|attestation" ;;
    REF33) echo "telemetry|observability|metric" ;;
    REF34) echo "glossary|retired" ;;
    *) echo "" ;;
  esac
}

preflight_check() {
  local errors=0
  log_header "PREFLIGHT"

  if command -v codex >/dev/null 2>&1; then
    log_ok "preflight" "codex CLI: $(command -v codex)"
  else
    log_err "preflight" "codex CLI not found"
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

  require_file "$REF_ROOT/README.md"
  require_file "$REF_ROOT/BATCHES.md"

  local batch
  for batch in "${ALL_BATCHES[@]}"; do
    require_file "$(batch_prompt_file "$batch")"
    require_file "$(batch_refinement_file "$batch")"
  done

  local dirty_count
  dirty_count=$(git -C "$ROKO_ROOT" status --porcelain | wc -l | tr -d ' ')
  if (( dirty_count > 0 )); then
    log_warn "preflight" "main repo has $dirty_count uncommitted change(s); the overnight worktree starts from committed HEAD only"
  else
    log_ok "preflight" "main repo is clean"
  fi

  return "$errors"
}
