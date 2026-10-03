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
   the visible test hashes against the rendered files, and that no canary is left anywhere in the agent's tree. With
   `latent`, it asked the generator for that latent version (`--latent`, S08 §4.6's `convention_flip`), and it checks
   that the manifest names it;
3. checks the pristine base without re-making it: the bundle lies in DIR, and the workdir's files and HEAD are the
   recorded tree and commit. The census restores test files from that bundle, and `hidden.py` reads `pristine.json`
   next to the manifest.

The agent gets the chosen spec's text as its task message. The spec files stay private, so an agent given one
variant cannot read another.

**The vague variant** (S08 §4.5, S07.3; task 3329), for F1 and F4 only (the other families' own task gives them
one): `gen.py` renders only `spec.precise.md`. The first `materialize(..., spec_variant="vague")` of an instance
builds `spec.vague.md` itself, caches it in the manifest (`spec.vague`, written back to `task.json`) so every
later call for the same instance reads the same file, and:

1. reads `spec.precise.md` into a `[[task]]`-shaped dict good enough for `specops.degrade`/`speclint.score_task`
   (`_task_dict`): the structured manifest fields (`files_in_scope`, `visible_verify`) plus a prose extraction
   from the markdown's own ``## `` sections (`_spec_prose`) -- the same two-family extraction the manipulation
   check (task 3234) needs, so it is written once, here;
2. applies D-v1's full `VAGUE` composition (`specops.degrade`) at the instance's own seed, and checks
   recoverability: a `recoverability` entry whose evidence is *only* `spec.precise.md#ACn` references, for an AC
   id the vague spec no longer states, has nothing left to recover from and fails materialization (`_recoverability_violations`);
3. renders the degraded dict back to markdown (`_render_vague_markdown`) and writes it to `spec.vague.md` beside
   `spec.precise.md`, with its own sha256 and D-v1's manifest (`operator`, `levels`, `operator_version`).

API:
    Materialized(instance_id, family_dir, workdir, private_dir, manifest, manifest_path, pristine, spec_variant,
                 spec_text)
    materialize(*, family_dir, instance_id, workdir, private_dir, spec_variant="precise", timeout_s=300.0,
                latent=None)
    MaterializeError
    VAGUE_FAMILIES = ("F1", "F4")
    task_dict(manifest: dict, precise_text: str) -> dict           # the module docstring's [[task]] projection
    recoverability_violations(manifest: dict, vague: dict) -> list[str]
    render_vague_markdown(task: dict) -> str
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import layout
import validate  # schema/validate.py, on sys.path through layout
from common import astcheck, canary, knobs, repo

if str(layout.VB_ROOT) not in sys.path:  # specops/, for the vague variant (D-v1)
    sys.path.insert(0, str(layout.VB_ROOT))
from specops import OPERATOR, VERSION, VAGUE, degrade  # noqa: E402

TASK_DIR = ".vb"  # gen.py's --out directory, under the private directory
MANIFEST = "task.json"
PRISTINE = "pristine.json"
VAGUE_SPEC_FILE = "spec.vague.md"
VAGUE_FAMILIES = ("F1", "F4")  # the families 3329 covers; the rest get a vague variant from their own task
_SECTION_RE = re.compile(r"(?m)^## (.+)$")
_AC_PREFIX_RE = re.compile(r"^(?:\*\*)?(AC\d+)[.:](?:\*\*)?\s*")
_CONTEXT_BULLET_RE = re.compile(r"^`([^`]+)`:\s*(.*)$")
_GOAL_RE = re.compile(r"\*\*Goal\.\*\*\s+(.*?)(?:\n\n|\Z)", re.DOTALL)
_TITLE_RE = re.compile(r"(?m)^#\s+(.+)$")
_SPEC_REF_RE = re.compile(r"^spec\.precise\.md#(\S+)$")


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
                spec_variant: str = "precise", timeout_s: float = 300.0, latent: str | None = None) -> Materialized:
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
               "--out", str(task_dir), "--workdir", str(workdir), *(["--latent", latent] if latent else [])]
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
    if latent and manifest["latent_version"] != latent:
        raise MaterializeError(f"{family} gen.py rendered latent {manifest['latent_version']} for {latent}")

    if spec_variant == "vague" and "vague" not in manifest["spec"]:
        manifest = _add_vague_spec(manifest, manifest_path, task_dir)

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


def _add_vague_spec(manifest: dict, manifest_path: Path, task_dir: Path) -> dict:
    """Build `spec.vague.md` from the precise spec, cache it in `manifest["spec"]["vague"]`, and rewrite
    `manifest_path` so every later call for the same instance reads the cached file (the module docstring)."""
    instance_id, family = manifest["instance_id"], manifest["family"]
    if family not in VAGUE_FAMILIES:
        raise MaterializeError(f"{instance_id}: {family} has no vague variant yet (3329 covers {VAGUE_FAMILIES})")
    precise = manifest["spec"]["precise"]
    precise_path = _private_file(precise["path"], task_dir, "spec")
    precise_text = precise_path.read_text(encoding="utf-8")
    if hashlib.sha256(precise_text.encode("utf-8")).hexdigest() != precise["sha256"].removeprefix("sha256:"):
        raise MaterializeError(f"{instance_id}: {precise['path']} does not match its manifest sha256")
    task = task_dict(manifest, precise_text)
    vague, _degrade_manifest = degrade(task, VAGUE, manifest["seed"])
    violations = recoverability_violations(manifest, vague)
    if violations:
        raise MaterializeError(f"{instance_id}: the vague spec drops an unrecoverable requirement: {violations[0]}")
    vague_text = render_vague_markdown(vague)
    vague_path = task_dir / VAGUE_SPEC_FILE
    if vague_path.exists():
        raise MaterializeError(f"{instance_id}: refusing to overwrite an existing {vague_path}")
    vague_path.write_text(vague_text, encoding="utf-8")
    os.chmod(vague_path, 0o600)
    manifest["spec"]["vague"] = {
        "path": VAGUE_SPEC_FILE, "sha256": "sha256:" + hashlib.sha256(vague_text.encode("utf-8")).hexdigest(),
        "operator": OPERATOR, "levels": list(VAGUE), "operator_version": VERSION,
    }
    errors = validate.validate("task", manifest)
    if errors:
        raise MaterializeError(f"{instance_id}: the manifest with its vague spec added is invalid: {errors[0]}")
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return manifest


