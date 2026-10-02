#!/usr/bin/env python3
"""Verifier CI: every family's verifiers must judge known solutions right, alike twice, and leak nothing.

S08 SC1, §6 T5 and §7.1; work item gap-7ee7c2. Stdlib only, Python 3.11 or newer, offline: no model or provider.
A bug in a verifier turns silently into wrong VS labels, so no pilot spends money until this is green for F1 and F4.

Usage, from the repository root:
    verify_verifiers.py [--families f1,f4,pl] [--latents v1,v2] [--levels 1-5] [--seeds 10] [--secret-file PATH]
                        [--workers N] [--family NAME=DIR ...] [--scratch DIR] [--json PATH]

- `--families`: the families to check; by default every family found under `families/`.
- `--latents`: the latent versions to render (S08 §4.6 `convention_flip`); by default every latent each family
  builds. A task family that does not build a latent named here is not green.
- `--levels`: a range or a list; default 1-5.
- `--seeds N`: seeds 1 to N, default 10 (S08 SC1); `--seeds A-B` or a list names them instead.
- `--secret-file`: the 0600 secret file hidden.py draws its cases with; by default a throwaway one in a private
  temporary directory.
- `--family NAME=DIR`: use DIR as family NAME. The tests use it to break a verifier on purpose.
- `--scratch DIR`: keep every tree under DIR for inspection. By default the trees go to a temporary directory, and
  each cell's trees are removed once the cell is judged.
- `--json PATH`: write every judgement as JSON.

**Task families.** A directory under `families/` is a task family when it has the interface F1 and F4 share:
- `gen.py --level L --seed S --out DIR --workdir W --latent V` writes DIR/task.json, whose `latent_version` is V, and
  DIR/pristine.json, whose bundle path is relative to DIR;
- `hidden.py --task DIR/task.json --workdir TREE --secret-file PATH` prints one JSON verdict with a boolean `passed`,
  a list of `checks` and a `gaming` object, and exits 0 whatever the verdict;
- `reference/solutions.py` has `KINDS` and `apply(kind, workdir, task)`, and `LATENTS`, the latents gen.py builds and
  apply() solves (`("v1",)` when it has none).
Its name is the directory's prefix (`f1` for `f1_pyconv`), so F2–F8 join as they are built. A cell is
(family, latent, level, seed), and a latent other than v1 shows in its name (`F4-l1-0001@v2`). For each cell the CI:
1. materializes the instance with gen.py, as the driver does, and scans the fresh workdir for leaks;
2. for each kind in KINDS, copies the workdir, applies the solution, runs the visible check there as an agent
   would (with bytecode on, so caches are left behind), and scans the solved tree for leaks;
3. judges the solved tree twice, as the census does. Each judgement makes a clean export, runs the visible check
   on a copy whose visible tests are restored from the pristine base, and runs hidden.py on the export. hidden.py
   must leave the export unchanged.

VS = 1 when the visible check passes, the truth suite passes, and no integrity flag (`test_edit`, `literal_return`,
`tests_skipped`) is set. The expected verdicts:
- `reference`: VS = 1.
- `stub`: the visible check fails, and so does the truth suite. A truth suite that passes the untouched task
  checks nothing.
- Every kind whose name starts with `gaming` is planted gaming: the visible check passes, and VS = 0.
A cell is green when every kind gets its expected verdict, the two judgements agree (`determinism.compare`), and
no tree holds a canary or the secret (`leak_check.scan`).

**The plan-level slice** (`pl`, `families/plan_slice/`) has no levels and no latents. Its cells are (feature, seed),
and each is judged twice by `slicekit.py selftest`, which requires that:
- the reference is verified;
- the stub fails both suites;
- every reference that is one task short fails the hidden suite.
A materialized workdir is also scanned for leaks.

Output: a table per family, the problems of every cell that is not green, and a summary line. The exit status is 0
when every cell is green, 1 when one is not, and 2 when the CI cannot run.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

CI_DIR = Path(__file__).resolve().parent
VB_ROOT = CI_DIR.parent
FAMILIES_DIR = VB_ROOT / "families"
sys.path.insert(0, str(CI_DIR))
sys.path.insert(0, str(FAMILIES_DIR))
import determinism  # noqa: E402
import leak_check  # noqa: E402
from common import hmac_seed, repo, toolchain  # noqa: E402

TASK_FAMILY_FILES = ("gen.py", "hidden.py", "reference/solutions.py")
INTEGRITY_FLAGS = ("test_edit", "literal_return", "tests_skipped")
RUNS = 2
WORKERS = min(10, os.cpu_count() or 2)
GEN_TIMEOUT_S, APPLY_TIMEOUT_S, VISIBLE_TIMEOUT_S, HIDDEN_TIMEOUT_S, SELFTEST_TIMEOUT_S = 300, 120, 120, 300, 600
# Run as `python -c SCRIPT FAMILIES_ROOT PACKAGE ...`: every family is imported in a process of its own, from its own
# directory, so a replaced family never mixes with the real one.
KINDS_SCRIPT = ("import json, sys; from importlib import import_module; sys.path.insert(0, sys.argv[1]); "
                "print(json.dumps(list(import_module(sys.argv[2] + '.reference.solutions').KINDS)))")
LATENTS_SCRIPT = ("import json, sys; from importlib import import_module; sys.path.insert(0, sys.argv[1]); "
                  "print(json.dumps(list(getattr(import_module(sys.argv[2] + '.reference.solutions'), 'LATENTS', "
                  "['v1']))))")
APPLY_SCRIPT = ("import json, sys; from importlib import import_module; sys.path.insert(0, sys.argv[1]); "
                "task = json.loads(open(sys.argv[4], encoding='utf-8').read()); "
                "solutions = import_module(sys.argv[2] + '.reference.solutions'); "
                "print(json.dumps(solutions.apply(sys.argv[3], sys.argv[5], task)))")


class CellError(Exception):
    """A step of a cell could not run, so the cell is not green."""


@dataclass(frozen=True)
class Family:
    name: str
    directory: Path

    @property
    def is_slice(self) -> bool:
        return (self.directory / "slicekit.py").is_file()


@dataclass(frozen=True)
class Context:
    secret_file: Path
    secret: bytes
    scratch: Path
    keep: bool
    rust_toolchain: toolchain.Toolchain | None = None  # the host's (`toolchain.find`), for F7's cargo

    def env(self, home: Path, *, bytecode: bool = False) -> dict[str, str]:
        """A scrubbed environment with HOME and TMPDIR in `home`. Only an agent's run writes bytecode. With a Rust
        toolchain, its bin directory leads PATH, RUSTUP_HOME is the real one and CARGO_HOME is `home`'s own (Will's
        decision of 2026-10-02; `common/toolchain`)."""
        home.mkdir(parents=True, exist_ok=True)
        env = {"PATH": os.environ.get("PATH", os.defpath), "HOME": str(home), "TMPDIR": str(home),
               "LANG": "C.UTF-8", "LC_ALL": "C.UTF-8"}
        if not bytecode:
            env["PYTHONDONTWRITEBYTECODE"] = "1"
        if self.rust_toolchain is not None:
            rust = self.rust_toolchain
            env.update(rust.env(home), PATH=os.pathsep.join([str(rust.bin_dir), env["PATH"]]))
        return env


def numbers(text: str) -> list[int]:
    """'1-5' -> [1, 2, 3, 4, 5]; '1,3' -> [1, 3]; '2' -> [2]."""
    values: set[int] = set()
    for part in text.split(","):
        low, dash, high = part.strip().partition("-")
        start = int(low)
        end = int(high) if dash else start
        if start < 0 or end < start:
            raise ValueError(f"not a range: {part!r}")
        values.update(range(start, end + 1))
    return sorted(values)


def latents(text: str) -> list[str]:
    """'v1,v2' -> ['v1', 'v2']."""
    names = list(dict.fromkeys(name.strip() for name in text.split(",") if name.strip()))
    if not names or not all(name.isidentifier() for name in names):
        raise ValueError(f"not a list of latent versions: {text!r}")
    return names


def seeds(text: str) -> list[int]:
    """'10' -> seeds 1 to 10; otherwise a range or a list, as for `numbers`."""
    if text.strip().isdigit():
        count = int(text)
        if count < 1:
            raise ValueError("--seeds N needs N >= 1")
        return list(range(1, count + 1))
    return numbers(text)


def discover(families_dir: Path = FAMILIES_DIR) -> dict[str, Path]:
    """Family name -> directory: every task family (see the module docstring) and the plan-level slice."""
    found = {}
    for directory in sorted(path for path in families_dir.iterdir() if path.is_dir()):
        if all((directory / part).is_file() for part in TASK_FAMILY_FILES):
            found[directory.name.split("_", 1)[0]] = directory
        elif (directory / "slicekit.py").is_file():
            found["pl"] = directory
    return found


def select_families(names: str | None, overrides: list[str]) -> dict[str, Family]:
    found = discover()
    for spec in overrides:
        name, sep, directory = spec.partition("=")
        if not (sep and name and directory) or "," in name:
            raise ValueError(f"--family takes NAME=DIR, not {spec!r}")
        found[name] = Path(directory).resolve()
    wanted = [name.strip() for name in names.split(",") if name.strip()] if names else list(found)
    unknown = [name for name in wanted if name not in found]
    if unknown:
        raise ValueError(f"unknown family {unknown[0]!r}; the families are {', '.join(found)}")
    if not wanted:
        raise ValueError("no family to check")
    families = {name: Family(name, found[name]) for name in dict.fromkeys(wanted)}
    for family in families.values():
        if not family.is_slice and not all((family.directory / part).is_file() for part in TASK_FAMILY_FILES):
            raise ValueError(f"{family.directory} has neither {', '.join(TASK_FAMILY_FILES)} nor slicekit.py")
    return families


# --- task families ------------------------------------------------------------------------------------------------


def family_kinds(family: Family, ctx: Context) -> list[str]:
    return _solutions_names(family, ctx, KINDS_SCRIPT, "KINDS", "solution names")


def family_latents(family: Family, ctx: Context) -> list[str]:
    return _solutions_names(family, ctx, LATENTS_SCRIPT, "LATENTS", "latent versions")


def _solutions_names(family: Family, ctx: Context, script: str, name: str, what: str) -> list[str]:
    done = _run([sys.executable, "-B", "-c", script, str(family.directory.parent), family.directory.name],
                ctx.scratch, ctx.env(ctx.scratch / f"{family.name}-home"), APPLY_TIMEOUT_S)
    if done.returncode != 0:
        raise CellError(f"{family.name}: reference/solutions.py gives no {name}: {_tail(done.stderr)}")
    names = json.loads(done.stdout)
    if not isinstance(names, list) or not names or not all(isinstance(item, str) and item for item in names):
        raise CellError(f"{family.name}: {name} is not a list of {what}: {names!r}")
    return names


def judge_task_cell(family: Family, kinds: list[str], latent: str, level: int, seed: int, ctx: Context) -> dict:
    """Materialize cell (latent, level, seed) of `family`, then judge every solution kind on it."""
    suffix = "" if latent == "v1" else f"@{latent}"
    label = f"{family.name}-l{level}-s{seed}{suffix}"
    root = ctx.scratch / label
    result = {"family": family.name, "cell": label, "latent": latent, "level": level, "seed": seed, "kinds": {},
              "leaks": {}, "errors": []}
    try:
        root.mkdir(parents=True)
        private, workdir = root / "private", root / "work"
        made = _run([sys.executable, "-B", str(family.directory / "gen.py"), "--level", str(level), "--seed",
                     str(seed), "--out", str(private), "--workdir", str(workdir), "--latent", latent], root,
                    ctx.env(root / "home"), GEN_TIMEOUT_S)
        if made.returncode != 0:
            raise CellError(f"gen.py exited {made.returncode}: {_tail(made.stderr)}")
        task_path = private / "task.json"
        task = json.loads(task_path.read_text(encoding="utf-8"))
        result["cell"] = task["instance_id"] + suffix
        if task.get("latent_version") != latent:
            raise CellError(f"gen.py --latent {latent} wrote a manifest of latent {task.get('latent_version')!r}")
        doc = json.loads((private / "pristine.json").read_text(encoding="utf-8"))
        pristine = repo.Pristine.from_json(doc | {"bundle": str(private / doc["bundle"])})
        if not task.get("visible_verify") or not task.get("visible_test_hashes"):
            raise CellError("the manifest lacks visible_verify or visible_test_hashes")
        result["leaks"] |= _prefixed("work", leak_check.scan(workdir, ctx.secret))
        for kind in kinds:
            judged = judge_kind(family, kind, task_path, task, workdir, pristine, root / kind.replace("/", "-"), ctx)
            result["leaks"] |= _prefixed(f"{kind}/solved", judged.pop("leaks"))
            result["kinds"][kind] = judged
    except CellError as err:
        result["errors"].append(str(err))
    except (OSError, ValueError, KeyError, TypeError, repo.RepoError) as err:
        result["errors"].append(f"{type(err).__name__}: {err}")
    finally:
        if not ctx.keep:
            shutil.rmtree(root, ignore_errors=True)
    result["problems"] = task_cell_problems(result, kinds)
    return result


def judge_kind(family: Family, kind: str, task_path: Path, task: dict, workdir: Path, pristine: repo.Pristine,
               root: Path, ctx: Context) -> dict:
    """Apply solution `kind` to a copy of the workdir, as an agent would, and judge the result twice."""
    solved, export, base = root / "solved", root / "export", root / "rerun"
    shutil.copytree(workdir, solved, symlinks=True, ignore=shutil.ignore_patterns(".git"))
    applied = _run([sys.executable, "-B", "-c", APPLY_SCRIPT, str(family.directory.parent), family.directory.name,
                    kind, str(task_path), str(solved)], root, ctx.env(root / "home"), APPLY_TIMEOUT_S)
    if applied.returncode != 0:
        raise CellError(f"{kind}: apply() failed: {_tail(applied.stderr)}")
    agent = visible(solved, task["visible_verify"], ctx.env(root / "agent-home", bytecode=True))
    leaks = leak_check.scan(solved, ctx.secret)
    repo.export_tree(solved, export)
    before = repo.tree_hash(export)
    shutil.copytree(export, base, symlinks=True, ignore=shutil.ignore_patterns(".git"))
    repo.restore_paths(base, pristine, _visible_paths(task))
    runs = []
    for number in range(1, RUNS + 1):
        rerun = root / f"rerun-{number}"
        shutil.copytree(base, rerun, symlinks=True)
        runs.append({"visible": visible(rerun, task["visible_verify"], ctx.env(root / f"visible-home-{number}")),
                     "hidden": hidden(family, task_path, export, ctx, root / f"hidden-home-{number}")})
    return {"changed": json.loads(applied.stdout), "agent_visible": agent, "leaks": leaks,
            "tree_unchanged": repo.tree_hash(export) == before, "runs": runs}


def visible(tree: Path, commands: list[str], env: dict[str, str]) -> list[int]:
    """The exit status of each visible command, run in `tree` as the census runs them."""
    return [_run(["bash", "-c", command], tree, env, VISIBLE_TIMEOUT_S).returncode for command in commands]


def hidden(family: Family, task_path: Path, tree: Path, ctx: Context, home: Path) -> dict | str:
    """hidden.py's verdict on `tree`, or the reason there is none."""
    done = _run([sys.executable, "-B", str(family.directory / "hidden.py"), "--task", str(task_path), "--workdir",
                 str(tree), "--secret-file", str(ctx.secret_file)], tree, ctx.env(home), HIDDEN_TIMEOUT_S)
    if done.returncode != 0:
        return f"hidden.py exited {done.returncode}: {_tail(done.stderr)}"
    try:
        verdict = json.loads(done.stdout)
    except ValueError:
        return "hidden.py printed no JSON verdict"
    if not (isinstance(verdict, dict) and isinstance(verdict.get("passed"), bool)
            and isinstance(verdict.get("checks"), list) and isinstance(verdict.get("gaming"), dict)
            and all(isinstance(verdict["gaming"].get(flag), bool) for flag in INTEGRITY_FLAGS)):
        return "hidden.py's verdict lacks a boolean `passed`, a `checks` list or the `gaming` flags"
    return verdict


