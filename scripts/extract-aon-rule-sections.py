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
import glob
import hashlib
from collections import Counter
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


def entries_digest(directory):
    lines = [f"{path.name}:{hashlib.sha256(path.read_bytes()).hexdigest()}\n" for path in sorted(Path(directory).glob("*.json"))]
    return hashlib.sha256("".join(lines).encode()).hexdigest()


def load_rule_evidence(path, entries_dir, books_path):
    if not path:
        return {}
    if not entries_dir:
        raise ValueError("passage evidence requires typed entries")
    evidence = json.loads(Path(path).read_text())
    if evidence.get("format") != "cast-iron-aon-rule-evidence" or evidence.get("version") != 1:
        raise ValueError("unsupported passage evidence format")
    if evidence.get("entries_sha256") != entries_digest(entries_dir):
        raise ValueError("passage evidence is stale for these typed entries; rerun the audit")
    if evidence.get("books_sha256") != hashlib.sha256(Path(books_path).read_bytes()).hexdigest():
        raise ValueError("mapping evidence is stale for this book index")
    books = json.loads(Path(books_path).read_text())
    entries = {entry["id"]: entry for file in sorted(Path(entries_dir).glob("*.json")) for entry in json.loads(file.read_text())}
    selected = {}
    for record in evidence["rows"]:
        if record["status"] != "proposed_target":
            continue
        key = (record["source"], record["url"])
        if key in selected:
            raise ValueError(f"duplicate passage evidence: {key}")
        target = record["target"]
        entry = entries.get(target["entry_id"])
        if not entry or entry.get("kind") != "rule" or record["source"] not in entry.get("source_ids", []):
            raise ValueError(f"passage target is missing or outside its source: {key}")
        def anchors(value):
            if isinstance(value, list):
                for child in value:
                    yield from anchors(child)
            elif isinstance(value, dict):
                if value.get("t") == "heading" and value.get("anchor"):
                    yield value["anchor"]
                for child in value.values():
                    if isinstance(child, (dict, list)):
                        yield from anchors(child)
        anchor = target.get("anchor")
        if anchor is not None and Counter(anchors(entry.get("body", [])))[anchor] != 1:
            raise ValueError(f"passage anchor is missing or ambiguous: {key}")
        proof = record.get("evidence", {})
        if proof.get("method") not in {"unique_exact_source_opening_passage", "exact_source_opening_passage_and_dedicated_article_title", "exact_source_book_chapter_with_shared_descendant_headings"} or not proof.get("source_locator") or not proof.get("page_title") or not re.fullmatch(r"[a-f0-9]{64}", proof.get("opening_words_sha256", "")):
            raise ValueError(f"passage target lacks primary-source evidence: {key}")
        if proof["method"] == "exact_source_book_chapter_with_shared_descendant_headings":
            path = proof.get("chapter_path", [])
            book = next((book for book in books["books"] if book["source"] == record["source"]), None)
            if not book or len(path) != 1 or type(path[0]) is not int or not 0 <= path[0] < len(book["chapters"]):
                raise ValueError(f"hierarchy evidence has no book chapter: {key}")
            chapter = book["chapters"][path[0]]
            if chapter.get("entry") != target["entry_id"] or anchor is not None or normalize(chapter["title"]) != normalize(record["label"]) or len(set(proof.get("shared_headings", []))) < 3:
                raise ValueError(f"hierarchy evidence does not identify its named chapter: {key}")
        selected[key] = record
    return selected


def apply_rule_evidence(row, source, evidence, used):
    key = (source, row["url"])
    record = evidence.get(key)
    if record is None:
        return
    if record["label"] != row["label"]:
        raise ValueError(f"passage evidence label changed: {key}")
    used.add(key)
    row["title_candidates"] = row["targets"]
    row["targets"] = [record["target"]]
    row["candidate_entry_ids"] = [record["target"]["entry_id"]]
    row["status"] = "matched_verified_chapter" if record["evidence"]["method"] == "exact_source_book_chapter_with_shared_descendant_headings" else "matched_verified_passage"
    row["evidence"] = {"source_url": record["url"], **record["evidence"]}


