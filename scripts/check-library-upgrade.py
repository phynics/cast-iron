#!/usr/bin/env python3
"""Check a new content library against a previous one.

Usage: python3 scripts/check-library-upgrade.py /path/to/old/content [new/content]

Without `aliases.json` the check is the original same-namespace check: a previously
published ID or exact title must still be present, a class must be in the right
category, and every schema-3 index (relations.json, lists.json, books.json) must
name an existing entry. Kind changes are reported, not failures.

When the new library ships `aliases.json`, the cutover rules apply. Each old ID is
resolved in one step, exactly like `Library::resolve`: the new catalog first, then
one alias entry, then the retirement list. The check fails on a missing or
unresolvable old ID, a missing alias target, a chain, an old ID in both lists, an
unexpected title change, a wrong class category, or a dangling reference. Title
equivalence uses the alias fit gate's comparison (`title_fit`), so documented
inversions and parentheticals pass. Title changes on shared and derived rows are
reported rather than failed because those rows are documented merges, not a
1:1 promise. Books may place one entry in several publications, so a repeated
placement is reported, not failed.
"""
from pathlib import Path
from collections import Counter
import json
import re
import sys

# The alias fit gate's measure and package tokens (`crates/content-compiler/src/alias.rs`).
ITEM_MEASURE_WORDS = frozenset(
    """
    ft foot feet in inch inches yd yard yards sq lb lbs pound pounds oz ounce ounces
    gallon gallons quart pint cup mug jar jug gourd bottle bottles wineskin sheet
    sheets bar dose doses handful handfuls vial vials flask keg barrel box case per
    day days hour hours minute minutes second seconds round rounds mile miles person
    persons block spool coil bundle set pair pack pouch sack bag cartridge bullet
    pellet charge use application serving
    """.split()
)


PUBLISHED_IDS = Path(__file__).resolve().parents[1] / "sources/canonical/legacy-ids.json"


def published_ids(path=PUBLISHED_IDS):
    """The frozen ids the last PSRD release published, or None without the snapshot.

    Only these can be held in a user's bookmarks or recents. A fresh PSRD build can
    contain ids that never shipped (for example records an earlier content policy
    omitted), and those need no alias.
    """
    if not path.exists():
        return None
    return {entry["id"] for entry in json.loads(path.read_text())["entries"]}


def library(root):
    """Return the manifest, catalog entries by id, and the version directory."""
    root = Path(root)
    manifest = json.loads((root / "manifest.json").read_text())
    directory = root / manifest["content_version"]
    entries = {
        entry["id"]: entry
        for shard in sorted((directory / "entries").glob("*.json"))
        for entry in json.loads(shard.read_text())
    }
    return manifest, entries, directory


def normalize_title(title):
    """The alias gate's comparison form: lowercase alphanumeric words only."""
    out = []
    pending_space = False
    for char in title:
        if char.isalnum():
            if pending_space and out:
                out.append(" ")
            out.extend(char.lower())
            pending_space = False
        else:
            pending_space = True
    return "".join(out)


def split_parenthetical(title):
    """Split one trailing parenthetical; nested or unterminated forms are left alone."""
    trimmed = title.rstrip()
    if not trimmed.endswith(")"):
        return title, None
    start = trimmed.rfind("(")
    if start <= 0:
        return title, None
    inner = trimmed[start + 1 : -1]
    if "(" in inner or ")" in inner:
        return title, None
    return trimmed[:start].rstrip(), inner


def invert_title(title):
    """The comma-inversion spelling, with the PSRD dragon-category word removed."""
    parts = [part.strip() for part in title.split(",") if part.strip()]
    if len(parts) < 2:
        return None
    if len(parts) >= 3 and parts[1].lower() in {"primal", "imperial"}:
        parts.pop(1)
    return " ".join(reversed(parts))