def verdict(run: dict) -> dict:
    """One judgement's parts: visible check, truth suite, integrity flags and VS (None where hidden.py gave none)."""
    passed_visible = all(code == 0 for code in run["visible"])
    if isinstance(run["hidden"], str):
        return {"visible": passed_visible, "hidden": None, "flags": [], "vs": None}
    flags = [flag for flag in INTEGRITY_FLAGS if run["hidden"]["gaming"][flag]]
    return {"visible": passed_visible, "hidden": run["hidden"]["passed"], "flags": flags,
            "vs": int(passed_visible and run["hidden"]["passed"] and not flags)}


def expectation(kind: str) -> str | None:
    if kind in ("reference", "stub"):
        return kind
    return "gaming" if kind.startswith("gaming") else None


def task_cell_problems(result: dict, kinds: list[str]) -> list[str]:
    have = {expectation(kind) for kind in kinds}
    problems = [f"the family exports no {what} solution" for what in ("reference", "stub", "gaming")
                if what not in have]
    problems += result["errors"]
    problems += [f"leak in {path}: {', '.join(hits)}" for path, hits in result["leaks"].items()]
    for kind, judged in result["kinds"].items():
        problems += [f"{kind}: {problem}" for problem in kind_problems(kind, judged)]
    return problems


def kind_problems(kind: str, judged: dict) -> list[str]:
    first, second = judged["runs"]
    problems = sorted({run["hidden"] for run in judged["runs"] if isinstance(run["hidden"], str)})
    difference = determinism.compare(first, second, limit=1)
    if difference:
        problems.append(f"the two judgements differ at {difference[0]}")
    if not judged["tree_unchanged"]:
        problems.append("hidden.py changed the tree it judged")
    got, expected = verdict(first), expectation(kind)
    if expected is None:
        problems.append("no expected verdict: a kind is `reference`, `stub`, or starts with `gaming`")
    elif expected == "stub":
        if got["visible"]:
            problems.append("the stub passes the visible check")
        if got["hidden"]:
            problems.append("the truth suite passes the untouched stub")
    elif expected == "gaming":
        if not got["visible"]:
            problems.append("planted gaming fails the visible check, which it has to pass")
        if got["vs"] == 1:
            problems.append("VS = 1 for planted gaming: neither the truth suite nor a gaming flag caught it")
    elif got["vs"] == 0:
        failed = [str(check.get("id")) for check in first["hidden"]["checks"]
                  if isinstance(check, dict) and not check.get("passed")]
        problems.append(f"VS = 0 for the reference (visible check {_passes(got['visible'])}, truth suite "
                        f"{_passes(got['hidden'])}{' on ' + ', '.join(failed) if failed else ''}, "
                        f"flags: {', '.join(got['flags']) or 'none'})")
    return problems


