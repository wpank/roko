"""Writes S09's Holm verdicts (`holm.py`'s `decide`/`write_verdicts`) for one experiment: the first caller
outside `holm.py`'s own tests (gap-04496d). Also projects the same decision into a `holm_reject` MetricRecord
per primary, the format `tab_t3_headline.py`'s footer already reads (`inputs.one("holm_reject", ...)`, dead
since nothing wrote one) -- one call to `holm.decide`, feeding both `verdicts.json` (`showcase/build_bundle.py`,
gap-2da8ec) and the MetricRecord, so T3 keeps reading what it already reads, never a second, divergent source
of the same fact (the item's own choice: "produce holm_reject from the same verdicts").

Not T6 itself: `paper/FIGURES-TABLES.md`'s archived spec names `tab_t6_primaries.py` for the full
pre-registered-primaries table (estimate, test statistic, replay-validation status, deviation ids, the
exploratory and deviations blocks), which needs a pre-registered lock and a deviation log this does not have.
This writes only the one piece gap-2da8ec and gap-04496d need, the decision.

Assembling the eleven node p-values `holm.decide` needs (H1's envelope chain, H2's pass^3 test, H3-H7's own
estimators), per hypothesis, from an experiment's own MetricRecords is not part of this change: `p` is still the
caller's own input, the same boundary `gap-2da8ec` left at the writer itself.

**adverse** (`holm.py`'s module docstring: the side fact that tells `NOT_SUPPORTED` from `INCONCLUSIVE`) is read
off an existing estimate's sign here, never computed fresh: `find_adverse` reads H1's `envelope_ratio_r` and
`envelope_ratio_c` against `figlib.REFERENCE_ARM` (S09 §4.4; the same records `tab_t3_headline.py`'s own
`_ratio` footer helper reads) -- adverse when R is below 1 (roko resolves less than the reference) or C is above
1 (roko costs more), whichever reading is present. H2-H7 have no ratio wired to a hypothesis yet, so they keep
`verdict_records`'s own conservative default.

API:
    adverse_from_ratio(r, c) -> bool | None
    find_adverse(inputs, experiment_id) -> dict[str, bool]
    holm_reject_records(result, template, *, hypotheses=holm.PRIMARIES) -> list[dict]
    write(inputs, experiment_id, p, out_dir, *, alpha=0.05, template_metric="envelope_level") -> dict
"""

from __future__ import annotations

import json
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))
import figlib  # noqa: E402
import holm  # noqa: E402

# The provenance fields every vb.metric_record/1 of the same cell shares; holm_reject_records copies them from
# an existing record of that cell rather than re-deriving them (module docstring).
_COPIED_PROVENANCE = ("experiment_id", "arms", "run_ids", "seeds", "commits", "config_hashes", "analysis_commit",
                     "computed_at", "preregistered", "prereg_id", "blinded", "label_source", "cost_basis",
                     "price_snapshot_id", "n")


def adverse_from_ratio(r: float | None, c: float | None) -> bool | None:
    """Whether an envelope comparison points against the hypothesis (module docstring): R below 1, or C above 1.
    None when neither ratio is available to read, so the caller's own default applies instead of a guess."""
    if r is not None and r < 1:
        return True
    if c is not None and c > 1:
        return True
    if r is None and c is None:
        return None
    return False


def find_adverse(inputs: figlib.Inputs, experiment_id: str) -> dict[str, bool]:
    """{"H1": ...} when H1's envelope ratio against the reference arm is in `inputs` (module docstring); empty
    when neither ratio is there, so `verdict_records`'s own default applies."""
    cut = {"experiment": experiment_id, "arm": "roko_full", "against": (figlib.REFERENCE_ARM,)}
    r, c = inputs.one("envelope_ratio_r", **cut), inputs.one("envelope_ratio_c", **cut)
    found = adverse_from_ratio(r.value if r else None, c.value if c else None)
    return {"H1": found} if found is not None else {}


def holm_reject_records(result: holm.HolmResult, template: Mapping,
                        hypotheses: Sequence[str] = holm.PRIMARIES) -> list[dict]:
    """One `holm_reject` `vb.metric_record/1` per `hypotheses` (module docstring): `result`'s own rejected bit
    (its first level, for H1) as the value, `template`'s provenance copied verbatim -- the same experiment, arms,
    runs and sample size any of its other metrics would carry. Only `metric`, `value`, `estimator` and
    `record_filter` (one more clause, `hypothesis == `) are this hypothesis's own; `template` needs no `ci` or
    `ladder` read, since neither applies to a decision and neither is copied."""
    base = {key: template[key] for key in _COPIED_PROVENANCE}
    records = []
    for primary in hypotheses:
        clause = f"hypothesis == {json.dumps(primary)}"
        record_filter = f"{template['record_filter']} and {clause}" if template.get("record_filter") else clause
        node = holm.H1_LEVELS[0] if primary == "H1" else primary
        records.append({
            "schema_version": "vb.metric_record/1", "metric": "holm_reject", "value": int(node in result.rejected),
            "estimator": "graphical_holm (S09 Holm decision)", "record_filter": record_filter, **base,
        })
    return records


def write(inputs: figlib.Inputs, experiment_id: str, p: Mapping[str, float], out_dir: Path, *,
         alpha: float = 0.05, template_metric: str = "envelope_level") -> dict:
    """`holm.decide(p, alpha)`, written as `out_dir`'s `verdicts.json` (module docstring), with H1's `adverse`
    read off its envelope ratio (`find_adverse`); and the same decision projected into one `holm_reject`
    MetricRecord per primary (`holm_reject_records`), its provenance copied from `inputs`' own
    `template_metric` record of `experiment_id` against `figlib.REFERENCE_ARM` (default `envelope_level`,
    already scoped to the roko_full/reference cell every primary's decision is about).

    Returns {"verdicts": <the written Path>, "holm_reject": <the MetricRecord dicts, for the caller to fold into
    its own metrics.json -- this function writes verdicts.json only, never mutates a metrics file itself>}."""
    result = holm.decide(dict(p), alpha)
    verdicts_path = holm.write_verdicts(out_dir, experiment_id, result, adverse=find_adverse(inputs, experiment_id))
    template = inputs.one(template_metric, experiment=experiment_id, arm="roko_full",
                          against=(figlib.REFERENCE_ARM,))
    if template is None:
        raise ValueError(f"no {template_metric} record of {experiment_id} (roko_full vs {figlib.REFERENCE_ARM}) "
                         "to copy a holm_reject record's provenance from")
    return {"verdicts": verdicts_path, "holm_reject": holm_reject_records(result, template.doc)}
