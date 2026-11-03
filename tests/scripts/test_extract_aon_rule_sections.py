import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "extract-aon-rule-sections.py"
spec = importlib.util.spec_from_file_location("aon_extract", SCRIPT)
aon = importlib.util.module_from_spec(spec)
spec.loader.exec_module(aon)


class MappingTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.catalog = self.root / "catalog.json"
        self.books = self.root / "books.json"
        self.entries = self.root / "entries"
        self.entries.mkdir()
        self.aliases = self.root / "aliases.json"
        self.write(self.catalog, {"sources": [
            {"id": "book-a", "title": "Book A"},
            {"id": "book-b", "title": "Book B"},
        ]})
        self.write(self.books, {"books": [
            {"source": "book-a", "chapters": [{"title": "Shared", "entry": "book-a/rules/shared", "children": []}]},
            {"source": "book-b", "chapters": [{"title": "Shared", "entry": "book-b/rules/shared", "children": []}]},
        ]})
        self.inventory = {"books": [
            {"label": "Book A", "heading_id": "a", "topics": [
                {"label": "Nested heading", "url": "u1", "subsections": []},
                {"label": "Shared", "url": "u2", "subsections": []},
                {"label": "Not present", "url": "u3", "subsections": []},
            ]},
            {"label": "Book B", "heading_id": "b", "topics": []},
            {"label": "Missing", "heading_id": "m", "topics": [
                {"label": "Not present", "url": "u4", "subsections": []}
            ]},
        ]}
        self.write(self.aliases, {"aliases": []})
        self.write(self.entries / "rule.json", [{
            "id": "book-a/rules/nested", "kind": "rule", "source_ids": ["book-a"],
            "body": [{"t": "list", "items": [{"content": [
                {"t": "heading", "anchor": "deep-anchor", "content": [
                    {"t": "strong", "content": [{"t": "text", "v": "Nested "}]},
                    {"t": "emph", "content": [{"t": "text", "v": "heading"}]},
                ]}
            ]}]}]
        }])

    def tearDown(self):
        self.tmp.cleanup()

    @staticmethod
    def write(path, value):
        Path(path).write_text(json.dumps(value), encoding="utf-8")

    def mapping(self):
        return aon.mapping_for(self.inventory, self.catalog, self.books, self.entries, self.aliases)

    def test_source_confinement_and_empty_installed_source(self):
        m = self.mapping()
        a, b, missing = m["sources"]
        shared = a["topics"][1]
        self.assertEqual(shared["status"], "matched_exact_title")
        self.assertEqual(shared["candidate_entry_ids"], ["book-a/rules/shared"])
        self.assertEqual(b["status"], "matched_exact_title")
        self.assertEqual(missing["status"], "source_not_installed")
        self.assertEqual(missing["topics"][0]["status"], "source_not_installed")

    def test_nested_heading_inline_text_and_anchor(self):
        topic = self.mapping()["sources"][0]["topics"][0]
        self.assertEqual(topic["status"], "matched_exact_title")
        self.assertEqual(topic["targets"], [{"entry_id": "book-a/rules/nested", "anchor": "deep-anchor"}])
        self.assertIn("typed rule-body headings", topic["evidence"])

    def test_duplicate_node_and_body_evidence_is_deduplicated(self):
        payload = json.loads((self.entries / "rule.json").read_text())
        payload.append({"id": "book-a/rules/shared", "kind": "rule", "source_ids": ["book-a"],
                        "body": [{"t": "heading", "anchor": "shared", "content": [{"t": "text", "v": "Shared"}]}]})
        self.write(self.entries / "rule.json", payload)
        shared = self.mapping()["sources"][0]["topics"][1]
        self.assertEqual(shared["status"], "ambiguous_exact_title")
        self.assertEqual(shared["targets"], [
            {"entry_id": "book-a/rules/shared", "anchor": None},
            {"entry_id": "book-a/rules/shared", "anchor": "shared"},
        ])

    def test_distinct_same_source_heading_targets_are_ambiguous(self):
        payload = json.loads((self.entries / "rule.json").read_text())
        payload.append({"id": "book-a/rules/other", "kind": "rule", "source_ids": ["book-a"],
                        "body": [{"t": "heading", "anchor": "other", "content": [{"t": "text", "v": "Nested heading"}]}]})
        self.write(self.entries / "rule.json", payload)
        topic = self.mapping()["sources"][0]["topics"][0]
        self.assertEqual(topic["status"], "ambiguous_exact_title")
        self.assertEqual(topic["candidate_entry_ids"], ["book-a/rules/nested", "book-a/rules/other"])

    def test_alias_requires_valid_target_and_evidence(self):
        self.write(self.aliases, {"aliases": [{"alias": "Book A alias", "canonical_book_id": "nope", "evidence": "proof"}]})
        with self.assertRaises(ValueError):
            self.mapping()
        self.write(self.aliases, {"aliases": [{"alias": "Book A alias", "canonical_book_id": "book-a", "evidence": " "}]})
        with self.assertRaises(ValueError):
            self.mapping()

    def test_reviewed_alias_resolves_once_and_preserves_its_evidence(self):
        self.inventory["books"][0]["label"] = "Book A legacy spelling"
        self.write(self.aliases, {"aliases": [{"alias": "Book A legacy spelling", "canonical_book_id": "book-a", "evidence": "Reviewed source-title variant"}]})
        source = self.mapping()["sources"][0]
        self.assertEqual(source["canonical_book_ids"], ["book-a"])
        self.assertEqual(source["title_aliases"][0]["evidence"], "Reviewed source-title variant")


if __name__ == "__main__":
    unittest.main()
