#!/usr/bin/env python3
"""Measure cross-reference coverage in a compiled library.

The issues #20 and #23-#26 were written against the PSRD library, which carried
few body links. The canonical Pf Data library encodes most cross-references as
links, so the numbers that matter now are the residue: bare italic or bold spans
that exactly name an entry of the category, and the share of that residue in
carried PSRD entries.

This script reads a compiled ``generated/content`` tree (canonical by default,
``--content generated/psrd`` for the carry-over input) and prints one section per
category. It also measures the spell-list structure that #42 asks about.

Usage:
    python3 scripts/measure-reference-links.py
    python3 scripts/measure-reference-links.py --content generated/psrd
    python3 scripts/measure-reference-links.py --json reports/reference-links.json

The category definitions come from the pinned Pf Data corpus where the corpus
has one (condition and special-ability sections) and from the entry kind
otherwise (spells, feats). Nothing is written unless ``--json`` is passed.
"""

from __future__ import annotations

import argparse
import collections
import glob
import json
import os
import re
import sys
from dataclasses import dataclass, field
from typing import Iterable, Iterator

SPELL_KINDS = {"spell"}
FEAT_KINDS = {"feat"}

# The standard PF1e condition list. The pinned corpus stores these under
# `json/misc.json` and `json/rules_combat01.json`; some also appear as mercy
# names, which are different entries with the same title.
CONDITION_TITLES = [
    "Blinded",
    "Confused",
    "Dazed",
    "Dazzled",
    "Deafened",
    "Fascinated",
    "Helpless",
    "Staggered",
    "Stunned",
    "Unconscious",
    "Disabled",
    "Dying",
    "Stable",
    "Dead",
    "Fatigued",
    "Exhausted",
    "Shaken",
    "Frightened",
    "Panicked",
    "Cowering",
    "Grappled",
    "Pinned",
    "Sickened",
    "Nauseated",
    "Entangled",
    "Flat-Footed",
    "Prone",
    "Petrified",
    "Paralyzed",
    "Invisible",
    "Incorporeal",
    "Energy Drained",
    "Broken",
    "Bleed",
]

CATEGORIES = ("spells", "feats", "special_abilities", "conditions")

PUNCTUATION = " \t\n\r,;:.!?"


def normalize(value: str) -> str:
    """Case-fold and collapse whitespace, then drop trailing punctuation."""
    value = " ".join(value.split()).casefold()
    return value.rstrip(" ,;.!?")


def text_of(value) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return "".join(text_of(item) for item in value)
    if isinstance(value, dict):
        if "v" in value:
            return value["v"]
        if value.get("t") == "break":
            return "\n"
        if "content" in value:
            return text_of(value["content"])
        if "rows" in value:
            return " ".join(text_of(row.get("value", [])) for row in value["rows"])
        return text_of(value.get("c", []))
    return ""


def contains_link(value) -> bool:
    if isinstance(value, list):
        return any(contains_link(item) for item in value)
    if isinstance(value, dict):
        return value.get("t") == "link" or contains_link(value.get("c", []))
    return False


def walk_inlines(inlines: Iterable[dict], inside_link: bool = False) -> Iterator[tuple]:
    """Yield ('link', target, text) and ('bare', emphasis, text) in body order."""
    for node in inlines:
        kind = node.get("t")
        if kind == "link":
            yield ("link", node.get("target", ""), text_of(node.get("c", [])))
            yield from walk_inlines(node.get("c", []), True)
        elif kind in ("em", "strong"):
            if not inside_link and not contains_link(node.get("c", [])):
                yield ("bare", kind, text_of(node.get("c", [])))
            yield from walk_inlines(node.get("c", []), inside_link)
        elif kind in ("sup", "sub"):
            yield from walk_inlines(node.get("c", []), inside_link)


