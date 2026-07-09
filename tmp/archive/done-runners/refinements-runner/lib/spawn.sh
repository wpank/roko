#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${REF_MODEL:=gpt-5.4}"
: "${REF_REASONING:=high}"
: "${REF_TIMEOUT:=5400}"

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-REFINEMENTS-RULES.md" \
    "$CONTEXT_DIR/01-TWO-FABRIC-PRIMER.md" \
    "$CONTEXT_DIR/02-TERMINOLOGY-TABLE.md" \
    "$CONTEXT_DIR/03-DOCS-TREE-MAP.md" \
    "$CONTEXT_DIR/04-SYNERGY-SUMMARY.md" \
    "$CONTEXT_DIR/05-REFINEMENTS-INDEX.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_refinement_source() {
  local batch="$1"
  local file
  file="$(batch_refinement_file "$batch")"
  [[ -f "$file" ]] || return 0
  echo "## Canonical refinement source"
  echo
  echo "This is the verbatim refinement proposal that this batch must propagate"
  echo "into \`docs/\`. Treat it as the authoritative source. Do not edit this"
  echo "file; only edit docs under \`docs/\`."
  echo
  printf -- "--- BEGIN %s ---\n\n" "$(basename "$file")"
  cat "$file"
  printf "\n--- END %s ---\n\n" "$(basename "$file")"
}

emit_delegation_guidance() {
  local batch="$1"
  cat <<'EOF'
## Delegation Requirement

You are authorized to use subagents. Prefer multiple parallel agents when
the target docs set is large.

Required delegation behavior:

- Form a plan first — for each candidate `docs/` file listed in the batch,
  decide (a) does it need changes, (b) how big, (c) is it self-contained.
- For large independent files, spawn a worker per file with a disjoint
  write scope.
- Every subagent gets the same context pack and the same refinement source.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally.
EOF

  printf '\nSuggested parallel split for batch `%s`:\n\n' "$batch"

  case "$batch" in
    REF01)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/06-synapse-traits.md` to soften the
  "one noun, six verbs" claim and point forward to the two-medium / two-fabric
  refinement.
- worker: update `docs/00-architecture/INDEX.md` and `docs/INDEX.md` to lead
  with the new framing.
- worker: annotate `docs/00-architecture/23-architectural-analysis-improvements.md`
  with a footer noting which audit items the refactor dissolves.
EOF
      ;;
    REF02)
      cat <<'EOF'
- worker: extend `docs/00-architecture/02-engram-data-type.md` with a Pulse
  sibling section and a link forward to 02b (if split); remove stale "Signal"
  disclaimers.
- worker: add a new `docs/00-architecture/02b-pulse-ephemeral-event.md` (or
  add a full section to 02) covering Pulse type, conversion law, and graduation
  policy.
- worker: update `docs/00-architecture/01-naming-and-glossary.md` with Pulse +
  related terms (Topic, TopicFilter, PulseSource).
EOF
      ;;
    REF03)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/07-substrate-trait.md` to present both
  kernel fabrics; add `07b-bus-transport-fabric.md` (or substantial Bus section).
- worker: update `docs/00-architecture/12-five-layer-taxonomy.md` to list Bus
  at L0 alongside Substrate.
- worker: update `docs/00-architecture/24-cross-section-integration-map.md`
  so the EngineEventBus proposal points at the Bus trait.
EOF
      ;;
    REF04)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/06-synapse-traits.md` around "two
  mediums, two fabrics, six operators."
- worker: update `docs/00-architecture/08-scorer-gate-router-composer-policy.md`
  with the new trait signatures (Datum, Pulse variants, PolicyOutputs).
- worker: annotate `docs/00-architecture/23-architectural-analysis-improvements.md`
  where the §2.2 / §3.2 concerns are resolved by the operator generalization.
EOF
      ;;
    REF05)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/09-universal-cognitive-loop.md` as a
  seven-step loop with co-equal PERSIST/BROADCAST and three sense sources.
- worker: update `docs/16-heartbeat/00-coala-9-step-pipeline.md` and
  `01-universal-loop-mapping.md` to reference the revised framing.
- worker: update `docs/00-architecture/13-cognitive-cross-cuts.md` with
  explicit wording that cross-cuts inject into operators, not into the loop
  as step 9.
EOF
      ;;
    REF06)
      cat <<'EOF'
