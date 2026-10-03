#!/usr/bin/env python3
"""The pre-registration lock: the analysis frozen before LOG1 spends anything (S09 SC1, §5, E4; task 3341).

    lock.py build [--spec S09.md] [--out experiments/prereg.lock.json] [--allow-dirty]
    lock.py --check FILE [--spec S09.md]    # recompute every hash: exit 0 when clean, 1 on drift, 2 when it can't run
    lock.py status [FILE] [--spec S09.md]   # whether a locked experiment may start under it

**What the lock records** (`vb.prereg_lock/1`, S09 §5), built deterministically from S09, `analysis/`, the streams
and the price snapshot of the repository the lock is written into:
- `spec_sha256`, the sha256 of S09's text, and `prereg_id`, "s09-v<S09's version>-<the hash's first 8 hex digits>";
- `analysis_commit` and `harness_sha`: HEAD, which must be clean under the benchmark tree and the price snapshots
  (`--allow-dirty` for a rehearsal only), and `locked_at`, HEAD's commit time, so a rebuild at one commit gives the
  same bytes;
- `price_snapshot_id`, the snapshot S09 §5 names, found by its `id` under `config/prices/`;
- `suite_hash`: every family's files under `families/` (caches excluded), keyed by family directory;
- `alpha_fw`, `multiplicity`, `primaries` and `exploratory`: the analysis plan, copied from S09 §5's lock block.
  `multiplicity` must equal `holm.MULTIPLICITY`, the graph the analysis runs (D28);
- `hashes`: the sha256 of every file the analysis rests on: `analysis/` and `audit/` (`cs.py` builds on
  `audit/estimate.py`), `streams/`, `requirements-analysis.lock` (decision 3336), the simulation report under
  `reports/simulation/` (3339) and the price snapshot.

**Check** (`check`): every field derived from a file is recomputed and compared. A changed, added or removed file
under the hashed paths, another spec text, another plan or graph, another suite or an unknown analysis commit is
drift; S09 §5 says a change after the lock is a logged deviation (`vb.deviation/1`), never a silent edit.

**Use** (`require`): an experiment that runs only under the lock (LOG1 and every live experiment, `campaign.py`'s
`requires_lock`) may start only when the lock file exists, is committed (tracked by git and unchanged since HEAD:
`committed` gives the commit time that first added it) and checks clean. Taking the lock, its commit, is task 3345
and needs Will's approval; this module only builds and checks it. `blind.py` refuses to unblind before `require`
passes.

API:
    LOCK_SCHEMA, DEFAULT_LOCK, DEFAULT_SPEC, HASHED
    build(spec=DEFAULT_SPEC, out=DEFAULT_LOCK, *, allow_dirty=False) -> dict
    write(lock, out) -> Path                          # canonical JSON text, one trailing newline
    check(path=DEFAULT_LOCK, spec=DEFAULT_SPEC) -> list[str]     # drift, empty when clean
    committed(path=DEFAULT_LOCK) -> str | None        # the commit time that first added it, if committed and unchanged
    require(path=DEFAULT_LOCK, spec=DEFAULT_SPEC) -> dict          # the lock, or LockError with the reason
    LockError
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ANALYSIS_DIR = Path(__file__).resolve().parent
if str(ANALYSIS_DIR) not in sys.path:
    sys.path.insert(0, str(ANALYSIS_DIR))
import holm  # noqa: E402

VB_ROOT = ANALYSIS_DIR.parent
LOCK_SCHEMA = "vb.prereg_lock/1"
DEFAULT_LOCK = VB_ROOT / "experiments" / "prereg.lock.json"
DEFAULT_SPEC = VB_ROOT.parents[1] / "tmp" / "cybernetic-harness" / "specs" / "S09-experiments.md"
HASHED = ("analysis", "audit", "streams", "requirements-analysis.lock", "reports/simulation")  # under the bench root
SKIPPED = ("__pycache__", ".pytest_cache", ".venv")  # caches, never code
PLAN_FIELDS = ("alpha_fw", "multiplicity", "primaries", "exploratory")
GIT_TIMEOUT_S = 60


class LockError(RuntimeError):
    """The lock cannot be built, read or used as it is."""


def build(spec: Path = DEFAULT_SPEC, out: Path = DEFAULT_LOCK, *, allow_dirty: bool = False) -> dict:
    """The lock for the repository `out` sits in (module docstring); `out` is where it will be written."""
    root, repo, prices = _sources(Path(out))
    text = _spec_text(spec)
    plan = _plan(text)
    if plan["multiplicity"] != holm.MULTIPLICITY:
        raise LockError("S09 §5's multiplicity block differs from holm.MULTIPLICITY, the graph the analysis runs")
    head = _git(repo, "rev-parse", "HEAD")
    itself = Path(out).absolute().resolve().relative_to(repo.resolve()).as_posix()  # the lock never dirties its build
    dirty = _git(repo, "status", "--porcelain", "--", str(root), str(prices), f":(exclude){itself}")
    if dirty and not allow_dirty:
        raise LockError("the benchmark tree or the price snapshots have uncommitted changes; the lock pins committed "
                        "code, so commit them first (--allow-dirty only for a rehearsal)")
    snapshot = _snapshot(prices, plan["price_snapshot_id"])
    spec_sha = hashlib.sha256(text.encode("utf-8")).hexdigest()
    lock = {"schema_version": LOCK_SCHEMA, "prereg_id": f"s09-v{_version(text)}-{spec_sha[:8]}",
            "spec_sha256": spec_sha, "analysis_commit": head, "price_snapshot_id": plan["price_snapshot_id"],
            "suite_hash": suite_hash(root), "harness_sha": head,
            "locked_at": _git(repo, "show", "-s", "--format=%cI", head)}
    lock.update({field: plan[field] for field in PLAN_FIELDS})
    lock["hashes"] = file_hashes(root, repo, snapshot)
    return lock


def write(lock: dict, out: Path = DEFAULT_LOCK) -> Path:
    """The lock as canonical JSON (sorted keys, two-space indent), so equal locks are equal files."""
    out = Path(out)
    out.write_text(json.dumps(lock, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    return out


def check(path: Path = DEFAULT_LOCK, spec: Path = DEFAULT_SPEC) -> list[str]:
    """Every way the lock at `path` no longer matches the files it pins (module docstring); empty when clean."""
    lock = _read(Path(path))
    root, repo, prices = _sources(Path(path))
    drift = []
    try:
        text = _spec_text(spec)
    except LockError as err:
        return [str(err)]
    if hashlib.sha256(text.encode("utf-8")).hexdigest() != lock.get("spec_sha256"):
        drift.append(f"spec_sha256: {spec} is not the text the lock pinned")
    plan = _plan(text)
    drift += [f"{field}: S09 §5's {field} differs from the lock's" for field in PLAN_FIELDS
              if plan[field] != lock.get(field)]
    if lock.get("multiplicity") != holm.MULTIPLICITY:
        drift.append("multiplicity: the lock's graph differs from holm.MULTIPLICITY, the one the analysis runs")
    if suite_hash(root) != lock.get("suite_hash"):
        drift.append("suite_hash: the families changed since the lock")
    try:
        snapshot = _snapshot(prices, lock.get("price_snapshot_id"))
    except LockError as err:
        drift.append(str(err))
        snapshot = None
    pinned = lock.get("hashes") or {}
    current = file_hashes(root, repo, snapshot)
    for name in sorted(pinned.keys() | current.keys()):
        if name not in current:
            drift.append(f"{name}: removed since the lock")
        elif name not in pinned:
            drift.append(f"{name}: added since the lock")
        elif pinned[name] != current[name]:
            drift.append(f"{name}: changed since the lock")
    commit = lock.get("analysis_commit") or ""
    if not re.fullmatch(r"[0-9a-f]{40}", commit) or _run(repo, "cat-file", "-e", f"{commit}^{{commit}}") != 0:
        drift.append(f"analysis_commit {commit!r} is not a commit of {repo}")
    return drift


def committed(path: Path = DEFAULT_LOCK) -> str | None:
    """The commit time that first added the lock, when git tracks it and it is unchanged since HEAD; else None."""
    path = Path(path).absolute()
    if not path.is_file():
        return None
    try:
        repo = Path(_git(path.parent, "rev-parse", "--show-toplevel"))
    except LockError:
        return None
    relative = path.resolve().relative_to(repo.resolve()).as_posix()
    if _run(repo, "ls-files", "--error-unmatch", "--", relative) != 0 or _run(repo, "diff", "--quiet", "HEAD", "--",
                                                                               relative) != 0:
        return None
    added = _git(repo, "log", "--diff-filter=A", "--format=%cI", "--", relative).splitlines()
    return added[-1] if added else None


def require(path: Path = DEFAULT_LOCK, spec: Path = DEFAULT_SPEC) -> dict:
    """The lock at `path` when a locked experiment may start under it; LockError with the reason otherwise."""
    path = Path(path)
    if not path.is_file():
        raise LockError(f"no pre-registration lock at {path}: build it (analysis/lock.py build) and commit it "
                        "(task 3345) before a locked experiment starts")
    if committed(path) is None:
        raise LockError(f"the lock at {path} is not committed: it is untracked, or changed since HEAD")
    drift = check(path, spec)
    if drift:
        raise LockError(f"the lock at {path} fails --check ({len(drift)} drift(s)): " + "; ".join(drift[:5]))
    return _read(path)


def suite_hash(root: Path) -> str:
    """sha256 of the canonical JSON {family directory: {relative path: sha256}} over `root`/families."""
    families = Path(root) / "families"
    digests = {directory.name: _tree(directory) for directory in sorted(families.iterdir()) if directory.is_dir()}
    return hashlib.sha256(json.dumps(digests, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def file_hashes(root: Path, repo: Path, snapshot: Path | None) -> dict[str, str]:
    """Relative path -> sha256 of every file under HASHED (relative to the bench root) and of the price snapshot
    (relative to the repository)."""
    hashes: dict[str, str] = {}
    for name in HASHED:
        start = Path(root) / name
        if start.is_file():
            hashes[name] = _sha256(start)
        elif start.is_dir():
            hashes.update({f"{name}/{relpath}": digest for relpath, digest in _tree(start).items()})
    if snapshot is not None:
        hashes[snapshot.resolve().relative_to(Path(repo).resolve()).as_posix()] = _sha256(snapshot)
    return dict(sorted(hashes.items()))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", nargs="?", choices=("build", "status"),
                        help="build the lock, or say whether it may be used")
    parser.add_argument("file", nargs="?", type=Path, help="status: the lock (default: experiments/prereg.lock.json)")
    parser.add_argument("--check", type=Path, metavar="FILE", help="recompute every hash of the lock FILE")
    parser.add_argument("--spec", type=Path, default=DEFAULT_SPEC, help="S09 (default: the untracked spec in tmp/)")
    parser.add_argument("--out", type=Path, default=DEFAULT_LOCK, help="build: where to write the lock")
    parser.add_argument("--allow-dirty", action="store_true", help="build from a dirty tree, for a rehearsal only")
    args = parser.parse_args(argv)
    try:
        if args.check is not None:
            drift = check(args.check, args.spec)
            for item in drift:
                print(item)
            print(f"lock --check: {args.check}: {'clean' if not drift else f'{len(drift)} drift(s)'}", file=sys.stderr)
            return 1 if drift else 0
        if args.command == "build":
            print(write(build(args.spec, args.out, allow_dirty=args.allow_dirty), args.out))
            return 0
        if args.command == "status":
            lock = require(args.file or DEFAULT_LOCK, args.spec)
            print(f"{lock['prereg_id']}: committed {committed(args.file or DEFAULT_LOCK)}, checks clean")
            return 0
    except LockError as err:
        print(f"lock: {err}", file=sys.stderr)
        return 2
    parser.error("name a command (build or status) or --check FILE")
    return 2


def _sources(lock_path: Path) -> tuple[Path, Path, Path]:
    """(the bench root, its repository, the repository's price snapshots) for a lock at `lock_path`."""
    root = Path(lock_path).absolute().parent.parent
    repo = Path(_git(root, "rev-parse", "--show-toplevel"))
    return root, repo, repo / "config" / "prices"


def _spec_text(spec: Path) -> str:
    try:
        return Path(spec).read_text(encoding="utf-8")
    except OSError as err:
        raise LockError(f"S09 cannot be read at {spec}: {err}") from None


def _plan(text: str) -> dict:
    """S09 §5's `vb.prereg_lock/1` block: the analysis plan the lock copies."""
    for block in re.findall(r"```json\n(.*?)\n```", text, re.S):
        try:
            doc = json.loads(block)
        except ValueError:
            continue
        if isinstance(doc, dict) and doc.get("schema_version") == LOCK_SCHEMA:
            missing = [field for field in (*PLAN_FIELDS, "price_snapshot_id") if field not in doc]
            if missing:
                raise LockError(f"S09 §5's lock block lacks {', '.join(missing)}")
            return doc
    raise LockError("S09 has no §5 lock block (a ```json block with schema_version vb.prereg_lock/1)")


def _version(text: str) -> str:
    found = re.search(r"^Status: \w+ v([0-9][0-9.]*)", text, re.M)
    if not found:
        raise LockError("S09's status line names no version (Status: draft vX.Y)")
    return found.group(1)


def _snapshot(prices: Path, snapshot_id: str | None) -> Path:
    for path in sorted(Path(prices).glob("*.toml")):
        try:
            if tomllib.loads(path.read_text(encoding="utf-8")).get("id") == snapshot_id:
                return path
        except (OSError, tomllib.TOMLDecodeError):
            continue
    raise LockError(f"no price snapshot with id {snapshot_id!r} under {prices}")


def _read(path: Path) -> dict:
    try:
        lock = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        raise LockError(f"the lock at {path} cannot be read: {err}") from None
    if not isinstance(lock, dict) or lock.get("schema_version") != LOCK_SCHEMA:
        raise LockError(f"{path} is not a {LOCK_SCHEMA} lock")
    return lock


def _tree(start: Path) -> dict[str, str]:
    return {path.relative_to(start).as_posix(): _sha256(path) for path in sorted(start.rglob("*"))
            if path.is_file() and not path.is_symlink() and not set(path.relative_to(start).parts) & set(SKIPPED)
            and path.suffix != ".pyc"}


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _git(cwd: Path, *args: str) -> str:
    try:
        done = subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True, timeout=GIT_TIMEOUT_S,
                              check=False)
    except (OSError, subprocess.SubprocessError) as err:
        raise LockError(f"git {' '.join(args)} failed: {err}") from None
    if done.returncode != 0:
        raise LockError(f"git {' '.join(args)} in {cwd} failed: {done.stderr.strip()[:300]}")
    return done.stdout.strip()


def _run(cwd: Path, *args: str) -> int:
    try:
        return subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, timeout=GIT_TIMEOUT_S,
                              check=False).returncode
    except (OSError, subprocess.SubprocessError):
        return 1


if __name__ == "__main__":
    sys.exit(main())
