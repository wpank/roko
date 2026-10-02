"""`vb.run_record/1` rows: build one per (task, seed), and append it only if it validates (S08 §4.12, §5.4, SC7).

The honesty rules, all enforced here or by `schema/validate.py` before a row reaches `records.jsonl`:
- `simulated` is always false; the label is the VS-census label (`census`), never the agent's own report;
- costs are the attempts' reported usage priced from one snapshot by `model_reported`; an unknown cost is null
  with source "unknown", never 0;
- `infra_error` and `leak_suspected` keep that status, so the report can exclude and count them.

The status is the runner's, overridden in this order: `leak_suspected` when the census found a canary, then
`infra_error` when a verifier failed or an attempt was served by a model other than the one requested (compared
without a date suffix). A task that a `model_swap` disturbance covers (gap-8bdf5e) declares the model the proxy served
in place of the pin: that one model passes the check too, and each attempt it served is marked `model_swapped`.

S09 §4.9's process measures come from what a runner's attempts carry, beside their usage: `queue_wait_s` (the
seconds the attempt's work waited for a dispatch slot or a provider rate limit) and `cost_class` (plan, execute,
retry, escalate or integrate). `execution.queue_wait_s` sums the attempts' waits, and is null unless every attempt
knows its own. `costs.by_class` sums the attempts' costs per class, and is null unless the runner classed every
attempt; a runner that classes its attempts accounts for all of the run's spend in them, so a class with no attempt
costs $0, and a class holding an attempt of unknown cost is null. The direct and CLI runners record neither, so
their records carry nulls.

`provenance.network_policy` is what the runner says of its agent's network (gap-0bd49a, `harness.TaskOutcome`): the
rule its agent's processes ran under (`network`: "none", or "loopback:<port>" for a proxy's port), the confinement
that applied it on this host (`sandbox`: "sandbox-exec+net", or "none" where nothing did, off macOS), and the
runner's own detail, such as the Claude Code arm's egress log. It is null when the runner did not say, as for a
runner that crashed.

`visible` is the census's clean rerun of the visible checks, which never meets a flake. In a run with `flaky_verify`
it also carries what the visible-verify wrapper logged of the arm's own visible checks (`vb_verify`): `verify_runs`
(null when no wrapper ran), `flakes` (each injected failure's run number and time) and `flake_injected`.

`config_hash` and `record_id` are `sha256:` digests of canonical JSON (sorted keys, no whitespace). S01 §4.7 wants
BLAKE3 `b3:` digests from `driver/fingerprint.py` with its golden vectors; neither exists yet, and the stdlib has
no BLAKE3, so the prefix says which algorithm made each value. The config holds no secret values (API keys are
named by their environment variable), so nothing needs redacting.

API:
    build(*, experiment_id, run_id, arm_id, seed, head, billed, config_hash, snapshot_id, suite, stream,
          materialized, outcome, result, final, archived, transcript_ref, meter_usd=None, verify_log=None,
          model_swap=None) -> dict
    append(path: Path, record: dict) -> None           # raises RecordError on an invalid record
    canonical_hash(value) -> str; harness_state() -> (sha, dirty); final_status(outcome, census, model_swap=None) -> str
    same_model(requested, reported) -> bool; swapped(attempt, model_swap) -> bool
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import subprocess
from pathlib import Path

import archive
import census
import harness
import layout
import ledger
import materialize
import validate  # schema/validate.py, on sys.path through layout


class RecordError(ValueError):
    """A run record failed validation; nothing was written."""


def canonical_hash(value: object) -> str:
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()


def harness_state() -> tuple[str, bool]:
    """(HEAD of the repository holding the driver, whether the benchmark tree or the price snapshots differ from it)."""
    try:
        sha = subprocess.run(["git", "-C", str(layout.REPO_ROOT), "rev-parse", "HEAD"], capture_output=True,
                             text=True, timeout=30, check=True).stdout.strip()
        status = subprocess.run(["git", "-C", str(layout.REPO_ROOT), "status", "--porcelain", "--",
                                 str(layout.VB_ROOT), str(layout.PRICES_DIR)], capture_output=True, text=True,
                                timeout=60, check=True).stdout
    except (OSError, subprocess.SubprocessError):
        return "unknown", True
    return sha, bool(status.strip())


def same_model(requested: str, reported: str | None) -> bool:
    if reported is None:
        return True  # nothing to compare; the cost is priced (or left unknown) by the reported model
    strip = ledger.DATED_SLUG.fullmatch
    return (strip(reported)[1] if strip(reported) else reported) == (strip(requested)[1] if strip(requested) else
                                                                     requested)


def final_status(outcome: harness.TaskOutcome, result: census.CensusResult, model_swap: str | None = None) -> str:
    if result.canary_hits:
        return "leak_suspected"
    if result.infra_error or any(not (same_model(a.model_requested, a.model_reported) or swapped(a, model_swap))
                                 for a in outcome.attempts):
        return "infra_error"
    return outcome.status


def swapped(attempt: harness.Attempt, model_swap: str | None) -> bool:
    """Whether the declared swap's model, and not the requested one, served the attempt."""
    reported = attempt.model_reported
    return bool(model_swap and reported and same_model(model_swap, reported)
                and not same_model(attempt.model_requested, reported))


