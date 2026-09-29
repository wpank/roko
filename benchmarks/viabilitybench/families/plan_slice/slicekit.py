#!/usr/bin/env python3
"""The plan-level slice (PL, RQ3): whole features that need multi-task plans, each with a held-out whole-feature suite.

Paper §5.3 and Appendix A.7; work item gap-89f393 builds the fixtures and gap-1cd676 runs them. Stdlib only, Python
3.11 or newer. It reuses the common library (`families/common/`) for task repos, canaries, surface renames and gaming
detectors, and `schema/validate.py` for the manifest schema.

A feature lives in `features/<dir>/`:

    feature.toml     vb.feature_source/1: id, package, visible tests and verify command, suite provenance, rename pools
    description.md   the feature description, the only input either arm gets; requirements are "- **R1.** ..."
    plan.toml        vb.plan_skeleton/1: the reference solution's task decomposition, without code. No arm gets it.
    base/            the repo the agent starts from; its visible tests fail until the feature exists (the stub)
    reference/       the reference solution: the files it adds or replaces, each owned by one skeleton task
    hidden/          the whole-feature suite (unittest), copied only into the census's own export

Every file carries the release canary on a marker line. Rendering strips the markers, so no task repo holds a canary,
and the private manifest that records it is written outside the workdir.

Lifecycle, as for the task families:
1. `materialize` renders base/ into a fresh workdir with the seed's surface renames (the package name), makes it a
   task repo with a pristine bundle, and writes the description and the manifest (`vb.feature/1`) to a private
   directory outside the workdir.
2. An arm works in the workdir from the description alone.
3. `census` exports the workdir as it is on disk, records test edits, skipped visible tests and canaries, restores
   the visible tests from the pristine base, runs them, adds the hidden suite, and runs it. Its `vf` is S09 §4.9's
   census-side verified feature: the hidden suite passes and nothing tampered with the visible tests. The driver
   adds "the arm declared the feature done", and a canary hit makes the run `leak_suspected`. `verified` is stricter
   (the visible tests pass too, and no canary): it is what verifier CI demands of a reference.

The manifest also carries `run_record_task`, the `task` object of the instance's `vb.run_record/1` rows.

`selftest` is the slice's verifier CI: per feature, the reference passes both suites cleanly, the stub (the untouched
base) fails both, and the reference with any one skeleton task left undone fails the hidden suite.

CLI:
    slicekit.py list | check | mark     # mark: add the release canary's marker line to every feature file
    slicekit.py materialize --feature ID --seed N --workdir DIR --private DIR
    slicekit.py census --manifest PRIVATE/feature.json --workdir DIR
    slicekit.py selftest [--feature ID ...] [--seed N] [--no-partials]
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import itertools
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
VB_ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE.parent))
from common import VERSION as COMMON_VERSION  # noqa: E402
from common import astcheck, canary, hmac_seed, mutate, repo  # noqa: E402

sys.path.insert(0, str(VB_ROOT / "schema"))
import validate as vb_validate  # noqa: E402

FAMILY = "PL"
FEATURES_DIR = HERE / "features"
GENERATOR_VERSION = "pl-1.0.0"
VERIFIER_VERSION = f"{GENERATOR_VERSION}+{COMMON_VERSION}"
SOURCE_SCHEMA = "vb.feature_source/1"
PLAN_SCHEMA = "vb.plan_skeleton/1"
MANIFEST_SCHEMA_FILE = HERE / "feature.schema.json"
HIDDEN_DIR = "tests_vb_hidden"
TASKS_MIN, TASKS_MAX, WIDTH_MIN, FEATURES_MIN, FEATURES_MAX = 4, 8, 3, 6, 10
RUN_TIMEOUT_S = 120.0
REQ_RE = re.compile(r"^- \*\*(R\d+)\.\*\*", re.MULTILINE)
TAG_RE = re.compile(r"^\s*(R\d+(?:\s*,\s*R\d+)*)\s*:")


class SliceError(ValueError):
    """A feature's files are malformed, or a workdir or private directory is unsafe."""


