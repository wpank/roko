#!/usr/bin/env python3
"""Deterministic cold/warm scorecard runner for Roko development lanes.

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
import json
import math
import os
import pathlib
import re
import shutil
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


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description="Run or summarize fixed-SHA Roko cold/warm development benchmarks."
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