def build(*, experiment_id: str, run_id: str, arm_id: str, seed: int, head: tuple[str, bool], billed: bool,
          config_hash: str, snapshot_id: str, suite: dict, stream: dict, materialized: materialize.Materialized,
          outcome: harness.TaskOutcome, result: census.CensusResult, final: archive.Final | None,
          archived: archive.Archive | None, transcript_ref: str | None, meter_usd: float | None = None,
          verify_log: list[dict] | None = None, model_swap: str | None = None) -> dict:
    manifest = materialized.manifest
    runs = verify_log or []  # the visible-verify wrapper's log of the arm's visible check runs (vb_verify)
    task = {"family": manifest["family"], "instance_id": manifest["instance_id"], "ladder": manifest["ladder"],
            "latent_version": manifest["latent_version"], "spec_variant": materialized.spec_variant,
            "is_honeypot": manifest["is_honeypot"]}
    variant = manifest["spec"][materialized.spec_variant]
    for key in ("operator", "levels", "operator_version"):
        if key in variant:
            task[key] = variant[key]
    attempts = [attempt.as_record() for attempt in outcome.attempts]
    if model_swap:  # the served-model check accepted the declared swap: say which attempts it served
        for attempt, row in zip(outcome.attempts, attempts):
            row["model_swapped"] = bool(row.get("model_swapped")) or swapped(attempt, model_swap)
    status = final_status(outcome, result, model_swap)
    failed = list(result.failed)
    if result.infra_error:
        failed.append(f"infra:{result.infra_error[:200]}")
    record = {
        "schema_version": "vb.run_record/1",
        "record_id": canonical_hash([experiment_id, run_id, arm_id, manifest["instance_id"], seed, 0]),
        "experiment_id": experiment_id, "run_id": run_id, "arm": arm_id, "seed": seed, "replicate": 0,
        "harness_sha": head[0], "dirty": head[1], "config_hash": config_hash,
        "price_snapshot_id": snapshot_id, "suite": suite, "stream": stream, "task": task,
        "execution": {"status": status, "reason": outcome.reason, "started_at": outcome.started_at,
                      "finished_at": outcome.finished_at, "queue_wait_s": _queue_wait(attempts),
                      "attempts": attempts},
        "visible": {"passed": result.visible_clean == 1, "clean_rerun": result.visible_clean is not None,
                    "flake_injected": any(row["flake"] for row in runs), "commands": result.visible_commands,
                    "exit_codes": result.visible_exit_codes,
                    "verify_runs": None if verify_log is None else len(runs),
                    "flakes": [{"run": row["run"], "at": row.get("at")} for row in runs if row["flake"]]},
        "vs": {"label": result.label, "unknown": result.unknown, "checks": result.checks,
               "truth_suite_version": manifest["truth_suite"]["version"], "failed": failed,
               "verifier_version": (result.hidden_output or {}).get("verifier_version"),
               "sandbox": (result.hidden_output or {}).get("sandbox")},  # gap-8c3752: how the agent's code was held
        "costs": {**_costs(outcome.attempts, billed), "by_class": _by_class(attempts)},
        "provenance": {"final_commit": final.commit if final else None,
                       "workdir_archive": f"archives/{archived.tarball.name}" if archived else None,
                       "bundle": f"archives/{archived.bundle.name}" if archived else None,
                       "diff_sha256": archived.diff_sha256 if archived else None, "transcript_ref": transcript_ref,
                       "s01_run_dir": outcome.s01_run_dir, "canary_hits": result.canary_hits,
                       "canary_places": sorted(result.canaries), "network_policy": outcome.network_policy},
        "simulated": False,
    }
    record["costs"]["meter_cross_check_usd"] = meter_usd  # the metering proxy's own figure for the task, if one ran
    return record