def item_parenthetical_is_measure(text):
    saw_word = False
    for token in re.split(r"[^0-9A-Za-z]+", text):
        if not token:
            continue
        token = token.lower()
        if token.isascii() and token.isdigit():
            continue
        if re.fullmatch(r"\d+(st|nd|rd|th)", token):
            return False
        if token in {"i", "ii", "iii", "iv", "v"}:
            return False
        if token not in ITEM_MEASURE_WORDS:
            return False
        saw_word = True
    return saw_word


def title_parenthetical_allowed(kind, title):
    _, parenthetical = split_parenthetical(title)
    if parenthetical is None:
        return False
    if kind in {"creature", "trait", "feat", "affliction"}:
        return True
    if kind == "item":
        normalized = normalize_title(parenthetical)
        return normalized in {"weapon", "armor"} or item_parenthetical_is_measure(
            parenthetical
        )
    return False


def title_fit(legacy_title, record_title, kind):
    """The alias fit gate's title comparison, returning the fit kind or None."""
    legacy_normalized = normalize_title(legacy_title)
    record_normalized = normalize_title(record_title)
    if legacy_normalized == record_normalized:
        return "exact"
    legacy_inverted = invert_title(legacy_title)
    record_inverted = invert_title(record_title)
    if (legacy_inverted is not None and normalize_title(legacy_inverted) == record_normalized) or (
        record_inverted is not None and legacy_normalized == normalize_title(record_inverted)
    ):
        return "inversion"
    if not title_parenthetical_allowed(kind, legacy_title) and not title_parenthetical_allowed(
        kind, record_title
    ):
        return None
    legacy_base, legacy_paren = split_parenthetical(legacy_title)
    record_base, record_paren = split_parenthetical(record_title)
    if (
        legacy_paren is not None
        and record_paren is not None
        and normalize_title(legacy_paren) != normalize_title(record_paren)
    ):
        return None
    legacy_base_normalized = normalize_title(legacy_base)
    record_base_normalized = normalize_title(record_base)
    if legacy_base_normalized == record_base_normalized:
        return "parenthetical"
    legacy_base_inverted = invert_title(legacy_base)
    record_base_inverted = invert_title(record_base)
    if (
        legacy_base_inverted is not None
        and normalize_title(legacy_base_inverted) == record_base_normalized
    ) or (
        record_base_inverted is not None
        and legacy_base_normalized == normalize_title(record_base_inverted)
    ):
        return "inversion_parenthetical"
    return None


def load_alias_map(directory):
    """Load and validate `aliases.json`, or return None when the library has none."""
    path = directory / "aliases.json"
    if not path.exists():
        return None
    data = json.loads(path.read_text())
    problems = []
    if data.get("format") != 1:
        problems.append(f"aliases.json: unsupported format {data.get('format')!r}")
    entries = data.get("entries", [])
    retired = data.get("retired", [])
    for name, rows in (("entries", entries), ("retired", retired)):
        olds = [row.get("old") for row in rows]
        if olds != sorted(olds):
            problems.append(f"aliases.json: {name} are not sorted by old id")
        if len(set(olds)) != len(olds):
            problems.append(f"aliases.json: {name} repeat an old id")
    entry_olds = {row.get("old") for row in entries}
    retired_olds = {row.get("old") for row in retired}
    for old in sorted(entry_olds & retired_olds):
        problems.append(f"aliases.json: {old!r} appears in both entries and retired")
    olds = entry_olds | retired_olds
    for row in entries:
        if row.get("target") in olds:
            problems.append(
                f"aliases.json: {row.get('old')!r} targets {row.get('target')!r}, "
                "which is also an old id; chains are forbidden"
            )
    return {
        "entries": {row["old"]: row for row in entries},
        "retired": {row["old"]: row for row in retired},
        "problems": problems,
    }


def class_category_errors(entries):
    wrong = []
    for key, entry in entries.items():
        if entry["kind"] not in {"class", "prestige_class"}:
            continue
        expected = (
            "prestige_class" if entry["fields"].get("class_type") == "prestige" else "class"
        )
        if entry["kind"] != expected:
            wrong.append(key)
    return wrong


