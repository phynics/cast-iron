#!/usr/bin/env python3
"""Generate the rule-derivation allowlist from the pinned PSRD and Pf Data corpora.

The canonical cutover promotes PSRD rule entries that Pf Data stores inside
another record. This script proposes the derivable subset:

  * a PSRD child rule whose title matches a Pf heading (``::mainheader``,
    ``::sh``, ``::gh``, ``::hN``, or Markdown ``#``/``##``...), or
  * a PSRD child rule whose title matches the first cell of a Pf definition
    table row (a two-column table whose second column is Description, Effect,
    Benefit, Text, or Rules) with rules-bearing text.

Structural stat-block labels (Description, Special Abilities, Statistics,
Offense, Defense, ...) are never treated as rules. Each selected rule is
assigned the best matching Pf host using publication, record kind, parent
title, and parent key. A rule whose canonical id would duplicate another
selected rule is omitted; the design requires a shared alias instead of a
numeric suffix.

Inputs are build artifacts, not committed research data:

    cargo xtask content --psrd
    cargo xtask data-report

Usage:

    python3 scripts/extract-derived-rule-allowlist.py \
        --psrd-content generated/psrd/v<version> \
        --inventory generated/migration/inventory.json \
        --out sources/pathfinder-data-1-e/derived-rules.json
"""

from __future__ import annotations

import argparse
import collections
import json
import re
from pathlib import Path

STRUCTURAL = {
    "description",
    "special abilities",
    "statistics",
    "offense",
    "defense",
    "tactics",
    "habitat",
    "construction",
    "ecology",
    "effect",
    "destruction",
    "traits",
    "creating",
    "variants",
    "origins",
    "ritual",
    "overcharge",
    "special attacks",
    "ability scores",
    "skills",
    "feats",
    "spells",
    "gear",
    "alignment",
    "prerequisites",
    "hit dice",
    "damage reduction",
    "class skills",
    "simple blasts",
    "composite blasts",
    "format",
    "location",
}

DEFINITION_HEADERS = {"description", "effect", "benefit", "text", "rules"}
HEAD_RE = re.compile(r"^::(mainheader|sh|gh|h[1-6]|h)\[(.*?)\]")
MD_HEAD_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
ROW_RE = re.compile(r"^::row\[(.*?)\]")


def normalize(value: str) -> str:
    """Compare titles without case, punctuation, or trailing (Ex)/(Su)/(Sp)."""
    value = re.sub(r"\s*\((Ex|Su|Sp)[^)]*\)\s*$", "", value, flags=re.I)
    value = value.lower().replace("\u2019", "'").replace("&", " and ")
    value = re.sub(r"[^a-z0-9]+", " ", value)
    return re.sub(r"\s+", " ", value).strip()


def slug(value: str) -> str:
    value = value.lower().replace("&", " and ")
    return re.sub(r"[^a-z0-9]+", "-", value).strip("-")


def record_kind(path: str) -> str:
    """Mirror `record_kind` in crates/content-compiler/src/pf_data.rs."""
    name = path.removeprefix("json/")
    for prefix, kind in [
        ("spells", "Spell"),
        ("feat", "Feat"),
        ("archetypes", "Archetype"),
        ("prestige_classes", "PrestigeClass"),
        ("classes", "Class"),
        ("npc_classes", "Class"),
        ("monsters", "Creature"),
        ("npcs", "Creature"),
        ("magic", "Item"),
        ("equipment", "Item"),
        ("race", "Race"),
        ("traits", "Trait"),
    ]:
        if name.startswith(prefix):
            return kind
    return "Rule"


def lines(record: dict) -> list[str]:
    description = record["raw"].get("description")
    return [line for line in description if isinstance(line, str)] if isinstance(description, list) else []


def headings(record: dict):
    for index, line in enumerate(lines(record)):
        text = line.strip()
        match = HEAD_RE.match(text)
        if match:
            yield index, match.group(2)
            continue
        match = MD_HEAD_RE.match(text)
        if match:
            yield index, match.group(2)
            continue
        match = ROW_RE.match(text)
        if match:
            yield index, match.group(1)


def definition_rows(record: dict):
    body = lines(record)
    index = 0
    while index < len(body):
        header_line = body[index].strip()
        if (
            header_line.startswith("|")
            and index + 1 < len(body)
            and set(body[index + 1].strip()) <= set("|-: ")
        ):
            header = [cell.strip() for cell in header_line.strip("|").split("|")]
            row_index = index + 2
            while row_index < len(body) and body[row_index].strip().startswith("|"):
                cells = [cell.strip() for cell in body[row_index].strip("|").split("|")]
                yield row_index, header, cells
                row_index += 1
            index = row_index
        else:
            index += 1


def load_rules(content: Path) -> dict[str, dict]:
    rules: dict[str, dict] = {}
    for shard in sorted(content.glob("entries/rule-*.json")):
        for entry in json.loads(shard.read_text()):
            rules[entry["id"]] = entry
    return rules