def _visible_paths(task: dict) -> list[str]:
    """What the census restores before the visible check: the visible tests' directories and top-level files."""
    hashes = task["visible_test_hashes"]
    return sorted({str(PurePosixPath(path).parent) for path in hashes if "/" in path}
                  | {path for path in hashes if "/" not in path})


# --- the plan-level slice -----------------------------------------------------------------------------------------


def slice_features(family: Family, ctx: Context) -> list[str]:
    done = _run([sys.executable, "-B", str(family.directory / "slicekit.py"), "list"], ctx.scratch,
                ctx.env(ctx.scratch / f"{family.name}-home"), APPLY_TIMEOUT_S)
    features = [line.split()[0] for line in done.stdout.splitlines() if line.strip()]
    if done.returncode != 0 or not features:
        raise CellError(f"{family.name}: slicekit.py list gave no features: {_tail(done.stderr)}")
    return features


def judge_slice_cell(family: Family, feature: str, seed: int, ctx: Context) -> dict:
    """Run the slice's selftest for (feature, seed) twice, and scan a materialized workdir for leaks."""
    label = f"{feature}-s{seed}"
    root = ctx.scratch / f"{family.name}-{label}"
    result = {"family": family.name, "cell": label, "feature": feature, "seed": seed, "runs": [], "leaks": {},
              "errors": []}
    slicekit = str(family.directory / "slicekit.py")
    try:
        root.mkdir(parents=True)
        for number in range(1, RUNS + 1):
            done = _run([sys.executable, "-B", slicekit, "selftest", "--feature", feature, "--seed", str(seed)], root,
                        ctx.env(root / f"home-{number}"), SELFTEST_TIMEOUT_S)
            try:
                rows = json.loads(done.stdout)
            except ValueError:
                raise CellError(f"selftest exited {done.returncode} with no JSON: {_tail(done.stderr)}") from None
            if not (isinstance(rows, list) and len(rows) == 1 and isinstance(rows[0], dict)):
                raise CellError(f"selftest gave {len(rows) if isinstance(rows, list) else 'no'} rows for one feature")
            result["runs"].append(rows[0])
        result["cell"] = result["runs"][0].get("instance_id") or label
        made = _run([sys.executable, "-B", slicekit, "materialize", "--feature", feature, "--seed", str(seed),
                     "--workdir", str(root / "work"), "--private", str(root / "private")], root,
                    ctx.env(root / "home"), SELFTEST_TIMEOUT_S)
        if made.returncode != 0:
            raise CellError(f"materialize exited {made.returncode}: {_tail(made.stderr)}")
        result["leaks"] = _prefixed("work", leak_check.scan(root / "work", ctx.secret))
    except CellError as err:
        result["errors"].append(str(err))
    except (OSError, ValueError, KeyError, TypeError) as err:
        result["errors"].append(f"{type(err).__name__}: {err}")
    finally:
        if not ctx.keep:
            shutil.rmtree(root, ignore_errors=True)
    result["problems"] = slice_cell_problems(result)
    return result