- worker: add a refactor-plan chapter under `docs/00-architecture/` (pick a
  stable number that doesn't collide, e.g. `33-refactor-plan.md` or similar),
  mirroring the Phase A/B/C/D breakdown.
- worker: update `docs/00-architecture/INDEX.md` with the new chapter.
- worker: optionally update `docs/00-architecture/31-implementation-readiness-audit.md`
  to reference the refactor phases.
EOF
      ;;
    REF07)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/01-naming-and-glossary.md` per the
  canonical vocabulary: Pulse, Bus, Topic, TopicFilter, Datum, PulseSource,
  Engram (already there), with a retired-terms table.
- worker: update `docs/00-architecture/INDEX.md` abstract with the
  "two mediums, two fabrics, six operators" one-liner.
EOF
      ;;
    REF08)
      cat <<'EOF'
- worker: add small Rust snippet sections to trait chapters (06, 07, 08)
  referencing the code sketches; no new chapter needed, snippets in-place.
EOF
      ;;
    REF09)
      cat <<'EOF'
- worker: update `docs/08-chain/` INDEX + key files with ChainBus vs
  ChainSubstrate split.
- worker: update `docs/10-dreams/` with Substrate scan + Bus-subscription
  inputs wording.
- worker: update `docs/13-coordination/` (stigmergy) to phrase pheromone
  deposit + mesh.pheromone Bus topic.
- worker: update `docs/16-heartbeat/` with HeartbeatPolicy-publishes-Pulses
  framing.
EOF
      ;;
    REF10)
      cat <<'EOF'
- worker: add/update files under `docs/05-learning/` to describe the
  predict-publish-correct loop, per-operator calibration, CalibrationPolicy.
- worker: update `docs/00-architecture/11-dual-process-and-active-inference.md`
  with the FEP-as-literal implementation note.
- worker: update `docs/00-architecture/16-autocatalytic-and-cybernetics.md`
  with the Bus-as-feedback-nervous-system framing.
- worker: update `docs/16-heartbeat/11-active-inference-state-space.md` with
  prediction/outcome topic references.
EOF
      ;;
    REF11)
      cat <<'EOF'
- worker: update `docs/06-neuro/` INDEX and relevant files with HDC-per-Engram
  framing and the default encoder sketch.
- worker: update `docs/00-architecture/02-engram-data-type.md` with the
  fingerprint field.
- worker: update `docs/00-architecture/07-substrate-trait.md` with the
  `query_similar` method.
- worker: update `docs/00-architecture/27-temporal-knowledge-topology.md` with
  HDC-cluster-driven tier progression.
EOF
      ;;
    REF12)
      cat <<'EOF'
- worker: update `docs/00-architecture/04-decay-variants.md` to introduce
  demurrage superseding decay; document balance/reinforcement/thaw.
- worker: update `docs/00-architecture/18-decay-tier-matrix.md` with the
  new tier graduation rules.
- worker: update `docs/00-architecture/25-attention-as-currency.md` with
  the demurrage economic framing.
- worker: update `docs/06-neuro/` and `docs/05-learning/` where playbook
  freshness / episode retention is discussed.
EOF
      ;;
    REF13)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/14-c-factor-collective-intelligence.md`
  to describe continuous measurement + Policy actuation.
- worker: update `docs/13-coordination/11-collective-intelligence-metrics.md`
  with the five-axis CohortMetrics and CohortWeightsLearner.
- worker: update `docs/13-coordination/INDEX.md` to link the c-factor chapter.
EOF
      ;;
    REF14)
      cat <<'EOF'
- worker: add/update files under `docs/05-learning/` to describe the Heuristic
  type, Calibrator, Worldview clustering, Dissonance detection.
- worker: update `docs/06-neuro/` where distilled knowledge is discussed.
- worker: update `docs/00-architecture/INDEX.md` to link the heuristic chapter.
EOF
      ;;
    REF15)
      cat <<'EOF'
- worker: update `docs/00-architecture/16-autocatalytic-and-cybernetics.md` with
  the seven compounding loops enumeration.
- worker: update `docs/00-architecture/30-cross-pollination-innovations.md`
  with the network-effect claims.
- worker: update files under `docs/20-technical-analysis/` with the superlinear
  scaling story.
- worker: update `docs/13-coordination/10-exponential-flywheel.md` if present.
EOF
      ;;
    REF16)
      cat <<'EOF'
- worker: add/update files under `docs/21-references/` with the paper →
  claim → heuristic → trial → calibration pipeline and replication ledger.