def mapping_for(inventory, catalog_path, books_path, entries_dir=None, aliases_path=None, evidence_path=None):
    catalog = json.loads(Path(catalog_path).read_text())
    index = json.loads(Path(books_path).read_text())
    sources = catalog.get("sources", [])
    aliases = json.loads(Path(aliases_path).read_text()) if aliases_path and Path(aliases_path).exists() else {"aliases": []}
    source_by_label = {}
    source_by_id = {}
    for source in sources:
        source_by_label.setdefault(normalize(source["title"]), []).append(source["id"])
        source_by_id[source["id"]] = source
    for alias in aliases.get("aliases", []):
        # Alias targets are checked against the current catalog on every run.
        if alias["canonical_book_id"] not in source_by_id:
            raise ValueError(f"alias target is not in installed catalog: {alias}")
        if not alias.get("evidence", "").strip():
            raise ValueError(f"alias lacks evidence: {alias}")
        source_by_label.setdefault(normalize(alias["alias"]), []).append(alias["canonical_book_id"])
    passage_evidence = load_rule_evidence(evidence_path, entries_dir, books_path)
    used_evidence = set()
    nodes = walk_book_nodes(index.get("books", []))
    # Typed entry bodies retain headings which are not represented as book-tree nodes.
    if entries_dir:
        for path in sorted(glob.glob(str(Path(entries_dir) / "*.json"))):
            for entry in json.loads(Path(path).read_text()):
                if entry.get("kind") != "rule":
                    continue
                for source_id in entry.get("source_ids", []):
                    table = nodes.setdefault(source_id, {})
                    def inline_text(value):
                        if isinstance(value, list):
                            return "".join(inline_text(item) for item in value)
                        if not isinstance(value, dict):
                            return ""
                        if value.get("t") == "text":
                            return value.get("v", "")
                        return "".join(inline_text(v) for k, v in value.items() if k not in {"t", "anchor", "level"})
                    def headings(value):
                        if isinstance(value, list):
                            for component in value:
                                headings(component)
                        elif isinstance(value, dict):
                            if value.get("t") == "heading":
                                title = inline_text(value.get("content", []))
                                if title:
                                    target = {"entry_id": entry["id"], "anchor": value.get("anchor")}
                                    table.setdefault(normalize(title), set()).add(json.dumps(target, sort_keys=True))
                            for key, child in value.items():
                                if key not in {"t", "anchor", "level"}:
                                    headings(child)
                    headings(entry.get("body", []))
    mapped_sources = []
    unmapped_sections = []
    for book in inventory["books"]:
        matches = sorted(set(source_by_label.get(normalize(book["label"]), [])))
        alias_evidence = {
            item["canonical_book_id"]: item["evidence"]
            for item in aliases.get("aliases", [])
            if normalize(item["alias"]) == normalize(book["label"])
        }
        source_record = {
            "aon_label": book["label"],
            "aon_heading_id": book["heading_id"],
            "canonical_book_ids": sorted(matches),
            "title_aliases": [
                {
                    "alias": book["label"],
                    "canonical_title": source_by_id[source_id]["title"],
                    "evidence": alias_evidence.get(source_id, "exact normalized title match against installed catalog.json"),
                }
                for source_id in sorted(matches)
            ],
            "status": "matched_exact_title" if len(set(matches)) == 1 else "ambiguous_exact_title" if matches else "source_not_installed",
            "evidence": "normalized exact source title or reviewed alias match against installed catalog.json",
            "topics": [],
        }
        # Repeated section names in a different source are not evidence of membership.
        book_nodes = nodes.get(matches[0], {}) if len(matches) == 1 else {}
        for topic in book["topics"]:
            topic_record = map_section(topic, book_nodes, not matches)
            source_record["topics"].append(topic_record)
            for row in [topic_record, *topic_record["subsections"]]:
                if len(matches) == 1:
                    apply_rule_evidence(row, matches[0], passage_evidence, used_evidence)
                if row["status"] not in {"matched_exact_title", "matched_verified_passage", "matched_verified_chapter"}:
                    unmapped_sections.append(section_issue(book, row, row["status"]))
        mapped_sources.append(source_record)
    if set(passage_evidence) != used_evidence:
        raise ValueError("passage evidence includes a target absent from the current directory")
    return {
        "evidence": "site_navigation_not_printed_contents",
        "mapping_method": "exact source/title matching plus explicitly selected source-validated passage and chapter evidence; unverified duplicate targets remain ambiguous; no fuzzy or cross-book assignments",
        "limitations": "Target identity does not establish complete article text or printed order. A section_not_in_snapshot row has no verified target and may be a navigation label rather than absent prose. Chapter targets retain opening-passage gaps explicitly.",
        "sources": mapped_sources,
        "unmapped_sections": unmapped_sections,
    }


def map_section(section, nodes, source_absent=False):
    def one(item):
        raw = nodes.get(normalize(item["label"]), set())
        targets = []
        for value in raw:
            if isinstance(value, str) and value.startswith("{"):
                target = json.loads(value)
            else:
                target = {"entry_id": value, "anchor": None}
            if target not in targets:
                targets.append(target)
        targets.sort(key=lambda t: (t["entry_id"], t["anchor"] or ""))
        candidates = sorted({target["entry_id"] for target in targets})
        status = "matched_exact_title" if len(targets) == 1 else "ambiguous_exact_title" if targets else "source_not_installed" if source_absent else "section_not_in_snapshot"
        return {
            "label": item["label"],
            "url": item["url"],
            "depth": item.get("depth", 0),
            "candidate_entry_ids": candidates,
            "targets": targets,
            "status": status,
            "evidence": "normalized exact title match against installed books.json nodes and typed rule-body headings",
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
    parser.add_argument("--entries", help="directory containing typed generated entry JSON")
    parser.add_argument("--evidence", help="reviewed passage/chapter evidence JSON from audit-book-content.py")
    parser.add_argument("--aliases", help="reviewed source title aliases JSON")
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
        dump(args.mapping_output, mapping_for(inventory, args.catalog, args.books, args.entries, args.aliases, args.evidence))
    if not inventory["books"]:
        print("No source headings found; check the saved page structure.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