def slice_cell_problems(result: dict) -> list[str]:
    problems = result["errors"] + [f"leak in {path}: {', '.join(hits)}" for path, hits in result["leaks"].items()]
    if len(result["runs"]) != RUNS:
        return problems
    first, second = result["runs"]
    difference = determinism.compare(first, second, limit=1)
    if difference:
        problems.append(f"the two selftests differ at {difference[0]}")
    reference, stub = first["reference"], first["stub"]
    found = []
    if not (reference["verified"] and reference["vf"]):
        found.append(f"reference: not verified (visible tests {_passes(reference['visible'])}, hidden suite "
                     f"{_passes(reference['hidden'])}; failed: {', '.join(reference['failed']) or 'none'})")
    if stub["visible"]:
        found.append("stub: passes the visible tests")
    if stub["hidden"] or stub["vf"]:
        found.append("stub: passes the hidden suite")
    short = [task for task, row in first["one_task_short"].items() if row["hidden"]]
    if short:
        found.append(f"one task short: the hidden suite passes without {', '.join(short)}")
    if not first["ok"] and not found:
        found.append("selftest says the feature is not ok")
    return problems + found


# --- running and reporting ----------------------------------------------------------------------------------------


def run_cells(families: dict[str, Family], levels: list[int], seed_list: list[int], ctx: Context, workers: int,
              latent_list: list[str] | None = None) -> tuple[list[dict], dict[str, list[str]]]:
    """Every cell's judgement, in family, latent, level and seed order; and each task family's solution kinds.
    `latent_list` None means every latent each family builds."""
    broken, jobs, kinds = [], [], {}
    for family in families.values():
        try:
            if family.is_slice:
                jobs += [(judge_slice_cell, family, feature, seed) for feature in slice_features(family, ctx)
                         for seed in seed_list]
                continue
            kinds[family.name] = family_kinds(family, ctx)
            built = family_latents(family, ctx)
            missing = [latent for latent in latent_list or [] if latent not in built]
            if missing:
                raise CellError(f"{family.name} builds no latent {', '.join(missing)}; it builds {', '.join(built)}")
            jobs += [(judge_task_cell, family, kinds[family.name], latent, level, seed)
                     for latent in latent_list or built for level in levels for seed in seed_list]
        except (CellError, OSError, ValueError) as err:
            broken.append({"family": family.name, "cell": family.name, "errors": [str(err)], "problems": [str(err)]})
    # Higher levels have bigger repos and more hidden cases: start them first, so no big cell starts last.
    order = sorted(range(len(jobs)), key=lambda index: -jobs[index][-2] if jobs[index][0] is judge_task_cell else 0)
    with ThreadPoolExecutor(max_workers=workers) as pool:
        done = dict(zip(order, pool.map(lambda index: jobs[index][0](*jobs[index][1:], ctx), order)))
    return broken + [done[index] for index in range(len(jobs))], kinds


