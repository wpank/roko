#!/usr/bin/env bash
# demo-benchmark.sh — Run the reusable SWE-bench proxy + C-factor demo.
# Usage: bash demo-benchmark.sh [workdir]
# Exit: 0 if the controls and the command adapter behave as expected.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../bin/common.sh
source "$SCRIPT_DIR/../bin/common.sh"

require_roko
require_python
require_cmd git

WORKDIR="${1:-$(mktemp -d "${TMPDIR:-/tmp}/roko-bench-demo.XXXXXX")}"
mkdir -p "$WORKDIR"

CONTROLS="$WORKDIR/.roko/bench/controls.jsonl"
SCORES="$WORKDIR/.roko/bench/scores.jsonl"
EPISODES="$WORKDIR/.roko/episodes.jsonl"
EFFICIENCY="$WORKDIR/.roko/learn/efficiency.jsonl"
CFACTOR="$WORKDIR/.roko/learn/c-factor.jsonl"
GOLD_PATCHES="$WORKDIR/predictions-gold.jsonl"
COMMAND_PATCHES="$WORKDIR/predictions-command.jsonl"

log "SWE-bench proxy demo workspace: $WORKDIR"
log "roko binary: $ROKO"

run_bench() {
    local label="$1"
    shift
    echo ""
    log "$label"
    "$ROKO" bench swe --batch-size 2 --workdir "$WORKDIR" "$@"
}

line_count() {
    if [[ -f "$1" ]]; then
        wc -l <"$1" | tr -d ' '
    else
        echo 0
    fi
}

LEARNED_BEFORE="$(line_count "$EPISODES") $(line_count "$EFFICIENCY") $(line_count "$CFACTOR")"

# Controls check the harness, not a model: they are labeled control=true,
# written to controls.jsonl, and never recorded as learning.
run_bench "Positive control: gold patches (checks the harness, not a model)" \
    --agent-mode gold \
    --export-predictions "$GOLD_PATCHES"

run_bench "Negative control: empty patches" \
    --agent-mode empty

LEARNED_AFTER="$(line_count "$EPISODES") $(line_count "$EFFICIENCY") $(line_count "$CFACTOR")"
[[ "$LEARNED_AFTER" == "$LEARNED_BEFORE" ]] ||
    die "a control wrote learning state (episodes, efficiency, C-factor lines: $LEARNED_BEFORE -> $LEARNED_AFTER)"

# The command adapter runs an oracle, not an agent: it replays the patches the
# gold control exported, looked up by the instance_id it gets on stdin. The
# payload never carries the gold patch, and oracle-agent.py fails if it does.
printf -v COMMAND_AGENT '%q %q %q' "$PYTHON" "$SCRIPT_DIR/oracle-agent.py" "$GOLD_PATCHES"
run_bench "Command adapter: an oracle replays the gold control's exported patches" \
    --agent-mode command \
    --agent-command "$COMMAND_AGENT" \
    --export-predictions "$COMMAND_PATCHES"

for artifact in "$CONTROLS" "$SCORES" "$EPISODES" "$EFFICIENCY" "$CFACTOR"; do
    [[ -f "$artifact" ]] || die "missing artifact: $artifact"
done

echo ""
log "Validating control, score, episode and C-factor records"
"$PYTHON" - "$CONTROLS" "$SCORES" "$EPISODES" "$GOLD_PATCHES" "$COMMAND_PATCHES" <<'PY'
import json
import sys
from pathlib import Path

controls_path, scores_path, episodes_path, gold_path, command_path = map(Path, sys.argv[1:6])


def rows(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


gold, empty = rows(controls_path)[-2:]
assert gold["agent_mode"] == "gold" and gold["control"] is True, gold
assert empty["agent_mode"] == "empty" and empty["control"] is True, empty
assert gold["resolved"] == gold["total"] == 2, gold
assert empty["resolved"] == 0 and empty["total"] == 2, empty
for control in (gold, empty):
    # Controls are never recorded as learning, so they have no C-factor.
    assert control["cfactor_before"] is None and control["cfactor_after"] is None, control

scores = rows(scores_path)
assert not any(row.get("control") for row in scores), "a control row landed in scores.jsonl"
command = scores[-1]
assert command["agent_mode"] == "command", command
assert command["resolved"] == command["total"] == 2, command
assert command["cfactor_after"] is not None, command

episodes = rows(episodes_path)
assert not [e for e in episodes if e["model"] in ("roko-bench/gold", "roko-bench/empty")], (
    "a control wrote an episode"
)
latest = [e for e in episodes if e["task_id"].startswith(command["run_id"] + "/")]
assert len(latest) == 2, latest
for episode in latest:
    # The bench never sees what a command agent spends: the cost is unknown, not $0.
    assert episode["extra"].get("cost_known") is False, episode["extra"]

gold_patches = {row["instance_id"]: row["model_patch"] for row in rows(gold_path)}
command_patches = {row["instance_id"]: row["model_patch"] for row in rows(command_path)}
assert command_patches == gold_patches, "the oracle did not replay the gold control's patches"

print("  ok controls.jsonl: gold 2/2 and empty 0/2, both control=true, neither learned")
print(f"  ok scores.jsonl: command {command['resolved']}/{command['total']}, "
      f"C-factor after {command['cfactor_after']:.3f}")
print("  ok episodes: 2 from the command run, cost recorded as unknown; none from the controls")
print("  ok the oracle replayed the gold control's exported patches")
PY

echo ""
log "Artifact counts"
wc -l "$CONTROLS" "$SCORES" "$EPISODES" "$EFFICIENCY" "$CFACTOR" "$COMMAND_PATCHES"

echo ""
log "Current status snapshot"
"$ROKO" status --workdir "$WORKDIR" --cfactor

echo ""
log "Benchmark demo passed"
cat <<EOF

Artifacts:
  controls:    $CONTROLS
  scores:      $SCORES
  runs:        $WORKDIR/.roko/bench/runs
  episodes:    $EPISODES
  efficiency:  $EFFICIENCY
  c-factor:    $CFACTOR
  predictions: $GOLD_PATCHES (gold control)
               $COMMAND_PATCHES (oracle command run)

EOF
