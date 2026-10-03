#!/usr/bin/env python3
"""The P1-ext contamination probe (S08 §4.8; task 3332; its live run is task 3353).

A memorized SWE-bench instance is one a model can solve from having seen its gold patch in training, not from
reading the issue: `probe_one` gives each of `models` only the issue text -- never `patch`, `test_patch` or a
test name, which would hand the answer or a shortcut to it straight back -- and asks it to name the files it
would edit. A candidate is excluded when at least `agreement` of the models name exactly the gold file set: that
many models converging on the same files, from the issue text alone, is the fingerprint of memorization S08
§4.8 flags, not skill (a hard issue leaves room to disagree; a memorized one does not). `probe_many`'s report
names the excluded share, so a contaminated slice is caught before it is spent on.

`ProbeModel` is any callable from issue text to a list of file paths; a live one calls a model through the
metering proxy (task 3353, not this module -- it never runs here, offline or in a test).

Usage:
    probe.py --dataset PATH.jsonl --selection selection.json
  PATH.jsonl is select.py's own input shape; selection.json is `select.SelectionResult.as_json()`'s output.
  Prints `probe_many`'s report as JSON; exit 1 if the excluded share is read with `--max-excluded` and above it.

API:
    ProbeModel = Callable[[str], Sequence[str]]
    gold_files(record: Mapping) -> set[str]                       # select.patch_files(record["patch"]), as a set
    ProbeResult(instance_id, excluded, agreeing, n_models, guesses); .as_json() -> dict
    probe_one(record: Mapping, models: Sequence[ProbeModel], *, agreement: int = AGREEMENT) -> ProbeResult
    ProbeReport(results); .excluded_share -> float; .as_json() -> dict
    probe_many(records: Iterable[Mapping], models: Sequence[ProbeModel], *, agreement: int = AGREEMENT) -> ProbeReport
    main(argv=None) -> int
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

# Loaded by path, not `import select`: that name is the standard library's I/O-multiplexing module, already in
# sys.modules by the time anything here runs, so a plain import would silently reuse it instead of this
# directory's select.py.
_spec = importlib.util.spec_from_file_location("swebench_select", Path(__file__).resolve().parent / "select.py")
select_mod = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = select_mod  # dataclasses looks its own module up by name while select.py executes
_spec.loader.exec_module(select_mod)

AGREEMENT = 2  # of N_MODELS
N_MODELS = 5

ProbeModel = Callable[[str], Sequence[str]]


def gold_files(record: Mapping) -> set[str]:
    return set(select_mod.patch_files(record["patch"]))


@dataclass(frozen=True)
class ProbeResult:
    instance_id: str
    excluded: bool
    agreeing: int
    n_models: int
    guesses: list[list[str]]

    def as_json(self) -> dict:
        return {"instance_id": self.instance_id, "excluded": self.excluded, "agreeing": self.agreeing,
                "n_models": self.n_models, "guesses": self.guesses}


def probe_one(record: Mapping, models: Sequence[ProbeModel], *, agreement: int = AGREEMENT) -> ProbeResult:
    """Ask each of `models` to name `record`'s files from its issue text alone; exclude it when `agreement` or
    more name exactly the gold set. `models` never sees `record["patch"]`, `test_patch` or a test name."""
    issue_text = record["problem_statement"]
    gold = gold_files(record)
    guesses = [sorted(set(model(issue_text))) for model in models]
    agreeing = sum(1 for guess in guesses if set(guess) == gold)
    return ProbeResult(instance_id=record["instance_id"], excluded=agreeing >= agreement, agreeing=agreeing,
                       n_models=len(models), guesses=guesses)


@dataclass(frozen=True)
class ProbeReport:
    results: list[ProbeResult]

    @property
    def excluded_share(self) -> float:
        return sum(1 for result in self.results if result.excluded) / len(self.results) if self.results else 0.0

    def as_json(self) -> dict:
        return {"ev": "swebench.contamination_probe", "n": len(self.results), "excluded_share": self.excluded_share,
                "results": [result.as_json() for result in self.results]}


def probe_many(records: Iterable[Mapping], models: Sequence[ProbeModel], *, agreement: int = AGREEMENT) -> ProbeReport:
    return ProbeReport([probe_one(record, models, agreement=agreement) for record in records])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--dataset", type=Path, required=True)
    parser.add_argument("--selection", type=Path, required=True, help="a select.py selection log (JSON)")
    parser.add_argument("--max-excluded", type=float, default=None, help="exit 1 if the excluded share exceeds it")
    args = parser.parse_args(argv)
    try:
        by_id = {json.loads(line)["instance_id"]: json.loads(line)
                for line in args.dataset.read_text(encoding="utf-8").splitlines() if line.strip()}
        selection = json.loads(args.selection.read_text(encoding="utf-8"))
        records = [by_id[instance_id] for instance_id in selection["instance_ids"]]
    except (OSError, ValueError, KeyError) as err:
        print(f"probe: {err}", file=sys.stderr)
        return 2
    print("probe: no live model is wired here (task 3353 runs it through the metering proxy); nothing probed",
         file=sys.stderr)
    del records  # read and validated, but there is no model to call in this module
    return 2


if __name__ == "__main__":
    sys.exit(main())
