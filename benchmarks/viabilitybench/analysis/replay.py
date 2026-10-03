"""Replay IO: one deterministic reader of a campaign's run records and the Roko arm's copies of Roko's own records
(S09 E6, S08 T16; task 3340). Every replay (H4-H6, M3, the demo) and every report reads through it.

**What it reads**, never writes: `<results>/<experiment>/<run_id>/records.jsonl` for each experiment named (or every
experiment directory under `results`), through `report.load_runs`, which validates each row against the run-record
schema and refuses simulated ones. For a record whose `provenance.s01_run_dir` names Roko's records copied by the Roko
arm (`run_roko._save_evidence`: `<run_id>/s01/<key>/`), it also reads `S01_FILES` and every `runs/*/attempts.jsonl`
there, as JSON rows. `.campaign/` and `<experiment>.abandoned/` directories are not experiments.

**In a stable order.** Entries come sorted by (experiment id, run id, record id), and each one's S01 rows by file
path, then line, whatever order the files were written in, so two loads of one tree give the same sequence and a
replay over it can be byte-identical (S09 E6). `streams` regroups the entries by (experiment, run, seed) in
`stream.position` order, the order the agent met the tasks, which a prequential replay walks.

**Corrupt rows are reported, never skipped.** A line that is not JSON or not a valid run record, a record filed
under another experiment's or run's directory, a repeated record id, an S01 row that is not a JSON object, an S01
file that is a symlink, and an `s01_run_dir` with nothing there are each a problem with its file and line. By
default `load` raises ReplayError listing every one; `strict=False` returns them with the rows that were sound.

**Blinding.** `blind`, a function from an arm id to its label (3341's `blind.py` gives salted hashes until the lock
allows unblinding), replaces each record's `arm` as it is read; the campaign says it is blinded.

**Positions** (F9's contract; `figlib`'s "Metric names"). A metric of the p-th task of a stream, such as the R-H4
replay's `regret_cum`, names its position as the clause `stream.position == p`; `position_cut` builds that cut, so
the figure scripts read p back from the MetricRecord's `record_filter`.

API:
    S01_FILES
    Problem(path, line, why); Entry(record, s01); Campaign(entries, problems, blinded)
        .records -> list[dict]; .streams() -> dict[(experiment, run_id, seed), list[Entry]]
    load(results, experiments=None, *, blind=None, strict=True) -> Campaign      # raises ReplayError
    position_cut(rows, position, clauses=()) -> metrics.Cut
    ReplayError
"""

from __future__ import annotations

import json
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

import metrics
import report
from metrics import Clause

S01_FILES = ("episodes.jsonl", "learn/costs.jsonl", "learn/efficiency.jsonl", "learn/run-metrics.jsonl")
ABANDONED = ".abandoned"  # campaign.py's suffix for a stopped unit's runs, beside the experiment's directory


class ReplayError(ValueError):
    """The campaign cannot be replayed as it is: its problems, each with its file and line."""


@dataclass(frozen=True)
class Problem:
    path: str
    line: int  # 0 for a problem with a whole file or directory
    why: str

    def __str__(self) -> str:
        return f"{self.path}:{self.line}: {self.why}" if self.line else f"{self.path}: {self.why}"


@dataclass(frozen=True)
class Entry:
    record: dict  # a vb.run_record/1 row; its `arm` is blinded when the campaign is
    s01: dict[str, tuple[dict, ...]]  # path under the record's S01 copy -> its rows, in order; {} without a copy


@dataclass(frozen=True)
class Campaign:
    entries: tuple[Entry, ...]
    problems: tuple[Problem, ...]
    blinded: bool

    @property
    def records(self) -> list[dict]:
        return [entry.record for entry in self.entries]

    def streams(self) -> dict[tuple[str, str, int], list[Entry]]:
        """The entries of each (experiment, run, seed) in `stream.position` order; a repeated position is an error."""
        grouped: dict[tuple[str, str, int], list[Entry]] = {}
        for entry in self.entries:
            record = entry.record
            grouped.setdefault((record["experiment_id"], record["run_id"], record["seed"]), []).append(entry)
        for key, entries in grouped.items():
            entries.sort(key=lambda entry: entry.record["stream"]["position"])
            positions = [entry.record["stream"]["position"] for entry in entries]
            if len(set(positions)) != len(positions):
                raise ReplayError(f"run {key[1]} of {key[0]}, seed {key[2]}, repeats a stream position")
        return dict(sorted(grouped.items()))


