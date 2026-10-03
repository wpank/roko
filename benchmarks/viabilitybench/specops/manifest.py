"""The manifest of one D-v1 degradation (3233, S07 §4.5).

Every item the operator removes, replaces or inserts is listed, so a reader can audit what a vague variant lost
and the scoring suite can keep the verify steps L1 hid from the agent. :meth:`Manifest.record` is the
``spec.degraded`` record S08 writes beside each run: the operator, its version, the levels and the seed.
"""

from __future__ import annotations

from dataclasses import dataclass, field

OPERATOR = "D-v1"
VERSION = "1"


@dataclass
class Manifest:
    """What one application of D-v1 changed in one task spec."""

    task_id: str
    levels: list[str]
    seed: int
    # {"level", "field", "item"}: whole items dropped (verify steps, acceptance items, context entries, sentences).
    removed: list[dict] = field(default_factory=list)
    # {"level", "field", "before", "after"}: anchors replaced by generic nouns.
    replaced: list[dict] = field(default_factory=list)
    # {"level", "field", "term", "position"}: lexicon terms inserted, position as a word index.
    inserted: list[dict] = field(default_factory=list)
    # The verify steps L1 hid from the agent; they still score VS.
    hidden_verify: list[dict] = field(default_factory=list)

    def changed(self) -> bool:
        """Whether the application changed anything."""
        return bool(self.removed or self.replaced or self.inserted)

    def record(self) -> dict:
        """The ``spec.degraded`` record."""
        return {
            "ev": "spec.degraded",
            "operator": OPERATOR,
            "operator_version": VERSION,
            "levels": list(self.levels),
            "seed": self.seed,
            "task_id": self.task_id,
            "removed": list(self.removed),
            "replaced": list(self.replaced),
            "inserted": list(self.inserted),
            "hidden_verify": list(self.hidden_verify),
        }