def walk_blocks(blocks: Iterable[dict]) -> Iterator[tuple]:
    """Yield inline items plus the block context they came from.

    A paragraph whose whole content is one emphasis node is a section label
    ("**Spell-Like Abilities**", "**Constrict:**"), not a reference in prose.
    Headings are labels too. The caller counts both separately so that a category
    entry whose title is also a section name does not look like a link backlog.
    """
    for block in blocks:
        kind = block.get("t")
        if kind in ("paragraph", "heading"):
            content = block.get("content", [])
            standalone = len(content) == 1 and content[0].get("t") in ("em", "strong")
            context = {"block": kind, "standalone": standalone}
            for item in walk_inlines(content):
                yield (*item, context)
        elif kind == "stat_block":
            for row in block.get("rows", []):
                for item in walk_inlines(row.get("value", [])):
                    yield (*item, {"block": "stat_row", "standalone": False})
        elif kind == "list":
            for item in block.get("items", []):
                yield from walk_blocks(item)
        elif kind == "quote":
            yield from walk_blocks(block.get("content", []))
        elif kind == "table":
            for part in ("head", "body", "foot"):
                for row in block.get(part, []):
                    for cell in row.get("cells", []):
                        for item in walk_inlines(cell.get("content", [])):
                            yield (*item, {"block": "table", "standalone": False})


def load_json(path: str):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def load_pf_sections(dataset: str) -> tuple[set[tuple[str, str]], set[tuple[str, str]]]:
    """Return (condition records, special-ability records) as (path, key) pairs."""
    conditions: set[tuple[str, str]] = set()
    abilities: set[tuple[str, str]] = set()
    combat = os.path.join(dataset, "json", "rules_combat01.json")
    if not os.path.exists(combat):
        return conditions, abilities
    records = load_json(combat)

    def walk(key: str, target: set[tuple[str, str]]) -> None:
        record = records.get(key)
        if not record:
            return
        target.add(("json/rules_combat01.json", key))
        for child in record.get("subtopics", []):
            walk(child, target)

    walk("conditions", conditions)
    walk("special_abilities_2", abilities)
    # The universal monster rules are the second half of the special-ability
    # category. They are one flat record file, and carried entries keep their
    # "Universal Monster Rules" section.
    umr = os.path.join(dataset, "json", "umr.json")
    if os.path.exists(umr):
        for key in load_json(umr):
            abilities.add(("json/umr.json", key))
    return conditions, abilities


@dataclass
class CategoryResult:
    name: str
    targets: int = 0
    logs: dict = field(default_factory=collections.Counter)
    linked_by_title: collections.Counter = field(default_factory=collections.Counter)
    bare_by_title: collections.Counter = field(default_factory=collections.Counter)
    bare_by_source: collections.Counter = field(default_factory=collections.Counter)
    self_bare: collections.Counter = field(default_factory=collections.Counter)
    examples: list = field(default_factory=list)
    entries_with_link: set = field(default_factory=set)
    entries_with_bare: set = field(default_factory=set)
    # The same counts restricted to the issue's scope. #20 is about spell body
    # text; #24-#26 are about rules text in any entry.
    scope_logs: dict = field(default_factory=collections.Counter)
    scope_entries_with_link: set = field(default_factory=set)
    scope_entries_with_bare: set = field(default_factory=set)
    scope_bare_by_title: collections.Counter = field(default_factory=collections.Counter)


def classify_targets(catalog: list[dict], entries: dict[str, dict], conditions, abilities):
    """Return category -> {normalized title: [entry ids]} and ids -> categories."""
    by_id: dict[str, set[str]] = collections.defaultdict(set)
    titles: dict[str, dict[str, list[str]]] = {name: {} for name in CATEGORIES}
    for row in catalog:
        entry_id = row["id"]
        kind = row["kind"]
        title = normalize(row["title"])
        categories = set()
        if kind in SPELL_KINDS:
            categories.add("spells")
        if kind in FEAT_KINDS:
            categories.add("feats")
        if normalize(row["title"]) in {normalize(t) for t in CONDITION_TITLES}:
            categories.add("conditions")
        entry = entries.get(entry_id)
        if entry is not None:
            provenance = entry.get("provenance") or []
            pairs = {(p.get("path"), p.get("record")) for p in provenance}
            if pairs & conditions:
                categories.add("conditions")
            if pairs & abilities:
                categories.add("special_abilities")
            sections = " > ".join(entry.get("section") or []).casefold()
            if entry_id.startswith("ref/carried/") and (
                "universal monster rules" in sections
                or "special abilities" in sections
            ):
                categories.add("special_abilities")
        for category in categories:
            by_id[entry_id].add(category)
            titles[category].setdefault(title, []).append(entry_id)
    return titles, by_id


