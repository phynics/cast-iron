#!/usr/bin/env python3
"""Inventory the pinned Pf Data 1e JSON and PCGen Pathfinder source manifests."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

PF_DATA_SHA = "08ea71f24375908d0190c822fd1fd080797667a9"
PCGEN_SHA = "69ad2ab9145038f4b0056c161ffeebf84daa154a"

BOOKS = [
    "Pathfinder #28: The Infernal Syndrome",
    "Heroes of the Wild",
    "Pathfinder Unchained",
    "Occult Adventures",
    "Ultimate Intrigue",
    "Inner Sea Intrigue",
    "Horror Adventures",
    "Adventurer's Guide",
    "Book of the Damned",
    "Ultimate Wilderness",
    "Pathfinder #128: Songbird, Scion, Saboteur",
    "Planar Adventures",
    "Heroes of Golarion",
    "Pathfinder #144: Midwives to Death",
]

# PCGen uses some series-prefixed spellings in SOURCELONG/COPYRIGHT fields.
PCGEN_MATCH_TERMS = {
    "Pathfinder #28: The Infernal Syndrome": ["the infernal syndrome"],
    "Heroes of the Wild": ["heroes of the wild"],
    "Pathfinder Unchained": ["pathfinder unchained"],
    "Occult Adventures": ["occult adventures"],
    "Ultimate Intrigue": ["ultimate intrigue"],
    "Inner Sea Intrigue": ["inner sea intrigue"],
    "Horror Adventures": ["horror adventures"],
    "Adventurer's Guide": ["adventurer's guide", "adventurers guide"],
    # The 2017 core book title is distinct from older Book of the Damned
    # volumes, which are reported separately as related source manifests.
    "Book of the Damned": ["pathfinder roleplaying game book of the damned"],
    "Ultimate Wilderness": ["ultimate wilderness"],
    "Pathfinder #128: Songbird, Scion, Saboteur": ["songbird, scion, saboteur"],
    "Planar Adventures": ["planar adventures"],
    "Heroes of Golarion": ["heroes of golarion"],
    "Pathfinder #144: Midwives to Death": ["midwives to death"],
}

PCGEN_RELATED_TERMS = {
    "Book of the Damned": ["book of the damned"],
}


def git_head(root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.PIPE,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ValueError(f"not a readable git checkout: {root}: {exc}") from exc


def check_pin(root: Path, expected: str, label: str) -> str:
    actual = git_head(root)
    if actual != expected:
        raise ValueError(f"{label} checkout must be pinned at {expected}; found {actual}")
    return actual


def walk_objects(value: object):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from walk_objects(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk_objects(child)


def data_family(path: Path) -> str:
    # File naming convention groups numbered shards (feat11, spells15) and
    # preserves subfamilies (rules_opt_combat01, archetypes_alchemist2).
    return re.sub(r"\d+$", "", path.stem).rstrip("_")


def inventory_pf_data(root: Path) -> list[dict[str, object]]:
    data_dir = root / "json"
    if not data_dir.is_dir():
        raise ValueError(f"missing Pf Data JSON directory: {data_dir}")

    file_counts: dict[str, Counter[str]] = {book: Counter() for book in BOOKS}
    book_set = set(BOOKS)
    for path in sorted(data_dir.glob("*.json")):
        try:
            parsed = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise ValueError(f"cannot parse {path}: {exc}") from exc
        relative = path.relative_to(root).as_posix()
        for obj in walk_objects(parsed):
            sources = obj.get("sources")
            if "name" not in obj or not isinstance(sources, list):
                continue
            for book in book_set.intersection(source for source in sources if isinstance(source, str)):
                file_counts[book][relative] += 1

    result = []
    for book in BOOKS:
        files = file_counts[book]
        family_counts: Counter[str] = Counter()
        for path, count in files.items():
            family_counts[data_family(Path(path))] += count
        result.append({
            "source_tag": book,
            "named_object_occurrences": sum(files.values()),
            "json_files_count": len(files),
            "files": [
                {"path": path, "family": data_family(Path(path)), "occurrences": count}
                for path, count in sorted(files.items())
            ],
            "families": [
                {"family": family, "occurrences": count}
                for family, count in sorted(family_counts.items())
            ],
        })
    return result


def parse_pcc(path: Path) -> tuple[list[str], list[str]]:
    source_names: list[str] = []
    copyright_lines: list[str] = []
    lst_references: list[str] = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if line.startswith("SOURCELONG:"):
            source_names.append(line.partition(":")[2].strip())
        elif line.startswith("COPYRIGHT:"):
            copyright_lines.append(line.partition(":")[2].strip())
        elif not line.startswith("#"):
            match = re.match(r"^[A-Z_]+:([^|]+\.lst)(?:\|.*)?$", line, flags=re.IGNORECASE)
            if match:
                lst_references.append(match.group(1).strip())
    return source_names + copyright_lines, lst_references


def inventory_pcgen(root: Path) -> list[dict[str, object]]:
    data_root = root / "data" / "pathfinder" / "paizo"
    if not data_root.is_dir():
        raise ValueError(f"missing PCGen Paizo Pathfinder data directory: {data_root}")
    manifests = sorted(data_root.rglob("*.pcc"))
    result = []
    for book in BOOKS:
        terms = [term.casefold() for term in PCGEN_MATCH_TERMS[book]]
        matches = []
        related = []
        for path in manifests:
            searchable, refs = parse_pcc(path)
            matched_fields = [
                value
                for value in searchable
                if any(term in value.casefold() for term in terms)
            ]
            if matched_fields:
                matches.append(
                    {
                        "manifest": path.relative_to(root).as_posix(),
                        "source_or_copyright_matches": sorted(set(matched_fields)),
                        "lst_reference_count": len(refs),
                        "lst_references": refs,
                    }
                )
            elif book in PCGEN_RELATED_TERMS:
                related_fields = [
                    value
                    for value in searchable
                    if any(term.casefold() in value.casefold() for term in PCGEN_RELATED_TERMS[book])
                ]
                if related_fields:
                    related.append(
                        {
                            "manifest": path.relative_to(root).as_posix(),
                            "source_or_copyright_matches": sorted(set(related_fields)),
                            "lst_reference_count": len(refs),
                            "lst_references": refs,
                        }
                    )
        result.append(
            {
                "source_tag": book,
                "status": "matched" if matches else "no_match",
                "manifest_count": len(matches),
                "manifests": matches,
                "related_manifest_count": len(related),
                "related_manifests": related,
            }
        )
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pf_data_dir", type=Path, help="Pf Data 1e checkout")
    parser.add_argument("pcgen_dir", type=Path, help="PCGen checkout")
    parser.add_argument("--output", type=Path, help="write JSON here (default: stdout)")
    args = parser.parse_args()

    try:
        pf_data_sha = check_pin(args.pf_data_dir, PF_DATA_SHA, "Pf Data 1e")
        pcgen_sha = check_pin(args.pcgen_dir, PCGEN_SHA, "PCGen")
        report = {
            "schema_version": 1,
            "pins": {"pf_data_1e": pf_data_sha, "pcgen": pcgen_sha},
            "count_method": (
                "A named-object occurrence is a JSON dictionary with a name key and an exact source title in its sources array. "
                "Occurrences are counted where stored, including repeated entities in separate files. File families are filename stems with trailing digits removed. "
                "PCGen reference counts are .lst directive lines in matching .pcc manifests; no-match means no Paizo Pathfinder manifest had a matching SOURCELONG or COPYRIGHT value. Related-only manifests are returned separately where they name an older similarly titled publication."
            ),
            "pf_data_1e": inventory_pf_data(args.pf_data_dir),
            "pcgen": inventory_pcgen(args.pcgen_dir),
        }
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    output = json.dumps(report, ensure_ascii=False, indent=2, sort_keys=False) + "\n"
    if args.output:
        args.output.write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
