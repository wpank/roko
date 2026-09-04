#!/usr/bin/env python3
"""Deterministic cold/warm scorecards and bounded history for Roko development lanes.

The runner is intentionally an orchestrator, not another benchmark workload.  It
checks out the same immutable commit for every sample, gives each cold sample a
new target directory, seeds a separate stable target for every warm lane,
and delegates bounded capture to ``run_evidence.py``.

Nothing in this file cleans Cargo's normal target directory. Settled,
evidence-bearing benchmark worktrees are removed through ``git worktree remove``;
disposable cold targets have a separate ownership-checked cleanup path.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import io
import json
import math
import os
import pathlib
import re
import shutil
import stat
import subprocess
import sys
import time
import uuid
from collections import defaultdict
from typing import Any, Sequence


SCHEMA_VERSION = 1
DEFAULT_MANIFEST = pathlib.Path("benchmarks/dev-audit/manifest.json")
DEFAULT_BASELINES = pathlib.Path("benchmarks/dev-audit/manual-baselines.json")
DEFAULT_OUTPUT_ROOT = pathlib.Path(".roko/benchmarks")
DEFAULT_HISTORY_FILENAME = "history.json"
DEFAULT_HISTORY_MARKDOWN_FILENAME = "HISTORY.md"
HISTORY_COMPARISON_METRICS = ("p50", "p95")
SESSION_DIR_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{8}$")
SECRET_RE = re.compile(
    r"(?:api[_-]?key|authorization|bearer|credential|password|private[_-]?key|secret|token)",
    re.IGNORECASE,
)
ID_RE = re.compile(r"^[a-z0-9][a-z0-9._-]{0,63}$")
ALLOWED_PLACEHOLDERS = {
    "base_sha",
    "bundle",
    "cache",
    "deadline",
    "fixture",
    "lane",
    "plan",
    "repetition",
    "repo",
    "roko_bin",
    "runner_deadline",
    "settlement_headroom",
    "target_dir",
    "worktree",
}
PHASE_ALIASES = {
    "startup": "startup_ms",
    "capacity_wait": "capacity_wait_ms",
    "context": "context_ms",
    "prompt": "prompt_ms",
    "agent": "agent_ms",
    "cargo_lock_wait": "cargo_lock_wait_ms",
    "compile": "compile_ms",
    "targeted_test": "targeted_test_ms",
    "smoke": "smoke_ms",
}


class BenchmarkError(RuntimeError):
    """A user-actionable benchmark configuration or admission error."""


class HistoryLimitError(BenchmarkError):
    """A history scan stopped before producing a potentially biased partial view."""


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds").replace(
        "+00:00", "Z"
    )


def safe_id(value: str, field: str = "id") -> str:
    if not ID_RE.fullmatch(value):
        raise BenchmarkError(
            f"{field} must match {ID_RE.pattern!r}; got {value!r}"
        )
    return value


def private_mkdir(path: pathlib.Path) -> None:
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(path, 0o700)


def write_json(path: pathlib.Path, value: Any) -> None:
    private_mkdir(path.parent)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
    os.chmod(temporary, 0o600)
    os.replace(temporary, path)


def write_text(path: pathlib.Path, value: str) -> None:
    private_mkdir(path.parent)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    temporary.write_text(value, encoding="utf-8")
    os.chmod(temporary, 0o600)
    os.replace(temporary, path)


def append_jsonl(path: pathlib.Path, value: Any) -> None:
    private_mkdir(path.parent)
    with path.open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(value, sort_keys=True, separators=(",", ":")))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.chmod(path, 0o600)


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise BenchmarkError(f"cannot read {path}: {error}") from error
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise BenchmarkError(f"invalid JSON in {path}: {error}") from error


def run_capture(
    argv: Sequence[str], cwd: pathlib.Path, timeout: float = 15.0
) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            list(argv),
            cwd=cwd,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise BenchmarkError(f"command failed to start or settle: {argv!r}: {error}") from error


def git_value(repo: pathlib.Path, *args: str) -> str:
    result = run_capture(["git", *args], repo)
    if result.returncode != 0:
        detail = result.stderr.strip()[:1000]
        raise BenchmarkError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout.strip()


def discover_repo(start: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    root = pathlib.Path(git_value(start, "rev-parse", "--show-toplevel")).resolve()
    common_raw = git_value(root, "rev-parse", "--git-common-dir")
    common = pathlib.Path(common_raw)
    if not common.is_absolute():
        common = root / common
    common = common.resolve()
    primary = common.parent.resolve()
    if not (common / "HEAD").is_file():
        raise BenchmarkError(f"unexpected Git common directory: {common}")
    return root, primary


def command_version_hash(path: pathlib.Path) -> dict[str, Any]:
    result: dict[str, Any] = {"path": str(path), "exists": path.is_file()}
    if not path.is_file():
        return result
    hasher = hashlib.sha256()
    try:
        with path.open("rb") as stream:
            while chunk := stream.read(1024 * 1024):
                hasher.update(chunk)
        stat = path.stat()
        result.update(
            {
                "bytes": stat.st_size,
                "executable": os.access(path, os.X_OK),
                "sha256": hasher.hexdigest(),
            }
        )
    except OSError as error:
        result["error"] = str(error)
    return result


def validate_argv(value: Any, field: str, *, optional: bool = False) -> list[str] | None:
    if value is None and optional:
        return None
    if not isinstance(value, list) or not value or not all(
        isinstance(item, str) and item for item in value
    ):
        raise BenchmarkError(f"{field} must be a non-empty JSON string array")
    for index, item in enumerate(value):
        lowered = item.lower()
        if SECRET_RE.search(item) and ("=" in item or index > 0 and SECRET_RE.fullmatch(value[index - 1].lstrip("-"))):
            raise BenchmarkError(
                f"{field} contains an inline secret-like argument; use ambient credentials"
            )
        for placeholder in re.findall(r"\{([^{}]+)\}", item):
            if placeholder not in ALLOWED_PLACEHOLDERS:
                raise BenchmarkError(
                    f"{field} uses unsupported placeholder {{{placeholder}}}"
                )
        if "\x00" in lowered:
            raise BenchmarkError(f"{field} contains a NUL byte")
    return list(value)


def validate_manifest(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("schema_version") != SCHEMA_VERSION:
        raise BenchmarkError(f"manifest schema_version must be {SCHEMA_VERSION}")
    defaults = value.get("defaults")
    if not isinstance(defaults, dict):
        raise BenchmarkError("manifest.defaults must be an object")
    repetitions = defaults.get("repetitions", 5)
    deadline = defaults.get("deadline_seconds", 600)
    if not isinstance(repetitions, int) or not 1 <= repetitions <= 20:
        raise BenchmarkError("defaults.repetitions must be in 1..20")
    if not isinstance(deadline, int) or not 10 <= deadline <= 3600:
        raise BenchmarkError("defaults.deadline_seconds must be in 10..3600")

    lanes = value.get("lanes")
    fixtures = value.get("fixtures")
    if not isinstance(lanes, list) or not lanes:
        raise BenchmarkError("manifest.lanes must be a non-empty array")
    if not isinstance(fixtures, list) or not fixtures:
        raise BenchmarkError("manifest.fixtures must be a non-empty array")
    lane_ids: set[str] = set()
    for lane in lanes:
        if not isinstance(lane, dict):
            raise BenchmarkError("every lane must be an object")
        lane_id = safe_id(lane.get("id", ""), "lane.id")
        if lane_id in lane_ids:
            raise BenchmarkError(f"duplicate lane id: {lane_id}")
        lane_ids.add(lane_id)
        kind = lane.get("kind")
        if kind not in {"command", "manual"}:
            raise BenchmarkError(f"lane {lane_id}: kind must be command or manual")
        if not isinstance(lane.get("network"), bool):
            raise BenchmarkError(f"lane {lane_id}: network must be explicit boolean")
        if lane["network"]:
            estimate = lane.get("estimated_max_cost_usd")
            if not isinstance(estimate, (int, float)) or estimate <= 0:
                raise BenchmarkError(
                    f"lane {lane_id}: network lanes require estimated_max_cost_usd > 0"
                )
        lane["argv"] = validate_argv(
            lane.get("argv"), f"lane {lane_id}.argv", optional=kind == "manual"
        )
        environment = lane.get("env", {})
        if not isinstance(environment, dict) or not all(
            isinstance(name, str) and isinstance(raw, str)
            for name, raw in environment.items()
        ):
            raise BenchmarkError(f"lane {lane_id}.env must be a string map")
        secret_names = [name for name in environment if SECRET_RE.search(name)]
        if secret_names:
            raise BenchmarkError(
                f"lane {lane_id}.env must not contain credentials: {', '.join(secret_names)}"
            )
        for name, raw in environment.items():
            validate_argv([raw], f"lane {lane_id}.env.{name}")
        evidence_args = lane.get("evidence_args", [])
        if not isinstance(evidence_args, list) or not all(
            isinstance(item, str) and item for item in evidence_args
        ):
            raise BenchmarkError(f"lane {lane_id}.evidence_args must be a string array")
        if "--allow-remote-endpoints" in evidence_args:
            raise BenchmarkError(
                f"lane {lane_id}: remote endpoint collection is prohibited in the stock scorecard"
            )
        unset_env = lane.get("unset_env", [])
        if not isinstance(unset_env, list) or not all(
            isinstance(name, str) and name for name in unset_env
        ):
            raise BenchmarkError(f"lane {lane_id}.unset_env must be a string array")

    fixture_ids: set[str] = set()
    for fixture in fixtures:
        if not isinstance(fixture, dict):
            raise BenchmarkError("every fixture must be an object")
        fixture_id = safe_id(fixture.get("id", ""), "fixture.id")
        if fixture_id in fixture_ids:
            raise BenchmarkError(f"duplicate fixture id: {fixture_id}")
        fixture_ids.add(fixture_id)
        plan = fixture.get("plan")
        if not isinstance(plan, str) or not plan or pathlib.PurePath(plan).is_absolute() or ".." in pathlib.PurePath(plan).parts:
            raise BenchmarkError(f"fixture {fixture_id}: plan must be a safe relative path")
        fixture["warmup_argv"] = validate_argv(
            fixture.get("warmup_argv"), f"fixture {fixture_id}.warmup_argv"
        )
    return value


def select_objects(
    objects: list[dict[str, Any]], selected: list[str] | None, kind: str
) -> list[dict[str, Any]]:
    by_id = {item["id"]: item for item in objects}
    if selected:
        missing = [item for item in selected if item not in by_id]
        if missing:
            raise BenchmarkError(f"unknown {kind}(s): {', '.join(missing)}")
        return [by_id[item] for item in selected]
    return [item for item in objects if item.get("enabled_by_default", True)]


def expand(value: str, context: dict[str, str]) -> str:
    try:
        return value.format_map(context)
    except KeyError as error:
        raise BenchmarkError(f"unresolved command placeholder: {error}") from error


def expand_argv(argv: Sequence[str], context: dict[str, str]) -> list[str]:
    return [expand(item, context) for item in argv]


def tree_size(path: pathlib.Path, *, deadline_seconds: float = 5.0) -> dict[str, Any]:
    if not path.exists():
        return {"bytes": 0, "files": 0, "complete": True}
    started = time.monotonic()
    total = 0
    files = 0
    try:
        for root, directories, names in os.walk(path, followlinks=False):
            directories[:] = [
                name for name in directories if not (pathlib.Path(root) / name).is_symlink()
            ]
            for name in names:
                candidate = pathlib.Path(root) / name
                if candidate.is_symlink():
                    continue
                try:
                    total += candidate.stat().st_size
                except OSError:
                    continue
                files += 1
                if files > 1_000_000 or time.monotonic() - started > deadline_seconds:
                    return {"bytes": total, "files": files, "complete": False}
    except OSError:
        return {"bytes": total, "files": files, "complete": False}
    return {"bytes": total, "files": files, "complete": True}


def resource_admission(
    session: pathlib.Path,
    target: pathlib.Path,
    *,
    min_free_gib: float,
    min_free_percent: float,
    max_session_gib: float,
    max_cache_gib: float,
) -> dict[str, Any]:
    disk = shutil.disk_usage(session)
    free_percent = (disk.free / disk.total * 100.0) if disk.total else 0.0
    session_size = tree_size(session)
    cache_size = tree_size(target)
    errors: list[str] = []
    gib = 1024**3
    max_cache_bytes = round(max_cache_gib * gib)
    max_session_bytes = round(max_session_gib * gib)
    reserved_growth = max(0, max_cache_bytes - int(cache_size["bytes"]))
    projected_free = max(0, disk.free - reserved_growth)
    projected_free_percent = (projected_free / disk.total * 100.0) if disk.total else 0.0
    projected_session = int(session_size["bytes"]) + reserved_growth
    if disk.free < min_free_gib * gib:
        errors.append(f"free disk is below {min_free_gib:g} GiB")
    if free_percent < min_free_percent:
        errors.append(f"free disk is below {min_free_percent:g}%")
    if projected_free < min_free_gib * gib:
        errors.append(
            f"free disk after reserving the target budget would be below {min_free_gib:g} GiB"
        )
    if projected_free_percent < min_free_percent:
        errors.append(
            f"free disk after reserving the target budget would be below {min_free_percent:g}%"
        )
    if not session_size["complete"]:
        errors.append("session size scan exceeded its five-second/one-million-file bound")
    elif session_size["bytes"] > max_session_bytes:
        errors.append(f"session exceeds {max_session_gib:g} GiB")
    elif projected_session > max_session_bytes:
        errors.append(
            f"session plus reserved target growth would exceed {max_session_gib:g} GiB"
        )
    if not cache_size["complete"]:
        errors.append("target size scan exceeded its five-second/one-million-file bound")
    elif cache_size["bytes"] > max_cache_bytes:
        errors.append(f"target exceeds {max_cache_gib:g} GiB")
    return {
        "checked_utc": utc_now(),
        "admitted": not errors,
        "errors": errors,
        "disk": {
            "free_bytes": disk.free,
            "free_percent": round(free_percent, 3),
            "projected_free_bytes": projected_free,
            "projected_free_percent": round(projected_free_percent, 3),
            "total_bytes": disk.total,
        },
        "limits": {
            "min_free_gib": min_free_gib,
            "min_free_percent": min_free_percent,
            "max_session_gib": max_session_gib,
            "max_cache_gib": max_cache_gib,
        },
        "session": session_size,
        "target": {
            "path": str(target),
            **cache_size,
            "reserved_growth_bytes": reserved_growth,
        },
        "projected_session_bytes": projected_session,
    }


def resolve_bundle(bundle_root: pathlib.Path) -> pathlib.Path | None:
    if not bundle_root.is_dir():
        return None
    candidates = sorted(
        path.resolve()
        for path in bundle_root.iterdir()
        if path.is_dir() and not path.is_symlink()
    )
    return candidates[0] if len(candidates) == 1 else None


def read_optional_json(path: pathlib.Path) -> dict[str, Any] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def normalize_phase_metrics(phases: Any) -> dict[str, int | float]:
    if not isinstance(phases, dict):
        return {}
    result: defaultdict[str, float] = defaultdict(float)
    for name, raw in phases.items():
        if not isinstance(raw, (int, float)):
            continue
        normalized = re.sub(r"[^a-z0-9]+", "_", str(name).lower()).strip("_")
        for needle, field in PHASE_ALIASES.items():
            if needle in normalized:
                result[field] += float(raw)
                break
    return {
        name: int(value) if value.is_integer() else value
        for name, value in result.items()
    }


def evidence_row(
    bundle: pathlib.Path | None,
    identity: dict[str, Any],
    wrapper_exit: int,
    wall_ms: int,
) -> dict[str, Any]:
    row = {
        "schema_version": SCHEMA_VERSION,
        **identity,
        "bundle": str(bundle) if bundle else None,
        "wrapper_exit_code": wrapper_exit,
        "wall_ms": wall_ms,
        "state": "capture_missing",
        "valid": False,
        "metrics": {"total_ms": wall_ms, "command_ms": wall_ms},
        "provider": {},
        "correctness": {},
    }
    if bundle is None:
        return row
    summary = read_optional_json(bundle / "summary.json") or {}
    metrics = read_optional_json(bundle / "metrics.json") or {}
    validation = read_optional_json(bundle / "validation.json") or {}
    latency = metrics.get("latency_ms", {}) if isinstance(metrics.get("latency_ms"), dict) else {}
    flattened: dict[str, Any] = {
        "command_ms": latency.get("command", wall_ms),
        "bundle_ms": latency.get("bundle_finalize"),
        "total_ms": latency.get("total", wall_ms),
    }
    flattened.update(normalize_phase_metrics(latency.get("runner_phases")))
    provider = metrics.get("provider") if isinstance(metrics.get("provider"), dict) else {}
    verification = metrics.get("verification") if isinstance(metrics.get("verification"), dict) else {}
    event_metrics = metrics.get("events") if isinstance(metrics.get("events"), dict) else {}
    git_metrics = metrics.get("git") if isinstance(metrics.get("git"), dict) else {}
    row.update(
        {
            "run_id": summary.get("run_id"),
            "state": summary.get("state", "unknown"),
            "valid": validation.get("valid") is True,
            "metrics": flattened,
            "provider": {
                "actual_launches": provider.get("actual_launches"),
                "prompt_estimated_tokens": provider.get("prompt_estimated_tokens"),
                "cost_usd": provider.get("cost_usd"),
                "retries": provider.get("retries"),
                "timeouts": provider.get("timeouts"),
            },
            "correctness": {
                "changed_files": git_metrics.get("files"),
                "changed_loc": git_metrics.get("loc"),
                "duplicate_dispatch_attempts": event_metrics.get("duplicate_dispatch_attempts"),
                "gates_passed": verification.get("gates_passed"),
                "gates_failed": verification.get("gates_failed"),
                "endpoints_2xx": verification.get("endpoints_2xx"),
                "endpoints_failed": verification.get("endpoints_failed"),
                "screenshots": verification.get("screenshots"),
            },
        }
    )
    return row


def worktree_registered(repo: pathlib.Path, path: pathlib.Path) -> bool:
    output = git_value(repo, "worktree", "list", "--porcelain")
    registered = {
        pathlib.Path(line.removeprefix("worktree ")).resolve()
        for line in output.splitlines()
        if line.startswith("worktree ")
    }
    return path.resolve() in registered


def bundle_process_group_absent(bundle: pathlib.Path | None) -> tuple[bool, str | None]:
    if bundle is None:
        return False, "evidence bundle is missing"
    commands = bundle / "commands.jsonl"
    try:
        rows = [json.loads(line) for line in commands.read_text(encoding="utf-8").splitlines() if line.strip()]
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return False, f"cannot prove process settlement from commands.jsonl: {error}"
    primary = [
        row
        for row in rows
        if isinstance(row, dict) and row.get("command_id") == "command-1"
    ]
    if len(primary) != 1:
        return False, f"expected one primary command record, found {len(primary)}"
    pgid = primary[0].get("process_group_id")
    if pgid is None:
        return True, None
    if not isinstance(pgid, int) or pgid <= 0 or not hasattr(os, "killpg"):
        return False, "process-group identity is invalid or unsupported"
    try:
        os.killpg(pgid, 0)
    except ProcessLookupError:
        return True, None
    except PermissionError:
        return False, f"process group {pgid} still exists but cannot be inspected"
    except OSError as error:
        return False, f"process-group settlement check failed: {error}"
    return False, f"process group {pgid} still exists"


def create_worktree(repo: pathlib.Path, path: pathlib.Path, base_sha: str) -> None:
    if path.exists() or path.is_symlink():
        raise BenchmarkError(f"refusing to overwrite benchmark worktree path: {path}")
    private_mkdir(path.parent)
    result = run_capture(
        ["git", "worktree", "add", "--detach", str(path), base_sha], repo, timeout=60.0
    )
    if result.returncode != 0:
        raise BenchmarkError(f"git worktree add failed: {result.stderr.strip()[:2000]}")
    if not worktree_registered(repo, path):
        raise BenchmarkError(f"Git did not register created worktree: {path}")


def remove_owned_worktree(
    repo: pathlib.Path, worktree_root: pathlib.Path, path: pathlib.Path
) -> tuple[bool, str | None]:
    root = worktree_root.resolve()
    candidate = path.resolve()
    if candidate.parent != root or not candidate.name.startswith("sample-"):
        return False, "ownership/path check failed"
    if not worktree_registered(repo, candidate):
        return False, "path is not a registered worktree"
    result = run_capture(
        ["git", "worktree", "remove", "--force", str(candidate)], repo, timeout=60.0
    )
    if result.returncode != 0:
        return False, result.stderr.strip()[:1000]
    return True, None


def remove_owned_cold_target(
    *,
    cache_root: pathlib.Path,
    candidate: pathlib.Path,
    repo_target: pathlib.Path,
    session_id: str,
    process_absent: bool,
    process_error: str | None,
) -> tuple[bool, str | None]:
    """Remove one disposable cold target after strict ownership/settlement checks."""

    if not process_absent:
        return False, process_error or "process settlement is unconfirmed"
    marker = cache_root / "OWNER.json"
    try:
        owner = json.loads(marker.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return False, f"cache ownership marker is unreadable: {error}"
    if owner.get("schema_version") != SCHEMA_VERSION or owner.get("session_id") != session_id:
        return False, "cache ownership marker does not match this session"
    if not candidate.exists():
        return True, None
    if candidate.is_symlink() or not candidate.is_dir():
        return False, "cold target is not a real directory"
    root = cache_root.resolve()
    cold_root = (root / "cold").resolve()
    resolved = candidate.resolve()
    if resolved.parent != cold_root or not resolved.name.startswith("sample-"):
        return False, "cold target is not an exact child of the benchmark-owned cold root"
    normal_target = repo_target.resolve()
    if resolved == normal_target or normal_target in resolved.parents or resolved in normal_target.parents:
        return False, "cold target overlaps the repository target"
    try:
        shutil.rmtree(resolved)
    except OSError as error:
        return False, str(error)
    return not resolved.exists(), None if not resolved.exists() else "target still exists"


def percentile(values: Sequence[int | float], fraction: float) -> int | float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, math.ceil(len(ordered) * fraction) - 1))
    return ordered[index]


def metric_summary(rows: Sequence[dict[str, Any]], name: str) -> dict[str, Any]:
    values = [
        row.get("metrics", {}).get(name)
        for row in rows
        if isinstance(row.get("metrics", {}).get(name), (int, float))
    ]
    return {
        "observed": len(values),
        "missing": len(rows) - len(values),
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "min": min(values) if values else None,
        "max": max(values) if values else None,
    }


def nested_metric_summary(
    rows: Sequence[dict[str, Any]], section: str, name: str
) -> dict[str, Any]:
    values = [
        row.get(section, {}).get(name)
        for row in rows
        if isinstance(row.get(section, {}).get(name), (int, float))
        and not isinstance(row.get(section, {}).get(name), bool)
    ]
    return {
        "observed": len(values),
        "missing": len(rows) - len(values),
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "min": min(values) if values else None,
        "max": max(values) if values else None,
    }


def baseline_rows(path: pathlib.Path) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    value = load_json(path)
    if not isinstance(value, dict) or value.get("schema_version") != SCHEMA_VERSION:
        raise BenchmarkError(f"baseline file {path} has unsupported schema")
    rows = value.get("rows", [])
    lanes = value.get("lanes", [])
    if not isinstance(rows, list) or not all(isinstance(row, dict) for row in rows):
        raise BenchmarkError(f"baseline file {path}: rows must be objects")
    if not isinstance(lanes, list) or not all(isinstance(row, dict) for row in lanes):
        raise BenchmarkError(f"baseline file {path}: lanes must be objects")
    normalized = []
    for index, row in enumerate(rows):
        lane_id = safe_id(str(row.get("lane_id", "")), "baseline lane_id")
        metrics = row.get("metrics")
        if not isinstance(metrics, dict):
            raise BenchmarkError(f"baseline row {index}: metrics must be an object")
        normalized.append(
            {
                "schema_version": SCHEMA_VERSION,
                "measured": True,
                "source": row.get("source", str(path)),
                "lane_id": lane_id,
                "fixture_id": str(row.get("fixture_id", "historical")),
                "cache": str(row.get("cache", "unknown")),
                "repetition": row.get("repetition", index + 1),
                "state": row.get("state", "historical"),
                "valid": row.get("valid"),
                "metrics": {
                    str(name): raw
                    for name, raw in metrics.items()
                    if isinstance(raw, (int, float))
                },
                "provider": row.get("provider", {}),
                "correctness": row.get("correctness", {}),
                "bundle": row.get("bundle"),
                "historical": True,
            }
        )
    return normalized, lanes


def read_jsonl(path: pathlib.Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.is_file():
        return rows
    try:
        with path.open("r", encoding="utf-8") as stream:
            for number, line in enumerate(stream, start=1):
                if not line.strip():
                    continue
                value = json.loads(line)
                if not isinstance(value, dict):
                    raise BenchmarkError(f"{path}:{number}: row must be an object")
                rows.append(value)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise BenchmarkError(f"invalid JSONL {path}: {error}") from error
    return rows


def build_scorecard(
    session: pathlib.Path,
    rows: list[dict[str, Any]],
    historical: list[dict[str, Any]],
    baseline_lanes: list[dict[str, Any]],
) -> dict[str, Any]:
    measured = [row for row in rows if row.get("measured") is True]
    all_rows = measured + historical
    metric_names = [
        "startup_ms",
        "capacity_wait_ms",
        "context_ms",
        "prompt_ms",
        "first_edit_ms",
        "agent_ms",
        "cargo_lock_wait_ms",
        "compile_ms",
        "gate_ms",
        "targeted_test_ms",
        "smoke_ms",
        "bundle_ms",
        "command_ms",
        "total_ms",
    ]
    groups: defaultdict[tuple[str, str, str], list[dict[str, Any]]] = defaultdict(list)
    lane_groups: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in all_rows:
        key = (
            str(row.get("lane_id")),
            str(row.get("fixture_id")),
            str(row.get("cache")),
        )
        groups[key].append(row)
        lane_groups[key[0]].append(row)

    summaries: list[dict[str, Any]] = []
    for (lane_id, fixture_id, cache), selected in sorted(groups.items()):
        summaries.append(
            {
                "lane_id": lane_id,
                "fixture_id": fixture_id,
                "cache": cache,
                "runs": len(selected),
                "succeeded": sum(row.get("state") == "succeeded" for row in selected),
                "valid": sum(row.get("valid") is True for row in selected),
                "timeouts": sum(
                    row.get("state") in {"timed_out", "timeout"} for row in selected
                ),
                "human_interventions": sum(
                    row.get("correctness", {}).get("human_intervention") is True
                    for row in selected
                ),
                "latency_ms": {
                    name: metric_summary(selected, name) for name in metric_names
                },
                "provider": {
                    name: nested_metric_summary(selected, "provider", name)
                    for name in (
                        "actual_launches",
                        "prompt_estimated_tokens",
                        "cost_usd",
                        "retries",
                        "timeouts",
                    )
                },
                "correctness": {
                    "escaped_regressions": nested_metric_summary(
                        selected, "correctness", "escaped_regressions"
                    ),
                    "human_interventions": sum(
                        row.get("correctness", {}).get("human_intervention") is True
                        for row in selected
                    ),
                    "duplicate_dispatch_runs": sum(
                        bool(row.get("correctness", {}).get("duplicate_dispatch_attempts"))
                        for row in selected
                    ),
                    "gate_failures": sum(
                        row.get("correctness", {}).get("gates_failed") or 0
                        for row in selected
                        if isinstance(row.get("correctness", {}).get("gates_failed"), int)
                    ),
                },
                "bundles": [row.get("bundle") for row in selected],
                "historical": all(row.get("historical") is True for row in selected),
            }
        )

    lane_rollups = []
    known_lane_ids = {str(row.get("id")) for row in baseline_lanes}
    known_lane_ids.update(lane_groups)
    for lane_id in sorted(known_lane_ids):
        selected = lane_groups.get(lane_id, [])
        lane_rollups.append(
            {
                "lane_id": lane_id,
                "runs": len(selected),
                "status": "measured" if selected else "awaiting_import_or_run",
                "valid": sum(row.get("valid") is True for row in selected),
                "total_ms": metric_summary(selected, "total_ms"),
            }
        )

    by_key = {
        (summary["lane_id"], summary["fixture_id"], summary["cache"]): summary
        for summary in summaries
    }
    comparison_keys = sorted(
        {(fixture, cache) for _, fixture, cache in by_key if fixture != "historical"}
    )
    reference_lanes = sorted(
        lane for lane in known_lane_ids if lane != "roko-fast"
    )
    comparisons = []
    for fixture_id, cache in comparison_keys:
        fast = by_key.get(("roko-fast", fixture_id, cache))
        fast_p50 = fast and fast["latency_ms"]["total_ms"]["p50"]
        fast_p95 = fast and fast["latency_ms"]["total_ms"]["p95"]
        for reference_lane in reference_lanes:
            reference = by_key.get((reference_lane, fixture_id, cache))
            reference_p50 = reference and reference["latency_ms"]["total_ms"]["p50"]
            reference_p95 = reference and reference["latency_ms"]["total_ms"]["p95"]
            comparisons.append(
                {
                    "fixture_id": fixture_id,
                    "cache": cache,
                    "reference_lane": reference_lane,
                    "candidate_lane": "roko-fast",
                    "reference_p50_ms": reference_p50,
                    "candidate_p50_ms": fast_p50,
                    "p50_speedup": round(reference_p50 / fast_p50, 4)
                    if isinstance(reference_p50, (int, float))
                    and isinstance(fast_p50, (int, float))
                    and fast_p50 > 0
                    else None,
                    "reference_p95_ms": reference_p95,
                    "candidate_p95_ms": fast_p95,
                    "p95_speedup": round(reference_p95 / fast_p95, 4)
                    if isinstance(reference_p95, (int, float))
                    and isinstance(fast_p95, (int, float))
                    and fast_p95 > 0
                    else None,
                    "status": "comparable"
                    if reference_p50 is not None and fast_p50 is not None
                    else "insufficient_samples",
                }
            )

    return {
        "schema_version": SCHEMA_VERSION,
        "generated_utc": utc_now(),
        "session": str(session),
        "policy": {
            "percentile": "nearest-rank",
            "failures_and_timeouts": "retained at observed wall/deadline duration",
            "missing_metrics": "reported, never imputed or discarded silently",
            "warmups": "recorded separately and excluded from measured percentiles",
        },
        "runs": {
            "measured": len(measured),
            "warmups": sum(row.get("role") == "warmup" for row in rows),
            "historical": len(historical),
        },
        "groups": summaries,
        "lane_rollups": lane_rollups,
        "comparisons": comparisons,
        "rows": all_rows,
    }


def render_scorecard(scorecard: dict[str, Any]) -> str:
    lines = [
        "# Development benchmark scorecard",
        "",
        f"Generated: `{scorecard['generated_utc']}`",
        "",
        "Failures and timeouts remain in the distribution. Warmups are evidence-bearing but are not scored.",
        "",
        "| Lane | Fixture | Cache | Runs | Valid | p50 total | p95 total |",
        "|---|---|---:|---:|---:|---:|---:|",
    ]
    for group in scorecard["groups"]:
        total = group["latency_ms"]["total_ms"]
        lines.append(
            "| {lane} | {fixture} | {cache} | {runs} | {valid} | {p50} | {p95} |".format(
                lane=group["lane_id"],
                fixture=group["fixture_id"],
                cache=group["cache"],
                runs=group["runs"],
                valid=group["valid"],
                p50=total["p50"] if total["p50"] is not None else "—",
                p95=total["p95"] if total["p95"] is not None else "—",
            )
        )
    lines.extend(
        [
            "",
            "## FAST comparisons",
            "",
            "| Fixture | Cache | Reference | p50 speedup | p95 speedup | Status |",
            "|---|---:|---|---:|---:|---|",
        ]
    )
    for comparison in scorecard["comparisons"]:
        lines.append(
            "| {fixture} | {cache} | {reference} | {p50} | {p95} | {status} |".format(
                fixture=comparison["fixture_id"],
                cache=comparison["cache"],
                reference=comparison["reference_lane"],
                p50=comparison["p50_speedup"] or "—",
                p95=comparison["p95_speedup"] or "—",
                status=comparison["status"],
            )
        )
    lines.extend(["", "Raw evidence bundle paths are in `scorecard.json`.", ""])
    return "\n".join(lines)


def summarize_session(
    session: pathlib.Path, baseline_paths: Sequence[pathlib.Path]
) -> dict[str, Any]:
    session = session.expanduser().resolve()
    if not session.is_dir() or session.is_symlink():
        raise BenchmarkError(f"session is not a real directory: {session}")
    rows = read_jsonl(session / "runs.jsonl")
    historical: list[dict[str, Any]] = []
    lane_metadata: list[dict[str, Any]] = []
    for path in baseline_paths:
        added, lanes = baseline_rows(path)
        historical.extend(added)
        lane_metadata.extend(lanes)
    scorecard = build_scorecard(session, rows, historical, lane_metadata)
    write_json(session / "scorecard.json", scorecard)
    write_text(session / "SCORECARD.md", render_scorecard(scorecard))
    return scorecard


def is_number(value: Any) -> bool:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        return False
    try:
        return math.isfinite(value)
    except (OverflowError, TypeError):
        return False


def percentage(numerator: int | float, denominator: int | float) -> float | None:
    if denominator <= 0:
        return None
    return round(float(numerator) / float(denominator) * 100.0, 4)


def valid_history_label(value: Any, *, max_length: int = 128) -> bool:
    return (
        isinstance(value, str)
        and 0 < len(value) <= max_length
        and "\n" not in value
        and "\r" not in value
        and "\x00" not in value
    )


def bounded_history_bytes(path: pathlib.Path, budget: dict[str, Any]) -> bytes:
    """Read one direct, regular artifact without exceeding the history budget."""

    if time.monotonic() > budget["deadline"]:
        raise HistoryLimitError("history scan exceeded --deadline-seconds")
    try:
        metadata = path.lstat()
    except OSError as error:
        raise BenchmarkError(f"cannot inspect {path.name}: {error}") from error
    if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        raise BenchmarkError(f"{path.name} is not a direct regular file")
    if metadata.st_size > budget["max_file_bytes"]:
        raise BenchmarkError(
            f"{path.name} is {metadata.st_size} bytes, above the "
            f"{budget['max_file_bytes']}-byte per-file limit"
        )
    if metadata.st_size > budget["remaining_bytes"]:
        raise HistoryLimitError("history artifacts exceed --max-total-mib")

    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = -1
    try:
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            raise BenchmarkError(f"{path.name} changed type while being opened")
        with os.fdopen(descriptor, "rb") as stream:
            descriptor = -1
            raw = stream.read(budget["max_file_bytes"] + 1)
    except OSError as error:
        raise BenchmarkError(f"cannot read {path.name}: {error}") from error
    finally:
        if descriptor >= 0:
            os.close(descriptor)

    if len(raw) > budget["max_file_bytes"]:
        raise BenchmarkError(f"{path.name} grew beyond the per-file limit while read")
    if time.monotonic() > budget["deadline"]:
        raise HistoryLimitError("history scan exceeded --deadline-seconds")
    if len(raw) > budget["remaining_bytes"]:
        raise HistoryLimitError("history artifacts exceed --max-total-mib")
    budget["remaining_bytes"] -= len(raw)
    budget["bytes_read"] += len(raw)
    budget["files_read"] += 1
    return raw


def bounded_history_json(
    path: pathlib.Path, budget: dict[str, Any]
) -> dict[str, Any]:
    raw = bounded_history_bytes(path, budget)
    try:
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, ValueError, RecursionError) as error:
        raise BenchmarkError(f"invalid JSON in {path.name}: {error}") from error
    if time.monotonic() > budget["deadline"]:
        raise HistoryLimitError("history scan exceeded --deadline-seconds")
    if not isinstance(value, dict):
        raise BenchmarkError(f"{path.name} must contain a JSON object")
    return value


def bounded_history_jsonl(
    path: pathlib.Path, budget: dict[str, Any], *, max_rows: int
) -> list[dict[str, Any]]:
    raw = bounded_history_bytes(path, budget)
    try:
        text_value = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise BenchmarkError(f"invalid UTF-8 in {path.name}: {error}") from error
    rows: list[dict[str, Any]] = []
    try:
        for number, line in enumerate(io.StringIO(text_value), start=1):
            if time.monotonic() > budget["deadline"]:
                raise HistoryLimitError("history scan exceeded --deadline-seconds")
            if not line.strip():
                continue
            if len(rows) >= max_rows:
                raise BenchmarkError(
                    f"{path.name} has more than --max-rows-per-session rows"
                )
            value = json.loads(line)
            if not isinstance(value, dict):
                raise BenchmarkError(f"{path.name}:{number}: row must be an object")
            rows.append(value)
    except (ValueError, RecursionError) as error:
        raise BenchmarkError(f"invalid JSONL in {path.name}: {error}") from error
    return rows


def direct_artifact_exists(path: pathlib.Path) -> bool:
    try:
        path.lstat()
        return True
    except OSError:
        return False


def discover_history_sessions(
    root: pathlib.Path,
    *,
    max_root_entries: int,
    max_sessions: int,
    deadline: float,
) -> tuple[list[pathlib.Path], dict[str, int]]:
    try:
        root_metadata = root.lstat()
    except OSError as error:
        raise BenchmarkError(f"cannot inspect benchmark root {root}: {error}") from error
    if not stat.S_ISDIR(root_metadata.st_mode) or stat.S_ISLNK(root_metadata.st_mode):
        raise BenchmarkError(f"benchmark root must be a real directory: {root}")

    candidates: list[pathlib.Path] = []
    entry_count = 0
    try:
        with os.scandir(root) as entries:
            for entry in entries:
                if time.monotonic() > deadline:
                    raise HistoryLimitError("history scan exceeded --deadline-seconds")
                if entry.name in {
                    DEFAULT_HISTORY_FILENAME,
                    DEFAULT_HISTORY_MARKDOWN_FILENAME,
                }:
                    continue
                entry_count += 1
                if entry_count > max_root_entries:
                    raise HistoryLimitError(
                        "benchmark root exceeds --max-root-entries; refusing a "
                        "filesystem-order-dependent partial scan"
                    )
                if entry.is_symlink() or not entry.is_dir(follow_symlinks=False):
                    continue
                candidate = root / entry.name
                if SESSION_DIR_RE.fullmatch(entry.name) or any(
                    direct_artifact_exists(candidate / name)
                    for name in ("session.json", "scorecard.json", "runs.jsonl")
                ):
                    candidates.append(candidate)
    except OSError as error:
        raise BenchmarkError(f"cannot enumerate benchmark root {root}: {error}") from error

    ordered = sorted(candidates, key=lambda path: path.name)
    selected = ordered[-max_sessions:]
    return selected, {
        "root_entries": entry_count,
        "sessions_discovered": len(ordered),
        "sessions_selected": len(selected),
        "sessions_omitted_oldest": len(ordered) - len(selected),
    }


def history_group_from_rows(
    lane_id: str,
    fixture_id: str,
    cache: str,
    rows: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    states: defaultdict[str, int] = defaultdict(int)
    valid_true = 0
    valid_false = 0
    valid_missing = 0
    total_values: list[int | float] = []
    bundles_present = 0
    for row in rows:
        state_value = row.get("state")
        state = state_value if valid_history_label(state_value, max_length=64) else "missing"
        states[state] += 1
        validity = row.get("valid")
        if validity is True:
            valid_true += 1
        elif validity is False:
            valid_false += 1
        else:
            valid_missing += 1
        metrics = row.get("metrics")
        total = metrics.get("total_ms") if isinstance(metrics, dict) else None
        if is_number(total):
            total_values.append(total)
        if isinstance(row.get("bundle"), str) and row["bundle"]:
            bundles_present += 1

    runs = len(rows)
    succeeded = states.get("succeeded", 0)
    timeouts = states.get("timed_out", 0) + states.get("timeout", 0)
    return {
        "lane_id": lane_id,
        "fixture_id": fixture_id,
        "cache": cache,
        "source_quality": "raw_measured_rows",
        "runs": runs,
        "succeeded": succeeded,
        "non_succeeded": runs - succeeded,
        "timeouts": timeouts,
        "valid_true": valid_true,
        "valid_false": valid_false,
        "valid_missing": valid_missing,
        "success_rate_percent": percentage(succeeded, runs),
        "non_success_rate_percent": percentage(runs - succeeded, runs),
        "timeout_rate_percent": percentage(timeouts, runs),
        "valid_rate_percent": percentage(valid_true, runs),
        "total_ms": {
            "observed": len(total_values),
            "missing": runs - len(total_values),
            "p50": percentile(total_values, 0.50),
            "p95": percentile(total_values, 0.95),
            "min": min(total_values) if total_values else None,
            "max": max(total_values) if total_values else None,
        },
        "states": dict(sorted(states.items())),
        "bundles_present": bundles_present,
    }


def history_groups_from_rows(
    rows: Any, *, max_rows: int, max_groups: int
) -> tuple[list[dict[str, Any]], dict[str, Any], list[str]]:
    if not isinstance(rows, list):
        raise BenchmarkError("scorecard rows must be an array")
    if len(rows) > max_rows:
        raise BenchmarkError(
            f"measured row source has {len(rows)} rows, above --max-rows-per-session"
        )

    grouped: defaultdict[tuple[str, str, str], list[dict[str, Any]]] = defaultdict(list)
    rejected = 0
    measured_rows: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            rejected += 1
            continue
        if row.get("measured") is not True or row.get("historical") is True:
            continue
        identity = (row.get("lane_id"), row.get("fixture_id"), row.get("cache"))
        if not all(valid_history_label(value) for value in identity):
            rejected += 1
            continue
        key = (identity[0], identity[1], identity[2])
        grouped[key].append(row)
        measured_rows.append(row)
    if len(grouped) > max_groups:
        raise BenchmarkError(
            f"measured row source has {len(grouped)} groups, above --max-groups-per-session"
        )

    groups = [
        history_group_from_rows(lane, fixture, cache, selected)
        for (lane, fixture, cache), selected in sorted(grouped.items())
    ]
    totals = history_group_from_rows("*", "*", "*", measured_rows)
    issues = [f"rejected {rejected} malformed measured row(s)"] if rejected else []
    return groups, totals, issues


def non_negative_int(value: Any, default: int = 0) -> int:
    return value if isinstance(value, int) and not isinstance(value, bool) and value >= 0 else default


def history_groups_from_scorecard(
    groups_value: Any, *, max_rows: int, max_groups: int
) -> tuple[list[dict[str, Any]], dict[str, Any], list[str]]:
    if not isinstance(groups_value, list):
        raise BenchmarkError("scorecard groups must be an array")
    if len(groups_value) > max_groups:
        raise BenchmarkError(
            f"scorecard has {len(groups_value)} groups, above --max-groups-per-session"
        )
    groups: list[dict[str, Any]] = []
    rejected = 0
    for value in groups_value:
        if not isinstance(value, dict) or value.get("historical") is True:
            if not isinstance(value, dict):
                rejected += 1
            continue
        identity = (value.get("lane_id"), value.get("fixture_id"), value.get("cache"))
        if not all(valid_history_label(item) for item in identity):
            rejected += 1
            continue
        runs = non_negative_int(value.get("runs"))
        succeeded = min(runs, non_negative_int(value.get("succeeded")))
        valid_true = min(runs, non_negative_int(value.get("valid")))
        timeouts = min(runs, non_negative_int(value.get("timeouts")))
        latency = value.get("latency_ms")
        total = latency.get("total_ms", {}) if isinstance(latency, dict) else {}
        total = total if isinstance(total, dict) else {}
        observed = min(runs, non_negative_int(total.get("observed")))
        groups.append(
            {
                "lane_id": identity[0],
                "fixture_id": identity[1],
                "cache": identity[2],
                "source_quality": "aggregate_scorecard_without_raw_rows",
                "runs": runs,
                "succeeded": succeeded,
                "non_succeeded": runs - succeeded,
                "timeouts": timeouts,
                "valid_true": valid_true,
                "valid_false": None,
                "valid_missing": None,
                "success_rate_percent": percentage(succeeded, runs),
                "non_success_rate_percent": percentage(runs - succeeded, runs),
                "timeout_rate_percent": percentage(timeouts, runs),
                "valid_rate_percent": percentage(valid_true, runs),
                "total_ms": {
                    "observed": observed,
                    "missing": max(0, runs - observed),
                    "p50": total.get("p50") if is_number(total.get("p50")) else None,
                    "p95": total.get("p95") if is_number(total.get("p95")) else None,
                    "min": total.get("min") if is_number(total.get("min")) else None,
                    "max": total.get("max") if is_number(total.get("max")) else None,
                },
                "states": None,
                "bundles_present": sum(
                    isinstance(item, str) and bool(item)
                    for item in value.get("bundles", [])
                )
                if isinstance(value.get("bundles"), list)
                else None,
            }
        )
    groups.sort(key=lambda group: (group["lane_id"], group["fixture_id"], group["cache"]))
    total_runs = sum(group["runs"] for group in groups)
    if total_runs > max_rows:
        raise BenchmarkError(
            f"scorecard groups claim {total_runs} runs, above --max-rows-per-session"
        )
    total_succeeded = sum(group["succeeded"] for group in groups)
    total_timeouts = sum(group["timeouts"] for group in groups)
    total_valid = sum(group["valid_true"] for group in groups)
    totals = {
        "lane_id": "*",
        "fixture_id": "*",
        "cache": "*",
        "source_quality": "aggregate_scorecard_without_raw_rows",
        "runs": total_runs,
        "succeeded": total_succeeded,
        "non_succeeded": total_runs - total_succeeded,
        "timeouts": total_timeouts,
        "valid_true": total_valid,
        "valid_false": None,
        "valid_missing": None,
        "success_rate_percent": percentage(total_succeeded, total_runs),
        "non_success_rate_percent": percentage(total_runs - total_succeeded, total_runs),
        "timeout_rate_percent": percentage(total_timeouts, total_runs),
        "valid_rate_percent": percentage(total_valid, total_runs),
        "total_ms": {
            "observed": 0,
            "missing": total_runs,
            "p50": None,
            "p95": None,
            "min": None,
            "max": None,
        },
        "states": None,
        "bundles_present": None,
    }
    issues = [f"rejected {rejected} malformed scorecard group(s)"] if rejected else []
    issues.append("raw measured rows unavailable; aggregate validity cannot distinguish false from missing")
    return groups, totals, issues


def load_history_session(
    session: pathlib.Path,
    budget: dict[str, Any],
    *,
    max_rows: int,
    max_groups: int,
) -> dict[str, Any]:
    if time.monotonic() > budget["deadline"]:
        raise HistoryLimitError("history scan exceeded --deadline-seconds")
    initial_bytes = budget["bytes_read"]
    initial_files = budget["files_read"]
    issues: list[str] = []
    metadata: dict[str, Any] = {}
    if direct_artifact_exists(session / "session.json"):
        try:
            metadata = bounded_history_json(session / "session.json", budget)
        except HistoryLimitError:
            raise
        except BenchmarkError as error:
            issues.append(str(error))
    else:
        issues.append("session.json is missing")

    groups: list[dict[str, Any]] = []
    totals = history_group_from_rows("*", "*", "*", [])
    source: str | None = None
    scorecard = session / "scorecard.json"
    if direct_artifact_exists(scorecard):
        try:
            value = bounded_history_json(scorecard, budget)
            if value.get("schema_version") != SCHEMA_VERSION:
                raise BenchmarkError("scorecard.json has an unsupported schema_version")
            if isinstance(value.get("rows"), list):
                groups, totals, source_issues = history_groups_from_rows(
                    value["rows"], max_rows=max_rows, max_groups=max_groups
                )
                source = "scorecard.json:rows"
            else:
                groups, totals, source_issues = history_groups_from_scorecard(
                    value.get("groups"), max_rows=max_rows, max_groups=max_groups
                )
                source = "scorecard.json:groups"
            issues.extend(source_issues)
        except HistoryLimitError:
            raise
        except BenchmarkError as error:
            issues.append(str(error))

    if source is not None and not groups:
        issues.append(f"{source} contained no measured groups; tried runs.jsonl")
    if source is None or not groups:
        runs_path = session / "runs.jsonl"
        if direct_artifact_exists(runs_path):
            try:
                rows_value = bounded_history_jsonl(
                    runs_path, budget, max_rows=max_rows
                )
                groups, totals, source_issues = history_groups_from_rows(
                    rows_value, max_rows=max_rows, max_groups=max_groups
                )
                source = "runs.jsonl"
                issues.extend(source_issues)
            except HistoryLimitError:
                raise
            except BenchmarkError as error:
                issues.append(str(error))
        elif source is None:
            issues.append("neither a usable scorecard.json nor runs.jsonl is available")

    status = "ok"
    if not groups:
        status = "incomplete" if source is not None else "unreadable"
    elif issues:
        status = "degraded"
    created = metadata.get("created_utc")
    base_sha = metadata.get("base_sha")
    return {
        "session_id": session.name,
        "created_utc": created if valid_history_label(created, max_length=64) else None,
        "base_sha": base_sha if valid_history_label(base_sha) else None,
        "status": status,
        "source": source,
        "issues": issues,
        "artifact_files_read": budget["files_read"] - initial_files,
        "artifact_bytes_read": budget["bytes_read"] - initial_bytes,
        "totals": totals,
        "groups": groups,
    }


def group_key(group: dict[str, Any]) -> tuple[str, str, str]:
    return (group["lane_id"], group["fixture_id"], group["cache"])


def regression_percent(candidate: int | float, baseline: int | float) -> float | None:
    if baseline <= 0:
        return None
    return round((float(candidate) - float(baseline)) / float(baseline) * 100.0, 4)


def build_history_comparison(
    candidate: dict[str, Any] | None,
    baseline: dict[str, Any] | None,
    thresholds: dict[str, int | float],
    *,
    baseline_explicit: bool,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "candidate_session": candidate and candidate["session_id"],
        "baseline_session": baseline and baseline["session_id"],
        "selection": "explicit_baseline" if baseline_explicit else "previous_session",
        "status": "inconclusive",
        "alerts": [],
        "inconclusive_reasons": [],
        "groups": [],
        "summary": {
            "groups": 0,
            "passed": 0,
            "regressed": 0,
            "inconclusive": 0,
            "alerts": 0,
        },
    }
    if candidate is None:
        result["inconclusive_reasons"].append("no candidate session exists")
        return result
    if baseline is None:
        result["inconclusive_reasons"].append("no baseline/previous session exists")
        return result
    if candidate["status"] in {"incomplete", "unreadable"}:
        result["inconclusive_reasons"].append(
            f"candidate session is {candidate['status']}"
        )
    if baseline["status"] in {"incomplete", "unreadable"}:
        result["inconclusive_reasons"].append(
            f"baseline session is {baseline['status']}"
        )

    candidate_groups = {group_key(group): group for group in candidate["groups"]}
    baseline_groups = {group_key(group): group for group in baseline["groups"]}
    keys = sorted(set(candidate_groups) | set(baseline_groups))
    for key in keys:
        current = candidate_groups.get(key)
        reference = baseline_groups.get(key)
        item: dict[str, Any] = {
            "lane_id": key[0],
            "fixture_id": key[1],
            "cache": key[2],
            "status": "inconclusive",
            "baseline_runs": reference and reference["runs"],
            "candidate_runs": current and current["runs"],
            "metrics": {},
            "alerts": [],
            "inconclusive_reasons": [],
        }
        if current is None:
            item["inconclusive_reasons"].append("candidate group is missing")
        elif reference is None:
            item["inconclusive_reasons"].append("baseline group is missing")
        elif (
            current["runs"] < thresholds["min_samples"]
            or reference["runs"] < thresholds["min_samples"]
        ):
            item["inconclusive_reasons"].append(
                f"fewer than {thresholds['min_samples']} samples in candidate or baseline"
            )
        else:
            for percentile_name in HISTORY_COMPARISON_METRICS:
                candidate_value = current["total_ms"][percentile_name]
                baseline_value = reference["total_ms"][percentile_name]
                limit = thresholds[f"max_{percentile_name}_regression_percent"]
                change = (
                    regression_percent(candidate_value, baseline_value)
                    if is_number(candidate_value) and is_number(baseline_value)
                    else None
                )
                item["metrics"][f"total_ms.{percentile_name}"] = {
                    "baseline": baseline_value,
                    "candidate": candidate_value,
                    "change_percent": change,
                    "max_regression_percent": limit,
                }
                if change is None:
                    item["inconclusive_reasons"].append(
                        f"total_ms.{percentile_name} is missing or has a zero baseline"
                    )
                elif change > limit:
                    item["alerts"].append(
                        {
                            "metric": f"total_ms.{percentile_name}",
                            "baseline": baseline_value,
                            "candidate": candidate_value,
                            "change": change,
                            "limit": limit,
                            "unit": "percent",
                        }
                    )

            rate_checks = (
                (
                    "non_success_rate_percent",
                    "increase",
                    "max_non_success_rate_increase_points",
                ),
                ("timeout_rate_percent", "increase", "max_timeout_rate_increase_points"),
                ("valid_rate_percent", "drop", "max_valid_rate_drop_points"),
            )
            for metric, direction, threshold_name in rate_checks:
                candidate_value = current[metric]
                baseline_value = reference[metric]
                limit = thresholds[threshold_name]
                delta = None
                if is_number(candidate_value) and is_number(baseline_value):
                    delta = round(
                        float(candidate_value) - float(baseline_value)
                        if direction == "increase"
                        else float(baseline_value) - float(candidate_value),
                        4,
                    )
                item["metrics"][metric] = {
                    "baseline": baseline_value,
                    "candidate": candidate_value,
                    f"{direction}_points": delta,
                    f"max_{direction}_points": limit,
                }
                if delta is None:
                    item["inconclusive_reasons"].append(f"{metric} is missing")
                elif delta > limit:
                    item["alerts"].append(
                        {
                            "metric": metric,
                            "baseline": baseline_value,
                            "candidate": candidate_value,
                            "change": delta,
                            "limit": limit,
                            "unit": "percentage_points",
                        }
                    )

        if item["alerts"]:
            item["status"] = "regressed"
        elif item["inconclusive_reasons"]:
            item["status"] = "inconclusive"
        else:
            item["status"] = "passed"
        for alert in item["alerts"]:
            result["alerts"].append(
                {
                    "lane_id": key[0],
                    "fixture_id": key[1],
                    "cache": key[2],
                    **alert,
                }
            )
        result["groups"].append(item)

    inconclusive_groups = sum(group["status"] == "inconclusive" for group in result["groups"])
    if result["alerts"]:
        result["status"] = "regressed"
    elif result["inconclusive_reasons"] or inconclusive_groups or not result["groups"]:
        result["status"] = "inconclusive"
    else:
        result["status"] = "passed"
    result["summary"] = {
        "groups": len(result["groups"]),
        "passed": sum(group["status"] == "passed" for group in result["groups"]),
        "regressed": sum(group["status"] == "regressed" for group in result["groups"]),
        "inconclusive": inconclusive_groups,
        "alerts": len(result["alerts"]),
    }
    return result


def build_history_series(sessions: Sequence[dict[str, Any]]) -> list[dict[str, Any]]:
    series: defaultdict[tuple[str, str, str], list[dict[str, Any]]] = defaultdict(list)
    for session in sessions:
        for group in session["groups"]:
            series[group_key(group)].append(
                {
                    "session_id": session["session_id"],
                    "base_sha": session["base_sha"],
                    "runs": group["runs"],
                    "succeeded": group["succeeded"],
                    "valid_true": group["valid_true"],
                    "non_success_rate_percent": group["non_success_rate_percent"],
                    "valid_rate_percent": group["valid_rate_percent"],
                    "p50_total_ms": group["total_ms"]["p50"],
                    "p95_total_ms": group["total_ms"]["p95"],
                }
            )
    return [
        {
            "lane_id": key[0],
            "fixture_id": key[1],
            "cache": key[2],
            "points": points,
        }
        for key, points in sorted(series.items())
    ]


def markdown_cell(value: Any) -> str:
    if value is None:
        return "—"
    return str(value).replace("|", "\\|").replace("\n", " ")


def signed(value: Any, suffix: str = "") -> str:
    if not is_number(value):
        return "—"
    return f"{float(value):+.2f}{suffix}"


def render_history(history: dict[str, Any]) -> str:
    comparison = history["comparison"]
    lines = [
        "# Development benchmark history",
        "",
        f"Input fingerprint: `{history['input_fingerprint_sha256']}`",
        "",
        "Sessions are ordered by deterministic session directory name. Failures, timeouts, "
        "missing validity, and missing latency remain visible; no value is imputed.",
        "",
        "## Session dashboard",
        "",
        "| Session | Created | Base | Status | Runs | Success | Valid | p50 total | p95 total | Issues |",
        "|---|---|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    for session in history["sessions"]:
        totals = session["totals"]
        lines.append(
            "| {session} | {created} | {base} | {status} | {runs} | {success} | {valid} | "
            "{p50} | {p95} | {issues} |".format(
                session=markdown_cell(session["session_id"]),
                created=markdown_cell(session["created_utc"]),
                base=markdown_cell(session["base_sha"][:12] if session["base_sha"] else None),
                status=session["status"],
                runs=totals["runs"],
                success=markdown_cell(totals["success_rate_percent"]),
                valid=markdown_cell(totals["valid_rate_percent"]),
                p50=markdown_cell(totals["total_ms"]["p50"]),
                p95=markdown_cell(totals["total_ms"]["p95"]),
                issues=len(session["issues"]),
            )
        )

    lines.extend(
        [
            "",
            "## Latest comparison",
            "",
            f"Status: **{comparison['status']}**. Candidate "
            f"`{comparison['candidate_session'] or 'none'}` versus "
            f"`{comparison['baseline_session'] or 'none'}`.",
            "",
            "| Lane | Fixture | Cache | Runs base/current | p50 change | p95 change | "
            "Non-success Δ | Valid drop | Verdict |",
            "|---|---|---:|---:|---:|---:|---:|---:|---|",
        ]
    )
    for group in comparison["groups"]:
        metrics = group["metrics"]
        lines.append(
            "| {lane} | {fixture} | {cache} | {baseline}/{candidate} | {p50} | {p95} | "
            "{failure} | {valid} | {status} |".format(
                lane=markdown_cell(group["lane_id"]),
                fixture=markdown_cell(group["fixture_id"]),
                cache=markdown_cell(group["cache"]),
                baseline=markdown_cell(group["baseline_runs"]),
                candidate=markdown_cell(group["candidate_runs"]),
                p50=signed(metrics.get("total_ms.p50", {}).get("change_percent"), "%"),
                p95=signed(metrics.get("total_ms.p95", {}).get("change_percent"), "%"),
                failure=signed(
                    metrics.get("non_success_rate_percent", {}).get("increase_points"), "pp"
                ),
                valid=signed(metrics.get("valid_rate_percent", {}).get("drop_points"), "pp"),
                status=group["status"],
            )
        )

    lines.extend(["", "## Regression alerts", ""])
    if comparison["alerts"]:
        for alert in comparison["alerts"]:
            unit = "%" if alert["unit"] == "percent" else " percentage points"
            lines.append(
                "- `{lane}/{fixture}/{cache}` {metric}: {change:+.4g}{unit} "
                "(limit {limit:g}{unit}).".format(
                    lane=alert["lane_id"],
                    fixture=alert["fixture_id"],
                    cache=alert["cache"],
                    metric=alert["metric"],
                    change=alert["change"],
                    limit=alert["limit"],
                    unit=unit,
                )
            )
    else:
        lines.append("- No configured threshold was breached.")

    inconclusive = [
        f"comparison: {reason}" for reason in comparison["inconclusive_reasons"]
    ]
    for group in comparison["groups"]:
        inconclusive.extend(
            f"{group['lane_id']}/{group['fixture_id']}/{group['cache']}: {reason}"
            for reason in group["inconclusive_reasons"]
        )
    lines.extend(["", "## Missing or inconclusive evidence", ""])
    if inconclusive:
        lines.extend(f"- {reason}" for reason in inconclusive)
    else:
        lines.append("- None.")
    for session in history["sessions"]:
        lines.extend(
            f"- `{session['session_id']}`: {issue}" for issue in session["issues"]
        )

    thresholds = history["policy"]["thresholds"]
    limits = history["policy"]["limits"]
    lines.extend(
        [
            "",
            "## Policy and bounds",
            "",
            f"- Minimum samples per compared group: {thresholds['min_samples']}",
            f"- Maximum p50/p95 regressions: {thresholds['max_p50_regression_percent']}% / "
            f"{thresholds['max_p95_regression_percent']}%",
            f"- Maximum non-success/timeout increases: "
            f"{thresholds['max_non_success_rate_increase_points']}pp / "
            f"{thresholds['max_timeout_rate_increase_points']}pp",
            f"- Maximum validated-rate drop: {thresholds['max_valid_rate_drop_points']}pp",
            f"- Scan bounds: {limits['max_sessions']} sessions, "
            f"{limits['max_root_entries']} root entries, {limits['max_rows_per_session']} rows/session, "
            f"{limits['max_groups_per_session']} groups/session, {limits['max_file_bytes']} bytes/file, "
            f"{limits['max_total_bytes']} bytes total, {limits['deadline_seconds']} seconds.",
            "",
        ]
    )
    return "\n".join(lines)


def sample_specs(
    lanes: list[dict[str, Any]],
    fixtures: list[dict[str, Any]],
    caches: list[str],
    repetitions: int,
) -> list[dict[str, Any]]:
    specs: list[dict[str, Any]] = []
    sequence = 0
    for fixture in fixtures:
        for lane in lanes:
            for cache in caches:
                if cache == "warm":
                    sequence += 1
                    specs.append(
                        {
                            "sequence": sequence,
                            "role": "warmup",
                            "measured": False,
                            "fixture": fixture,
                            "lane": lane,
                            "cache": cache,
                            "repetition": 0,
                        }
                    )
                for repetition in range(1, repetitions + 1):
                    sequence += 1
                    specs.append(
                        {
                            "sequence": sequence,
                            "role": "sample",
                            "measured": True,
                            "fixture": fixture,
                            "lane": lane,
                            "cache": cache,
                            "repetition": repetition,
                        }
                    )
    return specs


def describe_plan(
    manifest: dict[str, Any], args: argparse.Namespace, repo: pathlib.Path, base_sha: str
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    lanes = select_objects(manifest["lanes"], args.lane, "lane")
    fixtures = select_objects(manifest["fixtures"], args.fixture, "fixture")
    unavailable = [lane["id"] for lane in lanes if lane["kind"] != "command" or not lane.get("argv")]
    if unavailable:
        raise BenchmarkError(
            "selected manual lanes need adapter argv in a custom manifest: "
            + ", ".join(unavailable)
        )
    caches = args.cache or ["cold", "warm"]
    repetitions = args.repetitions or manifest["defaults"].get("repetitions", 5)
    if not isinstance(repetitions, int) or not 1 <= repetitions <= 20:
        raise BenchmarkError("repetitions must be in 1..20")
    specs = sample_specs(lanes, fixtures, caches, repetitions)
    network_specs = [spec for spec in specs if spec["role"] == "sample" and spec["lane"]["network"]]
    estimated_cost = sum(
        float(spec["lane"].get("estimated_max_cost_usd", 0)) for spec in network_specs
    )
    deadline = args.deadline or manifest["defaults"].get("deadline_seconds", 600)
    if not isinstance(deadline, int) or not 10 <= deadline <= 3600:
        raise BenchmarkError("deadline must be in 10..3600 seconds")
    summary = {
        "schema_version": SCHEMA_VERSION,
        "base_sha": base_sha,
        "repo": str(repo),
        "lanes": [lane["id"] for lane in lanes],
        "fixtures": [fixture["id"] for fixture in fixtures],
        "caches": caches,
        "repetitions": repetitions,
        "measured_runs": sum(spec["measured"] for spec in specs),
        "warmups": sum(spec["role"] == "warmup" for spec in specs),
        "maximum_wall_seconds": len(specs) * deadline,
        "estimated_max_provider_cost_usd": round(estimated_cost, 4),
        "network_authorization_required": bool(network_specs),
        "cold_semantics": "a unique, initially absent benchmark-owned CARGO_TARGET_DIR per sample",
        "warm_semantics": "one bounded benchmark-owned CARGO_TARGET_DIR per lane, fixture-seeded then reused",
    }
    return specs, summary


def execute(args: argparse.Namespace) -> int:
    invocation_root, primary = discover_repo(pathlib.Path.cwd())
    manifest_path = (invocation_root / args.manifest).resolve() if not args.manifest.is_absolute() else args.manifest.resolve()
    manifest = validate_manifest(load_json(manifest_path))
    base_sha = git_value(invocation_root, "rev-parse", "--verify", f"{args.base}^{{commit}}")
    specs, plan_summary = describe_plan(manifest, args, invocation_root, base_sha)

    if args.dry_run:
        rendered_specs = [
            {
                "sequence": spec["sequence"],
                "role": spec["role"],
                "lane": spec["lane"]["id"],
                "fixture": spec["fixture"]["id"],
                "cache": spec["cache"],
                "repetition": spec["repetition"],
                "network": spec["lane"]["network"] if spec["role"] == "sample" else False,
            }
            for spec in specs
        ]
        print(json.dumps({**plan_summary, "specs": rendered_specs}, indent=2, sort_keys=True))
        return 0

    if plan_summary["network_authorization_required"] and not args.allow_network:
        raise BenchmarkError(
            "selected lanes may contact providers; inspect --dry-run and pass --allow-network explicitly"
        )
    if plan_summary["network_authorization_required"] and args.max_cost_usd is None:
        raise BenchmarkError("network runs require an explicit --max-cost-usd budget")
    if args.max_cost_usd is not None and plan_summary["estimated_max_provider_cost_usd"] > args.max_cost_usd:
        raise BenchmarkError(
            "estimated worst-case provider cost "
            f"${plan_summary['estimated_max_provider_cost_usd']:.2f} exceeds --max-cost-usd ${args.max_cost_usd:.2f}"
        )
    if len(specs) > args.max_runs:
        raise BenchmarkError(f"planned run count {len(specs)} exceeds --max-runs {args.max_runs}")
    if plan_summary["maximum_wall_seconds"] > args.max_wall_hours * 3600:
        raise BenchmarkError(
            f"planned deadline envelope exceeds --max-wall-hours {args.max_wall_hours:g}"
        )

    needs_roko_binary = any(
        spec["lane"]["id"] in {"current-roko", "roko-fast"} for spec in specs
    )
    if needs_roko_binary and args.roko_bin is None:
        raise BenchmarkError(
            "stock Roko lanes require --roko-bin plus --binary-base; the runner never guesses a build"
        )
    roko_bin = (
        args.roko_bin.expanduser().resolve()
        if args.roko_bin is not None
        else (primary / "target/debug/roko").resolve()
    )
    binary_identity = command_version_hash(roko_bin)
    if needs_roko_binary and (
        not roko_bin.is_file() or not os.access(roko_bin, os.X_OK)
    ):
        raise BenchmarkError(f"Roko binary was not executable: {roko_bin}")
    asserted_binary_base: str | None = None
    if args.binary_base:
        asserted_binary_base = git_value(
            invocation_root,
            "rev-parse",
            "--verify",
            f"{args.binary_base}^{{commit}}",
        )
        if asserted_binary_base != base_sha:
            raise BenchmarkError(
                f"--binary-base resolves to {asserted_binary_base}, not benchmark base {base_sha}"
            )
    elif needs_roko_binary and not args.allow_unverified_binary:
        raise BenchmarkError(
            "binary/base identity is unproved; pass --binary-base <same-commit> or the explicit "
            "--allow-unverified-binary escape hatch"
        )
    binary_identity["asserted_base_sha"] = asserted_binary_base
    binary_identity["base_identity"] = (
        "operator_attested"
        if asserted_binary_base
        else "explicitly_unverified"
        if needs_roko_binary
        else "not_required"
    )

    output_root = args.output_root
    if not output_root.is_absolute():
        output_root = primary / output_root
    output_root = output_root.expanduser()
    if output_root.is_symlink():
        raise BenchmarkError(f"output root must not be a symlink: {output_root}")
    output_root = output_root.resolve()
    private_mkdir(output_root)
    session_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    session = output_root / session_id
    session.mkdir(mode=0o700)
    worktree_root = session / "worktrees"
    private_mkdir(worktree_root)
    cache_root = session / "caches"
    private_mkdir(cache_root)
    private_mkdir(cache_root / "cold")
    private_mkdir(cache_root / "warm")
    write_json(
        cache_root / "OWNER.json",
        {
            "schema_version": SCHEMA_VERSION,
            "session_id": session_id,
            "cache_root": str(cache_root.resolve()),
            "normal_repository_target": str((primary / "target").resolve()),
        },
    )
    deadline = args.deadline or manifest["defaults"].get("deadline_seconds", 600)
    settlement_headroom = 30 if deadline >= 120 else max(1, deadline // 4)
    runner_deadline = max(1, deadline - settlement_headroom)
    session_record = {
        **plan_summary,
        "session_id": session_id,
        "session": str(session),
        "created_utc": utc_now(),
        "manifest": str(manifest_path),
        "manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
        "roko_binary": binary_identity,
        "limits": {
            "deadline_seconds": deadline,
            "max_cost_usd": args.max_cost_usd,
            "max_runs": args.max_runs,
            "max_wall_hours": args.max_wall_hours,
            "min_free_gib": args.min_free_gib,
            "min_free_percent": args.min_free_percent,
            "max_session_gib": args.max_session_gib,
            "max_cache_gib": args.max_cache_gib,
        },
        "cleanup": {
            "worktrees": "all settled evidence-bearing benchmark worktrees"
            if not args.keep_worktrees
            else "retained by explicit request",
            "cold_targets": "disposed after evidence finalization"
            if not args.keep_targets
            else "retained by explicit request",
            "warm_targets": "retained for reuse and later reviewed GC",
        },
    }
    write_json(session / "session.json", session_record)
    print(f"[benchmark] session={session}", file=sys.stderr)
    print(f"[benchmark] base={base_sha}", file=sys.stderr)

    evidence_script = pathlib.Path(__file__).resolve().with_name("run_evidence.py")
    measured_failures = 0
    for spec in specs:
        lane = spec["lane"]
        fixture = spec["fixture"]
        sample_id = (
            f"sample-{spec['sequence']:04d}-{lane['id']}-{fixture['id']}-"
            f"{spec['cache']}-r{spec['repetition']}"
        )
        worktree = worktree_root / sample_id
        if spec["cache"] == "cold":
            target = cache_root / "cold" / sample_id
            if target.exists() or target.is_symlink():
                raise BenchmarkError(f"cold target unexpectedly exists: {target}")
        else:
            target = cache_root / "warm" / lane["id"]
        admission = resource_admission(
            session,
            target,
            min_free_gib=args.min_free_gib,
            min_free_percent=args.min_free_percent,
            max_session_gib=args.max_session_gib,
            max_cache_gib=args.max_cache_gib,
        )
        append_jsonl(
            session / "admission.jsonl",
            {"sample_id": sample_id, **admission},
        )
        if not admission["admitted"]:
            raise BenchmarkError(
                f"resource admission rejected {sample_id}: {'; '.join(admission['errors'])}"
            )

        print(
            f"[benchmark] {spec['sequence']}/{len(specs)} {spec['role']} "
            f"{lane['id']} {fixture['id']} {spec['cache']} r{spec['repetition']}",
            file=sys.stderr,
        )
        create_worktree(invocation_root, worktree, base_sha)
        plan_path = worktree / fixture["plan"]
        if not plan_path.is_dir():
            removed, cleanup_error = remove_owned_worktree(
                invocation_root, worktree_root, worktree
            )
            append_jsonl(
                session / "worktrees.jsonl",
                {
                    "sample_id": sample_id,
                    "path": str(worktree),
                    "created": True,
                    "cleanup": {
                        "attempted": True,
                        "removed": removed,
                        "error": cleanup_error,
                    },
                    "timestamp": utc_now(),
                },
            )
            raise BenchmarkError(
                f"fixture plan is absent at fixed base {base_sha}: {fixture['plan']}"
            )
        bundle_root = session / "samples" / sample_id / "evidence"
        private_mkdir(bundle_root)
        context = {
            "base_sha": base_sha,
            "bundle": "{bundle}",
            "cache": spec["cache"],
            "deadline": str(deadline),
            "fixture": fixture["id"],
            "lane": lane["id"],
            "plan": fixture["plan"],
            "repetition": str(spec["repetition"]),
            "repo": str(invocation_root),
            "roko_bin": str(roko_bin),
            "runner_deadline": str(runner_deadline),
            "settlement_headroom": str(settlement_headroom),
            "target_dir": str(target),
            "worktree": str(worktree),
        }
        if spec["role"] == "warmup":
            command = expand_argv(fixture["warmup_argv"], context)
            evidence_args: list[str] = []
        else:
            command = expand_argv(lane["argv"], context)
            evidence_args = list(lane.get("evidence_args", []))
        env = os.environ.copy()
        env.update(
            {
                "CARGO_TARGET_DIR": str(target),
                "LC_ALL": "C",
                "NO_COLOR": "1",
                "ROKO_BENCH_BASE_SHA": base_sha,
                "ROKO_BENCH_CACHE": spec["cache"],
                "ROKO_BENCH_FIXTURE": fixture["id"],
                "ROKO_BENCH_LANE": lane["id"],
                "ROKO_BENCH_NETWORK_ALLOWED": "1" if args.allow_network else "0",
                "ROKO_BENCH_REPETITION": str(spec["repetition"]),
                "SKIP_FRONTEND_BUILD": "1",
                "TZ": "UTC",
            }
        )
        if spec["role"] == "sample":
            for name in lane.get("unset_env", []):
                env.pop(name, None)
            env.update({name: expand(raw, context) for name, raw in lane.get("env", {}).items()})
        env["CARGO_NET_OFFLINE"] = "true"
        wrapper = [
            sys.executable,
            str(evidence_script),
            "--deadline",
            str(deadline),
            "--label",
            f"bench-{lane['id']}-{fixture['id']}-{spec['role']}",
            "--bundle-root",
            str(bundle_root),
            "--cwd",
            str(worktree),
            "--admit-resources",
            "--min-free-gib",
            str(args.min_free_gib),
            "--min-free-percent",
            str(args.min_free_percent),
            *evidence_args,
            "--",
            *command,
        ]
        started = time.monotonic()
        try:
            completed = subprocess.run(
                wrapper,
                cwd=worktree,
                env=env,
                check=False,
                stdin=subprocess.DEVNULL,
            )
            wrapper_exit = completed.returncode
        except OSError as error:
            wrapper_exit = 126
            append_jsonl(
                session / "runner-errors.jsonl",
                {"sample_id": sample_id, "timestamp": utc_now(), "error": str(error)},
            )
        wall_ms = round((time.monotonic() - started) * 1000)
        bundle = resolve_bundle(bundle_root)
        identity = {
            "sample_id": sample_id,
            "sequence": spec["sequence"],
            "role": spec["role"],
            "measured": spec["measured"],
            "lane_id": lane["id"],
            "fixture_id": fixture["id"],
            "cache": spec["cache"],
            "repetition": spec["repetition"],
            "base_sha": base_sha,
            "target_dir": str(target),
            "worktree": str(worktree),
        }
        row = evidence_row(bundle, identity, wrapper_exit, wall_ms)
        post_target = tree_size(target)
        target_limit_bytes = round(args.max_cache_gib * 1024**3)
        row["target"] = {
            "path": str(target),
            **post_target,
            "limit_bytes": target_limit_bytes,
            "within_limit": post_target["complete"]
            and post_target["bytes"] <= target_limit_bytes,
        }
        if not row["target"]["within_limit"]:
            row["state"] = "resource_limit_exceeded"
            row["valid"] = False

        process_absent, process_error = bundle_process_group_absent(bundle)
        target_cleanup = {
            "attempted": False,
            "removed": False,
            "error": None,
            "process_absence_confirmed": process_absent,
        }
        if spec["cache"] == "cold" and not args.keep_targets:
            target_cleanup["attempted"] = True
            target_cleanup["removed"], target_cleanup["error"] = remove_owned_cold_target(
                cache_root=cache_root,
                candidate=target,
                repo_target=primary / "target",
                session_id=session_id,
                process_absent=process_absent,
                process_error=process_error,
            )
            if not target_cleanup["removed"]:
                row["state"] = "cleanup_failed"
                row["valid"] = False
        append_jsonl(
            session / "targets.jsonl",
            {
                "sample_id": sample_id,
                "cache": spec["cache"],
                "target": row["target"],
                "cleanup": target_cleanup,
                "timestamp": utc_now(),
            },
        )

        worktree_cleanup = {"attempted": False, "removed": False, "error": None}
        if not args.keep_worktrees and process_absent and bundle is not None:
            worktree_cleanup["attempted"] = True
            worktree_cleanup["removed"], worktree_cleanup["error"] = remove_owned_worktree(
                invocation_root, worktree_root, worktree
            )
            if not worktree_cleanup["removed"]:
                row["state"] = "cleanup_failed"
                row["valid"] = False
        elif not args.keep_worktrees:
            worktree_cleanup["error"] = process_error or "evidence bundle is missing"
            row["state"] = "cleanup_failed"
            row["valid"] = False
        append_jsonl(
            session / "worktrees.jsonl",
            {
                "sample_id": sample_id,
                "path": str(worktree),
                "created": True,
                "cleanup": worktree_cleanup,
                "timestamp": utc_now(),
            },
        )
        row["cleanup"] = {
            "cold_target": target_cleanup,
            "worktree": worktree_cleanup,
        }
        append_jsonl(session / "runs.jsonl", row)
        succeeded = row["state"] == "succeeded" and row["valid"] is True
        if spec["measured"] and not succeeded:
            measured_failures += 1
        if spec["role"] == "warmup" and not succeeded:
            measured_failures += 1
            append_jsonl(
                session / "runner-errors.jsonl",
                {
                    "sample_id": sample_id,
                    "timestamp": utc_now(),
                    "error": "warm cache seed failed; warm samples were not executed",
                },
            )
            break
        if args.fail_fast and spec["measured"] and not succeeded:
            break

    baseline_paths = [
        (invocation_root / DEFAULT_BASELINES).resolve(),
        *[path.expanduser().resolve() for path in args.baseline],
    ]
    scorecard = summarize_session(session, baseline_paths)
    print(json.dumps({"session": str(session), "runs": scorecard["runs"]}, indent=2, sort_keys=True))
    return 1 if measured_failures else 0


def list_manifest(args: argparse.Namespace) -> int:
    root, _ = discover_repo(pathlib.Path.cwd())
    path = args.manifest if args.manifest.is_absolute() else root / args.manifest
    manifest = validate_manifest(load_json(path.resolve()))
    value = {
        "manifest": str(path.resolve()),
        "lanes": [
            {
                "id": lane["id"],
                "kind": lane["kind"],
                "network": lane["network"],
                "enabled_by_default": lane.get("enabled_by_default", True),
                "runnable": lane.get("argv") is not None,
            }
            for lane in manifest["lanes"]
        ],
        "fixtures": [
            {
                "id": fixture["id"],
                "category": fixture.get("category"),
                "plan": fixture["plan"],
            }
            for fixture in manifest["fixtures"]
        ],
        "defaults": manifest["defaults"],
    }
    if args.json:
        print(json.dumps(value, indent=2, sort_keys=True))
    else:
        print(f"Manifest: {value['manifest']}")
        print("Lanes:")
        for lane in value["lanes"]:
            status = "runnable" if lane["runnable"] else "baseline/import only"
            network = "network" if lane["network"] else "offline"
            default = "default" if lane["enabled_by_default"] else "opt-in"
            print(f"  {lane['id']}: {status}, {network}, {default}")
        print("Fixtures:")
        for fixture in value["fixtures"]:
            print(f"  {fixture['id']}: {fixture['category']} ({fixture['plan']})")
    return 0


def summarize_command(args: argparse.Namespace) -> int:
    root, _ = discover_repo(pathlib.Path.cwd())
    paths = [(root / DEFAULT_BASELINES).resolve()]
    paths.extend(path.expanduser().resolve() for path in args.baseline)
    scorecard = summarize_session(args.session, paths)
    print(json.dumps(scorecard, indent=2, sort_keys=True))
    return 0


def positive_int_argument(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be an integer") from error
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def positive_float_argument(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a number") from error
    if not math.isfinite(parsed) or parsed <= 0:
        raise argparse.ArgumentTypeError("must be a finite number greater than zero")
    return parsed


def non_negative_float_argument(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a number") from error
    if not math.isfinite(parsed) or parsed < 0:
        raise argparse.ArgumentTypeError("must be a finite number at least zero")
    return parsed


def validate_session_selector(value: str, flag: str) -> str:
    if (
        not value
        or value in {".", ".."}
        or len(value) > 255
        or pathlib.PurePath(value).name != value
        or "/" in value
        or "\\" in value
    ):
        raise BenchmarkError(f"{flag} must be one direct session directory name")
    return value


def absolute_from_primary(path: pathlib.Path, primary: pathlib.Path) -> pathlib.Path:
    expanded = path.expanduser()
    if not expanded.is_absolute():
        expanded = primary / expanded
    return pathlib.Path(os.path.abspath(expanded))


def history_command(args: argparse.Namespace) -> int:
    _, primary = discover_repo(pathlib.Path.cwd())
    history_root = absolute_from_primary(args.root, primary)
    started = time.monotonic()
    deadline = started + args.deadline_seconds
    session_paths, discovery = discover_history_sessions(
        history_root,
        max_root_entries=args.max_root_entries,
        max_sessions=args.max_sessions,
        deadline=deadline,
    )
    max_file_bytes = round(args.max_file_mib * 1024 * 1024)
    max_total_bytes = round(args.max_total_mib * 1024 * 1024)
    budget: dict[str, Any] = {
        "deadline": deadline,
        "max_file_bytes": max_file_bytes,
        "remaining_bytes": max_total_bytes,
        "bytes_read": 0,
        "files_read": 0,
    }
    sessions = [
        load_history_session(
            path,
            budget,
            max_rows=args.max_rows_per_session,
            max_groups=args.max_groups_per_session,
        )
        for path in session_paths
    ]
    by_id = {session["session_id"]: session for session in sessions}
    ordered_ids = [session["session_id"] for session in sessions]

    candidate_id = (
        validate_session_selector(args.candidate_session, "--candidate-session")
        if args.candidate_session is not None
        else ordered_ids[-1]
        if ordered_ids
        else None
    )
    if candidate_id is not None and candidate_id not in by_id:
        raise BenchmarkError(
            f"candidate session {candidate_id!r} is not in the newest "
            f"{args.max_sessions} selected sessions; increase --max-sessions"
        )
    candidate = by_id.get(candidate_id) if candidate_id is not None else None
    candidate_index = ordered_ids.index(candidate_id) if candidate_id is not None else -1

    baseline_id: str | None = None
    if args.baseline_session is not None:
        baseline_id = validate_session_selector(args.baseline_session, "--baseline-session")
        if baseline_id not in by_id:
            raise BenchmarkError(
                f"baseline session {baseline_id!r} is not in the newest "
                f"{args.max_sessions} selected sessions; increase --max-sessions"
            )
        if candidate_id == baseline_id:
            raise BenchmarkError("candidate and baseline sessions must differ")
        if ordered_ids.index(baseline_id) >= candidate_index:
            raise BenchmarkError("--baseline-session must sort before the candidate session")
    elif candidate_index > 0:
        baseline_id = ordered_ids[candidate_index - 1]
    baseline = by_id.get(baseline_id) if baseline_id is not None else None

    thresholds: dict[str, int | float] = {
        "min_samples": args.min_samples,
        "max_p50_regression_percent": args.max_p50_regression_percent,
        "max_p95_regression_percent": args.max_p95_regression_percent,
        "max_non_success_rate_increase_points": args.max_non_success_rate_increase_points,
        "max_timeout_rate_increase_points": args.max_timeout_rate_increase_points,
        "max_valid_rate_drop_points": args.max_valid_rate_drop_points,
    }
    comparison = build_history_comparison(
        candidate,
        baseline,
        thresholds,
        baseline_explicit=args.baseline_session is not None,
    )
    limits = {
        "max_sessions": args.max_sessions,
        "max_root_entries": args.max_root_entries,
        "max_rows_per_session": args.max_rows_per_session,
        "max_groups_per_session": args.max_groups_per_session,
        "max_file_bytes": max_file_bytes,
        "max_total_bytes": max_total_bytes,
        "deadline_seconds": args.deadline_seconds,
        "files_read": budget["files_read"],
        "bytes_read": budget["bytes_read"],
    }
    fingerprint_value = {
        "sessions": sessions,
        "thresholds": thresholds,
        "candidate_session": candidate_id,
        "baseline_session": baseline_id,
    }
    input_fingerprint = hashlib.sha256(
        json.dumps(
            fingerprint_value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode("utf-8")
    ).hexdigest()
    history = {
        "schema_version": SCHEMA_VERSION,
        "kind": "roko.dev-benchmark-history",
        "root": str(history_root),
        "input_fingerprint_sha256": input_fingerprint,
        "latest_session_created_utc": sessions[-1]["created_utc"] if sessions else None,
        "policy": {
            "ordering": "ascending direct session directory name; newest bounded suffix retained",
            "failures_and_timeouts": "retained; every non-succeeded row affects the non-success rate",
            "missing_metrics": "reported as inconclusive; never imputed",
            "exit": "regression exits 1 unless --report-only; inconclusive exits 1 only with --fail-on-inconclusive",
            "thresholds": thresholds,
            "limits": limits,
        },
        "discovery": discovery,
        "sessions": sessions,
        "series": build_history_series(sessions),
        "comparison": comparison,
    }

    output_dir = absolute_from_primary(args.output_dir, primary) if args.output_dir else history_root
    if output_dir.exists() or output_dir.is_symlink():
        try:
            output_metadata = output_dir.lstat()
        except OSError as error:
            raise BenchmarkError(f"cannot inspect output directory {output_dir}: {error}") from error
        if not stat.S_ISDIR(output_metadata.st_mode) or stat.S_ISLNK(output_metadata.st_mode):
            raise BenchmarkError(f"history output directory must be a real directory: {output_dir}")
    else:
        private_mkdir(output_dir)
    json_output = output_dir / DEFAULT_HISTORY_FILENAME
    markdown_output = output_dir / DEFAULT_HISTORY_MARKDOWN_FILENAME
    write_json(json_output, history)
    write_text(markdown_output, render_history(history))
    print(
        json.dumps(
            {
                "status": comparison["status"],
                "alerts": len(comparison["alerts"]),
                "inconclusive_groups": comparison.get("summary", {}).get("inconclusive", 0),
                "json": str(json_output),
                "markdown": str(markdown_output),
            },
            indent=2,
            sort_keys=True,
        )
    )
    if args.report_only:
        return 0
    if comparison["status"] == "regressed":
        return 1
    if args.fail_on_inconclusive and comparison["status"] == "inconclusive":
        return 1
    return 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Run, summarize, or compare fixed-SHA Roko cold/warm development benchmarks."
        )
    )
    subparsers = result.add_subparsers(dest="command", required=True)
    listing = subparsers.add_parser("list", help="list lanes and representative fixtures")
    listing.add_argument("--manifest", type=pathlib.Path, default=DEFAULT_MANIFEST)
    listing.add_argument("--json", action="store_true")
    listing.set_defaults(func=list_manifest)

    run = subparsers.add_parser("run", help="execute a bounded benchmark matrix")
    run.add_argument("--manifest", type=pathlib.Path, default=DEFAULT_MANIFEST)
    run.add_argument("--base", default="HEAD", help="immutable base revision for every sample")
    run.add_argument("--lane", action="append", help="lane id (repeatable)")
    run.add_argument("--fixture", action="append", help="fixture id (repeatable)")
    run.add_argument("--cache", action="append", choices=("cold", "warm"))
    run.add_argument("--repetitions", type=int)
    run.add_argument("--deadline", type=int)
    run.add_argument("--dry-run", action="store_true", help="print the exact matrix; execute nothing")
    run.add_argument("--allow-network", action="store_true")
    run.add_argument("--max-cost-usd", type=float)
    run.add_argument("--max-runs", type=int, default=400)
    run.add_argument("--max-wall-hours", type=float, default=48.0)
    run.add_argument("--min-free-gib", type=float, default=10.0)
    run.add_argument("--min-free-percent", type=float, default=5.0)
    run.add_argument("--max-session-gib", type=float, default=160.0)
    run.add_argument("--max-cache-gib", type=float, default=50.0)
    run.add_argument("--output-root", type=pathlib.Path, default=DEFAULT_OUTPUT_ROOT)
    run.add_argument("--roko-bin", type=pathlib.Path)
    run.add_argument(
        "--binary-base",
        help="operator-attested Git revision used to build --roko-bin; must equal --base",
    )
    run.add_argument(
        "--allow-unverified-binary",
        action="store_true",
        help="explicitly record that the supplied binary/base identity is unverified",
    )
    run.add_argument("--baseline", type=pathlib.Path, action="append", default=[])
    run.add_argument("--keep-worktrees", action="store_true")
    run.add_argument("--keep-targets", action="store_true")
    run.add_argument("--fail-fast", action="store_true")
    run.set_defaults(func=execute)

    summarize = subparsers.add_parser(
        "summarize", help="rebuild JSON/Markdown scorecards from an existing session"
    )
    summarize.add_argument("session", type=pathlib.Path)
    summarize.add_argument("--baseline", type=pathlib.Path, action="append", default=[])
    summarize.set_defaults(func=summarize_command)

    history = subparsers.add_parser(
        "history",
        help="build a bounded historical dashboard and enforce regression thresholds",
    )
    history.add_argument("--root", type=pathlib.Path, default=DEFAULT_OUTPUT_ROOT)
    history.add_argument(
        "--output-dir",
        type=pathlib.Path,
        help="write history.json and HISTORY.md here (default: benchmark root)",
    )
    history.add_argument("--candidate-session")
    history.add_argument("--baseline-session")
    history.add_argument("--max-sessions", type=positive_int_argument, default=100)
    history.add_argument("--max-root-entries", type=positive_int_argument, default=2_000)
    history.add_argument("--max-rows-per-session", type=positive_int_argument, default=2_000)
    history.add_argument("--max-groups-per-session", type=positive_int_argument, default=256)
    history.add_argument("--max-file-mib", type=positive_float_argument, default=32.0)
    history.add_argument("--max-total-mib", type=positive_float_argument, default=256.0)
    history.add_argument("--deadline-seconds", type=positive_float_argument, default=10.0)
    history.add_argument("--min-samples", type=positive_int_argument, default=3)
    history.add_argument(
        "--max-p50-regression-percent",
        type=non_negative_float_argument,
        default=15.0,
    )
    history.add_argument(
        "--max-p95-regression-percent",
        type=non_negative_float_argument,
        default=20.0,
    )
    history.add_argument(
        "--max-non-success-rate-increase-points",
        type=non_negative_float_argument,
        default=5.0,
    )
    history.add_argument(
        "--max-timeout-rate-increase-points",
        type=non_negative_float_argument,
        default=5.0,
    )
    history.add_argument(
        "--max-valid-rate-drop-points",
        type=non_negative_float_argument,
        default=5.0,
    )
    history.add_argument(
        "--fail-on-inconclusive",
        action="store_true",
        help="also exit 1 when the latest comparison lacks sufficient evidence",
    )
    history.add_argument(
        "--report-only",
        action="store_true",
        help="always exit zero after writing the dashboard",
    )
    history.set_defaults(func=history_command)
    return result


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    numeric_limits = (
        "max_cost_usd",
        "max_runs",
        "max_wall_hours",
        "min_free_gib",
        "min_free_percent",
        "max_session_gib",
        "max_cache_gib",
    )
    for name in numeric_limits:
        raw = getattr(args, name, None)
        if raw is not None and (
            raw <= 0 or isinstance(raw, float) and not math.isfinite(raw)
        ):
            print(f"dev-benchmark: --{name.replace('_', '-')} must be greater than zero", file=sys.stderr)
            return 2
    try:
        return int(args.func(args))
    except BenchmarkError as error:
        print(f"dev-benchmark: {error}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print(
            "dev-benchmark: interrupted; inspect the printed session path—unsettled owned "
            "worktrees/targets are retained rather than deleted without process proof",
            file=sys.stderr,
        )
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