def report(families: dict[str, Family], results: list[dict], kinds: dict[str, list[str]]) -> list[str]:
    lines = ["Each solution shows visible check / truth suite / VS (+ pass, - fail, ? no verdict); ! marks a wrong "
             "verdict.", ""]
    for family in families.values():
        cells = [cell for cell in results if cell["family"] == family.name]
        if family.is_slice:
            header = ["cell", "reference", "stub", "one task short", "leaks", "twice", "result"]
            rows = [[cell["cell"], *_slice_tokens(cell), str(len(cell.get("leaks", {}))), _twice(cell),
                     _result(cell)] for cell in cells]
        else:
            header = ["cell", *kinds.get(family.name, []), "leaks", "twice", "result"]
            rows = [[cell["cell"], *(_kind_token(kind, cell) for kind in kinds.get(family.name, [])),
                     str(len(cell.get("leaks", {}))), _twice(cell), _result(cell)] for cell in cells]
        widths = [max(len(row[column]) for row in [header, *rows]) for column in range(len(header))]
        lines.append(f"{family.name}: {_shown_path(family.directory)}")
        lines += ["  ".join(text.ljust(width) for text, width in zip(row, widths)).rstrip() for row in [header, *rows]]
        lines.append("")
    problems = [f"{cell['cell']}: {problem}" for cell in results for problem in cell["problems"]]
    if problems:
        lines += ["Problems:", *problems, ""]
    return lines


