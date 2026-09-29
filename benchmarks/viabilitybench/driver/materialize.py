"""Materialize one task instance for one run: render it, move its manifest out of the agent's reach, snapshot it.

`materialize(family_dir=…, instance_id=…, workdir=…, private_dir=…)`:

1. runs the family's generator, `gen.py --level ℓ --seed s --out <workdir>` (S08 §5.2), in a scrubbed environment;
2. moves `<workdir>/.vb/`, which holds the `vb.task/1` manifest with its canary and the spec variants, into
   `private_dir`. That directory lives under the run's results, outside every workdir: an agent that read the
   manifest would trip the canary (gap-2790c5);
3. checks the manifest against `schema/task.schema.json` and the requested instance id, the chosen spec's sha256,
   the visible test hashes against the rendered files, and that no canary is left anywhere in the agent's tree;
4. makes the workdir a git repo whose one commit is the pristine base, bundled into `private_dir/pristine.bundle`
   (`repo.init_task_repo`), where the census restores test files from.

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
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import layout
import validate  # schema/validate.py, on sys.path through layout
from common import astcheck, canary, knobs, repo

MANIFEST_DIR = ".vb"
MANIFEST = "task.json"


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
    if os.path.lexists(workdir):
        raise MaterializeError(f"the workdir {workdir} already exists; every (task, seed) gets a fresh one")
    if layout.within(private_dir, workdir) or layout.within(workdir, private_dir):
        raise MaterializeError("the private directory and the workdir must not contain each other")
    private_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    workdir.parent.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, str(family_dir / "gen.py"), "--level", str(level), "--seed", str(seed),
               "--out", str(workdir)]
    env = {"PATH": os.environ.get("PATH", os.defpath), "PYTHONDONTWRITEBYTECODE": "1", "LC_ALL": "C.UTF-8",
           "HOME": str(private_dir)}
    try:
        result = subprocess.run(command, cwd=private_dir, env=env, capture_output=True, timeout=timeout_s, check=False)
    except (OSError, subprocess.TimeoutExpired) as err:
        raise MaterializeError(f"{family} gen.py could not run: {err}") from None
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()[-800:]
        raise MaterializeError(f"{family} gen.py exited {result.returncode} for {instance_id}: {detail}")

    rendered = workdir / MANIFEST_DIR
    if not (rendered / MANIFEST).is_file() or rendered.is_symlink():
        raise MaterializeError(f"{family} gen.py wrote no {MANIFEST_DIR}/{MANIFEST} into {workdir}")
    hidden = private_dir / MANIFEST_DIR
    shutil.move(str(rendered), str(hidden))
    manifest_path = hidden / MANIFEST
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
    spec_path = _spec_path(spec["path"], workdir, private_dir)
    spec_bytes = spec_path.read_bytes()
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

    pristine = repo.init_task_repo(workdir, private_dir / "pristine.bundle")
    return Materialized(instance_id=instance_id, family_dir=family_dir, workdir=workdir, private_dir=private_dir,
                        manifest=manifest, manifest_path=manifest_path, pristine=pristine, spec_variant=spec_variant,
                        spec_text=spec_bytes.decode("utf-8"))


def _spec_path(raw: str, workdir: Path, private_dir: Path) -> Path:
    """A spec under `.vb/` moved to the private directory; any other spec path is in the rendered repo."""
    parts = Path(raw).parts
    if Path(raw).is_absolute() or ".." in parts or not parts:
        raise MaterializeError(f"unsafe spec path {raw!r}")
    return (private_dir if parts[0] == MANIFEST_DIR else workdir) / raw