def measure_links(content: str, dataset: str) -> dict:
    manifest = load_json(os.path.join(content, "manifest.json"))
    version = manifest["content_version"]
    base = os.path.join(content, version)
    catalog = load_json(os.path.join(base, "catalog.json"))["entries"]
    conditions, abilities = load_pf_sections(dataset)

    # A first pass reads the entries that carry the pinned-corpus provenance used
    # for the condition and special-ability categories. Reading all 31k entries
    # twice is cheap relative to loading them all at once (the tree is ~220 MB).
    provenance: dict[str, dict] = {}
    shards = sorted(glob.glob(os.path.join(base, "entries", "*.json")))
    for shard in shards:
        for entry in load_json(shard):
            entry_id = entry["id"]
            if entry_id.startswith("ref/carried/") or entry.get("provenance"):
                provenance[entry_id] = entry
    titles, by_id = classify_targets(catalog, provenance, conditions, abilities)
    del provenance
    id_titles = {row["id"]: row["title"] for row in catalog}

    results = {name: CategoryResult(name) for name in CATEGORIES}
    for name, mapping in titles.items():
        results[name].targets = sum(len(ids) for ids in mapping.values())
    body_links = 0
    entries_with_link = 0

    for shard in shards:
        for entry in load_json(shard):
            entry_id = entry["id"]
            source = "carried" if entry_id.startswith("ref/carried/") else "pf"
            own = normalize(entry["title"])
            entry_kind = entry["kind"]
            links_here = 0
            for item in walk_blocks(entry.get("body", [])):
                if item[0] == "link":
                    links_here += 1
                    target = item[1]
                    for category in by_id.get(target, ()):
                        result = results[category]
                        result.logs["linked"] += 1
                        result.entries_with_link.add(entry_id)
                        result.linked_by_title[normalize(id_titles.get(target, target))] += 1
                        if category != "spells" or entry_kind == "spell":
                            result.scope_logs["linked"] += 1
                            result.scope_entries_with_link.add(entry_id)
                    continue
                _, emphasis, text, context = item
                key = normalize(text)
                if not key:
                    continue
                for category in CATEGORIES:
                    ids = titles[category].get(key)
                    if not ids:
                        continue
                    result = results[category]
                    in_scope = category != "spells" or entry_kind == "spell"
                    if context["block"] == "heading" or context["standalone"]:
                        result.logs["labels"] += 1
                        continue
                    if key == own:
                        result.logs["self"] += 1
                        result.self_bare[(key, source)] += 1
                        if in_scope:
                            result.scope_logs["self"] += 1
                        continue
                    result.logs["bare"] += 1
                    result.logs[f"bare_{source}"] += 1
                    result.bare_by_title[(key, source)] += 1
                    result.bare_by_source[source] += 1
                    result.entries_with_bare.add(entry_id)
                    if in_scope:
                        result.scope_logs["bare"] += 1
                        result.scope_logs[f"bare_{source}"] += 1
                        result.scope_entries_with_bare.add(entry_id)
                        result.scope_bare_by_title[key] += 1
                    if len(result.examples) < 40:
                        result.examples.append(
                            {
                                "entry": entry_id,
                                "source": source,
                                "title": entry["title"],
                                "span": text,
                                "emphasis": emphasis,
                                "in_scope": in_scope,
                                "targets": ids[:3],
                            }
                        )
            body_links += links_here
            if links_here:
                entries_with_link += 1

    for result in results.values():
        result.by_title = {
            title: {
                "carried": result.bare_by_title[(title, "carried")],
                "pf": result.bare_by_title[(title, "pf")],
            }
            for title in sorted({t for t, _ in result.bare_by_title})
        }

    return {
        "content": content,
        "content_version": version,
        "schema_version": manifest["schema_version"],
        "entry_count": manifest["entry_count"],
        "body_links": body_links,
        "entries_with_body_link": entries_with_link,
        "categories": {
            name: {
                "targets": result.targets,
                "linked": result.logs["linked"],
                "entries_with_link": len(result.entries_with_link),
                "bare": result.logs["bare"],
                "bare_pf": result.logs["bare_pf"],
                "bare_carried": result.logs["bare_carried"],
                "entries_with_bare": len(result.entries_with_bare),
                "section_labels": result.logs["labels"],
                "self_bare": result.logs["self"],
                "scope": {
                    "scope": "spell bodies only" if name == "spells" else "every entry body",
                    "linked": result.scope_logs["linked"],
                    "entries_with_link": len(result.scope_entries_with_link),
                    "bare": result.scope_logs["bare"],
                    "bare_pf": result.scope_logs["bare_pf"],
                    "bare_carried": result.scope_logs["bare_carried"],
                    "entries_with_bare": len(result.scope_entries_with_bare),
                    "self_bare": result.scope_logs["self"],
                    "top_bare": [
                        {"title": title, "count": count}
                        for title, count in result.scope_bare_by_title.most_common(20)
                    ],
                },
                "top_bare": [
                    {"title": title, **counts}
                    for title, counts in sorted(
                        result.by_title.items(),
                        key=lambda item: (-(item[1]["pf"] + item[1]["carried"]), item[0]),
                    )[:20]
                ],
                "examples": result.examples,
            }
            for name, result in results.items()
        },
    }