def unplaced_entry_rows(coverage):
    rows = ((coverage or {}).get("relations") or {}).get("book_order") or {}
    return rows.get("unplaced", [])


def unplaced_entries(coverage):
    return {row["entry"] for row in unplaced_entry_rows(coverage)}


def dangling_index_references(new, directory):
    """Check relations, spell lists, books, and the optional backlinks against the catalog."""
    dangling = []
    placed = set()
    duplicate_placements = []

    def load(name):
        path = directory / name
        return json.loads(path.read_text()) if path.exists() else None

    relations, lists, books, coverage = (
        load(name) for name in ("relations.json", "lists.json", "books.json", "coverage.json")
    )
    for source, _kind, target in (relations or {}).get("edges", []):
        for end in (source, target):
            if end not in new:
                dangling.append(f"relations.json: {end}")
    for spell, _title in (lists or {}).get("spells", []):
        if spell not in new:
            dangling.append(f"lists.json: {spell}")
    for index, cls in enumerate((lists or {}).get("classes", [])):
        class_id = cls.get("class")
        if class_id is not None and class_id not in new:
            dangling.append(f"lists.json: class {index} names {class_id}")
    for tree in (books or {}).get("books", []):
        for chapter in tree["chapters"]:
            walk_book_node(chapter, new, placed, dangling, duplicate_placements)
    dangling.extend(backlink_reference_errors(new, directory))
    return dangling, placed, duplicate_placements, coverage


def backlink_reference_errors(new, directory):
    """Check the optional reverse body-link index against the new catalog.

    An older library has no `backlinks.json`; that is valid and returns no errors. When the
    descriptor is present, every target and source must exist, the rows must be sorted, no
    self-link or typed-relation duplicate may remain, and the descriptor counts must match.
    """
    dangling = []
    descriptor_path = directory / "backlinks.json"
    if not descriptor_path.exists():
        return dangling
    descriptor = json.loads(descriptor_path.read_text())
    shards = descriptor.get("shards")
    if not isinstance(shards, int) or not 1 <= shards <= 100:
        return ["backlinks.json: invalid shard count"]
    links = 0
    targets = 0
    seen_targets = set()
    for shard in range(shards):
        path = directory / "backlinks" / f"{shard:02}.json"
        if not path.exists():
            dangling.append(f"backlinks/{shard:02}.json: missing")
            continue
        rows = json.loads(path.read_text())
        previous_target = None
        for target, sources in rows:
            if previous_target is not None and target <= previous_target:
                dangling.append(f"backlinks/{shard:02}.json: targets are not sorted")
            previous_target = target
            if target in seen_targets:
                dangling.append(f"backlinks: {target} appears in more than one shard")
            seen_targets.add(target)
            if target not in new:
                dangling.append(f"backlinks: target {target}")
                continue
            targets += 1
            previous_source = None
            for source_row in sources:
                source = source_row[0]
                if previous_source is not None and source <= previous_source:
                    dangling.append(f"backlinks: {target} sources are not sorted")
                previous_source = source
                if source not in new:
                    dangling.append(f"backlinks: source {source}")
                    continue
                if source == target:
                    dangling.append(f"backlinks: self-link {source}")
                source_relations = {
                    row.get("target") for row in (new[source].get("relations") or [])
                }
                if target in source_relations:
                    dangling.append(f"backlinks: {source} already relates to {target}")
                links += 1
    if links != descriptor.get("links"):
        dangling.append(f"backlinks.json: links {descriptor.get('links')} != {links}")
    if targets != descriptor.get("targets"):
        dangling.append(f"backlinks.json: targets {descriptor.get('targets')} != {targets}")
    return dangling


