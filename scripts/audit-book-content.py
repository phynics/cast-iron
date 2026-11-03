#!/usr/bin/env python3
"""Compare unresolved AoN navigation with installed typed opening passages.

Research only: fetched pages stay in an explicit local cache. Output contains targets,
metadata and hashes, never upstream prose. This does not modify book order or the library.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import subprocess
import unicodedata
from urllib.parse import parse_qs, urlparse
from urllib.request import Request, urlopen

MIN_WORDS = 12
PREFIX_WORDS = 24
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"}


def words(text):
    text = text.translate(str.maketrans({"’": "'", "‘": "'", "—": " ", "–": " ", "‑": "-"}))
    text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode().lower()
    return tuple(re.findall(r"[a-z0-9]+", text))


def digest(value):
    return hashlib.sha256(value.encode()).hexdigest()


class ArticleParser(HTMLParser):
    """Collect only the first heading, source line, and its opening prose section."""
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack = []
        self.output_depth = None
        self.heading = None
        self.heading_text = []
        self.titles = []
        self.parts = []
        self.sections = []
        self.sources = []

    def flush(self):
        text = " ".join("".join(self.parts).split())
        self.parts = []
        if not text or not self.titles:
            return
        if text.startswith("Source "):
            self.sources.append(text.removeprefix("Source "))
        elif self.sections:
            self.sections[-1].append(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id") == "MainContent_DetailedOutput":
            self.output_depth = len(self.stack)
        if tag not in VOID:
            self.stack.append(tag)
        if self.output_depth is None:
            return
        if re.fullmatch(r"h[1-6]", tag):
            self.flush()
            self.heading = tag
            self.heading_text = []
        elif tag in {"br", "p", "table", "tr", "li"}:
            self.flush()
        elif tag in {"td", "th"}:
            self.parts.append(" ")

    def handle_data(self, data):
        if self.output_depth is None:
            return
        if self.heading:
            self.heading_text.append(data)
        else:
            self.parts.append(data)

    def handle_endtag(self, tag):
        if self.output_depth is not None:
            if tag == self.heading:
                self.titles.append(" ".join("".join(self.heading_text).split()))
                self.sections.append([])
                self.heading = None
            elif tag in {"p", "li", "table", "tr"}:
                self.flush()
        if tag in self.stack:
            index = len(self.stack) - 1 - self.stack[::-1].index(tag)
            if self.output_depth is not None and index <= self.output_depth:
                self.flush()
                self.output_depth = None
            self.stack = self.stack[:index]

    def result(self):
        self.flush()
        first = self.sections[0] if self.sections else []
        return {"title": self.titles[0] if self.titles else None,
                "source": self.sources[0] if self.sources else None,
                "paragraph": first[0] if first else None,
                "section_paragraph_count": len(first)}


def inline_text(value):
    if isinstance(value, list):
        return "".join(inline_text(item) for item in value)
    if isinstance(value, dict):
        if value.get("t") == "text":
            return value.get("v", "")
        if value.get("t") == "br":
            return " "
        return inline_text(value.get("c", []))
    return ""


def paragraphs(value, path=()):
    if isinstance(value, list):
        for index, item in enumerate(value):
            yield from paragraphs(item, (*path, index))
    elif isinstance(value, dict):
        if value.get("t") == "paragraph":
            yield path, inline_text(value.get("content", []))
        elif value.get("t") in {"list", "quote"}:
            field = "items" if value["t"] == "list" else "content"
            yield from paragraphs(value.get(field, []), (*path, field))


def entry_units(entry):
    body = entry.get("body", [])
    all_paragraphs = list(paragraphs(body))
    if not all_paragraphs:
        return
    first_path, first_text = all_paragraphs[0]
    yield {"entry_id": entry["id"], "anchor": None, "paragraph_path": first_path,
           "text": first_text, "title": entry["title"]}
    for path, text in all_paragraphs:
        previous = [block for block in body[:path[0]] if block.get("t") == "heading" and block.get("anchor")]
        heading = previous[-1] if previous else None
        yield {"entry_id": entry["id"], "anchor": heading["anchor"] if heading else None,
               "paragraph_path": path, "text": text,
               "title": inline_text(heading.get("content", [])) if heading else entry["title"]}


def prefix(text):
    tokens = words(text)
    return tokens[:PREFIX_WORDS] if len(tokens) >= MIN_WORDS else None


def build_index(entries):
    result = defaultdict(list)
    for entry in entries:
        if entry.get("kind") != "rule":
            continue
        for unit in entry_units(entry):
            key = prefix(unit["text"])
            if key:
                for source in entry.get("source_ids", []):
                    result[(source, key)].append(unit)
    return result


def match_passage(article, source, index):
    key = prefix(article.get("paragraph") or "")
    if key is None:
        return None, [], "no_opening_prose_or_too_short"
    matches = index.get((source, key), [])
    # Root and a leading heading at the exact same paragraph are equivalent start links.
    by_position = {}
    for unit in matches:
        position = (unit["entry_id"], unit["paragraph_path"])
        previous = by_position.get(position)
        if previous is None or unit["anchor"] is None:
            by_position[position] = unit
    choices = list({(unit["entry_id"], unit["anchor"]): unit for unit in by_position.values()}.values())
    dedicated = [unit for unit in choices if unit["anchor"] is None and words(unit["title"]) == words(article.get("title") or "")]
    if len(dedicated) == 1:
        chosen, reason = dedicated[0], "exact_source_opening_passage_and_dedicated_article_title"
    elif len(choices) == 1:
        chosen, reason = choices[0], "unique_exact_source_opening_passage"
    else:
        chosen, reason = None, "ambiguous_opening_passage" if choices else "opening_passage_not_matched"
    target = lambda unit: {"entry_id": unit["entry_id"], "anchor": unit["anchor"]}
    candidates = sorted((target(unit) for unit in choices), key=lambda t: (t["entry_id"], t["anchor"] or ""))
    return target(chosen) if chosen else None, candidates, reason


def unresolved_rows(mapping):
    for source in mapping["sources"]:
        if len(source["canonical_book_ids"]) != 1:
            continue
        for topic in source["topics"]:
            for row in [topic, *topic["subsections"]]:
                if row["status"] in {"ambiguous_exact_title", "section_not_in_snapshot"}:
                    yield source["canonical_book_ids"][0], row


def cache_file(directory, url):
    parsed = urlparse(url)
    identifier = parse_qs(parsed.query).get("ID", [""])[0]
    if parsed.scheme != "https" or parsed.netloc != "aonprd.com" or parsed.path != "/Rules.aspx" or not identifier.isdigit():
        raise ValueError(f"not an AoN rule identity: {url}")
    return directory / f"rules-{identifier}.html"


def chapter_target(article, source, row, books, entries):
    candidates = []
    for book in books["books"]:
        if book["source"] != source:
            continue
        for position, chapter in enumerate(book["chapters"]):
            if words(chapter["title"]) != words(row["label"]) or chapter.get("entry") not in row.get("candidate_entry_ids", []):
                continue
            local_titles = set()
            def walk(node):
                local_titles.add(" ".join(words(node["title"])))
                entry = entries.get(node.get("entry"))
                if entry:
                    for block in entry.get("body", []):
                        if block.get("t") == "heading":
                            local_titles.add(" ".join(words(inline_text(block.get("content", [])))))
                for child in node.get("children", []):
                    walk(child)
            walk(chapter)
            shared = sorted(local_titles & {" ".join(words(title)) for title in article["headings"][1:]})
            if len(shared) >= 3:
                candidates.append(({"entry_id": chapter["entry"], "anchor": None}, position, shared))
    return candidates[0] if len(candidates) == 1 else None


def entries_digest(directory):
    lines = [f"{path.name}:{hashlib.sha256(path.read_bytes()).hexdigest()}\n" for path in sorted(directory.glob("*.json"))]
    return digest("".join(lines))


def source_label(text):
    text = re.split(r"\s+pg\.\s*", text or "", maxsplit=1)[0]
    return " ".join(token for token in words(text) if token not in {"pathfinder", "rpg", "prpg"})


def expected_source_labels(catalog, aliases):
    result = {source["id"]: {source_label(source["title"])} for source in catalog["sources"]}
    for alias in aliases.get("aliases", []):
        target = alias["canonical_book_id"]
        if target not in result or not alias.get("evidence", "").strip():
            raise ValueError("source aliases must identify an installed source with evidence")
        result[target].add(source_label(alias["alias"]))
    return result


def supplemental_records(root, rows, pages_dir, source_labels):
    expected_pin = "08ea71f24375908d0190c822fd1fd080797667a9"
    pin = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
    if pin != expected_pin:
        raise ValueError(f"Pf Data checkout must be pinned to {expected_pin}, got {pin}")
    missing = [row for row in rows if row["status"] == "opening_passage_not_matched" or row.get("evidence", {}).get("opening_passage_result") == "opening_passage_not_matched"]
    needles = {}
    for row in missing:
        article = ArticleParser()
        article.feed(cache_file(pages_dir, row["url"]).read_text())
        needles[row["url"]] = prefix(article.result()["paragraph"])
    candidates = defaultdict(list)
    def named_objects(value, pointer=""):
        if isinstance(value, dict):
            if isinstance(value.get("name"), str) and isinstance(value.get("sources"), list) and isinstance(value.get("description"), list):
                yield pointer, value
            for key, child in value.items():
                yield from named_objects(child, pointer + "/" + key.replace("~", "~0").replace("/", "~1"))
        elif isinstance(value, list):
            for index, child in enumerate(value):
                yield from named_objects(child, pointer + f"/{index}")
    for file in sorted((root / "json").glob("*.json")):
        raw = file.read_bytes()
        for pointer, record in named_objects(json.loads(raw)):
            description = record["description"]
            if not all(isinstance(line, str) for line in description):
                continue
            text = "\n".join(description)
            normalized = " ".join(words(text))
            tags = {source_label(tag) for tag in record["sources"] if isinstance(tag, str)}
            for row in missing:
                needle = needles[row["url"]]
                if needle and tags & source_labels[row["source"]] and (" " + " ".join(needle) + " ") in (" " + normalized + " "):
                    directives = re.findall(r"‹([^›]*)›", text)
                    candidates[row["url"]].append({"file": "json/" + file.name, "file_sha256": hashlib.sha256(raw).hexdigest(),
                        "json_pointer": pointer, "name": record["name"], "sources": record["sources"],
                        "description_bytes": len(text.encode()), "description_sha256": digest(text),
                        "non_source_directives": sorted(set(d for d in directives if not d.startswith("SOURCE "))),
                        "markdown_headings": sum(bool(re.match(r"^#{1,6} ", line)) for line in description)})
    return {"repository": "https://github.com/jasontankapps/pathfinder-data-1-e", "commit": pin,
            "method": "exact source-tag and 24-word opening-passage match in named-object descriptions; candidate data only, not an import or completeness claim",
            "rows": [{"source": row["source"], "url": row["url"], "label": row["label"],
                      "status": "candidate_records" if candidates[row["url"]] else "no_candidate_passage",
                      "candidates": candidates[row["url"]]} for row in missing]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mapping", required=True, type=Path)
    parser.add_argument("--entries", required=True, type=Path)
    parser.add_argument("--pages-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--aliases", type=Path, help="Reviewed source-title aliases, if needed")
    parser.add_argument("--pf-data-dir", type=Path, help="Optional pinned Pf Data checkout to locate candidate missing passages")
    parser.add_argument("--fetch", action="store_true", help="Fetch only uncached unresolved installed-source URLs, sequentially")
    args = parser.parse_args()
    mapping = json.loads(args.mapping.read_text())
    entries = [entry for file in sorted(args.entries.glob("*.json")) for entry in json.loads(file.read_text())]
    index = build_index(entries)
    catalog = json.loads((args.entries.parent / "catalog.json").read_text())
    aliases = json.loads(args.aliases.read_text()) if args.aliases else {"aliases": []}
    source_labels = expected_source_labels(catalog, aliases)
    books_path = args.entries.parent / "books.json"
    books = json.loads(books_path.read_text())
    entries_by_id = {entry["id"]: entry for entry in entries}
    args.pages_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for source, row in unresolved_rows(mapping):
        saved = cache_file(args.pages_dir, row["url"])
        record = {"source": source, "url": row["url"], "label": row["label"], "previous_status": row["status"]}
        try:
            if args.fetch and not saved.exists():
                request = Request(row["url"], headers={"User-Agent": "CastIron-coverage-research/1.0 (bounded navigation audit)"})
                with urlopen(request, timeout=30) as response:
                    data = response.read(4 * 1024 * 1024 + 1)
                if len(data) > 4 * 1024 * 1024:
                    raise ValueError("page exceeds research size limit")
                saved.write_bytes(data)
            if not saved.exists():
                record.update(status="page_not_cached")
            else:
                article = ArticleParser()
                article.feed(saved.read_text(errors="replace"))
                parsed = article.result()
                parsed["headings"] = article.titles
                if source_label(parsed.get("source")) not in source_labels[source]:
                    record.update(status="primary_source_mismatch", page_title=parsed["title"], source_locator=parsed["source"])
                    rows.append(record)
                    continue
                selected, candidates, reason = match_passage(parsed, source, index)
                passage_reason = reason
                hierarchy = None
                if selected is None and row["status"] == "ambiguous_exact_title":
                    hierarchy = chapter_target(parsed, source, row, books, entries_by_id)
                    if hierarchy:
                        selected = hierarchy[0]
                        reason = "exact_source_book_chapter_with_shared_descendant_headings"
                opening = " ".join(words(parsed.get("paragraph") or ""))
                record.update(status="proposed_target" if selected else reason, target=selected, candidates=candidates,
                              evidence={"method": reason, "page_title": parsed["title"], "source_locator": parsed["source"],
                                        "opening_words_sha256": digest(opening), "matched_prefix_words": min(PREFIX_WORDS, len(words(parsed.get("paragraph") or ""))),
                                        "first_section_paragraph_count": parsed["section_paragraph_count"]})
                if hierarchy:
                    record["evidence"].update(chapter_path=[hierarchy[1]], shared_headings=hierarchy[2], opening_passage_result=passage_reason)
        except Exception as error:
            record.update(status="fetch_or_parse_error", error=str(error))
        rows.append(record)
    report = {"format": "cast-iron-aon-rule-evidence", "version": 1, "purpose": "research_proposals_not_imported_content_or_printed_order", "mapping_sha256": digest(args.mapping.read_text()),
              "corpus_version": args.entries.parent.name,
              "entries_sha256": entries_digest(args.entries),
              "books_sha256": hashlib.sha256(books_path.read_bytes()).hexdigest(),
              "source_aliases_sha256": digest(json.dumps(aliases, sort_keys=True)),
              "counts": dict(sorted(Counter(row["status"] for row in rows).items())), "rows": rows}
    if args.pf_data_dir:
        report["supplemental_candidates"] = supplemental_records(args.pf_data_dir, rows, args.pages_dir, source_labels)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    print(json.dumps(report["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
