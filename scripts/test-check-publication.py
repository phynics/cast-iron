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


class PublicationLicenseTests(unittest.TestCase):
    def accepted(self, refs, source_ids, known, cup_sources):
        return publication_license.has_reviewed_license_basis(
            {"license_refs": refs, "source_ids": source_ids}, known, cup_sources
        )

    def test_cup_only_entry_with_all_sources_cup_is_accepted(self):
        self.assertTrue(self.accepted([CUP], ["blog-a", "blog-b"],
                                      {"blog-a", "blog-b"}, {"blog-a", "blog-b"}))

    def test_no_license_refs_is_rejected(self):
        self.assertFalse(self.accepted([], ["blog-a"], {"blog-a"}, {"blog-a"}))

    def test_cup_ref_without_source_cup_row_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["book-a"], {"book-a"}, set()))

    def test_mixed_cup_and_ogl_default_sources_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["blog-a", "book-a"],
                                       {"blog-a", "book-a"}, {"blog-a"}))

    def test_unknown_source_is_rejected(self):
        self.assertFalse(self.accepted([CUP], ["unknown"], set(), {"unknown"}))


if __name__ == "__main__":
    unittest.main()
