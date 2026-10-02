#!/usr/bin/env python3
"""Compare ``roko plan validate --spec-quality --json`` with speclint's JSONL (gap-46ab3f, S07.9).

The Rust port of the rules (``roko_gate::spec_quality``) must give every task speclint's score
within 0.5 points and the same hard fails. Run it from the repo root with a built roko::

    python3 benchmarks/viabilitybench/speclint/rust_parity.py [--roko target/debug/roko] [plans]

``plan validate`` skips ``archive/`` and ``archived/`` directories, so the script runs it on the
plans directory and again on each of those directories, and runs speclint on exactly the files each
run scored: ``depends_on_plan`` outputs then resolve over the same plans in both tools. Bands, rule
scores, verify classes and hard-fail details are compared as well, and fail the run only under
``--strict``. The script also checks that the fixtures vendored under
``crates/roko-gate/tests/fixtures/speclint/`` still match ``fixtures/``, and runs ``roko plan validate
--spec-quality --dynamic --fixture`` on a copy of ``fixtures/dynamic/red-on-base`` (3214): every task's
red-on-base result, outcome, SQ06 and HF3 must match the fixture's ``expected.json``, as speclint's
``dynamic.py`` does in ``tests/test_dynamic.py``.

Exit 0 when every task matches, 1 on a difference, 2 when a tool fails. Standard library only.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

SPECLINT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SPECLINT_DIR))
import speclint  # noqa: E402

# The directory names `plan validate` does not descend into (plan_validate.rs collect_tasks_files).
SKIPPED_DIRS = ("archive", "archived")
VENDORED_FIXTURES = Path("crates/roko-gate/tests/fixtures/speclint")
DYNAMIC_FIXTURE = SPECLINT_DIR / "fixtures" / "dynamic" / "red-on-base"
RULE_TOLERANCE = 1e-3


class ToolError(Exception):
    """roko or speclint failed to produce records."""


def validated_files(run_root: Path) -> list[Path]:
    """The ``tasks.toml`` files ``roko plan validate <run_root>`` lints."""
    if run_root.is_file():
        return [run_root]
    found: list[Path] = []
    pending = [run_root]
    while pending:
        current = pending.pop()
        for entry in current.iterdir():
            if entry.is_dir():
                if entry.name not in SKIPPED_DIRS:
                    pending.append(entry)
            elif entry.is_file() and entry.name == "tasks.toml":
                found.append(entry)
    return sorted(found)


def run_roots(plans: Path) -> list[Path]:
    """The plans directory, then every directory inside it that ``plan validate`` skips."""
    if not plans.is_dir():
        return [plans]
    skipped = sorted(path for path in plans.rglob("*") if path.is_dir() and path.name in SKIPPED_DIRS)
    return [plans, *skipped]


def roko_records(roko: str, root: Path, run_root: Path) -> tuple[list[dict], set[str]]:
    """The spec-quality records and parse-error paths of one ``plan validate`` run."""
    command = [roko, "plan", "validate", str(run_root), "--spec-quality", "--json"]
    proc = subprocess.run(command, cwd=root, capture_output=True, text=True)
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError as err:
        raise ToolError(f"{' '.join(command)} exited {proc.returncode} without JSON ({err}):\n{proc.stderr[-2000:]}") from err
    spec = report.get("spec_quality")
    if spec is None:
        raise ToolError(f"{' '.join(command)}: no spec_quality in the JSON; is roko built with gap-46ab3f?")
    return spec["tasks"], {error["path"] for error in spec["parse_errors"]}


def speclint_records(files: list[Path], root: Path) -> tuple[list[dict], set[str]]:
    """speclint's JSONL records and parse-error paths for exactly ``files``."""
    if not files:
        return [], set()
    command = [sys.executable, str(SPECLINT_DIR / "speclint.py"), *map(str, files), "--root", str(root), "--out", "-"]
    proc = subprocess.run(command, capture_output=True, text=True)
    if proc.returncode != 0:
        raise ToolError(f"speclint exited {proc.returncode}:\n{proc.stderr[-2000:]}")
    records = [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]
    prefix = "  parse error: "
    errors = {line[len(prefix) :].split(": ", 1)[0] for line in proc.stderr.splitlines() if line.startswith(prefix)}
    return records, errors


@dataclass
class Comparison:
    tasks: int = 0
    max_delta: float = 0.0
    failures: list[str] = field(default_factory=list)  # score or hard-fail differences, missing tasks
    notes: list[str] = field(default_factory=list)  # band, rule, class and detail differences