def spell_level_shape(entry: dict) -> dict:
    """Describe how a spell entry presents its class/level list."""
    for block in entry.get("body", []):
        if block.get("t") != "stat_block":
            continue
        rows = [row for row in block.get("rows", []) if row.get("label") == "Level"]
        if not rows:
            continue
        values = [text_of(row.get("value", [])) for row in rows]
        # Both the grouped build ("3: Sorcerer, Wizard" on separate lines) and the
        # old flat build ("Wizard 3, Sorcerer 3") use one row. The leading level
        # group is what distinguishes them.
        structured = len(rows) > 1 or any(
            re.match(r"^\d+: ", value.strip()) for value in values
        )
        return {
            "rows": len(rows),
            "values": values,
            "structured": structured,
        }
    return {"rows": 0, "values": [], "structured": False}


SPELL_SECTION_LABELS = {
    "Spell-Like Abilities",
    "Spells Known",
    "Prepared Spells",
    "Psychic Magic",
    "Spellcasting",
}

RAW_CREATURE_LABELS = re.compile(r"^(l\d+|Cl|Con|Pe|Sr):?$")


def creature_spell_shape(entry: dict) -> dict | None:
    """Describe a creature's spell list blocks, if it has one."""
    labels = []
    raw = 0
    level_groups = 0
    blocks = entry.get("body", [])
    for index, block in enumerate(blocks):
        if block.get("t") != "paragraph":
            continue
        content = block.get("content", [])
        first = content[0] if content else {}
        if first.get("t") != "strong":
            continue
        label = text_of(first.get("c", [])).rstrip(":").strip()
        if label not in SPELL_SECTION_LABELS:
            continue
        labels.append(label)
        # The level/frequency labels follow as sibling paragraphs in the
        # canonical render of `::mspell`; the values follow as lists.
        for follow in blocks[index + 1 :]:
            if follow.get("t") == "heading":
                break
            if follow.get("t") != "paragraph":
                continue
            inner = follow.get("content", [])
            if not inner or inner[0].get("t") != "strong":
                continue
            value = text_of(inner[0].get("c", [])).rstrip(":")
            if RAW_CREATURE_LABELS.match(value):
                raw += 1
            if re.match(r"^(l\d+|\d+(st|nd|rd|th)-level spells|Cantrips)$", value):
                level_groups += 1
        # Carried creatures use one stat row per section with Break separators.
    for block in blocks:
        if block.get("t") == "stat_block":
            for row in block.get("rows", []):
                label = row.get("label", "").rstrip(":").strip()
                if "spell" in label.casefold():
                    labels.append(label)
                    value = text_of(row.get("value", []))
                    if "\n" in value:
                        level_groups += 1
    if not labels:
        return None
    return {
        "labels": labels,
        "raw_labels": raw,
        "level_groups": level_groups,
        "structured": level_groups > 0 and raw == 0,
    }


