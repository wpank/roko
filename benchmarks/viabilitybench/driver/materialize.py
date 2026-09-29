"""Materialize one task instance for one run: render it with its manifest out of the agent's reach, and check it.

`materialize(family_dir=…, instance_id=…, workdir=…, private_dir=…)`:

1. runs the family's generator, `gen.py --level ℓ --seed s --out <private_dir>/.vb --workdir <workdir>`, in a
   scrubbed environment. This is the families' contract (F1's and F4's gen.py), which replaces S08 §5.2's
   `<workdir>/.vb/`: the `--out` directory, DIR, gets the `vb.task/1` manifest `task.json` with its canary, the spec
   variants (the manifest's `spec` paths are relative to DIR) and the pristine base, `pristine.bundle` with its
   record `pristine.json` ({bundle, commit, tree}; the bundle path is relative to DIR). The workdir becomes a git repo
   whose one commit is the pristine base (`repo.init_task_repo`), holding no manifest, spec or canary. DIR keeps
   S08's name, `.vb`, but lives under `private_dir`, in the run's results and outside every workdir: an agent that
   read the manifest would trip the canary (gap-2790c5);
2. checks the manifest against `schema/task.schema.json` and the requested instance id, the chosen spec's sha256,
   the visible test hashes against the rendered files, and that no canary is left anywhere in the agent's tree;
3. checks the pristine base without re-making it: the bundle lies in DIR, and the workdir's files and HEAD are the
   recorded tree and commit. The census restores test files from that bundle, and `hidden.py` reads `pristine.json`
   next to the manifest.

The agent gets the chosen spec's text as its task message. The spec files stay private, so an agent given one
variant cannot read another.

API:
    Materialized(instance_id, family_dir, workdir, private_dir, manifest, manifest_path, pristine, spec_variant,
                 spec_text)
    materialize(*, family_dir, instance_id, workdir, private_dir, spec_variant="precise", timeout_s=300.0)
    MaterializeError
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import layout
import validate  # schema/validate.py, on sys.path through layout
from common import astcheck, canary, knobs, repo

TASK_DIR = ".vb"  # gen.py's --out directory, under the private directory
MANIFEST = "task.json"
PRISTINE = "pristine.json"


class MaterializeError(RuntimeError):
    """The generator failed, or its output breaks a rule the driver checks before any agent runs."""


@dataclass(frozen=True)
class Materialized:
    instance_id: str
    family_dir: Path
    workdir: Path
    private_dir: Path
    manifest: dict
    manifest_path: Path
    pristine: repo.Pristine
    spec_variant: str
    spec_text: str


def materialize(*, family_dir: Path, instance_id: str, workdir: Path, private_dir: Path,
                spec_variant: str = "precise", timeout_s: float = 300.0) -> Materialized:
    family, level, seed = knobs.parse_instance_id(instance_id)
    family_dir = Path(family_dir).resolve()
    workdir, private_dir = Path(workdir).absolute(), Path(private_dir).absolute()
    task_dir = private_dir / TASK_DIR
    if os.path.lexists(workdir):
        raise MaterializeError(f"the workdir {workdir} already exists; every (task, seed) gets a fresh one")
    if layout.within(private_dir, workdir) or layout.within(workdir, private_dir):
        raise MaterializeError("the private directory and the workdir must not contain each other")
    private_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    workdir.parent.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, str(family_dir / "gen.py"), "--level", str(level), "--seed", str(seed),
               "--out", str(task_dir), "--workdir", str(workdir)]
    env = {"PATH": os.environ.get("PATH", os.defpath), "PYTHONDONTWRITEBYTECODE": "1", "LC_ALL": "C.UTF-8",
           "HOME": str(private_dir)}
    try:
        result = subprocess.run(command, cwd=private_dir, env=env, capture_output=True, timeout=timeout_s, check=False)
    except (OSError, subprocess.TimeoutExpired) as err:
        raise MaterializeError(f"{family} gen.py could not run: {err}") from None
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()[-800:]
        raise MaterializeError(f"{family} gen.py exited {result.returncode} for {instance_id}: {detail}")

    manifest_path = task_dir / MANIFEST
    if task_dir.is_symlink() or manifest_path.is_symlink() or not manifest_path.is_file():
        raise MaterializeError(f"{family} gen.py wrote no {MANIFEST} into {task_dir}")
    os.chmod(manifest_path, 0o600)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    errors = validate.validate("task", manifest)
    if errors:
        raise MaterializeError(f"{instance_id}: invalid manifest: {errors[0]}")
    if manifest["instance_id"] != instance_id or manifest["family"] != family or manifest["ladder"] != level:
        raise MaterializeError(f"{family} gen.py rendered {manifest['instance_id']} for {instance_id}")

    spec = manifest["spec"].get(spec_variant)
    if spec is None:
        raise MaterializeError(f"{instance_id} has no {spec_variant!r} spec")
    spec_bytes = _private_file(spec["path"], task_dir, "spec").read_bytes()
    if hashlib.sha256(spec_bytes).hexdigest() != spec["sha256"].removeprefix("sha256:"):
        raise MaterializeError(f"{instance_id}: {spec['path']} does not match its manifest sha256")

    try:
        actual = astcheck.file_hashes(workdir, manifest["visible_test_hashes"])
    except FileNotFoundError as err:
        raise MaterializeError(f"{instance_id}: a visible test file is missing: {err}") from None
    expected = {path: digest.removeprefix("sha256:") for path, digest in manifest["visible_test_hashes"].items()}
    if actual != expected:
        raise MaterializeError(f"{instance_id}: the visible test hashes do not match the rendered files")
    leaks = canary.find_in_tree(workdir)
    if leaks:
        raise MaterializeError(f"{instance_id}: a canary reached the agent's tree: {sorted(leaks)[0]}")

    pristine = _pristine(task_dir, instance_id)
    if repo.tree_hash(workdir) != pristine.tree:
        raise MaterializeError(f"{instance_id}: the workdir's files are not the pristine tree {pristine.tree}")
    if _head(workdir) != pristine.commit:
        raise MaterializeError(f"{instance_id}: the workdir's HEAD is not the pristine commit {pristine.commit}")
    return Materialized(instance_id=instance_id, family_dir=family_dir, workdir=workdir, private_dir=private_dir,
                        manifest=manifest, manifest_path=manifest_path, pristine=pristine, spec_variant=spec_variant,
                        spec_text=spec_bytes.decode("utf-8"))


def _pristine(task_dir: Path, instance_id: str) -> repo.Pristine:
    """The pristine base `pristine.json` records, with its bundle resolved in DIR."""
    try:
        doc = json.loads(_private_file(PRISTINE, task_dir, "pristine record").read_text(encoding="utf-8"))
        return repo.Pristine.from_json(doc | {"bundle": str(_private_file(doc["bundle"], task_dir, "bundle"))})
    except (OSError, ValueError, KeyError, TypeError, repo.RepoError) as err:
        raise MaterializeError(f"{instance_id}: no usable {PRISTINE}: {err}") from None


def _private_file(raw: object, task_dir: Path, what: str) -> Path:
    """The file a path relative to DIR names, which must lie inside DIR."""
    relative = Path(raw) if isinstance(raw, str) else Path()
    path = task_dir / relative
    if relative.is_absolute() or ".." in relative.parts or not relative.parts or not layout.within(path, task_dir) \
            or not path.is_file():
        raise MaterializeError(f"the {what} path {raw!r} names no file inside {task_dir}")
    return path


def _head(workdir: Path) -> str | None:
    """The commit the workdir's HEAD names. No agent has run there yet; outside git config and variables are off."""
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, LC_ALL="C")
    try:
        result = subprocess.run(["git", "--git-dir", str(workdir / ".git"), "rev-parse", "--verify", "-q",
                                 "HEAD^{commit}"], env=env, capture_output=True, timeout=repo.GIT_TIMEOUT_S,
                                check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return result.stdout.decode("ascii", "replace").strip() if result.returncode == 0 else None
