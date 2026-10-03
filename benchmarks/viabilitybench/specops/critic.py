"""The LLM critic C-v1 (S07 §4.2, S07.5; task 3237): a fixed four-question rubric over a TSS v1 task spec.

``critique(spec, model) -> CriticResult`` asks `model` each of :data:`QUESTIONS` once and folds the answers into
``critic_score`` (the fraction answered yes) and ``n_questions`` (always 4). The rubric is fixed, not a vibe
check, so two runs of the same spec against the same model draw the same four questions:

1. Is the goal one testable outcome, not a bundle of unrelated goals?
2. Does every acceptance criterion state something an agent could check without asking a question back?
3. Does the given context (``read_files``, ``symbols``) hold every fact the spec assumes, leaving nothing an
   agent would have to guess?
4. Are the task's non-goals or scope boundaries explicit, so a correct implementation cannot wander into
   untested territory by accident?

A critic is advisory (S07 §8): it never changes the spec, `score` or `band` speclint already computed, since LLM
review has been shown to lower a human reviewer's own smell detection when it is trusted instead of read
(``broccia2026llmri``). No network call happens in this module; `model` is the caller's own adapter (a stub in
tests; a live provider is S09 block C, not yet wired).

API:
    QUESTIONS: tuple[str, ...]            # always 4, in order
    CriticModel = Callable[[dict, str], tuple[bool, float]]      # (spec, question) -> (answered yes, cost_usd)
    CriticResult(critic_score, n_questions, answers, cost_usd); .as_json() -> dict
    critique(spec: dict, model: CriticModel) -> CriticResult
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

QUESTIONS = (
    "Is the goal one testable outcome, not a bundle of unrelated goals?",
    "Does every acceptance criterion state something an agent could check without asking a question back?",
    "Does the given context hold every fact the spec assumes, leaving nothing for the agent to guess?",
    "Are the task's non-goals or scope boundaries explicit, so a correct implementation cannot wander into "
    "untested territory by accident?",
)

CriticModel = Callable[[dict, str], tuple[bool, float]]


@dataclass(frozen=True)
class CriticResult:
    critic_score: float
    n_questions: int
    answers: list[bool]
    cost_usd: float

    def as_json(self) -> dict:
        return {"critic_score": self.critic_score, "n_questions": self.n_questions}


def critique(spec: dict, model: CriticModel) -> CriticResult:
    """Ask `model` each of :data:`QUESTIONS` about `spec`, once, in order; never modifies `spec`."""
    answers: list[bool] = []
    cost_usd = 0.0
    for question in QUESTIONS:
        answer, question_cost = model(spec, question)
        answers.append(bool(answer))
        cost_usd += question_cost
    return CriticResult(critic_score=sum(answers) / len(QUESTIONS), n_questions=len(QUESTIONS), answers=answers,
                        cost_usd=cost_usd)