def _by_plan(records: list[dict]) -> dict[str, list[dict]]:
    plans: dict[str, list[dict]] = {}
    for record in records:
        plans.setdefault(record["plan_path"], []).append(record)
    return plans


def compare(ours: list[dict], theirs: list[dict], tolerance: float, result: Comparison) -> None:
    """Compare roko's records (``ours``) with speclint's, task by task in file order."""
    ours_by_plan, theirs_by_plan = _by_plan(ours), _by_plan(theirs)
    for plan in sorted(set(ours_by_plan) | set(theirs_by_plan)):
        mine, reference = ours_by_plan.get(plan, []), theirs_by_plan.get(plan, [])
        ids_mine = [record["task_id"] for record in mine]
        ids_reference = [record["task_id"] for record in reference]
        if ids_mine != ids_reference:
            result.failures.append(f"{plan}: task ids differ: roko {ids_mine}, speclint {ids_reference}")
            continue
        for got, want in zip(mine, reference):
            result.tasks += 1
            label = f"{plan} {got['task_id']}"
            delta = abs(got["score"] - want["score"])
            result.max_delta = max(result.max_delta, delta)
            if delta > tolerance:
                result.failures.append(f"{label}: score {got['score']} vs {want['score']}")
            if got["hard_fail"] != want["hard_fail"]:
                result.failures.append(f"{label}: hard fails {got['hard_fail']} vs {want['hard_fail']}")
            if got["band"] != want["band"]:
                result.notes.append(f"{label}: band {got['band']} vs {want['band']}")
            for rule, value in want["rules"].items():
                if abs(got["rules"].get(rule, float("nan")) - value) > RULE_TOLERANCE:
                    result.notes.append(f"{label}: {rule} {got['rules'].get(rule)} vs {value}")
            if got["verify_classes"] != want["verify_classes"]:
                result.notes.append(f"{label}: verify classes {got['verify_classes']} vs {want['verify_classes']}")
            if got["hard_fail_detail"] != want["hard_fail_detail"]:
                result.notes.append(f"{label}: hard-fail details {got['hard_fail_detail']} vs {want['hard_fail_detail']}")


def fixture_drift(root: Path) -> list[str]:
    """Differences between speclint's static fixtures and the copies roko-gate's tests read."""
    source, vendored = SPECLINT_DIR / "fixtures", root / VENDORED_FIXTURES

    def files(base: Path, skip_dynamic: bool) -> set[str]:
        if not base.is_dir():
            return set()
        rels = {path.relative_to(base).as_posix() for path in base.rglob("*") if path.is_file()}
        return {rel for rel in rels if not (skip_dynamic and rel.startswith("dynamic/"))}

    want, have = files(source, True), files(vendored, False)
    drift = [f"not vendored: {rel}" for rel in sorted(want - have)]
    drift += [f"vendored but not in speclint: {rel}" for rel in sorted(have - want)]
    drift += [f"differs: {rel}" for rel in sorted(want & have) if (source / rel).read_bytes() != (vendored / rel).read_bytes()]
    return drift


def expected_dynamic() -> dict[str, dict]:
    """The dynamic fixture's expected results, by "<plan_path> <task_id>"."""
    return json.loads((DYNAMIC_FIXTURE / "expected.json").read_text(encoding="utf-8"))["tasks"]


def dynamic_records(roko: str, scratch: Path) -> dict[str, dict]:
    """roko's red-on-base results for a copy of the dynamic fixture, by "<plan_path> <task_id>".

    Runs ``plan validate --spec-quality --dynamic --fixture`` on the copy and again on its
    ``archive/`` directory, which ``plan validate`` skips, with ``SPECLINT_FLAKY_COUNTER`` set as
    ``tests/test_dynamic.py`` sets it. Each result has the fields ``expected.json`` pins.
    """
    workspace = scratch / DYNAMIC_FIXTURE.name
    shutil.copytree(DYNAMIC_FIXTURE, workspace)
    env = dict(os.environ, SPECLINT_FLAKY_COUNTER=str(scratch / "flaky-counter"))
    found: dict[str, dict] = {}
    for run_root in (workspace, workspace / "archive"):
        command = [roko, "--repo", str(workspace), "plan", "validate", str(run_root), "--spec-quality", "--dynamic", "--fixture", "--json"]
        proc = subprocess.run(command, cwd=workspace, env=env, capture_output=True, text=True)
        try:
            report = json.loads(proc.stdout)
        except json.JSONDecodeError as err:
            raise ToolError(f"{' '.join(command)} exited {proc.returncode} without JSON ({err}):\n{proc.stderr[-2000:]}") from err
        if report.get("red_on_base") is None or report.get("spec_quality") is None:
            raise ToolError(f"{' '.join(command)}: no red_on_base in the JSON; is roko built with 3214?")
        outcomes = {f"{check['plan_path']} {check['task_id']}": check["outcome"] for check in report["red_on_base"]["checks"]}
        for record in report["spec_quality"]["tasks"]:
            key = f"{record['plan_path']} {record['task_id']}"
            found[key] = {
                "red_on_base": record["red_on_base"],
                "outcome": outcomes.get(key),
                "SQ06": record["rules"].get("SQ06"),
                "HF3": "HF3" in record["hard_fail"],
            }
    return found