@dataclass(frozen=True)
class Task:
    id: str
    title: str
    files: tuple[str, ...]
    depends_on: tuple[str, ...]
    covers: tuple[str, ...]


@dataclass(frozen=True)
class Feature:
    root: Path
    source: dict
    tasks: tuple[Task, ...]
    requirements: tuple[str, ...]

    @property
    def id(self) -> str:
        return self.source["id"]

    def reference_files(self) -> list[str]:
        base = self.root / "reference"
        return sorted(path.relative_to(base).as_posix() for path in base.rglob("*") if path.is_file())

    def hidden_tags(self) -> dict[str, list[str]]:
        """Hidden test method -> the requirement ids its docstring starts with, read statically."""
        tags = {}
        for path in sorted((self.root / "hidden").rglob("*.py")):
            for node in ast.walk(ast.parse(path.read_text(encoding="utf-8"))):
                if isinstance(node, ast.FunctionDef) and node.name.startswith("test"):
                    match = TAG_RE.match(ast.get_docstring(node) or "")
                    tags[f"{path.stem}.{node.name}"] = [tag.strip() for tag in match[1].split(",")] if match else []
        return tags


def load_feature(root: Path) -> Feature:
    root = Path(root)
    try:
        source = tomllib.loads((root / "feature.toml").read_text(encoding="utf-8"))
        plan = tomllib.loads((root / "plan.toml").read_text(encoding="utf-8"))
        description = (root / "description.md").read_text(encoding="utf-8")
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise SliceError(f"{root.name}: {err}") from None
    if source.get("schema_version") != SOURCE_SCHEMA or plan.get("schema_version") != PLAN_SCHEMA:
        raise SliceError(f"{root.name}: feature.toml must be {SOURCE_SCHEMA} and plan.toml {PLAN_SCHEMA}")
    if plan.get("feature") != source.get("id"):
        raise SliceError(f"{root.name}: plan.toml is for {plan.get('feature')!r}, not {source.get('id')!r}")
    tasks = tuple(Task(id=raw["id"], title=raw["title"], files=tuple(raw["files"]),
                       depends_on=tuple(raw.get("depends_on", ())), covers=tuple(raw["covers"]))
                  for raw in plan.get("task", []))
    return Feature(root=root, source=source, tasks=tasks, requirements=tuple(REQ_RE.findall(description)))


def load_features(root: Path = FEATURES_DIR) -> list[Feature]:
    return [load_feature(path) for path in sorted(Path(root).iterdir()) if (path / "feature.toml").is_file()]


def find_feature(feature_id: str, root: Path = FEATURES_DIR) -> Feature:
    for feature in load_features(root):
        if feature.id == feature_id:
            return feature
    raise SliceError(f"no feature {feature_id!r} under {root}")


def width(tasks: tuple[Task, ...]) -> int:
    """The most tasks that can run at once: the largest set of tasks none of which depends on another."""
    reach = {task.id: set(task.depends_on) for task in tasks}
    for _ in tasks:  # transitive closure; at most 8 tasks
        for deps in reach.values():
            deps.update(*(reach.get(dep, set()) for dep in list(deps)))
    ids = [task.id for task in tasks]
    for size in range(len(ids), 0, -1):
        for group in itertools.combinations(ids, size):
            if all(a not in reach[b] and b not in reach[a] for a, b in itertools.combinations(group, 2)):
                return size
    return 0


