"""Hard caps and the runaway detector for one task (S08 §4.10, SC6).

`Caps` holds the limits of an arm file's `[caps]` table. `Governor` enforces them for one task, before every model
call and every tool call, so a limit is reached but never exceeded:

- **Per attempt:** `turns_per_attempt` model turns and `input_tokens_per_attempt` cumulative input tokens. Reaching
  either ends the attempt, and the next attempt starts with a fresh context if the task has budget left.
- **Per task:** `turns_per_task` turns, `input_tokens_per_task` cumulative input tokens and `wallclock_s` seconds.
- **The runaway detector:** `model_calls_per_task` model calls (every request counts, failed ones too),
  `identical_calls` identical tool calls in a row, or a spend of `usd_per_task`.

A task stopped by a task cap or the runaway detector ends `aborted_cap`, and one out of time ends `timeout`; both
get VS = 0, because the agent did not complete (S08 §4.10: reported as censoring per arm).

Before a call the governor needs an upper bound on its input tokens. The loop passes the last reported prompt size
plus the bytes added since, since a token is never shorter than a byte. The dollar check prices that bound at the
model's input rate plus `max_output_tokens` at its output rate, and a call whose usage never came back is charged at
that bound. A model without a price row cannot be held to a dollar cap; `vb run` refuses one for a network run.

Identical tool calls are compared after collapsing whitespace, and only in a row: re-running the same test command
between edits is normal work, while five identical commands in a row is a loop.

API:
    Caps(...); Caps.from_table(table: Mapping) -> Caps                       # raises CapError
    Stop(kind, reason)          # kind: "end_attempt", "aborted_cap" or "timeout"
    Governor(caps, *, price_row, clock=time.monotonic)
    Governor.start_attempt(); .before_call(input_bound) -> Stop | None; .before_tool(command) -> Stop | None
    Governor.after_call(*, input_tokens, input_bound, cost_usd, completed)
    Governor.remaining_s() -> float; .reserve_attempt_usd() -> float; .spent_usd
    worst_task_usd(caps, row) -> float | None         # the most one task can cost under `caps`
"""

from __future__ import annotations

import time
from collections.abc import Callable, Mapping
from dataclasses import dataclass, fields

import ledger


class CapError(ValueError):
    """An arm's `[caps]` table has an unknown, missing or invalid value."""


@dataclass(frozen=True)
class Caps:
    turns_per_attempt: int = 12
    input_tokens_per_attempt: int = 150_000
    turns_per_task: int = 30
    input_tokens_per_task: int = 300_000
    wallclock_s: float = 1200.0
    model_calls_per_task: int = 30
    identical_calls: int = 5
    usd_per_task: float = 0.30
    max_output_tokens: int = 8192
    command_timeout_s: float = 60.0
    observation_chars: int = 10_000

    @classmethod
    def from_table(cls, table: Mapping) -> Caps:
        known = {field.name: field.type for field in fields(cls)}
        unknown = sorted(set(table) - set(known))
        if unknown:
            raise CapError(f"unknown cap(s): {', '.join(unknown)}")
        values = {}
        for name, value in table.items():
            wants_int = known[name] == "int"
            number = isinstance(value, (int, float)) and not isinstance(value, bool)
            if not number or (wants_int and not isinstance(value, int)):
                raise CapError(f"cap {name} must be {'an integer' if wants_int else 'a number'}, not {value!r}")
            if value <= 0:
                raise CapError(f"cap {name} must be above 0, not {value!r}")
            values[name] = value
        return cls(**values)


@dataclass(frozen=True)
class Stop:
    kind: str  # end_attempt | aborted_cap | timeout
    reason: str


def worst_task_usd(caps: Caps, row: dict | None) -> float | None:
    """The most one task can cost: the dollar cap, or less if the token caps bind first. None without a price row."""
    tokens = ledger.worst_case_usd(row, input_tokens=caps.input_tokens_per_task,
                                   output_tokens=caps.model_calls_per_task * caps.max_output_tokens)
    return None if tokens is None else min(caps.usd_per_task, tokens)


class Governor:
    def __init__(self, caps: Caps, *, price_row: dict | None, clock: Callable[[], float] = time.monotonic) -> None:
        self.caps = caps
        self.row = price_row
        self.clock = clock
        self.deadline = clock() + caps.wallclock_s
        self.calls = self.turns = self.input_tokens = 0
        self.attempts = self.attempt_calls = self.attempt_turns = self.attempt_input = 0
        self.spent_usd = 0.0  # known costs, plus the bound of every call whose cost is unknown
        self._last_command: str | None = None
        self._repeats = 0

    def remaining_s(self) -> float:
        return self.deadline - self.clock()

    def start_attempt(self) -> None:
        self.attempts += 1
        self.attempt_calls = self.attempt_turns = self.attempt_input = 0

    def before_call(self, input_bound: int) -> Stop | None:
        caps = self.caps
        if self.remaining_s() <= 0:
            return Stop("timeout", "wallclock")
        if self.calls >= caps.model_calls_per_task:
            return Stop("aborted_cap", "model_calls")
        if self.turns >= caps.turns_per_task:
            return Stop("aborted_cap", "task_turns")
        if self.input_tokens + input_bound > caps.input_tokens_per_task:
            return Stop("aborted_cap", "task_input_tokens")
        if self.attempt_turns >= caps.turns_per_attempt:
            return Stop("end_attempt", "attempt_turns")
        if self.attempt_input + input_bound > caps.input_tokens_per_attempt:
            # A fresh attempt that cannot make even its first call would loop forever: stop the task instead.
            return Stop("end_attempt" if self.attempt_calls else "aborted_cap", "attempt_input_tokens")
        worst = self._call_bound_usd(input_bound)
        if worst is not None and self.spent_usd + worst > caps.usd_per_task:
            return Stop("aborted_cap", "usd")
        return None

    def after_call(self, *, input_tokens: int | None, input_bound: int, cost_usd: float | None,
                   completed: bool) -> None:
        """Account one model call. `input_tokens` and `cost_usd` are None when the call's usage is unknown."""
        used = input_bound if input_tokens is None else input_tokens
        self.calls += 1
        self.attempt_calls += 1
        self.input_tokens += used
        self.attempt_input += used
        if completed:
            self.turns += 1
            self.attempt_turns += 1
        self.spent_usd += cost_usd if cost_usd is not None else (self._call_bound_usd(input_bound) or 0.0)

    def before_tool(self, command: str) -> Stop | None:
        normalized = " ".join(command.split())
        if normalized == self._last_command:
            self._repeats += 1
        else:
            self._last_command, self._repeats = normalized, 1
        if self._repeats >= self.caps.identical_calls:
            return Stop("aborted_cap", "identical_calls")
        return None

    def reserve_attempt_usd(self) -> float:
        """The worst-case cost of the attempt about to start, which the ledger row records as its reservation."""
        caps = self.caps
        left = max(caps.usd_per_task - self.spent_usd, 0.0)
        calls = min(caps.turns_per_attempt, max(caps.model_calls_per_task - self.calls, 0))
        tokens = ledger.worst_case_usd(self.row, output_tokens=calls * caps.max_output_tokens,
                                       input_tokens=min(caps.input_tokens_per_attempt,
                                                        max(caps.input_tokens_per_task - self.input_tokens, 0)))
        return left if tokens is None else min(left, tokens)

    def _call_bound_usd(self, input_bound: int) -> float | None:
        return ledger.worst_case_usd(self.row, input_tokens=input_bound, output_tokens=self.caps.max_output_tokens)
