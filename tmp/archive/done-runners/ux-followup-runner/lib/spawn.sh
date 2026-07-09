#!/usr/bin/env bash

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

: "${UX_MODEL:=gpt-5.4}"
: "${UX_REASONING:=high}"
: "${UX_TIMEOUT:=5400}"

emit_shared_context_pack() {
  cat <<'EOF'
## Shared Context Pack

EOF

  local file title
  for file in \
    "$CONTEXT_DIR/00-UX-FOLLOWUP-RULES.md" \
    "$CONTEXT_DIR/01-CATALOG-MAP.md" \
    "$CONTEXT_DIR/02-WORKSPACE-TOPOLOGY.md" \
    "$CONTEXT_DIR/03-STATE-FLOW.md" \
    "$CONTEXT_DIR/04-SAFETY-LAYER.md" \
    "$CONTEXT_DIR/05-MORI-REFERENCE-APPENDIX.md"; do
    [[ -f "$file" ]] || continue
    title="$(basename "$file" .md)"
    printf '### %s\n\n' "$title"
    cat "$file"
    printf '\n'
  done
}

emit_delegation_guidance() {
  local batch="$1"
  cat <<'EOF'
## Delegation Requirement

You are explicitly authorized to use multiple subagents for this batch.
Use them aggressively where it helps, but keep the immediate blocking work local.

Required delegation behavior:

- Before coding, form a short plan and identify 2-4 concrete sidecar subtasks.
- Spawn explorers for targeted codebase questions and workers for bounded code edits.
- Each subagent gets the same context pack (`context-pack/00-05`) plus its specific task.
- Give each worker a disjoint write scope and tell them they are not alone in the codebase.
- Do not wait idly for subagents if you can make progress locally.
- If subagents are unavailable in this environment, continue locally without failing.
EOF

  printf 'Suggested parallel split for batch `%s`:\n' "$batch"

  case "$batch" in
    UX01)
      cat <<'EOF'
- explorer: read roko-gate failure-record format + orchestrate.rs gate-loop site
- explorer: read roko-learn episode_logger failure paths + existing plan-gen entry-points
- worker: add PlanRevision event variant to roko-runtime::event_bus + tests
- worker: wire plan-regeneration prompt augmentation with failure context + dedupe
EOF
      ;;
    UX02)
      cat <<'EOF'
- explorer: read prd.rs:628 (existing maybe_generate_plan_after_promote) + event_bus API
- worker: add PrdPublished event variant + emit from promote handler
- worker: add subscriber in roko-serve that invokes prd plan + plan run + integration test
EOF
      ;;
    UX03)
      cat <<'EOF'
- explorer: catalogue existing tests/ layout + mock dispatcher capabilities
- worker: write tests/e2e_self_host.rs driving prd idea → draft → plan → run
- worker: extend roko-std mock dispatcher with scripted replies for e2e scenario
EOF
      ;;
    UX04)
      cat <<'EOF'
- explorer: read plan discovery + tasks.toml parser + gate-rung validators
- worker: lift validation into standalone `plan validate <dir>` subcommand
- worker: add unit tests for cycle detection, missing role templates, bad gate refs
EOF
      ;;
    UX05)
      cat <<'EOF'
- explorer: map StateHub construction sites + DashboardEvent emitters
- worker: add in-process hub spawn for standalone TUI + delete polling fallback branch
- worker: unit test asserting `if snapshot_rx.is_none()` branch is unreachable
EOF
      ;;
    UX06)
      cat <<'EOF'
- explorer: research `notify` crate v6 debouncing + existing mpsc shape
- worker: new fs_watch.rs module with Watcher + debounce + fallback poller
- worker: delete 500 ms polling thread in app.rs + route notify events through data_rx
EOF
      ;;
    UX07)
      cat <<'EOF'
- explorer: read load_signal_state / load_episodes_from_path / load_event_log
- worker: add per-file byte offset cursor + incremental tail reader for JSONL
- worker: integrate with fs_watch events + unit tests over a synthetic JSONL
EOF
      ;;
    UX08)
      cat <<'EOF'
- explorer: read load_task_outputs + task-output directory shape
- worker: directory watcher + per-file incremental tail reader
- worker: backfill wiring into current_plan_execution
EOF
      ;;
    UX09)
      cat <<'EOF'
