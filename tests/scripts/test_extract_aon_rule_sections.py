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

    def passage_file(self, **changes):
        row = {"status": "proposed_target", "source": "book-a", "url": "u3", "label": "Not present",
               "target": {"entry_id": "book-a/rules/nested", "anchor": "deep-anchor"},
               "evidence": {"method": "unique_exact_source_opening_passage", "source_locator": "Book A pg. 1",
                            "page_title": "Alias", "opening_words_sha256": "a" * 64}}
        row.update(changes)
        path = self.root / "evidence.json"
        self.write(path, {"format": "cast-iron-aon-rule-evidence", "version": 1,
                          "entries_sha256": aon.entries_digest(self.entries), "books_sha256": aon.hashlib.sha256(self.books.read_bytes()).hexdigest(), "rows": [row]})
        return path

    def with_evidence(self, path):
        return aon.mapping_for(self.inventory, self.catalog, self.books, self.entries, self.aliases, path)

    def test_verified_passage_resolves_alias_and_preserves_evidence(self):
        mapped = self.with_evidence(self.passage_file())
        row = mapped["sources"][0]["topics"][2]
        self.assertEqual(row["status"], "matched_verified_passage")
        self.assertEqual(row["targets"], [{"entry_id": "book-a/rules/nested", "anchor": "deep-anchor"}])
        self.assertEqual(row["title_candidates"], [])
        self.assertFalse(any(issue["url"] == "u3" for issue in mapped["unmapped_sections"]))
        self.assertEqual(row["evidence"]["source_locator"], "Book A pg. 1")

    def test_stale_passage_evidence_fails_instead_of_guessing(self):
        path = self.passage_file()
        self.write(self.entries / "new.json", [])
        with self.assertRaisesRegex(ValueError, "stale"):
            self.with_evidence(path)

    def test_passage_target_source_anchor_and_proof_are_validated(self):
        payload = json.loads((self.entries / "rule.json").read_text())
        payload.append({"id": "book-b/rules/copy", "kind": "rule", "source_ids": ["book-b"], "body": []})
        self.write(self.entries / "rule.json", payload)
        for change, message in [
            ({"target": {"entry_id": "book-b/rules/copy", "anchor": None}}, "outside its source"),
            ({"target": {"entry_id": "book-a/rules/nested", "anchor": "missing"}}, "anchor"),
            ({"evidence": {}}, "lacks primary-source evidence"),
        ]:
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, message):
                self.with_evidence(self.passage_file(**change))

    def test_unused_or_renamed_passage_evidence_fails(self):
        with self.assertRaisesRegex(ValueError, "absent from the current directory"):
            self.with_evidence(self.passage_file(url="u999"))
        with self.assertRaisesRegex(ValueError, "label changed"):
            self.with_evidence(self.passage_file(label="Renamed alias"))

    def test_changed_book_index_invalidates_evidence(self):
        path = self.passage_file()
        self.write(self.books, {"books": []})
        with self.assertRaisesRegex(ValueError, "stale for this book index"):
            self.with_evidence(path)

    def test_hierarchy_target_keeps_unverified_opening_explicit(self):
        payload = json.loads((self.entries / "rule.json").read_text())
        payload.append({"id": "book-a/rules/shared", "kind": "rule", "source_ids": ["book-a"], "body": []})
        self.write(self.entries / "rule.json", payload)
        evidence = {"method": "exact_source_book_chapter_with_shared_descendant_headings", "source_locator": "Book A pg. 1",
                    "page_title": "Shared", "opening_words_sha256": "a" * 64, "chapter_path": [0],
                    "shared_headings": ["first", "second", "third"], "opening_passage_result": "opening_passage_not_matched"}
        path = self.passage_file(url="u2", label="Shared", target={"entry_id": "book-a/rules/shared", "anchor": None}, evidence=evidence)
        row = self.with_evidence(path)["sources"][0]["topics"][1]
        self.assertEqual(row["status"], "matched_verified_chapter")
        self.assertEqual(row["evidence"]["opening_passage_result"], "opening_passage_not_matched")
        evidence["chapter_path"] = [True]
        with self.assertRaisesRegex(ValueError, "no book chapter"):
            self.with_evidence(self.passage_file(url="u2", label="Shared", target={"entry_id": "book-a/rules/shared", "anchor": None}, evidence=evidence))


if __name__ == "__main__":
    unittest.main()