def walk_book_node(node, new, placed, dangling, duplicate_placements):
    if node.get("entry") and not node.get("anchor"):
        if node["entry"] not in new:
            dangling.append(f"books.json: {node['entry']}")
        if node["entry"] in placed:
            duplicate_placements.append(node["entry"])
        placed.add(node["entry"])
    for child in node.get("children", []):
        walk_book_node(child, new, placed, dangling, duplicate_placements)


def check(old_root, new_root, published=None):
    """Compare two libraries, returning the report and a list of failure reasons.

    `published` restricts the cutover check to ids that were ever published; main()
    passes the frozen legacy snapshot.
    """
    old_manifest, old, _old_directory = library(old_root)
    new_manifest, new, new_directory = library(new_root)
    wrong_categories = class_category_errors(new)
    dangling, placed, duplicate_placements, coverage = dangling_index_references(
        new, new_directory
    )
    relations_path = new_directory / "relations.json"
    relation_edges = (
        sum(json.loads(relations_path.read_text()).get("counts", {}).values())
        if relations_path.exists()
        else 0
    )

    aliases = load_alias_map(new_directory)
    if aliases is None:
        return legacy_report(
            old_manifest,
            new_manifest,
            old,
            new,
            wrong_categories,
            dangling,
            placed,
            coverage,
            relation_edges,
        )
    unpublished = []
    if published is not None:
        unpublished = sorted(set(old) - published)
        old = {key: value for key, value in old.items() if key in published}
    report, failures = cutover_report(
        old_manifest,
        new_manifest,
        old,
        new,
        aliases,
        wrong_categories,
        dangling,
        placed,
        duplicate_placements,
        coverage,
        relation_edges,
    )
    # Reported, not failed: never-published ids cannot be in anyone's saved state.
    report["unpublished_old_ids"] = unpublished
    return report, failures


def legacy_report(
    old_manifest,
    new_manifest,
    old,
    new,
    wrong_categories,
    dangling,
    placed,
    coverage,
    relation_edges,
):
    """The original same-namespace check. No aliases.json means no renumbering."""
    missing = sorted(set(old) - set(new))
    changed_titles = sorted(
        key for key in old.keys() & new.keys() if old[key]["title"] != new[key]["title"]
    )
    changed_kinds = sorted(
        f'{key}: {old[key]["kind"]} -> {new[key]["kind"]}'
        for key in old.keys() & new.keys()
        if old[key]["kind"] != new[key]["kind"]
    )
    if new_manifest["schema_version"] >= 3:
        unplaced = unplaced_entries(coverage)
        for key in new:
            if (key in placed) == (key in unplaced):
                dangling.append(f"books.json: {key} is neither placed nor reported, or both")
    report = {
        "old_schema": old_manifest["schema_version"],
        "new_schema": new_manifest["schema_version"],
        "old_entries": len(old),
        "new_entries": len(new),
        "missing_ids": missing,
        "changed_titles": changed_titles,
        "incorrect_class_categories": wrong_categories,
        "changed_kinds_count": len(changed_kinds),
        "changed_kinds_first": changed_kinds[:10],
        "dangling_references": dangling,
        "relation_edges": relation_edges,
        "book_placed": len(placed) if new_manifest["schema_version"] >= 3 else None,
    }
    failures = []
    if missing:
        failures.append(f"{len(missing)} missing id(s)")
    if changed_titles:
        failures.append(f"{len(changed_titles)} changed title(s)")
    if wrong_categories:
        failures.append(f"{len(wrong_categories)} incorrect class category row(s)")
    if dangling:
        failures.append(f"{len(dangling)} dangling reference(s)")
    return report, failures