def shape_errors(feature: Feature) -> list[str]:
    """Everything that keeps `feature` from being a valid PL fixture; empty when it is one."""
    errors, name = [], feature.root.name
    tasks, ids = feature.tasks, [task.id for task in feature.tasks]
    if not TASKS_MIN <= len(tasks) <= TASKS_MAX:
        errors.append(f"{name}: {len(tasks)} tasks; a feature needs {TASKS_MIN}–{TASKS_MAX}")
    if len(set(ids)) != len(ids):
        errors.append(f"{name}: duplicate task ids")
    for task in tasks:
        unknown = [dep for dep in task.depends_on if dep not in ids]
        if unknown or task.id in task.depends_on:
            errors.append(f"{name}: {task.id} depends on unknown or itself: {unknown or [task.id]}")
        if not task.files or not task.covers:
            errors.append(f"{name}: {task.id} needs files and covered requirements")
    if not errors and _has_cycle(tasks):
        errors.append(f"{name}: the task dependencies form a cycle")
    if not errors and width(tasks) < WIDTH_MIN:
        errors.append(f"{name}: at most {width(tasks)} tasks can run in parallel; the slice needs {WIDTH_MIN}")
    owned = [path for task in tasks for path in task.files]
    if len(set(owned)) != len(owned):
        errors.append(f"{name}: two tasks own the same file, so parallel tasks would conflict")
    modules = {path for path in owned if path.endswith(".py")}
    if len(modules) < 2:
        errors.append(f"{name}: the feature spans {len(modules)} module(s); it needs two or more")
    if set(owned) != set(feature.reference_files()):
        errors.append(f"{name}: the tasks' files {sorted(set(owned))} differ from the reference's "
                      f"{feature.reference_files()}")
    reqs = set(feature.requirements)
    if len(reqs) != len(feature.requirements) or not reqs:
        errors.append(f"{name}: description.md needs distinct requirement ids")
    covered = {req for task in tasks for req in task.covers}
    if covered != reqs:
        errors.append(f"{name}: skeleton covers {sorted(covered)}, description states {sorted(reqs)}")
    tags = feature.hidden_tags()
    untagged = [test for test, found in tags.items() if not found or not set(found) <= reqs]
    if untagged or not tags:
        errors.append(f"{name}: hidden tests without valid requirement tags: {untagged or 'no tests'}")
    tested = {req for found in tags.values() for req in found}
    if reqs - tested:
        errors.append(f"{name}: no hidden test checks {sorted(reqs - tested)}")
    unmarked = [path.relative_to(feature.root).as_posix() for path in _template_files(feature)
                if canary.find(path.read_text(encoding="utf-8")) != [canary.RELEASE_CANARY]]
    if unmarked:
        errors.append(f"{name}: files without the release canary marker: {unmarked}")
    for key in ("id", "slug", "title", "package", "visible_tests", "visible_verify", "suite", "rename", "run_record"):
        if key not in feature.source:
            errors.append(f"{name}: feature.toml lacks {key!r}")
    if "run_record" in feature.source:
        errors += [f"{name}: [run_record]: {error}" for error in run_record_errors(run_record_task(feature, 1))]
    return errors


def run_record_task(feature: Feature, seed: int) -> dict:
    """The `task` object of this instance's vb.run_record/1 rows, from feature.toml's [run_record] (S09 §4.9)."""
    return {"family": FAMILY, "instance_id": instance_id(feature, seed), **feature.source["run_record"]}


def run_record_errors(task: dict) -> list[str]:
    """How `task` breaks the run-record schema's `task` object; empty when a PL row can carry it."""
    return vb_validate.schema_errors(task, vb_validate.load_schema("run-record")["properties"]["task"], "$.task")


def rename_mapping(feature: Feature, seed: int) -> dict[str, str]:
    """The seed's surface renames, drawn from the public surface stream (they are visible to the agent anyway)."""
    return mutate.choose(hmac_seed.surface_stream(FAMILY, instance_id(feature, seed)), feature.source["rename"])


def instance_id(feature: Feature, seed: int) -> str:
    if isinstance(seed, bool) or not isinstance(seed, int) or seed < 0:
        raise SliceError(f"seed must be a non-negative int, not {seed!r}")
    return f"{feature.id}-{seed:04d}"


def render(source: Path, dest: Path, mapping: dict[str, str], only: list[str] | None = None) -> list[str]:
    """Copy `source`'s files (or `only` of them) into `dest` without canary lines, then rename; the new paths."""
    files = sorted(path.relative_to(source).as_posix() for path in source.rglob("*") if path.is_file())
    for relpath in files if only is None else only:
        target = dest / relpath
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(canary.strip((source / relpath).read_text(encoding="utf-8")), encoding="utf-8")
    mutate.rename_tree(dest, mapping)
    return [mutate.rename_text(relpath, mapping) for relpath in (files if only is None else only)]