def append(path: Path, record: dict) -> None:
    errors = validate.validate("run-record", record)
    if errors:
        raise RecordError(f"refusing to write an invalid run record: {errors[0]}")
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    with os.fdopen(fd, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, sort_keys=True, ensure_ascii=False) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


def _costs(attempts: list[harness.Attempt], billed: bool) -> dict:
    """S01 §4.4's cost fields summed over the attempts; null as soon as one attempt's cost is unknown.

    `billed_usd` is the API-equivalent cost for a billed API arm and $0 for a subscription arm. A CLI runner's attempts
    (`run_cli.CliAttempt`) carry their source, `cli_usage`, and the CLI's own figure as `vendor_usd`. A CLI session
    killed before its `result` event was priced from its streamed messages (`cli.cost_basis` "stream"), a partial
    total that misses background calls, which its runner labels `estimated`, as in its ledger row (bug-f62293,
    bug-a49003). One estimated attempt makes the record's source `estimated`, which the report counts apart.
    """
    costs = [attempt.cost or ledger.Cost(None, None, "unknown") for attempt in attempts]
    if any(cost.source == "unknown" for cost in costs):
        return {"api_equiv_usd": None, "billed_usd": None, "without_cache_usd": None, "vendor_usd": None,
                "source": "unknown", "meter_cross_check_usd": None}
    api_equiv = sum(cost.api_equiv_usd for cost in costs)
    without_cache = sum(cost.without_cache_usd for cost in costs)
    sources = {cost.source for cost in costs}
    vendor = [getattr(attempt, "vendor_usd", None) for attempt in attempts]
    source = "estimated" if "estimated" in sources else sources.pop() if len(sources) == 1 else "provider_usage"
    return {"api_equiv_usd": api_equiv, "billed_usd": api_equiv if billed else 0.0, "without_cache_usd": without_cache,
            "vendor_usd": sum(vendor) if vendor and None not in vendor else None, "source": source,
            "meter_cross_check_usd": None}


def _by_class(attempts: list[dict]) -> dict | None:
    """The attempt records' API-equivalent cost per class (module docstring); None unless each has a `cost_class`."""
    classes = [attempt.get("cost_class") for attempt in attempts]
    if not attempts or None in classes:
        return None
    totals: dict[str, float | None] = dict.fromkeys(validate.COST_CLASSES, 0.0)
    for name, attempt in zip(classes, attempts):
        cost = attempt.get("api_equiv_usd")
        totals[name] = totals[name] + cost if totals[name] is not None and cost is not None else None
    return totals


def _queue_wait(attempts: list[dict]) -> float | None:
    """The attempt records' queue waits summed; None unless every attempt knows its own."""
    waits = [attempt.get("queue_wait_s") for attempt in attempts]
    return math.fsum(waits) if waits and None not in waits else None
