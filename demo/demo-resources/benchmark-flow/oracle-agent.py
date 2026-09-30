#!/usr/bin/env python3
"""Oracle command agent for the benchmark-flow demo.

This is not an agent. `roko bench swe --agent-mode command` sends it one
instance on stdin: the id, repo and problem statement, never the gold patch or
the tests that grade it. It looks the instance up in the predictions file that
the gold control exported (`--agent-mode gold --export-predictions ...`) and
prints that patch, so the demo can check the command-mode plumbing end to end.

Usage: oracle-agent.py <predictions-gold.jsonl>
"""

import json
import sys

# Fields the bench keeps from an agent. If one shows up, the harness is
# showing the agent the answer, and the demo must fail rather than pass.
HIDDEN_FIELDS = ("patch", "test_cmd", "test_patch", "test_files")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    instance = json.load(sys.stdin)
    leaked = [field for field in HIDDEN_FIELDS if field in instance]
    if leaked:
        print(f"agent payload carries hidden fields: {', '.join(leaked)}", file=sys.stderr)
        return 3

    instance_id = instance["instance_id"]
    with open(sys.argv[1], encoding="utf-8") as predictions:
        for line in predictions:
            if not line.strip():
                continue
            row = json.loads(line)
            if row.get("instance_id") == instance_id:
                sys.stdout.write(row["model_patch"])
                return 0
    print(f"the gold control exported no patch for {instance_id}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