def measure_spell_lists(content: str) -> dict:
    manifest = load_json(os.path.join(content, "manifest.json"))
    version = manifest["content_version"]
    base = os.path.join(content, version)
    catalog = {row["id"]: row for row in load_json(os.path.join(base, "catalog.json"))["entries"]}

    spells = collections.Counter()
    spell_classes = collections.Counter()
    spell_examples = collections.defaultdict(list)
    creatures = collections.Counter()
    creature_raw = collections.Counter()
    creature_examples = collections.defaultdict(list)

    for shard in sorted(glob.glob(os.path.join(base, "entries", "*.json"))):
        for entry in load_json(shard):
            source = "carried" if entry["id"].startswith("ref/carried/") else "pf"
            if entry["kind"] == "spell":
                shape = spell_level_shape(entry)
                if shape["rows"]:
                    spells[(source, shape["structured"])] += 1
                    parts = [p for value in shape["values"] for p in value.split(",")]
                    spell_classes[(source, len(parts))] += 1
                    if len(spell_examples[(source, shape["structured"])]) < 3:
                        spell_examples[(source, shape["structured"])].append(
                            {"entry": entry["id"], "title": entry["title"], "level": shape["values"]}
                        )
            if entry["kind"] != "creature":
                continue
            shape = creature_spell_shape(entry)
            if shape is None:
                continue
            creatures[(source, shape["structured"])] += 1
            creature_raw[(source, shape["raw_labels"] > 0)] += 1
            bucket = (source, shape["structured"], shape["raw_labels"] > 0)
            if len(creature_examples[bucket]) < 3:
                creature_examples[bucket].append(
                    {"entry": entry["id"], "title": entry["title"], **shape}
                )

    return {
        "spells": {
            "structured": {f"{s}": spells[(s, True)] for s in ("pf", "carried")},
            "flattened": {f"{s}": spells[(s, False)] for s in ("pf", "carried")},
            "class_count_histogram": {
                f"{s}": {
                    str(n): spell_classes[(s, n)]
                    for n in sorted({count for (source, count) in spell_classes if source == s})
                }
                for s in ("pf", "carried")
            },
            "examples": {
                f"{s}/{'structured' if ok else 'flattened'}": rows
                for (s, ok), rows in spell_examples.items()
            },
        },
        "creatures": {
            "structured": {s: creatures[(s, True)] for s in ("pf", "carried")},
            "flat_labels": {s: creatures[(s, False)] for s in ("pf", "carried")},
            "raw_labels": {s: creature_raw[(s, True)] for s in ("pf", "carried")},
            "examples": {
                f"{s}/{'structured' if ok else 'flattened'}/raw={raw}": rows
                for (s, ok, raw), rows in creature_examples.items()
            },
        },
    }


def spotlight(content: str) -> list[dict]:
    """Check the names that #20 uses as acceptance examples."""
    manifest = load_json(os.path.join(content, "manifest.json"))
    base = os.path.join(content, manifest["content_version"])
    wanted = {
        "ref/spell/cloudkill": ["fog cloud"],
        "ref/spell/protection-from-law": ["protection from evil"],
    }
    found = []
    for shard in sorted(glob.glob(os.path.join(base, "entries", "*.json"))):
        for entry in load_json(shard):
            names = wanted.get(entry["id"])
            if not names:
                continue
            for name in names:
                hits = {"link": 0, "em": 0, "strong": 0, "plain": 0}
                for item in walk_blocks(entry.get("body", [])):
                    if item[0] == "link" and normalize(item[2]) == normalize(name):
                        hits["link"] += 1
                    elif item[0] == "bare" and normalize(item[2]) == normalize(name):
                        if item[3]["block"] != "heading" and not item[3]["standalone"]:
                            hits["em" if item[1] == "em" else "strong"] += 1
                body_text = " ".join(
                    text_of(block) for block in entry.get("body", [])
                ).casefold()
                hits["plain"] = len(re.findall(re.escape(name.casefold()), body_text))
                hits["plain"] -= hits["em"] + hits["strong"] + hits["link"]
                found.append(
                    {
                        "entry": entry["id"],
                        "title": entry["title"],
                        "reference": name,
                        "carried": entry["id"].startswith("ref/carried/"),
                        **hits,
                    }
                )
    return found