- worker: update `docs/05-learning/` where claim-based parameters are referenced.
- worker: update `docs/00-architecture/INDEX.md` to link the replication
  chapter.
EOF
      ;;
    REF17)
      cat <<'EOF'
- worker: add/update files under `docs/18-tools/` or `docs/12-interfaces/`
  covering the five-tier plugin SPI, manifests, sandboxes.
- worker: update `docs/00-architecture/15-crate-map.md` with roko-spi and
  related new crates.
EOF
      ;;
    REF18)
      cat <<'EOF'
- worker: update files under `docs/20-technical-analysis/` with the five
  structural moat components.
- worker: update `docs/00-architecture/17-design-principles-and-frontier-summary.md`
  with the moat synthesis.
- worker: update `docs/00-architecture/30-cross-pollination-innovations.md`
  with the architectural-coherence claim.
EOF
      ;;
    REF19)
      cat <<'EOF'
- worker: update `docs/00-architecture/17-design-principles-and-frontier-summary.md`
  with the net-new innovations catalog.
- worker: update `docs/00-architecture/30-cross-pollination-innovations.md`
  with the primitive-composition story.
- worker: update files under `docs/20-technical-analysis/` with the
  publishable-claims list.
EOF
      ;;
    REF20)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/15-crate-map.md` to the target
  dependency graph (roko-bus, roko-hdc, roko-spi, splits of std/compose).
- worker: update `docs/00-architecture/12-five-layer-taxonomy.md` to reflect
  the kernel-tier additions.
- worker: update `docs/00-architecture/23-architectural-analysis-improvements.md`
  where the dep-graph audit items land.
EOF
      ;;
    REF21)
      cat <<'EOF'
- worker: update `docs/00-architecture/31-implementation-readiness-audit.md`
  with the from-scratch candidates and sequencing.
- worker: update `docs/00-architecture/23-architectural-analysis-improvements.md`
  with a pointer to the rewrite list.
EOF
      ;;
    REF22)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` describing the
  four-layer Rust SDK (one-liner / builder / trait / runtime).
- worker: update `docs/02-agents/` where custom-agent authoring is discussed.
EOF
      ;;
    REF23)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` for the unified
  verb set, four surfaces (CLI/TUI/Chat/Web), first-run flow, undo model.
EOF
      ;;
    REF24)
      cat <<'EOF'
- worker: add/update files under `docs/19-deployment/` covering the five
  deployment shapes, profiles, secrets, state portability, multi-tenancy.
EOF
      ;;
    REF25)
      cat <<'EOF'
- worker: update `docs/02-agents/` with the six domain profiles framing.
- worker: add/update files under `docs/18-tools/` with per-domain tool sets.
- worker: update `docs/12-interfaces/` with profile-install workflow.
EOF
      ;;
    REF26)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` describing StateHub
  as a kernel projection layer; projection trait; canonical projections.
- worker: update `docs/00-architecture/24-cross-section-integration-map.md`
  with StateHub as the consumer-fabric bridge.
EOF
      ;;
    REF27)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` on the realtime wire
  protocol (WS / SSE / gRPC), subscription channels, cursors, auth.
- worker: update `docs/19-deployment/` where external-consumer integration
  is discussed.
EOF
      ;;
    REF28)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` describing CLI parity
  with Claude-Code, slash commands, diff-first output, transcripts.
