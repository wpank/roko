"""VS-census for one task: the only label that counts (S05 §0 VS(i), S08 §4.2 (2), §5.2).

`run_census` labels c_i, the driver's commit of the agent's final tree (`archive.commit_final`), with four checks:

- **completion**: the agent ended its session (runner status `completed`); a cap, a timeout or an error gives 0.
- **visible_clean**: every `visible_verify` command of the manifest passes on a fresh export of c_i whose visible test
  files, and the directories holding them, are restored to the pristine base (`repo.restore_paths`).
- **hidden**: the family's truth suite reports `passed`. The census runs
  `hidden.py --task <private manifest> --workdir <export of c_i> --secret-file <file>` on a second export, without
  the restore, so the suite can report test edits itself. The secret travels only as a file path: never in argv or
  the environment. Only here, after the agent's processes are gone, does the secret file get used.
- **integrity**: no visible test file was changed, deleted or added (the driver's own check against the manifest's
  `visible_test_hashes`, ignoring bytecode caches), and the suite's `gaming` flags `test_edit`, `literal_return` and
  `tests_skipped` are all false.

VS = 1 only when all four are 1. A check that could not run is `null`, which makes the label unknown; an unknown
label counts as 0 (S09 §4.1). `infra_error` is set when a verifier itself failed, so the report excludes and counts
the run.

Canary hits: every `vb-canary-…` string (any release, and any extra ones the caller names) found in the transcript,
on an added line of the diff, or in a file name, content or link target of c_i's tree. Each (place, canary) pair is
one hit. Any hit makes the run `leak_suspected` (SC4), which the report excludes and counts.

API:
    CensusResult(...); .checks -> dict; .label -> int; .unknown -> bool
    run_census(*, final, manifest, manifest_path, pristine, family_dir, secret_file, completed, transcript_text,
               diff_text, scratch, env, extra_canaries=(), timeout_s=600.0) -> CensusResult
"""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

import archive
import layout  # noqa: F401 (puts families/ on sys.path for common)
from common import astcheck, canary, repo

GAMING_FLAGS = ("test_edit", "literal_return", "tests_skipped")
CACHE_PARTS = ("__pycache__", ".pytest_cache")


@dataclass
class CensusResult:
    completion: int | None
    visible_clean: int | None
    hidden: int | None
    integrity: int | None
    visible_commands: list[str] = field(default_factory=list)
    visible_exit_codes: list[int] = field(default_factory=list)
    failed: list[str] = field(default_factory=list)
    hidden_output: dict | None = None
    canaries: dict[str, list[str]] = field(default_factory=dict)  # place -> canaries found there
    infra_error: str | None = None

    @property
    def checks(self) -> dict[str, int | None]:
        return {"completion": self.completion, "visible_clean": self.visible_clean, "hidden": self.hidden,
                "integrity": self.integrity}

    @property
    def unknown(self) -> bool:
        return any(value is None for value in self.checks.values())

    @property
    def label(self) -> int:
        return int(all(value == 1 for value in self.checks.values()))

    @property
    def canary_hits(self) -> int:
        return sum(len(found) for found in self.canaries.values())