def load_records(inventory: dict) -> list[dict]:
    out_of_scope = {(row["path"], row["key"]) for row in inventory["out_of_scope"]}
    return [
        record
        for record in inventory["records"]
        if (record["path"], record["key"]) not in out_of_scope
        and record["shape"] in ("description", "copy", "compilation")
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--psrd-content", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    rules = load_rules(args.psrd_content)
    catalog = json.loads((args.psrd_content / "catalog.json").read_text())
    catalog_by_id = {entry["id"]: entry for entry in catalog["entries"]}
    inventory = json.loads(args.inventory.read_text())
    records = load_records(inventory)

    candidates: dict[str, list[dict]] = collections.defaultdict(list)
    for record in records:
        # Index-only records carry links, not rule prose, and have no
        # publication. They cannot be a derived record's parent.
        if not record["publications"]:
            continue
        host_title = normalize(record["name"])
        for index, title in headings(record):
            title_norm = normalize(title)
            if title_norm in STRUCTURAL:
                continue
            candidates[title_norm].append(
                {
                    "host": f"{record['path']}#{record['key']}",
                    "shape": "section",
                    "heading": title,
                    "line": index,
                    "publications": record["publications"],
                    "protocol": record["protocol"],
                    "kind": record_kind(record["path"]),
                    "host_title": host_title,
                    "path": record["path"],
                    "key": record["key"],
                }
            )
        for index, header, cells in definition_rows(record):
            if (
                len(header) != 2
                or header[1].lower() not in DEFINITION_HEADERS
                or len(cells) < 2
                or len(" ".join(cells[1:])) < 40
            ):
                continue
            title_norm = normalize(cells[0])
            if title_norm in STRUCTURAL:
                continue
            candidates[title_norm].append(
                {
                    "host": f"{record['path']}#{record['key']}",
                    "shape": "row",
                    "heading": cells[0],
                    "line": index,
                    "publications": record["publications"],
                    "protocol": record["protocol"],
                    "kind": record_kind(record["path"]),
                    "host_title": host_title,
                    "path": record["path"],
                    "key": record["key"],
                }
            )

    def source_slugs(entry: dict) -> set[str]:
        return set(entry.get("source_ids") or [])

    def publication_slugs(candidate: dict) -> set[str]:
        return {slug(value) for value in candidate["publications"]}

    def score(entry: dict, candidate: dict, parent_title: str, parent_key: str) -> int:
        value = 0
        if candidate["shape"] == "section" and normalize(candidate["heading"]) != candidate["host_title"]:
            value += 1000
        elif candidate["shape"] == "row":
            value += 500
        else:
            value += 100
        if source_slugs(entry) & publication_slugs(candidate):
            value += 100
        if candidate["kind"] == "Rule":
            value += 50
        if candidate["host_title"] == parent_title:
            value += 40
        if candidate["key"] == parent_key:
            value += 30
        if candidate["path"].removeprefix("json/").startswith(("rules_", "main", "umr")):
            value += 10
        return value

    selected: list[dict] = []
    used_ids: dict[str, str] = {}
    misses = collections.Counter()
    for rule_id in sorted(rules):
        entry = rules[rule_id]
        if not entry.get("parent"):
            continue
        title_norm = normalize(entry["title"])
        if title_norm in STRUCTURAL:
            continue
        parent = catalog_by_id.get(entry["id"].rsplit("/", 1)[0])
        parent_title = normalize(parent["title"]) if parent else ""
        parent_key = entry["id"].rsplit("/", 2)[-2] if entry["id"].count("/") >= 3 else ""
        options = candidates.get(title_norm, [])
        if not options:
            misses["no candidate"] += 1
            continue
        options.sort(
            key=lambda candidate: (
                -score(entry, candidate, parent_title, parent_key),
                candidate["path"],
                candidate["key"],
                candidate["shape"],
                candidate["heading"],
            )
        )
        for candidate in options:
            canonical_id = (
                f"ref/rule/{slug(candidate['protocol'] + '-' + candidate['key'])}"
                f"/{slug(entry['title'])}"
            )
            if canonical_id in used_ids:
                continue
            used_ids[canonical_id] = rule_id
            selected.append(
                {
                    "rule": rule_id,
                    "title": entry["title"],
                    "sources": sorted(source_slugs(entry)),
                    "host": candidate["host"],
                    "protocol": candidate["protocol"],
                    "shape": candidate["shape"],
                    "heading": candidate["heading"],
                    "id": canonical_id,
                }
            )
            break
        else:
            misses["id collision"] += 1

    selected.sort(key=lambda row: (row["rule"], row["host"], row["shape"], row["heading"]))
    output = {
        "format": "cast-iron-derived-rules",
        "version": 1,
        "purpose": "Allowlist of PSRD rule entries that Pf Data stores as nested sections or definition-table rows. Generated by scripts/extract-derived-rule-allowlist.py; review before changing the pin.",
        "psrd_commit": "76ea4784604145ba3dea40eb56f83bb904759467",
        "pf_commit": "08ea71f24375908d0190c822fd1fd080797667a9",
        "counts": {
            "sections": sum(row["shape"] == "section" for row in selected),
            "rows": sum(row["shape"] == "row" for row in selected),
            "total": len(selected),
        },
        "rules": selected,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(output, indent=1, ensure_ascii=False) + "\n")
    print(
        f"{output['counts']['sections']} sections, {output['counts']['rows']} rows, "
        f"{output['counts']['total']} derived rules; skipped {dict(misses)}"
    )


if __name__ == "__main__":
    main()
