"""Commit the agent's final tree as c_i, and archive it (S08 §4.2 (5), §4.10).

`commit_final(workdir, dest)` copies the agent's files as they are on disk into `dest`, a new repo the driver owns,
and commits them there with `repo.export_tree`. Nothing runs in the agent's repo, so hooks, an fsmonitor or filters
the agent planted in its `.git` never run in the driver. That commit is c_i, and the census labels it.

`archive_task(final, pristine, archives_dir, stem)` then writes next to the run's records:
- `<stem>.bundle`: a git bundle with c_i (`refs/heads/main`) and the pristine base (`refs/vb/pristine`);
- `<stem>.tar.gz`: the final tree, with fixed metadata so equal trees give equal bytes. It is gzip where S08 §5.1
  says tar.zst, because the standard library has no zstd before Python 3.14;
- `<stem>.diff`: the diff from the pristine base to c_i (binary-safe), whose sha256 goes into the record.

Git runs here only in repos the driver made, with hooks off and no global or system config.

API:
    Final(repo: Path, commit: str, tree: str)
    commit_final(workdir: Path, dest: Path) -> Final
    Archive(bundle: Path, tarball: Path, diff: Path, diff_sha256: str, diff_text: str)
    archive_task(final: Final, pristine: repo.Pristine, archives_dir: Path, stem: str) -> Archive
    remove_tree(path: Path) -> None          # also removes what an agent made read-only
"""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
import subprocess
import tarfile
from dataclasses import dataclass
from pathlib import Path

import layout  # noqa: F401 (puts families/ on sys.path for common)
from common import repo

GIT_TIMEOUT_S = 120.0


class ArchiveError(RuntimeError):
    """A git step in a driver-owned repo failed."""


@dataclass(frozen=True)
class Final:
    repo: Path
    commit: str
    tree: str


@dataclass(frozen=True)
class Archive:
    bundle: Path
    tarball: Path
    diff: Path
    diff_sha256: str
    diff_text: str


def commit_final(workdir: Path, dest: Path) -> Final:
    tree = repo.export_tree(workdir, dest)
    commit = _git(dest, "rev-parse", "--verify", "-q", f"refs/heads/{repo.BRANCH}").decode().strip()
    return Final(repo=Path(dest), commit=commit, tree=tree)


def archive_task(final: Final, pristine: repo.Pristine, archives_dir: Path, stem: str) -> Archive:
    archives_dir = Path(archives_dir)
    archives_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    _git(final.repo, "fetch", "-q", "--no-tags", str(Path(pristine.bundle).absolute()),
         f"+{repo.PRISTINE_REF}:{repo.PRISTINE_REF}")
    raw = _git(final.repo, "diff", "--binary", "--no-color", "--no-ext-diff", "--no-textconv", "--no-renames",
               pristine.commit, final.commit)
    diff = archives_dir / f"{stem}.diff"
    diff.write_bytes(raw)
    bundle = archives_dir / f"{stem}.bundle"
    _git(final.repo, "bundle", "create", "-q", str(bundle.absolute()), f"refs/heads/{repo.BRANCH}", repo.PRISTINE_REF)
    tarball = archives_dir / f"{stem}.tar.gz"
    _tar_tree(final.repo, tarball)
    return Archive(bundle=bundle, tarball=tarball, diff=diff, diff_sha256="sha256:" + hashlib.sha256(raw).hexdigest(),
                   diff_text=raw.decode("utf-8", "replace"))


def remove_tree(path: Path) -> None:
    path = Path(path)
    if not os.path.lexists(path):
        return
    if path.is_symlink() or not path.is_dir():
        path.unlink()
        return
    for root, dirnames, _ in os.walk(path):
        for name in dirnames:
            child = Path(root) / name
            if not child.is_symlink():
                os.chmod(child, stat.S_IRWXU)
    os.chmod(path, stat.S_IRWXU)
    shutil.rmtree(path)


def _tar_tree(root: Path, dest: Path) -> None:
    def fixed(info: tarfile.TarInfo) -> tarfile.TarInfo:
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        info.mtime = 0
        return info

    with tarfile.open(dest, "w:gz", compresslevel=9) as tar:
        for directory, dirnames, filenames in os.walk(root):
            dirnames[:] = sorted(name for name in dirnames if name.lower() != ".git")
            links = [name for name in dirnames if os.path.islink(os.path.join(directory, name))]
            for name in sorted(filenames) + links:
                path = Path(directory) / name
                tar.add(path, arcname=path.relative_to(root).as_posix(), recursive=False, filter=fixed)


def _git(cwd: Path, *args: str) -> bytes:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0", LC_ALL="C",
               GIT_CEILING_DIRECTORIES=str(Path(cwd).resolve().parent))
    argv = ["git", "-c", f"core.hooksPath={os.devnull}", "-c", "core.fsmonitor=false", *args]
    try:
        result = subprocess.run(argv, cwd=cwd, capture_output=True, env=env, timeout=GIT_TIMEOUT_S, check=False)
    except (OSError, subprocess.TimeoutExpired) as err:
        raise ArchiveError(f"git {args[0]} could not run in {cwd}: {err}") from None
    if result.returncode != 0:
        raise ArchiveError(f"git {args[0]} failed in {cwd}: {result.stderr.decode('utf-8', 'replace').strip()[:500]}")
    return result.stdout