def task_dict(manifest: dict, precise_text: str) -> dict:
    """A `[[task]]`-shaped dict for `specops.degrade`/`speclint.score_task`: `files_in_scope` and
    `visible_verify` straight from the manifest, plus a prose extraction from `precise_text`'s own
    ``## `` sections (`_spec_prose`), common to F1's and F4's rendered spec.precise.md."""
    prose = _spec_prose(precise_text)
    return {
        "id": manifest["instance_id"], "title": prose["title"], "goal": prose["goal"],
        "description": prose["description"], "acceptance": prose["acceptance"], "non_goals": prose["non_goals"],
        "files": list(manifest["files_in_scope"]),
        "context": {"read_files": prose["read_files"]} if prose["read_files"] else {},
        "verify": [{"phase": "test", "command": command} for command in manifest["visible_verify"]],
    }


def _spec_prose(text: str) -> dict:
    """title, goal, description, acceptance, non_goals and read_files, from a rendered spec's own markdown."""
    matches = list(_SECTION_RE.finditer(text))
    sections = {match[1].strip(): text[match.end():(matches[index + 1].start() if index + 1 < len(matches)
               else len(text))].strip() for index, match in enumerate(matches)}
    title_match = _TITLE_RE.search(text)
    title = title_match[1].strip() if title_match else ""
    first_section = _SECTION_RE.search(text)
    lead = text[title_match.end():first_section.start()] if title_match and first_section else ""
    goal_match = _GOAL_RE.search(lead)
    if goal_match:
        goal, description = " ".join(goal_match[1].split()), " ".join(lead[goal_match.end():].split())
    else:
        goal, description = " ".join(sections.get("Goal", "").split()), ""
    acceptance = [_AC_PREFIX_RE.sub(r"\1: ", bullet) for bullet in _bullets(sections.get("Acceptance criteria", ""))]
    non_goals = _bullets(sections.get("Non-goals") or sections.get("Constraints") or "")
    read_files = _context_entries(sections.get("Context", ""))
    return {"title": title, "goal": goal, "description": description, "acceptance": acceptance,
           "non_goals": non_goals, "read_files": read_files}


def _bullets(block: str) -> list[str]:
    """Each top-level "- " bullet of `block`, joining a bullet's wrapped continuation lines."""
    items: list[str] = []
    current: list[str] | None = None
    for line in block.splitlines():
        if line.startswith("- "):
            if current is not None:
                items.append(" ".join(current))
            current = [line[2:].strip()]
        elif current is not None and line.strip():
            current.append(line.strip())
    if current is not None:
        items.append(" ".join(current))
    return items


def _context_entries(block: str) -> list[dict]:
    entries = []
    for bullet in _bullets(block):
        match = _CONTEXT_BULLET_RE.match(bullet)
        entries.append({"path": match[1], "why": match[2]} if match else {"path": bullet, "why": ""})
    return entries


def render_vague_markdown(task: dict) -> str:
    """`task` (a `specops.degrade`-ed `task_dict`) rendered back to a plain markdown spec: a section is left out
    entirely once degrade empties or removes its field, exactly as the agent is meant to see it."""
    parts = [f"# {task.get('title', '')}".rstrip(), "", (task.get("goal") or "").strip()]
    if task.get("description"):
        parts += ["", task["description"]]
    if task.get("acceptance"):
        parts += ["", "## Acceptance criteria", "", *(f"- {item}" for item in task["acceptance"])]
    if task.get("verify"):
        parts += ["", "## Verify", "", *(f"    {step['command']}" for step in task["verify"])]
    read_files = (task.get("context") or {}).get("read_files")
    if read_files:
        parts += ["", "## Context", "", *(f"- `{e['path']}`: {e['why']}".rstrip(": ") for e in read_files)]
    if task.get("non_goals"):
        parts += ["", "## Non-goals", "", *(f"- {item}" for item in task["non_goals"])]
    parts += ["", "## Scope", "", "Files in scope: " + ", ".join(f"`{path}`" for path in task.get("files", [])) + "."]
    return "\n".join(parts).strip() + "\n"


def recoverability_violations(manifest: dict, vague: dict) -> list[str]:
    """Every `manifest["recoverability"]` entry whose evidence is *only* `spec.precise.md#ACn` references, for an
    AC id `vague`'s own `acceptance` no longer states: nothing is left in the vague spec or the repo to recover
    it from (the module docstring's recoverability check)."""
    stated = {match[1] for item in (vague.get("acceptance") or []) if (match := re.match(r"(AC\d+)", item))}
    problems = []
    for entry in manifest.get("recoverability", []):
        spec_refs = [item for item in entry["evidence"] if _SPEC_REF_RE.match(item)]
        other = [item for item in entry["evidence"] if not _SPEC_REF_RE.match(item)]
        if not spec_refs or other:
            continue
        referenced = {_SPEC_REF_RE.match(item)[1] for item in spec_refs}
        if not referenced & stated:
            problems.append(f"{entry['req']!r}: only evidence is {spec_refs}, none still stated in the vague spec")
    return problems


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