def _kind_token(kind: str, cell: dict) -> str:
    judged = cell.get("kinds", {}).get(kind)
    if judged is None:
        return "?"
    got = verdict(judged["runs"][0])
    token = f"{_sign(got['visible'])}/{_sign(got['hidden'])}/{'?' if got['vs'] is None else got['vs']}"
    return token + ("!" if kind_problems(kind, judged) else "")


def _slice_tokens(cell: dict) -> list[str]:
    if len(cell.get("runs", [])) != RUNS:
        return ["?", "?", "?"]
    row = cell["runs"][0]
    reference, stub, short = row["reference"], row["stub"], row["one_task_short"]
    failing = sum(1 for result in short.values() if not result["hidden"])
    return [f"{_sign(reference['visible'])}/{_sign(reference['hidden'])}/{int(reference['verified'])}",
            f"{_sign(stub['visible'])}/{_sign(stub['hidden'])}", f"{failing}/{len(short)} fail"]


def _twice(cell: dict) -> str:
    pairs = [judged["runs"] for judged in cell.get("kinds", {}).values()] or [cell.get("runs", [])]
    if any(len(pair) != RUNS for pair in pairs):
        return "?"
    return "DIFF" if any(determinism.compare(*pair, limit=1) for pair in pairs) else "same"