- explorer: read roko-agent-server /stream handler + agents_view.rs render
- worker: new ws_client.rs using tokio-tungstenite with reconnect back-off
- worker: hook into TuiState.agent_streams ring buffer + Agents-tab render
EOF
      ;;
    UX10)
      cat <<'EOF'
- explorer: read git_view.rs collect_git_data + .git layout
- worker: fs-watch on .git/HEAD + .git/refs/heads/* with debounce
- worker: replace 3 s sleep loop + keep non-inotify fallback
EOF
      ;;
    UX11)
      cat <<'EOF'
- explorer: audit mpsc channels used across tui/*
- worker: swap unbounded channels for tokio::sync::watch or sync_channel(1)
- worker: persist DashboardGenerationState to .roko/state/dashboard-gen.json
EOF
      ;;
    UX12)
      cat <<'EOF'
- explorer: read ExecutorSnapshot struct + save_snapshot_atomic + load path
- worker: add schema_version + reject-unknown path + synthetic v0 fixture
- worker: new snapshot_migrate.rs with v0→v1 dispatch + tests
EOF
      ;;
    UX13)
      cat <<'EOF'
- explorer: read plan discovery + Resume flow
- worker: add post-load validation that all snapshot plan_ids appear in discovered set
- worker: clear error message + resume refuses to proceed on mismatch
EOF
      ;;
    UX14)
      cat <<'EOF'
- explorer: read roko-runtime::process::shutdown + force_kill + nix::signal API
- worker: SIGTERM-then-grace-then-SIGKILL escalation + CancellationToken plumbing
- worker: impl Drop for ProcessSupervisor + unit test with panicking parent task
EOF
      ;;
    UX15)
      cat <<'EOF'
- explorer: locate all Kind::GateVerdict writers + FileSubstrate query API
- worker: substrate verdicts reader + per-gate pass/fail rolling computer
- worker: gate timeline widget on TUI Gate tab (sparkline per gate)
EOF
      ;;
    UX16)
      cat <<'EOF'
- explorer: read roko-conductor::diagnosis + StateHubSnapshot fields
- worker: add StateHubSnapshot.diagnoses + push from conductor into hub
- worker: TUI Diagnosis panel + GET /api/diagnosis/recent endpoint
EOF
      ;;
    UX17)
      cat <<'EOF'
- explorer: read .roko/learn/efficiency.jsonl schema + Learning tab render
- worker: new roko-learn::aggregate::efficiency_trend bucketing helper
- worker: Learning-tab sparkline for tokens / latency / cost over 24h / 7d buckets
EOF
      ;;
    UX18)
      cat <<'EOF'
- explorer: enumerate metric names in roko-core::obs + roko-agent-server::state
- worker: choose canonical set + re-export or extract shared MetricSchema
- worker: /metrics unification test + field-name audit log
EOF
      ;;
    UX19)
      cat <<'EOF'
- explorer: read ExperimentStore.concluded_winners + current Learning tab render
- worker: add "Concluded Experiments" panel with win-rate bars
- worker: snapshot test of render output
EOF
      ;;
    UX20)
      cat <<'EOF'
- explorer: read GET /api/agents/topology response shape
- worker: fetch topology from StateHub/HTTP + render nested list widget
- worker: Ctrl+T binding to open topology modal / dedicated tab
EOF
      ;;
    UX21)
      cat <<'EOF'
- explorer: read roko-agent-server routes + aggregator proxy pattern
- worker: add GET /logs?tail=N to roko-agent-server with bounded tail file
- worker: aggregator proxy at /api/agents/{id}/logs with integration test
EOF
      ;;
    UX22)
      cat <<'EOF'
- explorer: read .roko/learn/c-factor.jsonl + existing single-point endpoint
- worker: 24h / 7d rolling aggregator helper
- worker: GET /api/c-factor/trend + trend widget on Learning tab
EOF
      ;;
    UX23)
      cat <<'EOF'
- explorer: enumerate all Gate impls in roko-gate + run_gate_rung call-site
- worker: extend rung selector to dispatch Fact/Symbol/Generated/Property/VerifyChain/LlmJudge/Integration
- worker: integration tests per new rung
EOF
      ;;
    UX24)
      cat <<'EOF'
- explorer: read PlaybookStore APIs + dispatcher pre-prompt hook surface
- worker: wire playbook_store.query into prompt-builder memory layer
- worker: integration test over a two-playbook fixture
EOF
      ;;
    UX25)
      cat <<'EOF'
- explorer: read roko-primitives::hdc APIs + Episode struct / extra bag
- worker: compute HdcVector from (prompt, outcome) at episode write site
- worker: serialise as base64 on Episode + unit test round-trip
EOF
      ;;
    UX26)
      cat <<'EOF'
- explorer: read safety/contract.rs + SafetyLayer::check_pre_execution
- worker: load AgentContract at dispatcher boot + enforce invariants
- worker: integration tests for governance / recovery paths
EOF
      ;;
    UX27)
      cat <<'EOF'
- explorer: read roko.toml schema + SafetyLayer role plumbing
- worker: add [agent.<role>].tools whitelist to agent_config.rs + SafetyLayer lookup
- worker: dispatch-reject test when role lacks whitelist entry for a tool
EOF
      ;;
    UX28)
      cat <<'EOF'
- explorer: confirm the current Enriching-phase seam in orchestrate.rs and any dependency-cycle risk
- worker: wire roko-compose enrichment into orchestrate.rs::handle_enriching, not roko-agent::dispatcher
- worker: add the smallest useful tests/logging around backend selection and selected steps
EOF
      ;;
    UX29)
      cat <<'EOF'
- explorer: enumerate actual consumers of roko-dreams/daimon/chain and whether default-members is the right seam
- worker: adjust root build-surface metadata only if the current dependency graph supports it
- worker: audit MCP github / slack / scripts / stdio crates and record ship decisions
EOF
      ;;
    UX30)
      cat <<'EOF'
- explorer: capture a 10-turn Codex session to tests/fixtures/codex/
- worker: conformance test harness replaying fixture through codex_agent
- worker: assertion helpers for reasoning / tool / partial frames
EOF
      ;;
    UX31)
      cat <<'EOF'
- explorer: read cursor_agent.rs non-streaming path + Cursor API streaming
- worker: implement send_turn_streaming on LlmBackend for Cursor
- worker: streaming conformance test
EOF
      ;;
    UX32)
      cat <<'EOF'
- explorer: diff test coverage Claude vs Cursor/Codex (happy/stream/tool/error/session)
- worker: replicate 5 canonical scenarios for Codex + Cursor
- worker: reusable harness primitives for future backends
EOF
      ;;
    UX33)
      cat <<'EOF'
- explorer: diff ExecAgent vs ClaudeCliAgent responsibilities + enumerate call-sites
- worker: keep both adapters, but add doc-comments that state when to use each
- worker: consolidate only the overlapping ollama files into the directory form; verify gemini/perplexity already follow it
EOF
      ;;
    UX34)
      cat <<'EOF'
- explorer: read cascade_router.rs + model_router.rs + dispatcher trait
- worker: tests/cascade_router_integration.rs booting dispatcher with two mock backends
- worker: assert persisted .roko/learn/cascade-router.json reflects winner
EOF
      ;;
    UX35)
      cat <<'EOF'
- explorer: audit roko-gate::adaptive_threshold load/write + run_gate_rung selector
- worker: wire AdaptiveThresholds::load into run_gate_rung per-rung
- worker: integration test for two-session persistence + EMA update
EOF
      ;;
    UX36)
      cat <<'EOF'
- explorer: diff roko.toml example keys vs agent_config.rs fields
- worker: extend agent_config.rs to parse + store missing keys (role, budget, thresholds, routing_overrides)
- worker: propagate into dispatch-site + unit tests per key
EOF
      ;;
    UX37)
      cat <<'EOF'
- explorer: enumerate 6 SystemPromptBuilder layers + role templates
- worker: snapshot test per role (implementer/reviewer/planner/researcher/…) writing to testdata/
- worker: diff against golden files + fix any discovered drift
EOF
      ;;
    UX38)
      cat <<'EOF'
- explorer: prioritise hot-path unwraps (middleware.rs, system_prompt_builder.rs)
- worker: convert to Result propagation in routes/middleware.rs (22 unwraps)
- worker: convert to Result propagation in system_prompt_builder.rs (20 unwraps)
EOF
      ;;
    UX39)
      cat <<'EOF'
- explorer: enumerate roko-serve routes + existing error types
- worker: Json<T> validation layer (validator or custom) + shaped ApiError response
- worker: utoipa-generated openapi.json + CI check that schema is current
EOF
      ;;
    UX40)
      cat <<'EOF'
- explorer: enumerate dispatch sites that write Episode + backend strings
- worker: add backend: String to Episode + populate at write
- worker: bump snapshot schema_version if applicable (cross-ref UX12)
EOF
      ;;
    UX41)
      cat <<'EOF'
- explorer: confirm whether .github/workflows exists yet and the minimal cargo-llvm-cov install story
- worker: create .github/workflows/coverage.yml producing HTML artifact
- worker: add tools/coverage.sh and keep this artifact-only, with no threshold gate
EOF
      ;;
    UX42)
      cat <<'EOF'
- explorer: enumerate crate-level clippy::missing_* allow directives
- worker: add # Errors / # Panics doc sections + remove per-crate allows
- worker: add CI=true timeout guard for 100ms-timeout tests
EOF
      ;;
    UX43)
      cat <<'EOF'
- explorer: read MORI-PARITY-CHECKLIST format + current repo layout
- worker: tools/ CLI that greps current code for each checklist item
- worker: regenerate CHECKLIST with accurate ✓ / ✗ per item
EOF
      ;;
    UX44)
      cat <<'EOF'
- explorer: enumerate CLAUDE.md "What to work on" items 1-9 + their claimed wiring
- worker: one smoke test per item under tests/smoke/
- worker: assert observable side-effect (file surface / episode entry)
EOF
      ;;
    UX45)
      cat <<'EOF'
- explorer: grep -rn "grimoire\|styx\|clade\|mortal\|death" in live docs
- worker: rename in CLAUDE.md + tmp/*.md + README.md per naming memory
- worker: write a stale-snapshot sidecar note under tmp/ instead of mutating bardo-backup/
EOF
      ;;
    UX46)
      cat <<'EOF'
- explorer: read tmp/implementation-plans/00-INDEX.md, the Mori appendix, and actual wiring
- worker: update status markers for items landed in PR #13
- worker: generate a path-corrected sidecar from the current Mori appendix; do not edit bardo-backup/
EOF
      ;;
    UX47)
      cat <<'EOF'
- explorer: read tui-parity/lib/common.sh + BATCHES.md + 20260416 stop symptoms
- worker: add TUI_PARITY_MAX_BATCHES + TUI_PARITY_MAX_RETRIES + startup log
- worker: CI step running run-tui-parity.sh --dry-run + log retention policy (.gitignore + rotate)
EOF
      ;;
    *)
      cat <<'EOF'
- explorer: targeted read-only architecture questions per the batch prompt
- worker: first bounded implementation slice
- worker: second bounded implementation slice
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
    echo "# UX Follow-up Batch $batch"
    echo
    echo "Run id: $run_id"
    echo "Attempt: $attempt"
    echo "Model: $UX_MODEL"
    echo "Reasoning: $UX_REASONING"
    echo "Catalog refs: $(batch_catalog_refs "$batch")"
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
  local target_dir
  target_dir=$(batch_target_dir "$run_id" "$batch" "codex" "$attempt")
  : > "$last_message_file"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  local start_ts
  start_ts=$(date +%s)
  local exit_code=0

  {
    echo "=== Batch: $batch ($(batch_title "$batch")) ==="
    echo "=== Started: $(date -Iseconds) ==="
    echo "=== Worktree: $worktree ==="
    echo "=== Model: $UX_MODEL ==="
    echo "=== Reasoning: $UX_REASONING ==="
    echo "=== Timeout: $UX_TIMEOUT ==="
    echo "=== Cargo target: $target_dir ==="
    echo "=== Prompt snapshot: $prompt_snapshot ==="
    echo
  } > "$log_file"

  record_status "$run_id" "$batch" "$attempt" "spawn_started" "codex exec started"

  do_timeout "$UX_TIMEOUT" \
    env CARGO_TARGET_DIR="$target_dir" \
    codex exec \
      --model "$UX_MODEL" \
      --sandbox workspace-write \
      --full-auto \
      -c "model_reasoning_effort=$UX_REASONING" \
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
    log_err "$batch" "Timed out after $(fmt_duration "$UX_TIMEOUT")"
    return 124
  fi

  record_status "$run_id" "$batch" "$attempt" "spawn_failed" "codex exec exited with $exit_code"
  log_err "$batch" "Codex exited with code $exit_code"
  return "$exit_code"
}