def load(results: Path, experiments: Sequence[str] | None = None, *, blind: Callable[[str], str] | None = None,
         strict: bool = True) -> Campaign:
    """Every run record of `experiments` (default: every experiment under `results`) with its S01 rows (module
    docstring). Raises ReplayError on any problem unless `strict` is False."""
    results = Path(results)
    if not results.is_dir():
        raise ReplayError(f"no results directory at {results}")
    names = list(experiments) if experiments is not None else sorted(
        path.name for path in results.iterdir()
        if path.is_dir() and not path.name.startswith(".") and not path.name.endswith(ABANDONED))
    problems: list[Problem] = []
    entries: list[Entry] = []
    seen: dict[str, str] = {}
    for name in sorted(set(names)):
        try:
            runs = report.load_runs(results / name)
        except report.ReportError as err:
            problems.append(Problem(str(results / name), 0, str(err)))
            continue
        for run in runs:
            problems += [_problem(text) for text in run.problems]
            for where, record in run.records:
                if record["experiment_id"] != name or record["run_id"] != run.path.name:
                    problems.append(_problem(f"{where}: a record of {record['experiment_id']}/{record['run_id']} "
                                             f"filed under {name}/{run.path.name}"))
                    continue
                if record["record_id"] in seen:
                    problems.append(_problem(f"{where}: record id {record['record_id']} repeats "
                                             f"{seen[record['record_id']]}"))
                    continue
                seen[record["record_id"]] = where
                s01 = _s01(run.path, record, problems)
                if blind is not None:
                    record = {**record, "arm": blind(record["arm"])}
                entries.append(Entry(record=record, s01=s01))
    entries.sort(key=lambda entry: (entry.record["experiment_id"], entry.record["run_id"], entry.record["record_id"]))
    found = Campaign(entries=tuple(entries), problems=tuple(problems), blinded=blind is not None)
    if strict and problems:
        raise ReplayError(f"{len(problems)} problem(s) in {results}:\n  " + "\n  ".join(map(str, problems)))
    return found


def position_cut(rows: Iterable[dict], position: int, clauses: Sequence[Clause] = ()) -> metrics.Cut:
    """The cut of `clauses` at stream position `position`: its filter ends `stream.position == <position>`, the
    clause F9 reads a metric's position from."""
    if isinstance(position, bool) or not isinstance(position, int) or position < 1:
        raise ValueError(f"a stream position is a positive integer, not {position!r}")
    return metrics.cut(rows, (*clauses, Clause("stream.position", "==", position)))


def _s01(run_dir: Path, record: dict, problems: list[Problem]) -> dict[str, tuple[dict, ...]]:
    """The record's copy of Roko's records, file by file in path order; every unreadable row is a problem."""
    relpath = (record.get("provenance") or {}).get("s01_run_dir")
    if not relpath:
        return {}
    root = run_dir / relpath
    if root.is_symlink() or not root.is_dir():
        problems.append(Problem(str(root), 0, "the record's s01_run_dir is not a directory here"))
        return {}
    found: dict[str, tuple[dict, ...]] = {}
    paths = [root / name for name in S01_FILES] + sorted(root.glob("runs/*/attempts.jsonl"))
    for path in paths:
        if not path.exists() and not path.is_symlink():
            continue
        name = path.relative_to(root).as_posix()
        if path.is_symlink() or not path.is_file():
            problems.append(Problem(str(path), 0, "not a regular file, so it is not read"))
            continue
        rows = []
        for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            if not line.strip():
                continue
            try:
                row = json.loads(line)
            except ValueError as err:
                problems.append(Problem(str(path), number, f"not JSON ({err})"))
                continue
            if not isinstance(row, dict):
                problems.append(Problem(str(path), number, "not a JSON object"))
                continue
            rows.append(row)
        found[name] = tuple(rows)
    return found


def _problem(text: str) -> Problem:
    """A Problem from report.py's "<file>:<line>: <why>" (or "<path>: <why>") text."""
    head, _, why = text.partition(": ")
    path, _, line = head.rpartition(":")
    return Problem(path, int(line), why) if path and line.isdigit() else Problem(head, 0, why)
