import json
import tempfile
import unittest
from pathlib import Path

from tools.docs_integrity import check_citation_errata as cce


def work(**kw):
    base = {"key": "arxiv:2412.10425", "class": "I0", "codes": ["M2a"], "severity": "major",
            "anchors": {"arxiv": ["2412.10425"], "titles": []}, "bad": {},
            "correct": {"authors": ["Rithvik Prakki"], "year": 2024, "title": "Active Inference for Multi-LLM Systems",
                        "venue": "arXiv preprint", "arxiv": "2412.10425", "doi": None, "label": "Prakki"}}
    base.update(kw)
    return base


class CitationErrataTests(unittest.TestCase):
    def run_md(self, works, text):
        errata = cce.Errata({"works": works})
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "doc.md"
            path.write_text(text, encoding="utf-8")
            return cce.check_markdown(path, "doc.md", errata)

    def test_wrong_author_label_is_reported_and_the_fix_is_clean(self):
        w = work(bad={"authors": ["Koudahl et al."]})
        found = self.run_md([w], "**[Koudahl et al., 2024]** *Active Inference.* arXiv:2412.10425.\n")
        self.assertEqual([f.line for f in found], [1])
        self.assertIn("cite as Prakki (2024)", found[0].message)
        self.assertEqual(self.run_md([w], "**[Prakki, 2024]** *Active Inference.* arXiv:2412.10425.\n"), [])

    def test_a_system_name_counts_as_an_author_only_before_a_year(self):
        w = work(key="arxiv:2502.04780", codes=["M2c"], anchors={"arxiv": ["2502.04780"], "titles": []},
                 bad={"authors": ["SiriuS"]})
        self.assertEqual(len(self.run_md([w], "**[SiriuS, 2025]** *SiriuS.* arXiv:2502.04780.\n")), 1)
        self.assertEqual(self.run_md([w], "SiriuS (arXiv:2502.04780) keeps failed episodes.\n"), [])

    def test_a_wrapped_citation_is_reported_once_where_it_ends(self):
        w = work(bad={"authors": ["Koudahl et al."]})
        found = self.run_md([w], "Following Koudahl\net al. (2024, arXiv:2412.10425): model the agent.\n")
        self.assertEqual([f.line for f in found], [2])

    def test_a_wrong_title_must_be_the_whole_title_and_a_wrapped_correct_title_is_fine(self):
        correct = dict(work()["correct"], title="Regularized Proportional Fairness Mechanism for Resource Allocation "
                                                "Without Money")
        w = work(key="arxiv:2501.01111", codes=["M1"], anchors={"arxiv": ["2501.01111"], "titles": []},
                 bad={"titles": ["Regularized Proportional Fairness"]}, correct=correct)
        self.assertEqual(len(self.run_md([w], "*Regularized Proportional Fairness.* arXiv:2501.01111.\n")), 1)
        wrapped = ('Zeng et al. (2025), "Regularized Proportional Fairness Mechanism for Resource\n'
                   'Allocation Without Money." arXiv:2501.01111.\n')
        self.assertEqual(self.run_md([w], wrapped), [])

    def test_any_mention_of_a_fabricated_work_is_reported(self):
        w = work(key="title:death and progress", **{"class": "I1"}, codes=[], correct=None,
                 anchors={"arxiv": [], "titles": ["Death and Progress"]}, bad={"all": True})
        found = self.run_md([w], "Intro.\n\n**[Wensink et al., 2020]** *Death and Progress.* Evolutionary Biology.\n")
        self.assertEqual([(f.line, "fabricated" in f.message) for f in found], [(3, True)])

    def test_a_wrong_identifier_is_reported_with_its_replacement(self):
        w = work(key="arxiv:2507.02849", **{"class": "I2"}, codes=[], anchors={"arxiv": ["2507.02849"], "titles": []},
                 bad={"arxiv": ["2507.02849"]}, correct=dict(work()["correct"], arxiv="2506.09033"))
        found = self.run_md([w], "| Router-R1 | arXiv:2507.02849 |\n")
        self.assertEqual(len(found), 1)
        self.assertIn("use arXiv:2506.09033", found[0].message)

    def test_citations_json_records_are_checked_by_field(self):
        w = work(bad={"authors": ["Koudahl et al."]})
        record = {"id": "koudahl-2024", "authors": ["Koudahl, M."], "year": 2024, "title": "Active Inference",
                  "arxiv_id": "2412.10425"}
        errata = cce.Errata({"works": [w]})
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "citations.json"
            path.write_text(json.dumps([record], indent=2), encoding="utf-8")
            self.assertEqual(len(cce.check_citations_json(path, "citations.json", errata)), 1)
            path.write_text(json.dumps([dict(record, authors=["Prakki, R."])], indent=2), encoding="utf-8")
            self.assertEqual(cce.check_citations_json(path, "citations.json", errata), [])

    def test_the_manifest_is_well_formed(self):
        manifest = json.loads(cce.MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(manifest["schema"], "roko.citation_errata/1")
        for w in manifest["works"]:
            self.assertIn(w["class"], {"I0", "I1", "I2", "I3", "I4"}, w["key"])
            self.assertTrue(w["anchors"]["arxiv"] or w["anchors"]["titles"], w["key"])

    def test_docs_v3_has_no_known_erratum(self):
        self.assertEqual(cce.main(["docs/v3"]), 0)


if __name__ == "__main__":
    unittest.main()