def compare_dynamic(got: dict[str, dict], expected: dict[str, dict]) -> list[str]:
    """The differences between roko's red-on-base results and the fixture's expected ones."""
    failures = [f"{key}: missing from roko's records" for key in sorted(set(expected) - set(got))]
    failures += [f"{key}: not in expected.json" for key in sorted(set(got) - set(expected))]
    for key in sorted(set(expected) & set(got)):
        for name, want in expected[key].items():
            if got[key].get(name) != want:
                failures.append(f"{key}: {name} {got[key].get(name)!r} vs {want!r}")
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="rust_parity.py", description=__doc__.split("\n\n")[0])
    parser.add_argument("plans", nargs="?", type=Path, default=Path("plans"), help="plans directory (default: plans)")
    parser.add_argument("--roko", help="the roko binary (default: <root>/target/debug/roko)")
    parser.add_argument("--root", type=Path, help="workspace root (default: nearest .git or roko.toml)")
    parser.add_argument("--tolerance", type=float, default=0.5, help="largest allowed score difference")
    parser.add_argument("--show", type=int, default=20, help="how many differences of each kind to list")
    parser.add_argument("--strict", action="store_true", help="also fail on band, rule, class and detail differences")
    args = parser.parse_args(argv)

    root = (args.root or speclint.find_root(Path.cwd())).resolve()
    plans = args.plans if args.plans.is_absolute() else root / args.plans
    roko = args.roko or str(root / "target" / "debug" / "roko")
    if not Path(roko).is_file():
        print(f"rust_parity: no roko binary at {roko}; build it or pass --roko", file=sys.stderr)
        return 2

    result = Comparison()
    runs = run_roots(plans)
    n_files = 0
    try:
        for run_root in runs:
            files = validated_files(run_root)
            n_files += len(files)
            ours, our_errors = roko_records(roko, root, run_root)
            theirs, their_errors = speclint_records(files, root)
            if our_errors != their_errors:
                result.failures.append(f"{run_root}: parse errors differ: roko {sorted(our_errors)}, speclint {sorted(their_errors)}")
            compare(ours, theirs, args.tolerance, result)
        with tempfile.TemporaryDirectory(prefix="rust-parity-dynamic-") as scratch:
            dynamic = dynamic_records(roko, Path(scratch))
    except ToolError as err:
        print(f"rust_parity: {err}", file=sys.stderr)
        return 2
    drift = fixture_drift(root)
    dynamic_failures = compare_dynamic(dynamic, expected_dynamic())

    print(f"rust parity ({speclint.LINTER}): {len(runs)} plan validate runs, {n_files} files, {result.tasks} tasks compared")
    print(f"  score: max |delta| {result.max_delta:.2f} (tolerance {args.tolerance})")
    print(f"  failures (score, hard fails, task ids, parse errors): {len(result.failures)}")
    for line in result.failures[: args.show]:
        print(f"    {line}")
    print(f"  other differences (bands, rules, verify classes, details): {len(result.notes)}")
    for line in result.notes[: args.show]:
        print(f"    {line}")
    print(f"  vendored fixtures: {'in sync' if not drift else f'{len(drift)} differences'}")
    for line in drift[: args.show]:
        print(f"    {line}")
    print(f"  dynamic fixture (red on base): {len(dynamic)} tasks, {len(dynamic_failures)} differences")
    for line in dynamic_failures[: args.show]:
        print(f"    {line}")
    ok = not result.failures and not drift and not dynamic_failures and result.tasks > 0 and not (args.strict and result.notes)
    print("PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