def render(report: dict, lists: dict, spots: list[dict]) -> str:
    lines = []
    add = lines.append
    add(f"# Reference links: {report['content']} ({report['content_version']})")
    add("")
    add(f"{report['entry_count']} entries, schema {report['schema_version']}.")
    add("")
    add(
        f"{report['body_links']:,} body links in "
        f"{report['entries_with_body_link']:,} entries "
        f"({100 * report['entries_with_body_link'] / report['entry_count']:.1f}%)."
    )
    add("")
    add("| Category | Targets | Linked refs | Entries with a link | Bare matching spans | Pf | Carried | Section labels | Self |")
    add("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for name in CATEGORIES:
        row = report["categories"][name]
        add(
            f"| {name.replace('_', ' ')} | {row['targets']} | {row['linked']} | "
            f"{row['entries_with_link']} | {row['bare']} | {row['bare_pf']} | "
            f"{row['bare_carried']} | {row['section_labels']} | {row['self_bare']} |"
        )
    add("")
    add("Restricted to the scope each issue names:")
    add("")
    add("| Category | Scope | Linked refs | Entries with a link | Bare matching spans | Pf | Carried | Self |")
    add("|---|---|---:|---:|---:|---:|---:|---:|")
    for name in CATEGORIES:
        row = report["categories"][name]["scope"]
        add(
            f"| {name.replace('_', ' ')} | {row['scope']} | {row['linked']} | "
            f"{row['entries_with_link']} | {row['bare']} | {row['bare_pf']} | "
            f"{row['bare_carried']} | {row['self_bare']} |"
        )
    add("")
    for name in CATEGORIES:
        row = report["categories"][name]
        add(f"## {name.replace('_', ' ').title()}")
        add("")
        if row["scope"]["top_bare"]:
            add("Largest remaining bare spans in scope:")
            add("")
            for item in row["scope"]["top_bare"][:10]:
                add(f"- *{item['title']}*: {item['count']}")
        else:
            add("No bare span matches a target title in scope.")
        add("")
    add("## Acceptance names from #20")
    add("")
    for spot in spots:
        add(
            f"- `{spot['entry']}` names **{spot['reference']}**: "
            f"{spot['link']} link, {spot['em']} em, {spot['strong']} strong, "
            f"{spot['plain']} plain"
        )
    add("")
    add("## Spell-list structure (#42)")
    add("")
    add("| Shape | Pf | Carried |")
    add("|---|---:|---:|")
    add(f"| Spells with a structured level list | {lists['spells']['structured']['pf']} | {lists['spells']['structured']['carried']} |")
    add(f"| Spells with a flattened level list | {lists['spells']['flattened']['pf']} | {lists['spells']['flattened']['carried']} |")
    add(f"| Creatures with a class/level spell list | {lists['creatures']['structured']['pf']} | {lists['creatures']['structured']['carried']} |")
    add(f"| Creatures with frequency-only spell lists | {lists['creatures']['flat_labels']['pf']} | {lists['creatures']['flat_labels']['carried']} |")
    add(f"| Creatures with raw directive labels | {lists['creatures']['raw_labels']['pf']} | {lists['creatures']['raw_labels']['carried']} |")
    add("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--content", default="generated/content")
    parser.add_argument("--dataset", default=".cache/sources/pathfinder-data-1-e")
    parser.add_argument("--json", help="Write the full JSON report to this path")
    args = parser.parse_args()

    if not os.path.exists(os.path.join(args.content, "manifest.json")):
        print(f"{args.content} has no manifest; run cargo xtask content first", file=sys.stderr)
        return 1

    report = measure_links(args.content, args.dataset)
    lists = measure_spell_lists(args.content)
    spots = spotlight(args.content)
    print(render(report, lists, spots))
    if args.json:
        os.makedirs(os.path.dirname(args.json) or ".", exist_ok=True)
        with open(args.json, "w", encoding="utf-8") as handle:
            json.dump({"links": report, "spell_lists": lists, "spotlight": spots}, handle, indent=2)
            handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
