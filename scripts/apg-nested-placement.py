#!/usr/bin/env python3
"""Place the Advanced Player's Guide tree by printed chapter and section.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id or an entry file. Run it against a corpus checkout, then run
``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/apg-nested-placement.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

BOOK = "advanced-players-guide"
BOOKS_FILE = "books.toml"

# ---------------------------------------------------------------------------
# TOML writer for the BookIndex shape


def _string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _array(values: list[str]) -> str:
    return "[" + ", ".join(_string(value) for value in values) + "]"


def _write_node(node: dict, path: str, out: list[str]) -> None:
    out.append(f"[[{path}]]")
    out.append(f"title = {_string(node['title'])}")
    if node.get("entry") is not None:
        out.append(f"entry = {_string(node['entry'])}")
    if node.get("anchor") is not None:
        out.append(f"anchor = {_string(node['anchor'])}")
    if node.get("review") is not None:
        out.append(f"review = {_string(node['review'])}")
    out.append("")
    for child in node.get("children", []):
        _write_node(child, path + ".children", out)


def dump_books(books: dict) -> str:
    out: list[str] = []
    for book in books["books"]:
        out.append("[[books]]")
        out.append(f"source = {_string(book['source'])}")
        out.append(f"title = {_string(book['title'])}")
        if book.get("basis") is not None:
            out.append(f"basis = {_string(book['basis'])}")
        if book.get("review") is not None:
            out.append(f"review = {_array(book['review'])}")
        out.append("")
        for chapter in book.get("chapters", []):
            _write_node(chapter, "books.chapters", out)
    return "\n".join(out).rstrip("\n") + "\n"


# ---------------------------------------------------------------------------
# Tree helpers


def find_book(books: dict) -> dict:
    for book in books["books"]:
        if book["source"] == BOOK:
            return book
    raise SystemExit(f"unknown book source: {BOOK}")


def find_node(book: dict, entry_id: str) -> dict | None:
    def walk(node: dict) -> dict | None:
        if node.get("entry") == entry_id:
            return node
        for child in node.get("children", []):
            found = walk(child)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter)
        if found is not None:
            return found
    return None


def index_parents(book: dict) -> dict[str, dict]:
    parents: dict[str, dict] = {}

    def walk(node: dict) -> None:
        for child in node.get("children", []):
            if child.get("entry"):
                parents[child["entry"]] = node
            walk(child)

    for chapter in book.get("chapters", []):
        walk(chapter)
    return parents


def path_of(book: dict, entry_id: str) -> str:
    def walk(node: dict, trail: list[str]) -> list[str] | None:
        here = trail + [node["title"]]
        if node.get("entry") == entry_id:
            return here
        for child in node.get("children", []):
            found = walk(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter, [])
        if found is not None:
            return " > ".join(found)
    return "(missing)"


def remove_node(book: dict, entry_id: str, parents: dict[str, dict]) -> dict:
    parent = parents[entry_id]
    node = next(child for child in parent["children"] if child.get("entry") == entry_id)
    parent["children"].remove(node)
    return node


def resolve_path(book: dict, path: list[str]) -> dict:
    nodes = book["chapters"]
    parent = None
    for title in path:
        node = next((n for n in nodes if n["title"] == title), None)
        if node is None:
            node = {"title": title, "children": []}
            nodes.append(node)
        parent = node
        nodes = node.setdefault("children", [])
    if parent is None:
        raise SystemExit("empty target path")
    return parent


def add_child(book: dict, entry_id: str, target: list[str], parents: dict[str, dict]) -> dict:
    node = remove_node(book, entry_id, parents)
    parent = resolve_path(book, target)
    parent.setdefault("children", []).append(node)
    parents[entry_id] = parent
    return node


def reorder(parent: dict, order: list[str]) -> None:
    by_title = {child["title"]: child for child in parent.get("children", [])}
    parent["children"] = [by_title[title] for title in order if title in by_title] + [
        child for child in parent.get("children", []) if child["title"] not in order
    ]


def sort_children(parent: dict, key, keep_first: list[str] | None = None) -> None:
    keep = list(keep_first or [])
    head = [child for child in parent.get("children", []) if child["title"] in keep]
    rest = [child for child in parent.get("children", []) if child["title"] not in keep]
    head.sort(key=lambda child: keep.index(child["title"]))
    rest.sort(key=key)
    parent["children"] = head + rest


# ---------------------------------------------------------------------------
# Page evidence


APG_PAGE = re.compile(r"Advanced Player's Guide(?: pg\.?| p\.) ?(\d+)")


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = APG_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


def page_key(corpus: Path, node: dict):
    page = page_of(corpus, node.get("entry"))
    return (page if page is not None else 10**9, title_key(node))


def title_key(node: dict):
    return tuple(re.findall(r"[a-z0-9]+", node["title"].lower()))


# ---------------------------------------------------------------------------
# The transform


# entry id -> target path.  Group nodes are created when missing.
TARGETS: dict[str, list[str]] = {
    # Races
    "ref/rule/favored-class": ["Races"],
    # Classes: base classes
    "ref/ability/poison-use": ["Classes", "Base Classes", "Alchemist"],
    "ref/ability/orders": ["Classes", "Base Classes", "Cavalier", "Cavalier Orders"],
    "ref/ability/judgments": ["Classes", "Base Classes", "Inquisitor", "Judgments"],
    "ref/ability/mysteries": ["Classes", "Base Classes", "Oracle", "Mysteries"],
    "ref/sidekick/eidolon": ["Classes", "Base Classes", "Summoner", "Eidolons"],
    # Classes: core classes
    "ref/rule/archetypes": ["Classes", "Core Classes"],
    "ref/ability/domains": ["Classes", "Core Classes", "Cleric", "Domains"],
    "ref/ability/arcane-schools": ["Classes", "Core Classes", "Wizard", "Arcane Schools"],
    # Feats
    "ref/rule/feats-multiperson": ["Feats", "Feats", "Types of Feats"],
    # New Rules
    "ref/rule/dirty-trick": ["New Rules", "Combat Maneuvers"],
    "ref/rule/drag": ["New Rules", "Combat Maneuvers"],
    "ref/rule/reposition": ["New Rules", "Combat Maneuvers"],
    "ref/rule/steal": ["New Rules", "Combat Maneuvers"],
    "ref/rule/awarding-hero-points": ["New Rules", "Hero Points"],
    "ref/rule/using-hero-points": ["New Rules", "Hero Points"],
    "ref/rule/antiheroes": ["New Rules", "Hero Points"],
    "ref/rule/hero-points-for-gms": ["New Rules", "Hero Points"],
    "ref/rule/gaining-traits": ["New Rules", "Traits"],
    "ref/rule/restrictions-on-trait-selection": ["New Rules", "Traits"],
    "ref/rule/types-of-traits": ["New Rules", "Traits"],
    "ref/spell/malediction-hero-points": ["New Rules", "Hero Points", "Hero Point Spells"],
    "ref/spell/heroic-fortune-mass": ["New Rules", "Hero Points", "Hero Point Spells"],
    "ref/trait/calistrian-courtesan": ["New Rules", "Traits", "Religion Traits"],
    "ref/trait/favored-son-or-daughter": ["New Rules", "Traits", "Campaign Traits"],
    "ref/trait/mathematical-prodigy": ["New Rules", "Traits", "Basic Traits", "Magic Traits"],
    # Classes: druid animal shamans
    "ref/arc-druid/bear-shaman": ["Classes", "Core Classes", "Druid", "Druid Archetypes", "Animal Shamans"],
    "ref/arc-druid/eagle-shaman": ["Classes", "Core Classes", "Druid", "Druid Archetypes", "Animal Shamans"],
    "ref/arc-druid/lion-shaman": ["Classes", "Core Classes", "Druid", "Druid Archetypes", "Animal Shamans"],
    "ref/arc-druid/serpent-shaman": ["Classes", "Core Classes", "Druid", "Druid Archetypes", "Animal Shamans"],
    "ref/arc-druid/wolf-shaman": ["Classes", "Core Classes", "Druid", "Druid Archetypes", "Animal Shamans"],
}

# Class-option groups from the entry front matter.
GROUP_TARGETS: dict[str, list[str]] = {
    "alchemist-discovery": ["Classes", "Base Classes", "Alchemist", "Discoveries"],
    "arcane-school": ["Classes", "Core Classes", "Wizard", "Arcane Schools"],
    "cleric-domain": ["Classes", "Core Classes", "Cleric", "Domains"],
    "oracle-curse": ["Classes", "Base Classes", "Oracle", "Curses"],
    "oracle-mystery": ["Classes", "Base Classes", "Oracle", "Mysteries"],
    "order": ["Classes", "Base Classes", "Cavalier", "Cavalier Orders"],
    "rage-power": ["Classes", "Core Classes", "Barbarian", "Rage Powers"],
    "rogue-talent": ["Classes", "Core Classes", "Rogue", "Rogue Talents"],
    "sorcerer-bloodline": ["Classes", "Core Classes", "Sorcerer", "Bloodlines"],
    "summoner-evolution": ["Classes", "Base Classes", "Summoner", "Evolutions"],
    "witch-hex": ["Classes", "Base Classes", "Witch", "Hexes"],
    "witch-patron": ["Classes", "Base Classes", "Witch", "Patron Spells"],
}

def front_matter(corpus: Path, entry_id: str) -> dict:
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    text = path.read_text(encoding="utf-8")
    return tomllib.loads(text.split("+++", 2)[1])


def kind_of(corpus: Path, entry_id: str) -> str:
    return front_matter(corpus, entry_id)["kind"]


def group_of(corpus: Path, entry_id: str) -> str | None:
    return front_matter(corpus, entry_id).get("fields", {}).get("group")


def target_for(corpus: Path, entry_id: str) -> list[str] | None:
    if entry_id in TARGETS:
        return TARGETS[entry_id]
    kind = kind_of(corpus, entry_id)
    if kind == "class_option":
        group = group_of(corpus, entry_id)
        if group in GROUP_TARGETS:
            return GROUP_TARGETS[group]
        raise SystemExit(f"{entry_id}: unknown class-option group {group!r}")
    if kind == "spell":
        return ["Spells"]
    if kind == "trait":
        raise SystemExit(f"{entry_id}: trait without an explicit target")
    return None


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, str]]:
    before = {}
    def walk_paths(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            before[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_paths(child, here)
    for chapter in book["chapters"]:
        walk_paths(chapter, [])

    parents = index_parents(book)
    new_entries = next(
        chapter for chapter in book["chapters"] if chapter["title"] == "New entries (page order)"
    )
    pool = list(new_entries.get("children", []))

    # 1. Move explicit targets and class-option groups out of the pool.
    for node in pool:
        entry_id = node.get("entry")
        if not entry_id:
            continue
        target = target_for(corpus, entry_id)
        if target is None:
            continue
        add_child(book, entry_id, target, parents)

    # 2. Move the printed page-bearing spells and items out of the pool.
    for node in list(pool):
        entry_id = node.get("entry")
        if not entry_id or entry_id in before and before[entry_id].startswith("Spells >"):
            continue
        kind = kind_of(corpus, entry_id)
        page = page_of(corpus, entry_id)
        if kind == "spell" and page is not None and entry_id not in TARGETS:
            add_child(book, entry_id, ["Spells"], parents)
        elif kind == "item" and page is not None:
            if entry_id.startswith("ref/eq-"):
                target = ["Equipment", "Weapons" if entry_id.startswith("ref/eq-weapon/") else "Armor"]
                if entry_id.startswith("ref/eq-misc/"):
                    target = ["Equipment", "Goods and Services"]
                add_child(book, entry_id, target, parents)
            elif entry_id.startswith("ref/magic-enh/"):
                target = ["Magic Items", "Armor", "Armor Special Abilities"] if page <= 285 else ["Magic Items", "Weapons", "Weapon Special Abilities"]
                add_child(book, entry_id, target, parents)
            elif entry_id.startswith("ref/magic-armor/"):
                add_child(book, entry_id, ["Magic Items", "Armor", "Specific Armors"], parents)
            elif entry_id.startswith("ref/magic-ring/"):
                add_child(book, entry_id, ["Magic Items", "Rings"], parents)
            elif entry_id.startswith("ref/magic-rod/"):
                add_child(book, entry_id, ["Magic Items", "Rods"], parents)
            elif entry_id.startswith("ref/magic-wondrous/"):
                add_child(book, entry_id, ["Magic Items", "Wondrous Items"], parents)

    # 3. Equipment goods and the trap go to their printed subsections.
    goods_targets = {
        "ref/eq-misc/buoy": "Adventuring Gear",
        "ref/eq-misc/silent-whistle": "Adventuring Gear",
        "ref/eq-misc/block-and-tackle": "Tools and Skill Kits",
        "ref/eq-misc/mapmakers-kit": "Tools and Skill Kits",
        "ref/eq-misc/tear-away-hot-weather-outfit": "Clothing",
        "ref/trap/bear-trap": "Tools and Skill Kits",
    }
    for entry_id, section in goods_targets.items():
        if find_node(book, entry_id) is not None:
            add_child(book, entry_id, ["Equipment", "Goods and Services", section], parents)
    bear_item = find_node(book, "ref/eq-misc/bear-trap")
    bear_trap = find_node(book, "ref/trap/bear-trap")
    if bear_item is not None and bear_trap is not None:
        current = parents.get("ref/trap/bear-trap")
        if current is not None:
            current["children"].remove(bear_trap)
        bear_item.setdefault("children", []).append(bear_trap)
        parents["ref/trap/bear-trap"] = bear_item
    for entry_id in [
        "ref/eq-misc/fortune-tellers-deck-common",
        "ref/eq-misc/fortune-tellers-deck-masterwork",
        "ref/eq-misc/fortune-tellers-deck-quality",
    ]:
        add_child(book, entry_id, ["Equipment", "Goods and Services", "Tools and Skill Kits"], parents)
    for entry_id in [
        "ref/eq-misc/loaded-dice-average",
        "ref/eq-misc/loaded-dice-superior",
    ]:
        add_child(book, entry_id, ["Equipment", "Goods and Services", "Entertainment Items"], parents)
    for entry_id in [
        "ref/eq-misc/cage-aquatic",
        "ref/eq-misc/cage-diminutive-or-fine",
        "ref/eq-misc/cage-diminutive-or-fine-aquatic",
        "ref/eq-misc/cage-huge",
        "ref/eq-misc/cage-huge-aquatic",
        "ref/eq-misc/cage-large",
        "ref/eq-misc/cage-large-aquatic",
        "ref/eq-misc/cage-small-or-medium",
        "ref/eq-misc/cage-small-or-medium-aquatic",
        "ref/eq-misc/cage-tiny",
        "ref/eq-misc/cage-tiny-aquatic",
        "ref/eq-misc/dog-sled-travel-per-mile",
    ]:
        add_child(book, entry_id, ["Equipment", "Goods and Services", "Animal-Related Gear"], parents)

    # 4. Nest printed variants under their base entry.
    variant_parents = {
        # Equipment
        "ref/carried/advanced-players-guide/rules/gear/weapons/arrow/blunt": ["ref/eq-weapon/blunt-arrows"],
        "ref/carried/advanced-players-guide/rules/gear/weapons/arrow/flight": ["ref/eq-weapon/flight-arrows"],
        "ref/carried/advanced-players-guide/rules/gear/armor/quickdraw-shield/light-wooden-or-steel": [
            "ref/eq-armor/light-steel-quickdraw-shield",
            "ref/eq-armor/light-wooden-quickdraw-shield",
        ],
        "ref/carried/advanced-players-guide/rules/gear/armor/wooden-armor": ["ref/eq-armor/wooden"],
        "ref/eq-misc/chest": [
            "ref/eq-misc/chest-huge",
            "ref/eq-misc/chest-large",
            "ref/eq-misc/chest-medium",
            "ref/eq-misc/chest-small",
        ],
        "ref/eq-misc/hourglass-1-hour": [
            "ref/eq-misc/hourglass-1-minute",
            "ref/eq-misc/hourglass-6-seconds",
        ],
        "ref/carried/advanced-players-guide/rules/gear/goods-and-services/adventuring-gear/poison-pill-ring": [
            "ref/eq-misc/poison-pill-signet-ring"
        ],
        "ref/carried/advanced-players-guide/rules/gear/goods-and-services/adventuring-gear/tent": [
            "ref/eq-misc/tent-large",
            "ref/eq-misc/tent-medium",
            "ref/eq-misc/tent-pavilion",
            "ref/eq-misc/tent-small",
        ],
        "ref/eq-misc/fortune-telling-deck": [
            "ref/eq-misc/fortune-tellers-deck-common",
            "ref/eq-misc/fortune-tellers-deck-masterwork",
            "ref/eq-misc/fortune-tellers-deck-quality",
        ],
        "ref/carried/advanced-players-guide/rules/gear/goods-and-services/special-substances-and-items/weapon-blanch": [
            "ref/eq-misc/weapon-blanch-adamantine",
            "ref/eq-misc/weapon-blanch-cold-iron",
            "ref/eq-misc/weapon-blanch-silver",
        ],
        "ref/eq-misc/cage": [
            "ref/eq-misc/cage-aquatic",
            "ref/eq-misc/cage-diminutive-or-fine",
            "ref/eq-misc/cage-diminutive-or-fine-aquatic",
            "ref/eq-misc/cage-huge",
            "ref/eq-misc/cage-huge-aquatic",
            "ref/eq-misc/cage-large",
            "ref/eq-misc/cage-large-aquatic",
            "ref/eq-misc/cage-small-or-medium",
            "ref/eq-misc/cage-small-or-medium-aquatic",
            "ref/eq-misc/cage-tiny",
            "ref/eq-misc/cage-tiny-aquatic",
        ],
        "ref/eq-misc/dog-sled": ["ref/eq-misc/dog-sled-travel-per-mile"],
        "ref/carried/advanced-players-guide/rules/gear/goods-and-services/entertainment-items/loaded-dice": [
            "ref/eq-misc/loaded-dice-average",
            "ref/eq-misc/loaded-dice-superior",
        ],
        "ref/carried/advanced-players-guide/rules/gear/goods-and-services/tools-and-skill-kits/map-makers-kit": [
            "ref/eq-misc/mapmakers-kit"
        ],
        # Magic items
        "ref/carried/advanced-players-guide/rules/magic-items/armor/specific-armors/giant-hide-armor": [
            "ref/magic-armor/giant-hide-armor-cloud-giant",
            "ref/magic-armor/giant-hide-armor-hill-giant",
            "ref/magic-armor/giant-hide-armor-ogre",
            "ref/magic-armor/giant-hide-armor-stone-fire-or-frost-giant",
            "ref/magic-armor/giant-hide-armor-storm-giant",
            "ref/magic-armor/giant-hide-armor-troll",
        ],
        "ref/carried/advanced-players-guide/rules/magic-items/rings/dungeon-ring": [
            "ref/magic-ring/dungeon-ring-jailers",
            "ref/magic-ring/dungeon-ring-prisoners",
        ],
        "ref/magic-ring/ring-of-delayed-doom": sorted(
            node["entry"]
            for node in pool
            if node.get("entry", "").startswith("ref/magic-ring/ring-of-delayed-doom-")
        ),
        "ref/carried/advanced-players-guide/rules/magic-items/rings/ring-of-revelation": [
            "ref/magic-ring/ring-of-revelation-greater",
            "ref/magic-ring/ring-of-revelation-lesser",
            "ref/magic-ring/ring-of-revelation-superior",
        ],
        "ref/carried/advanced-players-guide/rules/magic-items/wondrous-items/assisting-glove": [
            "ref/magic-wondrous/assisting-gloves"
        ],
        "ref/carried/advanced-players-guide/rules/magic-items/wondrous-items/knights-pennon": [
            "ref/magic-wondrous/knights-pennon-battle",
            "ref/magic-wondrous/knights-pennon-honor",
            "ref/magic-wondrous/knights-pennon-parley",
        ],
        "ref/carried/advanced-players-guide/rules/magic-items/wondrous-items/lords-banner": [
            "ref/magic-wondrous/lords-banner-crusades",
            "ref/magic-wondrous/lords-banner-swiftness",
            "ref/magic-wondrous/lords-banner-terror",
            "ref/magic-wondrous/lords-banner-victory",
        ],
        "ref/carried/advanced-players-guide/rules/magic-items/wondrous-items/mask-of-giants": [
            "ref/magic-wondrous/mask-of-giants-greater",
            "ref/magic-wondrous/mask-of-giants-lesser",
        ],
        "ref/carried/advanced-players-guide/rules/magic-items/wondrous-items/robes-of-arcane-heritage": [
            "ref/magic-wondrous/robe-of-arcane-heritage"
        ],
        "ref/magic-wondrous/sniper-goggles": ["ref/magic-wondrous/sniper-goggles-greater"],
        "ref/carried/advanced-players-guide/rules/magic-items/cursed-items/one-way-window": [
            "ref/magic-wondrous/one-way-window-cursed"
        ],
    }
    # Rod metamagic variants.
    for family in [
        "bouncing", "dazing", "disruptive", "ectoplasmic", "elemental", "focused",
        "intensified", "lingering", "merciful", "persistent", "reach", "selective",
        "sickening", "thundering",
    ]:
        title = family.capitalize()
        variant_parents[f"ref/carried/advanced-players-guide/rules/magic-items/rods/metamagic/{family}"] = [
            f"ref/magic-rod/{family}",
            f"ref/magic-rod/rod-of-metamagic-{family}-greater",
            f"ref/magic-rod/rod-of-metamagic-{family}-lesser",
            f"ref/magic-rod/rod-of-metamagic-{family}-normal",
        ]

    for parent_id, child_ids in variant_parents.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing variant parent {parent_id}")
        for child_id in child_ids:
            child = find_node(book, child_id)
            if child is None:
                continue
            current = parents.get(child_id)
            if current is not None:
                current["children"].remove(child)
            parent.setdefault("children", []).append(child)
            parents[child_id] = parent
        sort_children(parent, title_key)

    # 5. Move Antipaladin out of Paladin to its printed position.
    paladin = find_node(book, "ref/rule/class-paladin/paladin")
    antipaladin = find_node(book, "ref/class/antipaladin")
    if paladin is not None and antipaladin is not None and antipaladin in paladin.get("children", []):
        paladin["children"].remove(antipaladin)
        core = resolve_path(book, ["Classes", "Core Classes"])
        children = core["children"]
        children.insert(children.index(paladin) + 1, antipaladin)
        parents["ref/class/antipaladin"] = core

    # 6. Order the class feature groups.
    lead = {
        "Cavalier Orders": "ref/ability/orders",
        "Judgments": "ref/ability/judgments",
        "Mysteries": "ref/ability/mysteries",
        "Eidolons": "ref/sidekick/eidolon",
        "Domains": "ref/ability/domains",
        "Arcane Schools": "ref/ability/arcane-schools",
    }
    for title, lead_id in lead.items():
        for chapter in book["chapters"]:
            def find_group(node: dict) -> dict | None:
                if node["title"] == title and not node.get("entry"):
                    return node
                for child in node.get("children", []):
                    found = find_group(child)
                    if found is not None:
                        return found
                return None
            group = find_group(chapter)
            if group is not None:
                sort_children(group, lambda node: page_key(corpus, node), keep_first=[next(c["title"] for c in group["children"] if c.get("entry") == lead_id)] if any(c.get("entry") == lead_id for c in group["children"]) else [])

    # Groups without a lead entry are alphabetical or page ordered.
    for title in ["Discoveries", "Curses", "Evolutions", "Hexes", "Patron Spells", "Rage Powers", "Rogue Talents"]:
        for chapter in book["chapters"]:
            def find_group(node: dict) -> dict | None:
                if node["title"] == title and not node.get("entry"):
                    return node
                for child in node.get("children", []):
                    found = find_group(child)
                    if found is not None:
                        return found
                return None
            group = find_group(chapter)
            if group is not None:
                sort_children(group, title_key)
    for title in ["Animal Shamans", "Bloodlines"]:
        for chapter in book["chapters"]:
            def find_group(node: dict) -> dict | None:
                if node["title"] == title and not node.get("entry"):
                    return node
                for child in node.get("children", []):
                    found = find_group(child)
                    if found is not None:
                        return found
                return None
            group = find_group(chapter)
            if group is not None:
                sort_children(group, lambda node: page_key(corpus, node))

    # 7. Put the class feature groups in printed order under their class.
    class_order = {
        "ref/class/alchemist": ["Poison Use", "Discoveries"],
        "ref/class/cavalier": ["Cavalier Orders"],
        "ref/class/inquisitor": ["Judgments"],
        "ref/class/oracle": ["Curses", "Mysteries"],
        "ref/class/summoner": ["Eidolons", "Evolutions"],
        "ref/class/witch": ["Hexes", "Patron Spells"],
        "ref/rule/class-barbarian/barbarian": ["Rage Powers", "Barbarian Archetypes"],
        "ref/rule/class-cleric/cleric": ["Domains"],
        "ref/rule/class-rogue/rogue": ["Rogue Talents", "Rogue Archetypes"],
        "ref/rule/class-sorcerer/sorcerer": ["Bloodlines"],
        "ref/rule/class-wizard/wizard": ["Arcane Schools"],
    }
    for entry_id, order in class_order.items():
        node = find_node(book, entry_id)
        if node is not None:
            by_title = {child["title"]: child for child in node.get("children", [])}
            head = [by_title[title] for title in order if title in by_title]
            rest = [child for child in node.get("children", []) if child["title"] not in order]
            node["children"] = head + rest

    # 8. Feats: keep the chapter opening entries first, then alphabetical.
    feats = next(chapter for chapter in book["chapters"] if chapter["title"] == "Feats")
    sort_children(feats, title_key, keep_first=["Feats"])

    # 9. Spells: keep Spell Lists first, then alphabetical.
    spells = next(chapter for chapter in book["chapters"] if chapter["title"] == "Spells")
    sort_children(spells, title_key, keep_first=["Spell Lists"])

    # 10. Equipment sections: alphabetical.
    equipment = next(chapter for chapter in book["chapters"] if chapter["title"] == "Equipment")
    reorder(equipment, ["Equipment", "Weapons", "Armor", "Goods and Services"])
    for title in ["Weapons", "Armor"]:
        node = next((child for child in equipment["children"] if child["title"] == title), None)
        if node is not None:
            sort_children(node, title_key)
    goods = next((child for child in equipment["children"] if child["title"] == "Goods and Services"), None)
    if goods is not None:
        for section in goods.get("children", []):
            sort_children(section, title_key)

    # 11. Magic Items: section order and alphabetical item order.
    magic = next(chapter for chapter in book["chapters"] if chapter["title"] == "Magic Items")
    magic_order = [
        "Magic Items", "Armor", "Weapons", "Rings", "Rods", "Staves",
        "Wondrous Items", "Cursed Items", "Minor Artifacts", "Major Artifacts",
    ]
    reorder(magic, magic_order)
    armor = next((child for child in magic["children"] if child["title"] == "Armor"), None)
    if armor is not None:
        reorder(armor, ["Armor Special Abilities", "Specific Armors", "Specific Shields"])
        special = next((child for child in armor["children"] if child["title"] == "Armor Special Abilities"), None)
        if special is not None:
            sort_children(special, lambda node: page_key(corpus, node))
        for section in armor.get("children", []):
            if section["title"] in ("Specific Armors", "Specific Shields"):
                sort_children(section, title_key)
    weapons = next((child for child in magic["children"] if child["title"] == "Weapons"), None)
    if weapons is not None:
        reorder(weapons, ["Weapon Special Abilities", "Specific Weapons"])
        special = next((child for child in weapons["children"] if child["title"] == "Weapon Special Abilities"), None)
        if special is not None:
            sort_children(special, lambda node: page_key(corpus, node))
        section = next((child for child in weapons["children"] if child["title"] == "Specific Weapons"), None)
        if section is not None:
            sort_children(section, title_key)
    for title in ["Rings", "Rods", "Staves", "Wondrous Items", "Cursed Items", "Minor Artifacts", "Major Artifacts"]:
        node = next((child for child in magic["children"] if child["title"] == title), None)
        if node is not None:
            sort_children(node, title_key)

    # 12. New Rules: printed order inside the three sections.
    rules = next(chapter for chapter in book["chapters"] if chapter["title"] == "New Rules")
    combat = next(child for child in rules["children"] if child["title"] == "Combat Maneuvers")
    sort_children(combat, lambda node: page_key(corpus, node))
    hero = next(child for child in rules["children"] if child["title"] == "Hero Points")
    hero_head = [
        "Awarding Hero Points", "Using Hero Points", "Antiheroes", "Hero Points for GMs",
        "Hero Point Feats", "Hero Point Spells", "Hero Point Magic Items",
    ]
    reorder(hero, hero_head)
    traits = next(child for child in rules["children"] if child["title"] == "Traits")
    traits_head = [
        "Gaining Traits", "Restrictions on Trait Selection", "Types of Traits",
        "Basic Traits", "Campaign Traits", "Race Traits", "Regional Traits", "Religion Traits",
    ]
    reorder(traits, traits_head)
    def sort_trait_tree(node: dict) -> None:
        for child in node.get("children", []):
            sort_trait_tree(child)
        if node is not traits:
            sort_children(node, title_key)

    sort_trait_tree(traits)

    # 13. Put the page-bearing entries at their printed opening.
    races = next(chapter for chapter in book["chapters"] if chapter["title"] == "Races")
    favored = find_node(book, "ref/rule/favored-class")
    racial_favored = find_node(book, "ref/carried/advanced-players-guide/rules/races/racial-favored-classes")
    if favored is not None and racial_favored is not None:
        races["children"].remove(favored)
        races["children"].insert(races["children"].index(racial_favored) + 1, favored)

    core = resolve_path(book, ["Classes", "Core Classes"])
    archetypes = find_node(book, "ref/rule/archetypes")
    if archetypes is not None:
        core["children"].remove(archetypes)
        core["children"].insert(1, archetypes)

    hero_spells = next((child for child in hero["children"] if child["title"] == "Hero Point Spells"), None)
    if hero_spells is not None:
        def hero_page_key(node):
            page = page_of(corpus, node.get("entry"))
            return (page if page is not None else 324, title_key(node))
        sort_children(hero_spells, hero_page_key)

    # 14. The residual chapter keeps only entries with no APG section.
    # (No action; the pool nodes that were not moved stay in place.)

    after = {}
    def walk_after(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            after[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_after(child, here)
    for chapter in book["chapters"]:
        walk_after(chapter, [])

    if set(before) != set(after):
        missing = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        raise SystemExit(f"entry set changed: lost {missing}, added {added}")

    moves = []
    for entry_id in sorted(before):
        if before[entry_id] != after[entry_id]:
            moves.append((after[entry_id].split(" > ")[-1], before[entry_id], after[entry_id], page_of(corpus, entry_id)))
    return moves


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus", type=Path, help="corpus checkout that holds books.toml")
    parser.add_argument("--check", action="store_true", help="fail instead of writing when books.toml is not in the target shape")
    args = parser.parse_args()

    books_path = args.corpus / BOOKS_FILE
    original_text = books_path.read_text(encoding="utf-8")
    books = tomllib.loads(original_text)
    book = find_book(books)
    moves = transform(book, args.corpus)
    text = dump_books(books)

    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the APG nested-placement shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {len(moves)} APG moves", file=sys.stderr)
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} APG moves)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
