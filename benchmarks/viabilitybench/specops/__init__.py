"""Spec operators for ViabilityBench's H3 (S07 §4.5): the spec-degradation operator D-v1.

``degrade(spec, levels, seed) -> (spec', manifest)`` makes the vague twin of a precise TSS v1 task spec. S08.T10
calls it and never defines its own (R.S07-5). Python standard library only.
"""

from .degrade import LEVELS, VAGUE, degrade
from .manifest import OPERATOR, VERSION, Manifest

__all__ = ["LEVELS", "OPERATOR", "VAGUE", "VERSION", "Manifest", "degrade"]
