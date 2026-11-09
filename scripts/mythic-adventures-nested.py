#!/usr/bin/env python3
"""Place the Mythic Adventures tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/mythic-adventures-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.

The residual ``New entries (page order)`` chapter keeps the entries whose
primary publication is another book. Those base entries have no printed
Mythic Adventures position: only their mythic add-on prints in this book, and
that add-on is the placed row.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

BOOK = "mythic-adventures"
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


def walk(node: dict):
    yield node
    for child in node.get("children", []):
        yield from walk(child)


def find_node(book: dict, entry_id: str) -> dict | None:
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry") == entry_id:
                return node
    return None


def find_title(node: dict, title: str) -> dict | None:
    for candidate in walk(node):
        if candidate.get("title") == title:
            return candidate
    return None


def parent_of(book: dict, node: dict) -> dict | None:
    for chapter in book.get("chapters", []):
        for candidate in walk(chapter):
            if node in candidate.get("children", []):
                return candidate
    return None


def detach(book: dict, node: dict) -> None:
    parent = parent_of(book, node)
    if parent is None:
        raise SystemExit(f"{BOOK}: node {node.get('title')} is already a chapter")
    parent["children"].remove(node)
    if not parent["children"]:
        parent.pop("children", None)


def take(book: dict, entry_id: str) -> dict:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"{BOOK}: missing entry {entry_id}")
    detach(book, node)
    return node


def flatten(node: dict) -> list[dict]:
    return list(walk(node))


def book_entries(book: dict) -> set[str]:
    placed: set[str] = set()
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry"):
                placed.add(node["entry"])
    return placed


def path_of(book: dict, node: dict) -> str:
    def walk(current: dict, trail: list[str]) -> list[str] | None:
        here = trail + [current["title"]]
        if current is node:
            return here
        for child in current.get("children", []):
            found = walk(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk(chapter, [])
        if found is not None:
            return " > ".join(found)
    return "(missing)"


# ---------------------------------------------------------------------------
# Page evidence and ordering


MYTHIC_PAGE = re.compile(r"Mythic Adventures(?: pg\.?| p\.) ?(\d+)")


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    match = MYTHIC_PAGE.search(path.read_text(encoding="utf-8"))
    return int(match.group(1)) if match else None


ROMAN = {
    "i": 1, "ii": 2, "iii": 3, "iv": 4, "v": 5,
    "vi": 6, "vii": 7, "viii": 8, "ix": 9,
}


def sort_key(title: str) -> str:
    """Return the key print uses inside an alphabetical section."""
    text = title.lower()
    text = text.replace("\u2019", "'").replace("'", "")
    text = re.sub(r", mythic$", "", text)
    text = re.sub(r"[^a-z0-9 ]", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    # "Greater X", "Lesser X", "Mass X" and "Communal X" are alphabetized
    # under X.
    for prefix in ("greater ", "lesser ", "mass ", "communal "):
        if text.startswith(prefix):
            text = text[len(prefix) :] + " " + prefix.strip()
            break
    match = re.match(r"^(.*) (i|ii|iii|iv|v|vi|vii|viii|ix)$", text)
    if match:
        text = f"{match.group(1)} {ROMAN[match.group(2)]:02d}"
    return text


# The base items that print at a known page inside Other Magic Items. Their
# own records carry no Mythic Adventures page line.
BASE_PAGE = {
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "cape-of-free-will": 148,
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "cloak-of-quick-reflexes": 149,
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "pauldrons-of-unflinching-fortitude": 155,
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "root-of-the-world-tree": 156,
}

VARIANT_GROUPS = {
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "cape-of-free-will": [
        "ref/magic-wondrous/cape-of-free-will-1-2w",
        "ref/magic-wondrous/cape-of-free-will-2-3w",
        "ref/magic-wondrous/cape-of-free-will-3-4w",
        "ref/magic-wondrous/cape-of-free-will-4-5w",
        "ref/magic-wondrous/cape-of-free-will-5-6w",
    ],
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "cloak-of-quick-reflexes": [
        "ref/magic-wondrous/cloak-of-quick-reflexes-1-2r",
        "ref/magic-wondrous/cloak-of-quick-reflexes-2-3r",
        "ref/magic-wondrous/cloak-of-quick-reflexes-3-4r",
        "ref/magic-wondrous/cloak-of-quick-reflexes-4-5r",
        "ref/magic-wondrous/cloak-of-quick-reflexes-5-6r",
    ],
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "pauldrons-of-unflinching-fortitude": [
        "ref/magic-wondrous/pauldrons-of-unflinching-fortitude-1-2f",
        "ref/magic-wondrous/pauldrons-of-unflinching-fortitude-2-3f",
        "ref/magic-wondrous/pauldrons-of-unflinching-fortitude-3-4f",
        "ref/magic-wondrous/pauldrons-of-unflinching-fortitude-4-5f",
        "ref/magic-wondrous/pauldrons-of-unflinching-fortitude-5-6f",
    ],
    "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
    "root-of-the-world-tree": [
        "ref/magic-wondrous/root-of-the-world-tree-greater",
        "ref/magic-wondrous/root-of-the-world-tree-lesser",
        "ref/magic-wondrous/root-of-the-world-tree-normal",
    ],
}

# Dataset splits that print inside another entry.
NESTED_SPLITS = {
    "ref/carried/mythic-adventures/monsters/lich/mythic": [
        "ref/template/mythic-lich"
    ],
    "ref/carried/mythic-adventures/monsters/skeleton/mythic": [
        "ref/template/mythic-skeleton"
    ],
    "ref/carried/mythic-adventures/monsters/vampire/mythic": [
        "ref/template/mythic-vampire"
    ],
    "ref/magic-enh/potent": ["ref/rule/magic-enh-potent/potent"],
}


# ---------------------------------------------------------------------------
# Printed sections
#
# A node is ("e", entry) for a leaf entry, ("ec", entry, children) for an
# entry that holds printed subsections, and ("g", title, children) for a
# printed heading with no entry of its own.

PATH_TIERS = {
    "ref/rule/archmage": [
        "ref/rule/archmage-1",
        "ref/rule/archmage-3",
        "ref/rule/archmage-6",
    ],
    "ref/rule/champion": [
        "ref/rule/champion-1",
        "ref/rule/champion-3",
        "ref/rule/champion-6",
    ],
    "ref/rule/guardian": [
        "ref/rule/guardian-1",
        "ref/rule/guardian-3",
        "ref/rule/guardian-6",
    ],
    "ref/carried/mythic-adventures/rules/mythic-heroes/hierophant": [
        "ref/rule/hierophant-1",
        "ref/rule/hierophant-3",
        "ref/rule/hierophant-6",
    ],
    "ref/rule/marshal-2": [
        "ref/rule/marshal-2-1",
        "ref/rule/marshal-2-3",
        "ref/rule/marshal-2-6",
    ],
    "ref/rule/trickster": [
        "ref/rule/trickster-1",
        "ref/rule/trickster-3",
        "ref/rule/trickster-6",
    ],
    "ref/rule/universal-path-abilities": [
        "ref/rule/upa-1",
        "ref/rule/upa-3",
        "ref/rule/upa-6",
    ],
}


def heroes_spec() -> list:
    spec: list = [
        ("e", "ref/rule/mythic-heroes"),
        ("e", "ref/rule/creating-a-mythic-character"),
        ("e", "ref/rule/mythic-ascension"),
        ("e", "ref/rule/mythic-paths"),
        ("e", "ref/rule/selecting-a-path"),
        ("e", "ref/rule/base-mythic-abilities"),
        ("e", "ref/rule/gaining-tiers"),
    ]
    for path in (
        "ref/rule/archmage",
        "ref/rule/champion",
        "ref/rule/guardian",
        "ref/carried/mythic-adventures/rules/mythic-heroes/hierophant",
        "ref/rule/marshal-2",
        "ref/rule/trickster",
        "ref/rule/universal-path-abilities",
    ):
        spec.append(
            ("ec", path, [("e", entry) for entry in PATH_TIERS[path]])
        )
    return spec


def running_spec() -> list:
    return [
        ("e", "ref/rule/running-a-mythic-game"),
        ("e", "ref/rule/different-scales-of-mythic-campaigns"),
        ("e", "ref/rule/making-a-mythic-atmosphere"),
        (
            "ec",
            "ref/rule/mythic-story-structure",
            [
                ("e", "ref/rule/contact"),
                ("e", "ref/rule/awakening"),
                ("e", "ref/rule/journey"),
                ("e", "ref/rule/life-afterward"),
                ("e", "ref/rule/return"),
                ("e", "ref/rule/elements-of-a-mythic-adventure"),
            ],
        ),
        (
            "ec",
            "ref/rule/mythic-themes",
            [
                ("e", "ref/rule/eldritch-magic"),
                ("e", "ref/rule/the-worlds-reaction"),
                ("e", "ref/rule/legendary-encounter"),
                ("e", "ref/rule/lost-inheritance"),
                ("e", "ref/rule/planar-might"),
                ("e", "ref/rule/power-of-the-gods"),
                ("e", "ref/rule/stolen-might"),
            ],
        ),
        (
            "ec",
            "ref/rule/designing-encounters-ma",
            [("e", "ref/rule/rewarding-mythic-characters")],
        ),
        (
            "ec",
            "ref/rule/mythic-trials",
            [("e", "ref/rule/sample-trials")],
        ),
        ("e", "ref/rule/mythic-boons"),
        ("e", "ref/rule/recurring-mythic-villains"),
        ("e", "ref/rule/mythic-flaws"),
        (
            "ec",
            "ref/rule/ideas-for-mythic-adventures",
            [
                ("e", "ref/rule/eternal-gaze"),
                ("e", "ref/rule/falling-stars"),
                ("e", "ref/rule/for-love-of-sport"),
                ("e", "ref/rule/legacy-of-earth"),
                ("e", "ref/rule/perilous-peaks"),
                ("e", "ref/rule/the-nascent-sphere"),
                ("e", "ref/rule/unexpected-travels"),
            ],
        ),
    ]


def sorted_entries(corpus: Path, entry_ids: list[str]) -> list[str]:
    """Order a printed alphabetical list, using the page when it has one."""
    return sorted(
        entry_ids,
        key=lambda entry: (
            page_of(corpus, entry) or BASE_PAGE.get(entry, 0),
            sort_key(find_node_title(entry)),
        ),
    )


TITLES: dict[str, str] = {}


def find_node_title(entry_id: str) -> str:
    return TITLES[entry_id]


# ---------------------------------------------------------------------------
# The transform


def build(spec: list, book: dict, root: dict) -> list[dict]:
    out: list[dict] = []
    for item in spec:
        kind = item[0]
        if kind == "e":
            node = take(book, item[1])
            node.pop("children", None)
            out.append(node)
        elif kind == "ec":
            node = find_node(book, item[1])
            if node is None:
                raise SystemExit(f"{BOOK}: missing entry {item[1]}")
            children = build(item[2], book, node)
            detach(book, node)
            node["children"] = children
            out.append(node)
        elif kind == "g":
            node = find_title(root, item[1])
            if node is None or node.get("entry") is not None:
                node = {"title": item[1]}
            node["children"] = build(item[2], book, node)
            out.append(node)
        else:
            raise SystemExit(f"{BOOK}: bad spec {item!r}")
    return out


def magic_items_children(corpus: Path) -> tuple[list[str], dict[str, list[str]]]:
    other = [
        "ref/magic-wondrous/ambrosia",
        "ref/magic-wondrous/anchoring-belt",
        "ref/magic-wondrous/blind-helm",
        "ref/magic-wondrous/book-of-banishing",
        "ref/magic-wondrous/book-of-perfect-jokes",
        "ref/magic-wondrous/boots-of-earth-and-wind",
        "ref/magic-wondrous/bountiful-bottle",
        "ref/magic-wondrous/bracers-of-might",
        "ref/magic-wondrous/bracers-of-the-shield-mates",
        "ref/magic-wondrous/canopic-jar",
        "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
        "cape-of-free-will",
        "ref/magic-wondrous/caydens-cup",
        "ref/magic-wondrous/censer-of-sanctuary",
        "ref/magic-wondrous/chime-of-disillusionment",
        "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
        "cloak-of-quick-reflexes",
        "ref/magic-wondrous/cloak-of-the-hunt",
        "ref/magic-wondrous/cornucopia-of-plenty",
        "ref/magic-wondrous/death-wardens-bandolier",
        "ref/magic-rod/dolorous-rod",
        "ref/magic-wondrous/everburning-lantern",
        "ref/magic-wondrous/eye-orb",
        "ref/magic-wondrous/figurine-of-wondrous-power-basalt-dragon",
        "ref/magic-wondrous/gallows-rope",
        "ref/magic-wondrous/gloves-of-distant-action",
        "ref/magic-wondrous/gloves-of-spell-snaring",
        "ref/magic-wondrous/golden-holy-symbol",
        "ref/magic-wondrous/headband-of-sealed-thoughts",
        "ref/magic-wondrous/helm-of-the-serpent-king",
        "ref/magic-wondrous/herbs-of-the-primal-beast",
        "ref/magic-wondrous/immolation-cloak",
        "ref/magic-wondrous/inescapable-gloves",
        "ref/magic-wondrous/laurel-wreath",
        "ref/magic-wondrous/lyre-of-storms",
        "ref/magic-wondrous/mantle-of-the-faithful-vessel",
        "ref/magic-wondrous/mirroring-belt",
        "ref/magic-wondrous/mithral-rose",
        "ref/magic-wondrous/monocle-of-unveiled-auras",
        "ref/magic-wondrous/moonstone-cat",
        "ref/magic-wondrous/necklace-of-spectral-strikes",
        "ref/magic-wondrous/nectar-of-the-gods",
        "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
        "pauldrons-of-unflinching-fortitude",
        "ref/magic-wondrous/penitents-robes",
        "ref/magic-wondrous/phoenix-cloak",
        "ref/magic-ring/ring-of-energy-dampening",
        "ref/magic-ring/ring-of-transcendent-spells",
        "ref/carried/mythic-adventures/rules/mythic-items/magic-items/other-magic-items/"
        "root-of-the-world-tree",
        "ref/magic-wondrous/seven-league-boots",
        "ref/magic-wondrous/stonefist-gloves",
        "ref/magic-wondrous/torc-of-truespeech",
    ]
    specific_weapons = [
        "ref/magic-weapon/bow-of-erastil",
        "ref/magic-weapon/brutal-axe",
        "ref/magic-weapon/chaos-hammer",
        "ref/magic-weapon/dagger-of-a-thousand-bites",
        "ref/magic-weapon/dragonbreath-bow",
        "ref/magic-weapon/fire-goddesss-blade",
        "ref/magic-weapon/gun-with-no-name",
        "ref/magic-weapon/pick-of-stonecleaving",
        "ref/magic-weapon/sacred-avenger",
        "ref/magic-weapon/shadow-spike",
        "ref/magic-weapon/shadows-touch",
        "ref/magic-weapon/skirmishing-spear",
        "ref/magic-weapon/spellbreaker",
        "ref/magic-weapon/stormcaller",
        "ref/magic-weapon/sword-of-inner-fire",
    ]
    return sorted_entries(corpus, other), {
        "specific_weapons": sorted_entries(corpus, specific_weapons)
    }


def magic_items_spec(corpus: Path) -> list:
    other, extra = magic_items_children(corpus)
    other_spec_items: list = []
    for entry in other:
        if entry in VARIANT_GROUPS:
            other_spec_items.append(
                ("ec", entry, [("e", variant) for variant in VARIANT_GROUPS[entry]])
            )
        else:
            other_spec_items.append(("e", entry))
    return [
        ("e", "ref/rule/mythic-magic-items"),
        (
            "e",
            "ref/carried/mythic-adventures/rules/mythic-items/mythic-items",
        ),
        (
            "ec",
            "ref/rule/magic-items-ma",
            [
                (
                    "g",
                    "Armor and Shield Special Abilities",
                    [
                        ("e", "ref/magic-enh/deflecting"),
                        ("e", "ref/magic-enh/denying"),
                        ("e", "ref/magic-enh/resonating"),
                    ],
                ),
                (
                    "g",
                    "Specific Armors and Shields",
                    [
                        ("e", "ref/magic-armor/bastion-of-the-inheritor"),
                        ("e", "ref/magic-armor/spiritwalk-armor"),
                        ("e", "ref/magic-armor/stalwart-breastplate"),
                    ],
                ),
                (
                    "g",
                    "Weapon Special Abilities",
                    [
                        ("e", "ref/magic-enh/disjoining"),
                        ("e", "ref/magic-enh/harvesting"),
                        ("e", "ref/magic-enh/mythic-bane"),
                        (
                            "ec",
                            "ref/magic-enh/potent",
                            [("e", "ref/rule/magic-enh-potent/potent")],
                        ),
                        ("e", "ref/magic-enh/sacrosanct"),
                    ],
                ),
                (
                    "g",
                    "Specific Weapons",
                    [("e", entry) for entry in extra["specific_weapons"]],
                ),
                ("g", "Other Magic Items", other_spec_items),
            ],
        ),
        (
            "ec",
            "ref/carried/mythic-adventures/rules/mythic-items/artifacts",
            [
                (
                    "g",
                    "Minor Artifacts",
                    [
                        ("e", "ref/magic-artifact/aegis"),
                        ("e", "ref/magic-artifact/black-iron-axe"),
                        ("e", "ref/magic-artifact/bullroarers-of-outburst"),
                        ("e", "ref/magic-artifact/elemental-chain"),
                        ("e", "ref/magic-artifact/fleshhook-of-mythic-sustenance"),
                        ("e", "ref/magic-artifact/fortunes-arrow"),
                        ("e", "ref/magic-artifact/glabrezu-claw"),
                        ("e", "ref/magic-artifact/hermetic-flask"),
                        ("e", "ref/magic-artifact/nexus-crystal"),
                        ("e", "ref/magic-artifact/ring-of-equilibrium"),
                        ("e", "ref/magic-artifact/rod-of-spell-sundering"),
                        ("e", "ref/magic-artifact/screaming-spear-of-the-sun"),
                        ("e", "ref/magic-artifact/staff-of-eldritch-sovereignty"),
                        ("e", "ref/magic-artifact/sword-of-the-mists"),
                        ("e", "ref/magic-artifact/torc-of-the-heavens"),
                        ("e", "ref/magic-artifact/witherfang"),
                    ],
                ),
                (
                    "g",
                    "Major Artifacts",
                    [
                        ("e", "ref/magic-artifact/apocalypse-box"),
                        ("e", "ref/magic-artifact/diadem-of-nod"),
                        ("e", "ref/magic-artifact/emperors-mammoth"),
                        ("e", "ref/magic-artifact/frozen-heart-of-cocytus"),
                        ("e", "ref/magic-artifact/legendsbane"),
                        ("e", "ref/magic-artifact/netherworld-cauldron"),
                        ("e", "ref/magic-artifact/nimbus-of-radiant-truth"),
                        ("e", "ref/magic-artifact/plaguebringer"),
                        ("e", "ref/magic-artifact/scepter-of-the-shining-lord"),
                        ("e", "ref/magic-artifact/shadowwraith-heart"),
                        ("e", "ref/magic-artifact/silver-maiden"),
                        ("e", "ref/magic-artifact/tarnhelm"),
                        ("e", "ref/magic-artifact/trueforge"),
                    ],
                ),
            ],
        ),
        (
            "ec",
            "ref/rule/legendary-items",
            [
                ("e", "ref/rule/becoming-legendary"),
                ("e", "ref/rule/legendary-artifacts"),
                ("e", "ref/rule/base-legendary-abilities"),
                ("e", "ref/rule/legendary-item-abilities"),
            ],
        ),
    ]


UMR = [
    "ref/umr/block-attacks",
    "ref/umr/dragon-blood",
    "ref/umr/dragon-cantrips",
    "ref/umr/dragon-fury",
    "ref/umr/dual-initiative",
    "ref/umr/feral-savagery",
    "ref/umr/fortification",
    "ref/umr/greensight",
    "ref/umr/lingering-breath",
    "ref/umr/mistsight",
    "ref/umr/mythic-magic",
    "ref/umr/poisonous-blood",
    "ref/umr/powerful-blows",
    "ref/umr/sand-glide",
    "ref/umr/second-save",
    "ref/umr/simple-arcane-spellcasting",
    "ref/umr/simple-divine-spellcasting",
    "ref/umr/smother",
    "ref/umr/steal",
    "ref/umr/x-ray-vision",
]

MONSTERS = [
    "ref/carried/mythic-adventures/monsters/aboleth/mythic",
    "ref/carried/mythic-adventures/monsters/barghest/greater/mythic",
    "ref/carried/mythic-adventures/monsters/cockatrice/mythic",
    "ref/carried/mythic-adventures/monsters/cyclops/great/mythic",
    "ref/carried/mythic-adventures/monsters/demon/marilith/mythic",
    "ref/carried/mythic-adventures/monsters/demon/nalfeshnee/mythic",
    "ref/carried/mythic-adventures/monsters/demon/vrock/mythic",
    "ref/carried/mythic-adventures/monsters/devil/bone/mythic",
    "ref/carried/mythic-adventures/monsters/devil/ice/mythic",
    "ref/carried/mythic-adventures/monsters/dragon/black/mythic",
    "ref/carried/mythic-adventures/monsters/dragon/blue/mythic",
    "ref/carried/mythic-adventures/monsters/dragon/green/mythic",
    "ref/carried/mythic-adventures/monsters/dragon/red/red-dragon/young/mythic",
    "ref/carried/mythic-adventures/monsters/dragon/red/red-dragon/wyrm/mythic",
    "ref/carried/mythic-adventures/monsters/dragon/white/mythic",
    "ref/carried/mythic-adventures/monsters/elemental/air/mythic",
    "ref/carried/mythic-adventures/monsters/elemental/earth/mythic",
    "ref/carried/mythic-adventures/monsters/elemental/fire/mythic",
    "ref/carried/mythic-adventures/monsters/elemental/water/mythic",
    "ref/carried/mythic-adventures/monsters/ettin/mythic",
    "ref/carried/mythic-adventures/monsters/gargoyle/mythic",
    "ref/carried/mythic-adventures/monsters/giant/fire/mythic",
    "ref/carried/mythic-adventures/monsters/giant/hill/mythic",
    "ref/monster/golem-flesh-golem-mythic",
    "ref/carried/mythic-adventures/monsters/griffon/mythic",
    "ref/carried/mythic-adventures/monsters/hell-hound/mythic",
    "ref/carried/mythic-adventures/monsters/hell-hound/nessian-warhound/mythic",
    "ref/carried/mythic-adventures/monsters/hydra/mythic",
    "ref/carried/mythic-adventures/monsters/lamia/mythic",
    "ref/carried/mythic-adventures/monsters/lich/mythic",
    "ref/carried/mythic-adventures/monsters/manticore/mythic",
    "ref/carried/mythic-adventures/monsters/medusa/mythic",
    "ref/carried/mythic-adventures/monsters/minotaur/mythic",
    "ref/carried/mythic-adventures/monsters/mummy/mythic",
    "ref/carried/mythic-adventures/monsters/ogre/mythic",
    "ref/carried/mythic-adventures/monsters/owlbear/mythic",
    "ref/carried/mythic-adventures/monsters/owlbear/the-first/mythic",
    "ref/carried/mythic-adventures/monsters/phoenix/mythic",
    "ref/carried/mythic-adventures/monsters/rakshasa/mythic",
    "ref/carried/mythic-adventures/monsters/skeleton/mythic",
    "ref/carried/mythic-adventures/monsters/sphinx/mythic",
    "ref/carried/mythic-adventures/monsters/treant/mythic",
    "ref/carried/mythic-adventures/monsters/troll/mythic",
    "ref/carried/mythic-adventures/monsters/vampire/mythic",
    "ref/carried/mythic-adventures/monsters/worg/mythic",
    "ref/carried/mythic-adventures/monsters/worg/winter-wolf",
    "ref/carried/mythic-adventures/monsters/wyvern/mythic",
]


def monsters_spec() -> list:
    spec: list = []
    for entry in MONSTERS:
        if entry in NESTED_SPLITS:
            spec.append(
                ("ec", entry, [("e", split) for split in NESTED_SPLITS[entry]])
            )
        else:
            spec.append(("e", entry))
    return spec


def monsters_chapter_spec() -> list:
    return [
        ("e", "ref/rule/mythic-monsters"),
        ("e", "ref/rule/mythic-monster-origins"),
        ("e", "ref/rule/mythic-rank"),
        ("e", "ref/rule/reading-a-mythic-monster-stat-block"),
        ("g", "Mythic Monster Entries", monsters_spec()),
        (
            "ec",
            "ref/rule/mythic-monster-advancement",
            [
                (
                    "ec",
                    "ref/rule/mythic-simple-templates",
                    [
                        ("e", "ref/rule/create-a-new-monster"),
                        ("e", "ref/rule/modify-an-existing-monster"),
                    ],
                ),
                ("e", "ref/rule/building-a-mythic-creature"),
            ],
        ),
        ("e", "ref/rule/mythic-subtype"),
        (
            "g",
            "New Universal Monster Rules",
            [("e", entry) for entry in UMR],
        ),
        ("e", "ref/rule/evaluating-monster-statistics"),
    ]


def feats_spec(book: dict) -> list:
    chapter = next(c for c in book["chapters"] if c["title"] == "Mythic Feats")
    intro = ("ec", "ref/rule/mythic-feats", [
        ("e", "ref/carried/mythic-adventures/rules/mythic-feats/types-of-feats"),
        (
            "e",
            "ref/carried/mythic-adventures/rules/mythic-feats/feat-descriptions",
        ),
    ])
    feats = [
        node["entry"]
        for node in chapter.get("children", [])
        if node.get("entry") and node["entry"] != intro[1]
    ]
    feats.sort(key=lambda entry: sort_key(TITLES[entry]))
    return [intro] + [("e", entry) for entry in feats]


MYTHIC_SPELL_INTRO = [
    "ref/carried/mythic-adventures/rules/mythic-spells/learning-mythic-spells",
    "ref/carried/mythic-adventures/rules/mythic-spells/casting-mythic-spells",
    "ref/carried/mythic-adventures/rules/mythic-spells/effects-of-mythic-spells",
    "ref/carried/mythic-adventures/rules/mythic-spells/augmented-mythic-spells",
    "ref/carried/mythic-adventures/rules/mythic-spells/mythic-spells-in-magic-items",
    "ref/carried/mythic-adventures/rules/mythic-spells/mythic-spells-in-stat-blocks",
    "ref/carried/mythic-adventures/rules/mythic-spells/resilient",
    "ref/carried/mythic-adventures/rules/mythic-spells/"
    "tiers-in-mythic-spell-descriptions",
]


def spells_spec(book: dict, corpus: Path) -> list:
    chapter = next(c for c in book["chapters"] if c["title"] == "Mythic Spells")
    entries = [
        node["entry"]
        for node in walk(chapter)
        if node.get("entry")
        and node["entry"] not in MYTHIC_SPELL_INTRO
        and node["entry"] not in ("ref/rule/mythic-spells", "ref/rule/magic-enh-potent/potent")
    ]
    mythic_intro = ("ec", "ref/rule/mythic-spells", [
        ("e", entry) for entry in MYTHIC_SPELL_INTRO
    ])
    myths = [entry for entry in entries if (page_of(corpus, entry) or 999) < 113]
    new = [entry for entry in entries if (page_of(corpus, entry) or 999) >= 113]
    return [
        mythic_intro,
        ("g", "Mythic Spell Descriptions", [("e", entry) for entry in myths]),
        ("g", "New Spells", [("e", entry) for entry in new]),
    ]


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    global TITLES
    TITLES = {
        node["entry"]: node["title"]
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }
    before_paths = {
        node["entry"]: path_of(book, node)
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }

    chapters = {chapter["title"]: chapter for chapter in book["chapters"]}
    residual = chapters["New entries (page order)"]

    specs = {
        "Mythic Heroes": heroes_spec(),
        "Mythic Feats": feats_spec(book),
        "Mythic Magic Items": magic_items_spec(corpus),
        "Mythic Spells": spells_spec(book, corpus),
        "Running a Mythic Game": running_spec(),
        "Mythic Monsters": monsters_chapter_spec(),
    }
    for title, spec in specs.items():
        chapter = chapters[title]
        built = build(spec, book, chapter)
        chapter["children"] = built

    after_paths = {
        node["entry"]: path_of(book, node)
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }

    if set(before_paths) != set(after_paths):
        missing = sorted(set(before_paths) - set(after_paths))
        added = sorted(set(after_paths) - set(before_paths))
        raise SystemExit(f"entry set changed: lost {missing}, added {added}")

    moves = []
    for entry_id in sorted(before_paths):
        if before_paths[entry_id] != after_paths[entry_id]:
            moves.append(
                (
                    TITLES[entry_id],
                    before_paths[entry_id],
                    after_paths[entry_id],
                    page_of(corpus, entry_id),
                )
            )
    return moves


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("corpus", type=Path, help="corpus checkout that holds books.toml")
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when books.toml is not in the target shape",
    )
    args = parser.parse_args()

    books_path = args.corpus / BOOKS_FILE
    original_text = books_path.read_text(encoding="utf-8")
    books = tomllib.loads(original_text)
    book = find_book(books)
    before = book_entries(book)
    moves = transform(book, args.corpus)
    after = book_entries(book)
    if before != after:
        print(f"{BOOK}: entry set changed", file=sys.stderr)
        for entry in sorted(before - after):
            print(f"  lost: {entry}", file=sys.stderr)
        for entry in sorted(after - before):
            print(f"  added: {entry}", file=sys.stderr)
        return 1

    book["basis"] = (
        "The chapter titles, their order, and their first pages are checked "
        "against the printed contents page of Mythic Adventures. The Glossary "
        "is front matter in print, so it is placed first here. Each chapter "
        "holds its printed section headings, and each entry sits in its "
        "printed section in page order. See "
        "docs/research/book-order-evidence.md. Canonical ids were substituted "
        "through the reviewed alias map; chapters and order are unchanged."
    )
    book["review"] = [
        "The printed chapter 7: Fire Over Blackcrag has no corresponding chapter in the dataset.",
        "The 269 mythic spell records are not part of the library (see coverage).",
        "Nested placement follows the printed section headings; entries that share a printed page keep the dataset order.",
        "Canonical ids were substituted through sources/canonical/id-map.toml. A legacy entry with no installed target is omitted and listed in coverage.",
    ]

    text = dump_books(books)
    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the nested Mythic Adventures shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {BOOK}; entries placed: {len(after)}")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} moves, {len(after)} entries placed)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