EOF
      ;;
    REF29)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` for the five-page
  first-party web UI (Home, Chat, Plans, Beliefs, Settings).
EOF
      ;;
    REF30)
      cat <<'EOF'
- worker: add/update files under `docs/12-interfaces/` for the ten rich UX
  primitives (reasoning streams, tool banners, heuristic footnotes, replay
  scrubber, uncertainty bars, etc).
EOF
      ;;
    REF31)
      cat <<'EOF'
- worker: add a synergy-integration-map chapter under `docs/00-architecture/`
  (pick a non-colliding number) that walks the 10-primitive matrix and the
  ten named synergies.
- worker: update `docs/00-architecture/24-cross-section-integration-map.md`
  to link the synergy map.
- worker: update `docs/00-architecture/INDEX.md` to index the new chapter.
EOF
      ;;
    REF32)
      cat <<'EOF'
- worker: update `docs/11-safety/` with the safety spine: role auth, sandboxes,
  pre/post checks, taint, attestation, chain-of-custody.
- worker: update `docs/00-architecture/05-provenance-and-attestation.md`
  with the Custody record shape and the attestation levels.
- worker: update `docs/00-architecture/26-cognitive-immune-system.md` with
  the taint-propagation + detection framing.
EOF
      ;;
    REF33)
      cat <<'EOF'
- worker: add/update files under `docs/19-deployment/` with the logs /
  metrics / traces / events / replay surfaces and the Roko-specific metrics.
- worker: update `docs/00-architecture/21-performance-numerical-stability.md`
  where performance observability is discussed.
- worker: update `docs/00-architecture/32-comprehensive-test-strategy.md`
  with replay-as-test framing.
EOF
      ;;
    REF34)
      cat <<'EOF'
- worker: rewrite `docs/00-architecture/01-naming-and-glossary.md` to the
  full canonical glossary with a retired-terms table.
- worker: update `docs/INDEX.md` and `docs/00-architecture/INDEX.md` with
  glossary backrefs.
EOF
      ;;
    REF35)
      cat <<'EOF'
- worker: add a consolidated-roadmap chapter under `docs/00-architecture/`
  that sequences the refinements into a multi-quarter plan.
- worker: update `docs/00-architecture/INDEX.md` with the new chapter.
- worker: update `docs/INDEX.md` if a top-level roadmap pointer is needed.
EOF
      ;;
    *)
      cat <<'EOF'
- worker: targeted per-file edits within the batch's write scope
- worker: index/cross-reference updates for affected INDEX.md files
EOF
      ;;
  esac

  echo
}

do_timeout() {
  local seconds="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$seconds" "$@"
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "$seconds" "$@"
  else
    "$@"
  fi
}

compose_prompt_snapshot() {
  local batch="$1"
  local run_id="$2"
  local attempt="$3"
  local failure_file="$4"
  local out
  out="$(run_prompt_snapshot "$run_id" "$batch")"
  ensure_dir "$(dirname "$out")"

  {
    echo "# Refinements Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $REF_MODEL"
    echo "Reasoning: $REF_REASONING"
    echo "Refinement source: $(batch_refinement_file "$batch")"
    echo "Target docs (candidates): $(batch_target_docs "$batch")"
    echo
    if [[ -s "$failure_file" ]]; then
      echo "## Previous attempt failure context"
      echo
      cat "$failure_file"
      echo
      echo "Use that context to avoid repeating the same failure."
      echo
    fi
    emit_shared_context_pack
    emit_delegation_guidance "$batch"
    emit_refinement_source "$batch"
    cat "$(batch_prompt_file "$batch")"
  } > "$out"

  echo "$out"
}

spawn_batch() {
  local batch="$1"
  local run_id="$2"
  local worktree="$3"
  local attempt="$4"
  local failure_file="$5"

  local prompt_snapshot
  prompt_snapshot=$(compose_prompt_snapshot "$batch" "$run_id" "$attempt" "$failure_file")
  local log_file
  log_file=$(run_log_file "$run_id" "$batch")
  local last_message_file
  last_message_file=$(run_last_message_file "$run_id" "$batch")
  : > "$last_message_file"

  local start_ts
  start_ts=$(date +%s)
  local exit_code=0

  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $REF_MODEL ==="
    echo "=== Reasoning: $REF_REASONING ==="
    echo "=== Timeout: $REF_TIMEOUT ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  do_timeout "$REF_TIMEOUT" \
    codex exec \
      --model "$REF_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$REF_REASONING" \
      --cd "$worktree" \
      -o "$last_message_file" \
      - \
      < "$prompt_snapshot" >> "$log_file" 2>&1 || exit_code=$?

  local end_ts
  end_ts=$(date +%s)
  local elapsed=$((end_ts - start_ts))

  {
    echo
    echo "=== Finished: $(date -Iseconds) ==="
    echo "=== Duration: $(fmt_duration "$elapsed") ==="
    echo "=== Exit code: $exit_code ==="
  } >> "$log_file"

  if [[ "$exit_code" -eq 0 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_succeeded" "codex exec completed"
    log_ok "$batch" "Codex completed in $(fmt_duration "$elapsed")"
    return 0
  fi

  if [[ "$exit_code" -eq 124 ]]; then
    record_status "$run_id" "$batch" "$attempt" "spawn_timed_out" "codex exec timed out"
    log_err "$batch" "Timed out after $(fmt_duration "$REF_TIMEOUT")"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exec exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