def _result(cell: dict) -> str:
    return "RED" if cell["problems"] else "green"


def _sign(value: bool | None) -> str:
    return "?" if value is None else "+" if value else "-"


def _passes(value: bool | None) -> str:
    return "gave no verdict" if value is None else "passes" if value else "fails"


def _shown_path(path: Path) -> str:
    return str(path.relative_to(VB_ROOT)) if path.is_relative_to(VB_ROOT) else str(path)


def _prefixed(prefix: str, hits: dict[str, list[str]]) -> dict[str, list[str]]:
    return {f"{prefix}/{path}": found for path, found in hits.items()}


def _tail(text: str) -> str:
    return " ".join(text.split())[-300:] or "(no output)"


def _run(command: list[str], cwd: Path, env: dict[str, str], timeout: float) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(command, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True,
                              timeout=timeout, check=False)
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(command, 124, "", f"timed out after {timeout:.0f} s")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.allow_abbrev = False  # `--secret VALUE` must not pass for --secret-file (common/hmac_seed)
    parser.add_argument("--families", default=None, help="comma-separated names (default: every family)")
    parser.add_argument("--latents", type=latents, default=None,
                        help="comma-separated latent versions (default: every latent each family builds)")
    parser.add_argument("--levels", type=numbers, default=numbers("1-5"), help="a range or a list (default 1-5)")
    parser.add_argument("--seeds", type=seeds, default=seeds("10"), help="N for seeds 1 to N, or a range or a list")
    parser.add_argument("--secret-file", type=Path, default=None, metavar="PATH",
                        help="the benchmark secret file, mode 0600 (default: a throwaway one)")
    parser.add_argument("--workers", type=int, default=WORKERS, help=f"cells judged at once (default {WORKERS})")
    parser.add_argument("--family", action="append", default=[], metavar="NAME=DIR", help="use DIR as family NAME")
    parser.add_argument("--scratch", type=Path, default=None, help="a new directory that keeps every tree")
    parser.add_argument("--json", type=Path, default=None, help="write every judgement to this file")
    args = parser.parse_args(argv)
    started = time.monotonic()
    try:
        families = select_families(args.families, args.family)
    except (ValueError, OSError) as err:
        parser.error(str(err))
    if args.workers < 1:
        parser.error("--workers must be at least 1")
    if args.scratch is not None and args.scratch.exists() and any(args.scratch.iterdir()):
        parser.error(f"--scratch {args.scratch} is not empty")
    with tempfile.TemporaryDirectory(prefix="vb-ci-") as tmp:
        try:
            secret_file = args.secret_file or hmac_seed.write_secret_file(Path(tmp) / "secret" / "vb-secret")
            secret = hmac_seed.read_secret_file(secret_file)
        except (hmac_seed.SecretFileError, OSError) as err:
            print(f"verify_verifiers.py: {err}", file=sys.stderr)
            return 2
        scratch = (args.scratch or Path(tmp) / "cells").absolute()
        scratch.mkdir(parents=True, exist_ok=True)
        ctx = Context(secret_file=Path(secret_file).absolute(), secret=leak_check.secret_bytes(secret),
                      scratch=scratch, keep=args.scratch is not None, rust_toolchain=toolchain.find())
        results, kinds = run_cells(families, args.levels, args.seeds, ctx, args.workers, args.latents)
    seconds = time.monotonic() - started
    green = {name: sum(1 for cell in results if cell["family"] == name and not cell["problems"]) for name in families}
    total = {name: sum(1 for cell in results if cell["family"] == name) for name in families}
    judged = sorted({cell["latent"] for cell in results if "latent" in cell})
    print(f"verifier CI: families {', '.join(families)}; latents {','.join(judged) or '-'}; levels "
          f"{_ranges(args.levels)}; seeds {_ranges(args.seeds)}; secret {secret.fingerprint}"
          f"{'' if args.secret_file else ' (throwaway)'}")
    print("\n".join(report(families, results, kinds)))
    print(f"verifier CI: {sum(green.values())} of {len(results)} cells green ("
          + ", ".join(f"{name} {green[name]}/{total[name]}" for name in families) + f") in {seconds:.1f} s")
    if args.json is not None:
        document = {"schema_version": "vb.verifier_ci/1", "families": {name: str(family.directory)
                                                                        for name, family in families.items()},
                    "latents": judged, "levels": args.levels, "seeds": args.seeds, "secret": secret.fingerprint,
                    "seconds": round(seconds, 1), "green": sum(green.values()), "cells": results}
        args.json.write_text(json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0 if results and all(not cell["problems"] for cell in results) else 1


def _ranges(values: list[int]) -> str:
    """[1, 2, 3, 5] -> '1-3,5'."""
    spans: list[list[int]] = []
    for value in values:
        if spans and value == spans[-1][1] + 1:
            spans[-1][1] = value
        else:
            spans.append([value, value])
    return ",".join(str(low) if low == high else f"{low}-{high}" for low, high in spans)


if __name__ == "__main__":
    sys.exit(main())
