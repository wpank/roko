#!/bin/bash
# Demo the plan workflow via CLI commands: prompt -> plan -> review -> run.
# Usage: bash demo-plan-cli.sh ["<prompt>"]
# Requires: roko workspace initialized (roko init) with a provider configured

set -euo pipefail

ROKO="${ROKO:-roko}"
PROMPT="${1:-Wire knowledge into matchmaking}"
# The plan's directory name: the prompt's words, lowercased and joined by dashes.
SLUG="$(printf '%s' "$PROMPT" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')"
PLAN_DIR="plans/$SLUG"

pause() {
    echo ""
    read -rp "  [press enter to continue] " < /dev/tty
    echo ""
}

echo "═══════════════════════════════════════════"
echo "  PLAN WORKFLOW DEMO (CLI)"
echo "═══════════════════════════════════════════"

echo ""
echo "STEP 1: WRITE A PLAN FROM A PROMPT"
echo "   roko plan generate \"$PROMPT\""
$ROKO plan generate "$PROMPT"
if [[ ! -f "$PLAN_DIR/tasks.toml" ]]; then
    echo "   No $PLAN_DIR/tasks.toml was written. Plans on disk:"
    $ROKO plan list
    exit 1
fi
pause

echo "STEP 2: REVIEW THE PLAN"
echo "   $PLAN_DIR/tasks.toml (first 40 lines)"
sed -n '1,40p' "$PLAN_DIR/tasks.toml"
echo ""
echo "   Edit $PLAN_DIR/tasks.toml and plan.md now if you want to change anything."
pause

echo "STEP 3: LINT THE PLAN"
$ROKO plan validate "$PLAN_DIR"
pause

echo "STEP 4: PREVIEW THE RUN"
$ROKO plan run "$PLAN_DIR" --dry-run
pause

echo "STEP 5: RUN THE PLAN"
read -rp "  Run it now? This dispatches agents. [y/N] " answer < /dev/tty
if [[ "$answer" == [yY] ]]; then
    $ROKO run "$PLAN_DIR"
else
    echo "   Skipped."
fi

echo ""
echo "═══════════════════════════════════════════"
echo "  Done. Next steps:"
echo "  • run the plan:      roko run $PLAN_DIR"
echo "  • task states:       roko plan status $PLAN_DIR"
echo "  • resume a run:      roko plan run $PLAN_DIR --resume-plan"
echo "  • watch it live:     roko dashboard (F2 Plans)"
echo "═══════════════════════════════════════════"