def materialize(feature: Feature, seed: int, workdir: Path, private: Path) -> dict:
    """Render the task repo into `workdir` and write description.md and feature.json into `private`."""
    workdir, private = Path(workdir).absolute(), Path(private).absolute()
    if workdir == private or workdir in private.parents or private in workdir.parents:
        raise SliceError("the private directory and the workdir must not contain each other")
    if os.path.lexists(workdir):
        raise SliceError(f"refusing to overwrite {workdir}")
    mapping = rename_mapping(feature, seed)
    with tempfile.TemporaryDirectory(prefix="vb-pl-render-") as tmp:
        render(feature.root / "base", Path(tmp) / "base", mapping)
        shutil.copytree(Path(tmp) / "base", workdir)
    private.mkdir(mode=0o700, parents=True, exist_ok=True)
    description = mutate.rename_text(canary.strip((feature.root / "description.md").read_text(encoding="utf-8")),
                                     mapping)
    (private / "description.md").write_text(description, encoding="utf-8")
    pristine = repo.init_task_repo(workdir, private / "pristine.bundle")
    visible = feature.source["visible_tests"]
    manifest = {
        "schema_version": "vb.feature/1", "instance_id": instance_id(feature, seed), "family": FAMILY,
        "feature": feature.id, "feature_dir": feature.root.name, "slug": feature.source["slug"],
        "generator_version": GENERATOR_VERSION, "seed": seed, "rename": mapping,
        "package": mapping.get(feature.source["package"], feature.source["package"]),
        "description": {"path": "description.md", "sha256": hashlib.sha256(description.encode()).hexdigest()},
        "visible_verify": feature.source["visible_verify"], "visible_tests": visible,
        "visible_test_hashes": astcheck.file_hashes(workdir, visible),
        "requirements": list(feature.requirements),
        "recoverability": [{"req": req, "evidence": [f"description.md#{req}"]} for req in feature.requirements],
        "plan_skeleton": {"tasks": len(feature.tasks), "width": width(feature.tasks),
                          "sha256": hashlib.sha256((feature.root / "plan.toml").read_bytes()).hexdigest()},
        "truth_suite": dict(feature.source["suite"]), "run_record_task": run_record_task(feature, seed),
        "pristine": pristine.as_json(),
        "canary": canary.RELEASE_CANARY, "workdir": str(workdir),
    }
    errors = manifest_errors(manifest)
    if errors:
        raise SliceError(f"{feature.id}: invalid manifest: {errors}")
    (private / "feature.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    return manifest


def manifest_errors(manifest: dict) -> list[str]:
    schema = json.loads(MANIFEST_SCHEMA_FILE.read_text(encoding="utf-8"))
    vb_validate.check_schema(schema)
    return vb_validate.schema_errors(manifest, schema)


def apply_reference(feature: Feature, workdir: Path, mapping: dict[str, str], skip: tuple[str, ...] = ()) -> None:
    """Write the reference solution into `workdir` as an agent would, leaving the tasks in `skip` undone."""
    skipped = {path for task in feature.tasks if task.id in skip for path in task.files}
    wanted = [path for path in feature.reference_files() if path not in skipped]
    with tempfile.TemporaryDirectory(prefix="vb-pl-reference-") as tmp:
        for relpath in render(feature.root / "reference", Path(tmp), mapping, only=wanted):
            target = Path(workdir) / relpath
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(Path(tmp) / relpath, target)


def census(manifest: dict, workdir: Path) -> dict:
    """Score the workdir as it is on disk: the visible tests on a clean copy, then the hidden suite."""
    feature = load_feature(FEATURES_DIR / manifest["feature_dir"])
    pristine = repo.Pristine.from_json(manifest["pristine"])
    visible_tests = manifest["visible_tests"]
    with tempfile.TemporaryDirectory(prefix="vb-pl-census-") as tmp:
        export = Path(tmp) / "export"
        repo.export_tree(Path(workdir), export)
        canary_hits = canary.find_in_tree(export)
        findings = astcheck.test_edits(export, manifest["visible_test_hashes"])
        findings += astcheck.skipped_tests(astcheck.python_sources(export, [p for p in visible_tests
                                                                            if (export / p).exists()]))
        repo.restore_paths(export, pristine, visible_tests)
        visible = run_suite(export, visible_tests[0], Path(tmp) / "visible.json")
        shutil.rmtree(export / HIDDEN_DIR, ignore_errors=True)
        render(feature.root / "hidden", export / HIDDEN_DIR, manifest["rename"])
        hidden = run_suite(export, HIDDEN_DIR, Path(tmp) / "hidden.json")
    gaming = astcheck.gaming_summary(findings)
    return {
        "instance_id": manifest["instance_id"], "passed": hidden["passed"],
        "checks": [{"id": row["id"].removeprefix(HIDDEN_DIR + "."), "reqs": row["reqs"],
                    "passed": row["status"] == "pass", "detail": row["detail"][-500:]}
                   for row in hidden["tests"]],
        "visible": {"passed": visible["passed"], "ran": visible["ran"], "commands": manifest["visible_verify"]},
        "gaming": gaming, "findings": [f"{f.check} {f.path}:{f.line} {f.detail}" for f in findings],
        "canary_hits": canary_hits, "leak_suspected": bool(canary_hits),
        "vf": hidden["passed"] and not gaming["test_edit"] and not gaming["tests_skipped"],
        "verified": hidden["passed"] and visible["passed"] and not any(gaming.values()) and not canary_hits,
        "truth_suite": manifest["truth_suite"], "verifier_version": VERIFIER_VERSION,
    }


def run_suite(root: Path, start: str, out: Path) -> dict:
    """Run one suite in `root` in a child process; passed means it ran tests and every one passed."""
    env = {key: value for key, value in os.environ.items() if key in ("PATH", "HOME", "LANG", "TMPDIR", "SYSTEMROOT")}
    try:
        subprocess.run([sys.executable, "-I", "-B", str(HERE / "runner.py"), "--start", start, "--json", str(out)],
                       cwd=root, env=env, capture_output=True, timeout=RUN_TIMEOUT_S, check=False)
        result = json.loads(out.read_text(encoding="utf-8"))
    except subprocess.TimeoutExpired:
        return {"passed": False, "ran": 0, "tests": [{"id": start, "status": "error", "reqs": [],
                                                      "detail": f"timed out after {RUN_TIMEOUT_S:.0f} s"}]}
    except (OSError, ValueError) as err:
        return {"passed": False, "ran": 0, "tests": [{"id": start, "status": "error", "reqs": [],
                                                      "detail": f"the runner failed: {err}"}]}
    passed = result["ran"] > 0 and all(row["status"] == "pass" for row in result["tests"])
    return {"passed": passed, **result}


def selftest(feature: Feature, seed: int = 1, *, partials: bool = True) -> dict:
    """Verifier CI for one feature: reference passes, stub fails, and every one-task-short reference fails."""
    with tempfile.TemporaryDirectory(prefix="vb-pl-selftest-") as tmp:
        tmp = Path(tmp)
        manifest = materialize(feature, seed, tmp / "work", tmp / "private")
        shutil.copytree(tmp / "work", tmp / "base", symlinks=True)
        stub = census(manifest, tmp / "work")
        apply_reference(feature, tmp / "work", manifest["rename"])
        reference = census(manifest, tmp / "work")
        short = {}
        for task in feature.tasks if partials else ():
            shutil.copytree(tmp / "base", tmp / task.id, symlinks=True)
            apply_reference(feature, tmp / task.id, manifest["rename"], skip=(task.id,))
            verdict = census(manifest, tmp / task.id)
            short[task.id] = {"hidden": verdict["passed"], "visible": verdict["visible"]["passed"]}
    ok = (reference["verified"] and reference["vf"] and not stub["passed"] and not stub["visible"]["passed"]
          and not stub["vf"] and not any(row["hidden"] for row in short.values()))
    return {"feature": feature.id, "seed": seed, "instance_id": manifest["instance_id"], "ok": ok,
            "reference": {"hidden": reference["passed"], "visible": reference["visible"]["passed"],
                          "verified": reference["verified"], "vf": reference["vf"], "checks": len(reference["checks"]),
                          "failed": [c["id"] for c in reference["checks"] if not c["passed"]]},
            "stub": {"hidden": stub["passed"], "visible": stub["visible"]["passed"], "vf": stub["vf"]},
            "one_task_short": short}


def _has_cycle(tasks: tuple[Task, ...]) -> bool:
    deps = {task.id: set(task.depends_on) for task in tasks}
    done: set[str] = set()
    while len(done) < len(deps):
        ready = [task for task, needs in deps.items() if task not in done and needs <= done]
        if not ready:
            return True
        done.update(ready)
    return False


def _template_files(feature: Feature) -> list[Path]:
    files = [feature.root / name for name in ("feature.toml", "plan.toml", "description.md")]
    for part in ("base", "reference", "hidden"):
        files += sorted(path for path in (feature.root / part).rglob("*") if path.is_file())
    return files


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list", help="the features and their shapes")
    sub.add_parser("check", help="static checks of every feature (shape, coverage, canaries)")
    sub.add_parser("mark", help="add the release canary's marker line to every feature file that lacks it")
    make = sub.add_parser("materialize", help="render one instance")
    make.add_argument("--feature", required=True)
    make.add_argument("--seed", type=int, required=True)
    make.add_argument("--workdir", type=Path, required=True)
    make.add_argument("--private", type=Path, required=True)
    score = sub.add_parser("census", help="score a workdir; prints the verdict as JSON")
    score.add_argument("--manifest", type=Path, required=True)
    score.add_argument("--workdir", type=Path, required=True)
    test = sub.add_parser("selftest", help="verifier CI: reference passes, stub and one-task-short references fail")
    test.add_argument("--feature", action="append")
    test.add_argument("--seed", type=int, default=1)
    test.add_argument("--no-partials", action="store_true")
    args = parser.parse_args(argv)
    features = load_features()
    if args.command == "list":
        for feature in features:
            files = {path for task in feature.tasks for path in task.files}
            print(f"{feature.id}  {feature.source['slug']:<24} tasks={len(feature.tasks)} width={width(feature.tasks)}"
                  f" files={len(files)} reqs={len(feature.requirements)} hidden={len(feature.hidden_tags())}")
        return 0
    if args.command == "check":
        errors = [error for feature in features for error in shape_errors(feature)]
        if not FEATURES_MIN <= len(features) <= FEATURES_MAX:
            errors.append(f"{len(features)} features; the slice needs {FEATURES_MIN}–{FEATURES_MAX}")
        print("\n".join(errors) or f"ok: {len(features)} features")
        return 1 if errors else 0
    if args.command == "mark":
        changed = [path for feature in features for path in _template_files(feature) if canary.mark_file(path)]
        print(f"marked {len(changed)} file(s)")
        return 0
    if args.command == "materialize":
        manifest = materialize(find_feature(args.feature), args.seed, args.workdir, args.private)
        print(json.dumps({"instance_id": manifest["instance_id"], "workdir": manifest["workdir"]}))
        return 0
    if args.command == "census":
        verdict = census(json.loads(args.manifest.read_text(encoding="utf-8")), args.workdir)
        print(json.dumps(verdict, indent=1, sort_keys=True))
        return 0 if verdict["verified"] else 1
    chosen = [f for f in features if not args.feature or f.id in args.feature]
    rows = [selftest(feature, args.seed, partials=not args.no_partials) for feature in chosen]
    print(json.dumps(rows, indent=1))
    return 0 if rows and all(row["ok"] for row in rows) else 1


if __name__ == "__main__":
    sys.exit(main())
