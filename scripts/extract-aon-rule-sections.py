#!/usr/bin/env python3
"""Extract AoN Rules directory navigation from a saved Rules.aspx page.

This records labels and links only. It does not fetch pages or retain article text.
"""
import argparse
from html.parser import HTMLParser
import json
import re
import sys
import unicodedata
from pathlib import Path
from urllib.parse import urljoin


def clean(value):
    return re.sub(r"\s+", " ", value).strip().removesuffix("►").strip()


class DirectoryParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack = []
        self.books = []
        self.book = None
        self.heading = None
        self.active_link = None
        self.inside_output = False
        self.ul_depth = 0

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id") == "MainContent_DetailedOutput":
            self.inside_output = True
        self.stack.append((tag, attrs))
        if not self.inside_output:
            return
        if tag == "ul":
            self.ul_depth += 1
        if tag == "h1":
            self.heading = {"text": "", "attrs": attrs}
        if tag == "a" and attrs.get("href"):
            self.active_link = {"href": attrs["href"], "text": ""}

    def handle_data(self, data):
        if self.heading is not None:
            self.heading["text"] += data
        if self.active_link is not None:
            self.active_link["text"] += data

    def handle_endtag(self, tag):
        if not self.inside_output:
            if tag == "span":
                self.stack = [item for item in self.stack if item[0] != tag]
            return
        if tag == "ul":
            self.ul_depth = max(0, self.ul_depth - 1)
        if tag == "a" and self.active_link is not None:
            text = clean(self.active_link["text"])
            href = urljoin("https://aonprd.com/", self.active_link["href"])
            if text and self.book is not None:
                if self.ul_depth and self.book["topics"]:
                    topic = self.book["topics"][-1]
                    depth = max(0, self.ul_depth - 1)
                    topic["subsections"].append({"label": text, "url": href, "depth": depth})
                elif not self.ul_depth:
                    self.book["topics"].append({"label": text, "url": href, "subsections": []})
            self.active_link = None
        if tag == "h1" and self.heading is not None:
            label = clean(self.heading["text"])
            if label:
                self.book = {"label": label, "heading_id": self.heading["attrs"].get("id"), "topics": []}
                self.books.append(self.book)
            self.heading = None
        if tag == "div" and self.book is not None:
            # Source group divs end before the next h1. Closing nested elements is harmless;
            # only seal when this is an AoN source group container.
            attrs = next((a for name, a in reversed(self.stack) if name == "div"), {})
            if attrs.get("id", "").startswith("SO_Rules"):
                self.book = None
        for i in range(len(self.stack) - 1, -1, -1):
            if self.stack[i][0] == tag:
                self.stack = self.stack[:i]
                break
        if tag == "span" and not any(a.get("id") == "MainContent_DetailedOutput" for _, a in self.stack):
            self.inside_output = False


def normalize(value):
    value = unicodedata.normalize("NFKD", value).encode("ascii", "ignore").decode().lower()
    value = re.sub(r"\b(pathfinder|prpg|rpg)\b", " ", value)
    return re.sub(r"[^a-z0-9]+", " ", value).strip()


def walk_book_nodes(books):
    by_source = {}
    def walk(node, found):
        if node.get("entry"):
            found.setdefault(normalize(node["title"]), set()).add(node["entry"])
        for child in node.get("children", []):
            walk(child, found)
    for book in books:
        found = by_source.setdefault(book["source"], {})
        for node in book.get("chapters", []):
            walk(node, found)
    return by_source


def mapping_for(inventory, catalog_path, books_path):
    catalog = json.loads(Path(catalog_path).read_text())
    index = json.loads(Path(books_path).read_text())
    sources = catalog.get("sources", [])
    source_by_label = {}
    source_by_id = {}
    for source in sources:
        source_by_label.setdefault(normalize(source["title"]), []).append(source["id"])
        source_by_id[source["id"]] = source
    nodes = walk_book_nodes(index.get("books", []))
    mapped_sources = []
    unmapped_sections = []
    for book in inventory["books"]:
        matches = source_by_label.get(normalize(book["label"]), [])
        source_record = {
            "aon_label": book["label"],
            "aon_heading_id": book["heading_id"],
            "canonical_book_ids": sorted(matches),
            "title_aliases": [
                {
                    "alias": book["label"],
                    "canonical_title": source_by_id[source_id]["title"],
                    "evidence": "exact normalized title match against installed catalog.json",
                }
                for source_id in sorted(matches)
            ],
            "status": "matched_exact_title" if len(matches) == 1 else "ambiguous_exact_title" if matches else "unmapped",
            "evidence": "normalized exact title match against installed catalog.json",
            "topics": [],
        }
        # Repeated section names in a different source are not evidence of membership.
        book_nodes = nodes.get(matches[0], {}) if len(matches) == 1 else {}
        for topic in book["topics"]:
            topic_record = map_section(topic, book_nodes)
            source_record["topics"].append(topic_record)
            if topic_record["status"] != "matched_exact_title":
                unmapped_sections.append(section_issue(book, topic, topic_record["status"]))
            for subsection, mapped in zip(topic["subsections"], topic_record["subsections"]):
                if mapped["status"] != "matched_exact_title":
                    unmapped_sections.append(section_issue(book, subsection, mapped["status"]))
        mapped_sources.append(source_record)
    return {
        "evidence": "site_navigation_not_printed_contents",
        "mapping_method": "repeatable normalized exact source title matching against catalog.json, then exact section titles within that book in books.json; no fuzzy or cross-book assignments",
        "sources": mapped_sources,
        "unmapped_sections": unmapped_sections,
    }


def map_section(section, nodes):
    def one(item):
        candidates = sorted(nodes.get(normalize(item["label"]), set()))
        return {
            "label": item["label"],
            "url": item["url"],
            "depth": item.get("depth", 0),
            "candidate_entry_ids": candidates,
            "status": "matched_exact_title" if len(candidates) == 1 else "ambiguous_exact_title" if candidates else "unmapped",
            "evidence": "normalized exact title match against installed books.json entry nodes",
        }
    return {**one(section), "subsections": [one(item) for item in section["subsections"]]}


def section_issue(book, section, status):
    return {"source_label": book["label"], "label": section["label"], "url": section["url"], "status": status}


def dump(path, data):
    text = json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    Path(path).write_text(text, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("html", help="saved AoN Rules.aspx HTML file")
    parser.add_argument("output", help="inventory JSON output path")
    parser.add_argument("--catalog", help="installed generated catalog.json for mapping")
    parser.add_argument("--books", help="installed generated books.json for mapping")
    parser.add_argument("--mapping-output", help="optional mapping JSON output path")
    args = parser.parse_args()
    parsed = DirectoryParser()
    parsed.feed(Path(args.html).read_text(encoding="utf-8", errors="replace"))
    inventory = {
        "source_url": "https://aonprd.com/Rules.aspx?Name=Source&Category=None",
        "evidence": "site_navigation_not_printed_contents",
        "books": parsed.books,
    }
    dump(args.output, inventory)
    if args.mapping_output:
        if not args.catalog or not args.books:
            parser.error("--mapping-output requires both --catalog and --books")
        dump(args.mapping_output, mapping_for(inventory, args.catalog, args.books))
    if not inventory["books"]:
        print("No source headings found; check the saved page structure.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
