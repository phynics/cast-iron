#!/usr/bin/env python3
"""Test publication license decisions with synthetic inputs."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "publication_license", Path(__file__).with_name("publication-license.py")
)
publication_license = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publication_license)

CUP = publication_license.COMMUNITY_USE
OGL = "ogl-1.0a"

# Raw `publication_licenses` rows, as the corpus `legal.toml` carries them.
LICENSE_ROWS = [
    {"publication_id": "blog-a", "license_refs": [CUP]},
    {"publication_id": "blog-b", "license_refs": [CUP]},
    {"publication_id": "book-ogl", "license_refs": [OGL]},
]


class CupPublicationTests(unittest.TestCase):
    def test_only_rows_naming_community_use_count(self):
        self.assertEqual(
            publication_license.cup_publications(LICENSE_ROWS), {"blog-a", "blog-b"}
        )

    def test_no_rows(self):
        self.assertEqual(publication_license.cup_publications([]), set())


class LicenseBasisTests(unittest.TestCase):
    def accepted(self, refs, source_ids):
        cup_sources = publication_license.cup_publications(LICENSE_ROWS)
        return publication_license.has_license_basis(
            {"license_refs": refs, "source_ids": source_ids}, cup_sources
        )

    def test_cup_only_entry_with_all_sources_cup_is_accepted(self):
        self.assertTrue(self.accepted([CUP], ["blog-a", "blog-b"]))

    def test_ogl_entry_is_accepted(self):
        self.assertTrue(self.accepted([OGL, CUP], ["book-a"]))

    def test_no_license_refs_is_rejected(self):
        self.assertFalse(self.accepted([], ["blog-a"]))

    def test_cup_ref_without_source_cup_row_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["book-a"]))

    def test_cup_ref_with_ogl_only_license_row_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["book-ogl"]))

    def test_mixed_cup_and_ogl_default_sources_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["blog-a", "book-a"]))

    def test_cup_ref_without_sources_is_rejected(self):
        self.assertFalse(self.accepted([CUP], []))

    def test_unknown_source_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["unknown"]))


if __name__ == "__main__":
    unittest.main()
