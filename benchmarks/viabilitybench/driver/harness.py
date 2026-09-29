"""The runner protocol: what `vb run` hands a harness module for one task, and what it gets back.

`vb run` picks the module from the arm file: `[arm] runner`, or else the `harness` id with "-" read as "_"
(`mini-loop` -> `mini_loop.py`). It imports the module from `driver/` and calls `run_task(ctx) -> TaskOutcome`. A new
arm therefore adds an arm file and, if its harness is new, one module with `run_task`; `vb.py` does not change.

A runner:
- works in `ctx.workdir` only, and starts every agent process with `ctx.agent_env` (`agent_env.build`);
- appends one ledger row per attempt it dispatched (`ctx.ledger`), when the attempt ends, even on failure;
- returns when the agent has ended its session (status `completed`) or a cap stopped it (`aborted_cap`, `timeout`),
  or with `infra_error` when the harness itself failed;
- never commits in the agent's repo. `vb run` exports the final tree afterwards (`archive.commit_final`) and the
  census labels it.

API:
    TaskContext(...)                  # frozen; see the fields
    Attempt(...); Attempt.as_record() -> dict          # one vb.run_record/1 execution.attempts[] entry
    TaskOutcome(status, reason, attempts, transcript, started_at, finished_at)
    RUNNER_STATUSES; utc_now() -> str
"""

from __future__ import annotations

import datetime as dt
from dataclasses import dataclass, field
from pathlib import Path

import caps
import ledger
import provider

RUNNER_STATUSES = ("completed", "failed", "timeout", "aborted_cap", "infra_error")


def utc_now() -> str:
    return dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


@dataclass(frozen=True)
class TaskContext:
    experiment_id: str
    run_id: str
    arm: dict  # the parsed arm file
    model: str
    endpoint: provider.Endpoint
    provider: provider.ChatProvider
    snapshot: ledger.Snapshot
    caps: caps.Caps
    ledger: ledger.Ledger
    billed: bool  # false for subscription arms, whose billed cost is $0
    instance_id: str
    seed: int
    key: str  # "<instance_id>.s<seed>", unique within a run
    workdir: Path
    spec_text: str
    agent_env: dict[str, str]

    @property
    def chain_key(self) -> str:
        """S01's chain key for this task; attempt keys are "<chain_key>:<n>"."""
        return f"{self.run_id}/{self.key}"

    @property
    def price_row(self) -> dict | None:
        return self.snapshot.row(self.model)


@dataclass
class Attempt:
    number: int
    attempt_key: str
    model_requested: str
    provider: str
    reserved_usd: float
    model_reported: str | None = None
    turns: int = 0
    calls: int = 0
    usage: dict = field(default_factory=lambda: ledger.vb_usage(0, 0))
    usage_unknown: bool = False  # some call may have been billed without reporting usage
    ended_by: str = ""
    tree: str | None = None  # the tree hash of the workdir when the attempt ended
    cost: ledger.Cost | None = None

    def reported_usage(self) -> dict | None:
        return None if self.usage_unknown else self.usage

    def as_record(self) -> dict:
        cost = self.cost or ledger.Cost(None, None, "unknown")
        return {"emitter": "vb-driver", "attempt_key": self.attempt_key, "model_requested": self.model_requested,
                "model_reported": self.model_reported, "provider": self.provider, "turns": self.turns,
                "usage": self.reported_usage(), "fault_injected": None, "calls": self.calls,
                "ended_by": self.ended_by, "tree": self.tree, "api_equiv_usd": cost.api_equiv_usd,
                "reserved_usd": round(self.reserved_usd, 6)}


@dataclass
class TaskOutcome:
    status: str  # one of RUNNER_STATUSES
    reason: str
    attempts: list[Attempt]
    transcript: list[dict]
    started_at: str
    finished_at: str