def cutover_report(
    old_manifest,
    new_manifest,
    old,
    new,
    aliases,
    wrong_categories,
    dangling,
    placed,
    duplicate_placements,
    coverage,
    relation_edges,
):
    """One-step alias resolution, title fit, retirement accounting, and books."""
    entry_rows = aliases["entries"]
    retired_rows = aliases["retired"]
    alias_problems = list(aliases["problems"])
    for old_id, row in sorted(entry_rows.items()):
        if row.get("target") not in new:
            alias_problems.append(
                f"aliases.json: {old_id!r} targets {row.get('target')!r}, "
                "which is not in the new catalog"
            )

    missing = []
    unresolvable = []
    retired_by_reason = Counter()
    bases = Counter()
    changed_titles = []
    reported_title_changes = []
    changed_kinds = []

    for old_id in sorted(old):
        old_entry = old[old_id]
        if old_id in new:
            target = old_id
            basis = "present"
        elif old_id in entry_rows:
            row = entry_rows[old_id]
            target = row.get("target")
            basis = row.get("basis", "unknown")
            if target not in new:
                unresolvable.append({"old": old_id, "target": target})
                continue
        elif old_id in retired_rows:
            retired_by_reason[retired_rows[old_id].get("reason", "unknown")] += 1
            continue
        else:
            missing.append(old_id)
            continue

        bases[basis] += 1
        new_entry = new[target]
        if old_entry["kind"] != new_entry["kind"]:
            changed_kinds.append(
                f'{old_id}: {old_entry["kind"]} -> {new_entry["kind"]} ({target})'
            )
        if title_fit(old_entry["title"], new_entry["title"], old_entry["kind"]) is None:
            detail = {
                "old": old_id,
                "old_title": old_entry["title"],
                "target": target,
                "new_title": new_entry["title"],
                "basis": basis,
            }
            if basis in {"shared", "derived"}:
                reported_title_changes.append(detail)
            else:
                changed_titles.append(detail)

    if new_manifest["schema_version"] >= 3:
        unplaced_rows = unplaced_entry_rows(coverage)
        unplaced = {row["entry"] for row in unplaced_rows}
        for key in new:
            if key not in placed and key not in unplaced:
                dangling.append(f"books.json: {key} is neither placed nor reported")
        for key in sorted(unplaced):
            if key not in new:
                dangling.append(f"books.json: {key} is unplaced but has no entry")
        if len(unplaced) != len(unplaced_rows):
            dangling.append("coverage.json: book_order.unplaced repeats an entry")

    report = {
        "old_schema": old_manifest["schema_version"],
        "new_schema": new_manifest["schema_version"],
        "old_entries": len(old),
        "new_entries": len(new),
        "alias_map": True,
        "missing_ids": missing,
        "unresolvable_ids": unresolvable,
        "alias_problems": sorted(alias_problems),
        "changed_titles": changed_titles,
        "reported_title_changes": reported_title_changes,
        "incorrect_class_categories": wrong_categories,
        "alias_bases": dict(sorted(bases.items())),
        "retired_by_reason": dict(sorted(retired_by_reason.items())),
        "changed_kinds_count": len(changed_kinds),
        "changed_kinds_first": changed_kinds[:10],
        "dangling_references": dangling,
        "relation_edges": relation_edges,
        "book_placed": len(placed) if new_manifest["schema_version"] >= 3 else None,
        "reported_duplicate_book_placements": len(duplicate_placements),
    }
    failures = []
    if missing:
        failures.append(f"{len(missing)} missing id(s)")
    if unresolvable:
        failures.append(f"{len(unresolvable)} unresolvable id(s)")
    if alias_problems:
        failures.append(f"{len(alias_problems)} alias map problem(s)")
    if changed_titles:
        failures.append(f"{len(changed_titles)} unexpected title change(s)")
    if wrong_categories:
        failures.append(f"{len(wrong_categories)} incorrect class category row(s)")
    if dangling:
        failures.append(f"{len(dangling)} dangling reference(s)")
    return report, failures


def main(argv):
    if len(argv) not in (1, 2):
        print(
            "usage: check-library-upgrade.py /path/to/old/content [new/content]",
            file=sys.stderr,
        )
        return 2
    old_root = argv[0]
    new_root = argv[1] if len(argv) > 1 else "dist/content"
    report, failures = check(old_root, new_root, published_ids())
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