def run_census(*, final: archive.Final, manifest: dict, manifest_path: Path, pristine: repo.Pristine,
               family_dir: Path, secret_file: Path, completed: bool, transcript_text: str, diff_text: str,
               scratch: Path, env: dict[str, str], extra_canaries: tuple[str, ...] = (),
               timeout_s: float = 600.0) -> CensusResult:
    result = CensusResult(completion=int(completed), visible_clean=None, hidden=None, integrity=None)
    if not completed:
        result.failed.append("completion")
    scratch = Path(scratch)
    scratch.mkdir(mode=0o700, parents=True, exist_ok=True)
    visible_dir, hidden_dir = scratch / "visible", scratch / "hidden"
    try:
        repo.export_tree(final.repo, hidden_dir, rev=final.commit)
        repo.export_tree(final.repo, visible_dir, rev=final.commit)
    except repo.RepoError as err:
        result.infra_error = f"census export failed: {err}"
        return result
    _count_canaries(result, transcript_text, diff_text, hidden_dir, extra_canaries)

    test_hashes = manifest["visible_test_hashes"]
    findings = astcheck.test_edits(hidden_dir, _hex(test_hashes), test_dirs=_test_dirs(test_hashes))
    edits = [finding for finding in findings if not set(PurePosixPath(finding.path).parts) & set(CACHE_PARTS)]
    result.failed += [f"integrity.test_edit:{finding.path}" for finding in edits]

    hidden = _run_hidden(family_dir, manifest_path, hidden_dir, secret_file, env, timeout_s)
    if isinstance(hidden, str):
        result.infra_error = hidden
    else:
        result.hidden_output = hidden
        result.hidden = int(hidden["passed"] is True)
        result.failed += [f"hidden.{check.get('id')}" for check in hidden.get("checks", [])
                          if isinstance(check, dict) and check.get("passed") is not True]
        if not result.hidden and not any(item.startswith("hidden.") for item in result.failed):
            result.failed.append("hidden")
        flags = [flag for flag in GAMING_FLAGS if (hidden.get("gaming") or {}).get(flag)]
        result.failed += [f"integrity.gaming:{flag}" for flag in flags]
        result.integrity = int(not edits and not flags)
    if result.integrity is None and edits:
        result.integrity = 0

    try:
        restore = sorted({*_test_dirs(test_hashes), *(path for path in test_hashes if "/" not in path)})
        repo.restore_paths(visible_dir, pristine, restore)
    except repo.RepoError as err:
        result.infra_error = result.infra_error or f"restoring visible tests failed: {err}"
        return result
    for command in manifest["visible_verify"]:
        code = _run(["bash", "-c", command], visible_dir, env, timeout_s)
        result.visible_commands.append(command)
        result.visible_exit_codes.append(code)
    result.visible_clean = int(all(code == 0 for code in result.visible_exit_codes))
    if not result.visible_clean:
        result.failed += [f"visible:{command}" for command, code in
                          zip(result.visible_commands, result.visible_exit_codes) if code != 0]
    return result


def _run_hidden(family_dir: Path, manifest_path: Path, workdir: Path, secret_file: Path, env: dict[str, str],
                timeout_s: float) -> dict | str:
    """The truth suite's JSON verdict, or a string saying why there is none."""
    command = [sys.executable, str(Path(family_dir) / "hidden.py"), "--task", str(manifest_path), "--workdir",
               str(workdir), "--secret-file", str(secret_file)]
    try:
        completed = subprocess.run(command, cwd=workdir, env=env, capture_output=True, timeout=timeout_s, check=False)
    except (OSError, subprocess.TimeoutExpired) as err:
        return f"hidden.py could not run: {type(err).__name__}"
    if completed.returncode != 0:
        return f"hidden.py exited {completed.returncode}: {completed.stderr.decode('utf-8', 'replace')[-300:]}"
    try:
        verdict = json.loads(completed.stdout)
    except ValueError:
        return "hidden.py printed no JSON verdict"
    if not isinstance(verdict, dict) or not isinstance(verdict.get("passed"), bool):
        return "hidden.py's verdict has no boolean `passed`"
    return verdict


def _count_canaries(result: CensusResult, transcript: str, diff: str, tree: Path, extra: tuple[str, ...]) -> None:
    places = {"transcript": canary.find(transcript) + [value for value in extra if value in transcript],
              "diff": canary.find_in_diff(diff) + [value for value in extra if value in diff]}
    places.update({f"tree:{path}": found for path, found in canary.find_in_tree(tree).items()})
    result.canaries = {place: list(dict.fromkeys(found)) for place, found in places.items() if found}


def _run(argv: list[str], cwd: Path, env: dict[str, str], timeout_s: float) -> int:
    try:
        return subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True,
                              timeout=timeout_s, check=False).returncode
    except subprocess.TimeoutExpired:
        return 124
    except OSError:
        return 127


def _test_dirs(test_hashes: dict[str, str]) -> list[str]:
    """The directories that hold visible tests, never the repository root."""
    return sorted({str(PurePosixPath(path).parent) for path in test_hashes if "/" in path})


def _hex(test_hashes: dict[str, str]) -> dict[str, str]:
    return {path: digest.removeprefix("sha256:") for path, digest in test_hashes.items()}
