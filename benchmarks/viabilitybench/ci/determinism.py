#!/usr/bin/env python3
"""Determinism: two runs of a verdict must give the same JSON (S08 SC1, "two runs give identical verdicts").

A verifier whose verdict depends on the run (a clock, a random draw, directory or set order, leftovers of an earlier
run) puts noise into the VS labels. verify_verifiers.py judges every solution twice and compares the two judgements
with `compare`. The CLI runs any command that prints one JSON document, such as a family's hidden.py, several times
and compares what the runs printed, together with their exit status:

    determinism.py [--runs N] [--cwd DIR] [--timeout S] -- COMMAND [ARG ...]

It prints the differences, first ones first, and exits 0 when every run gave the same exit status and the same JSON,
1 when two runs differ, and 2 when a run could not start, timed out or printed no JSON. Objects compare without
regard to key order; arrays compare element by element; `1`, `1.0` and `true` all differ.

API:
    compare(first, second, *, limit: int = 20) -> list[str]   # e.g. "$.checks[2].passed: true != false"; [] if equal
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

SHOWN_CHARS = 80


def compare(first, second, *, limit: int = 20) -> list[str]:
    """Where two JSON documents differ, as JSON paths from the root `$`; at most `limit` of them."""
    differences: list[str] = []
    _walk(first, second, "$", differences, limit)
    return differences


def _walk(first, second, path: str, out: list[str], limit: int) -> None:
    if len(out) >= limit:
        return
    if type(first) is not type(second):
        out.append(f"{path}: {_show(first)} != {_show(second)}")
    elif isinstance(first, dict):
        for key in sorted(set(first) | set(second)):
            if key not in first:
                out.append(f"{path}.{key}: missing in the first run")
            elif key not in second:
                out.append(f"{path}.{key}: missing in the second run")
            else:
                _walk(first[key], second[key], f"{path}.{key}", out, limit)
            if len(out) >= limit:
                return
    elif isinstance(first, list):
        if len(first) != len(second):
            out.append(f"{path}: {len(first)} != {len(second)} items")
        for index, (one, other) in enumerate(zip(first, second)):
            _walk(one, other, f"{path}[{index}]", out, limit)
    elif first != second:
        out.append(f"{path}: {_show(first)} != {_show(second)}")
    del out[limit:]


def _show(value) -> str:
    text = json.dumps(value, ensure_ascii=False, sort_keys=True)
    return text if len(text) <= SHOWN_CHARS else text[:SHOWN_CHARS - 1] + "…"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0],
                                     usage="%(prog)s [--runs N] [--cwd DIR] [--timeout S] -- COMMAND [ARG ...]")
    parser.add_argument("--runs", type=int, default=2, help="how many times to run the command (at least 2)")
    parser.add_argument("--cwd", type=Path, default=None, help="where to run it")
    parser.add_argument("--timeout", type=float, default=600.0, help="seconds per run")
    argv = list(sys.argv[1:] if argv is None else argv)
    if "--" not in argv:
        parser.error("put the command after --")
    at = argv.index("--")
    args, command = parser.parse_args(argv[:at]), argv[at + 1:]
    if not command:
        parser.error("no command after --")
    if args.runs < 2:
        parser.error("--runs must be at least 2")
    results = []
    for number in range(1, args.runs + 1):
        try:
            done = subprocess.run(command, cwd=args.cwd, stdin=subprocess.DEVNULL, capture_output=True, text=True,
                                  timeout=args.timeout, check=False)
            results.append({"exit": done.returncode, "json": json.loads(done.stdout)})
        except (OSError, subprocess.TimeoutExpired) as err:
            print(f"determinism.py: run {number} could not finish: {type(err).__name__}: {err}", file=sys.stderr)
            return 2
        except ValueError:
            print(f"determinism.py: run {number} printed no JSON (exit status {done.returncode})", file=sys.stderr)
            return 2
    differences = [f"run {number}: {difference}" for number, result in enumerate(results[1:], 2)
                   for difference in compare(results[0], result)]
    print("\n".join(differences) or f"same exit status and JSON in {args.runs} runs")
    return 1 if differences else 0


if __name__ == "__main__":
    sys.exit(main())
