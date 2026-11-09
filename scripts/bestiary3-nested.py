#!/usr/bin/env python3
"""Place the Bestiary 3 tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/bestiary3-nested.py /path/to/cast-iron-data
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

BOOK = "bestiary-3"
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
    if node.get("collapse"):
        out.append("collapse = true")
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


def parent_of(book: dict, node: dict) -> dict | None:
    def walk(current: dict) -> dict | None:
        if node in current.get("children", []):
            return current
        for child in current.get("children", []):
            found = walk(child)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        if chapter is node:
            return None
        found = walk(chapter)
        if found is not None:
            return found
    return None


def find_with_parent(root: dict, node: dict) -> dict | None:
    for child in root.get("children", []):
        if child is node:
            return root
        found = find_with_parent(child, node)
        if found is not None:
            return found
    return None


def find_in(root: dict, entry_id: str) -> dict | None:
    if root.get("entry") == entry_id:
        return root
    for child in root.get("children", []):
        found = find_in(child, entry_id)
        if found is not None:
            return found
    return None


def extract(book: dict, root: dict, entry_id: str) -> dict:
    """Remove a node from a detached subtree, or from the book."""
    node = find_in(root, entry_id)
    if node is not None and node is not root:
        parent = find_with_parent(root, node)
        if parent is not None:
            parent["children"].remove(node)
            return node
    return detach(book, entry_id)


def detach(book: dict, entry_id: str) -> dict:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")
    parent = parent_of(book, node)
    if parent is None:
        raise SystemExit(f"entry {entry_id} is already a chapter")
    parent["children"].remove(node)
    return node


def attach(parent: dict, node: dict) -> None:
    parent.setdefault("children", []).append(node)


def move(book: dict, entry_id: str, target: dict) -> None:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")
    if parent_of(book, node) is target:
        return
    attach(target, detach(book, entry_id))


def chapter(book: dict, title: str) -> dict:
    for node in book["chapters"]:
        if node["title"] == title:
            return node
    raise SystemExit(f"missing chapter {title}")


def optional_chapter(book: dict, title: str) -> dict | None:
    return next((node for node in book["chapters"] if node["title"] == title), None)


def chapter_or_heading(book: dict, title: str) -> dict:
    node = optional_chapter(book, title)
    if node is not None:
        return node
    node = {"title": title, "children": []}
    book["chapters"].append(node)
    return node


def group_heading(book: dict, parent: dict, title: str) -> dict:
    """Detach a bare heading with this title from the parent, or make one."""
    node = next(
        (
            child
            for child in parent.get("children", [])
            if child["title"] == title and not child.get("entry")
        ),
        None,
    )
    if node is None:
        node = {"title": title, "children": []}
    else:
        parent["children"].remove(node)
    return node


def sort_children(parent: dict, key) -> None:
    parent["children"] = sorted(parent.get("children", []), key=key)


def title_key(node: dict):
    return tuple(re.findall(r"[a-z0-9]+", node["title"].lower()))


# ---------------------------------------------------------------------------
# Page evidence


BESTIARY_3_PAGE = re.compile(r"Bestiary 3 (?:pg\.?|p\.?) ?(\d+)")

# Entries whose source line names no page, or names the wrong page. Each value
# is the printed page of the entry in Bestiary 3.
PAGE_OVERRIDES: dict[str, int] = {
    # Printed group headings without a page on their dataset entry.
    "ref/family/asura": 20,
    "ref/family/behemoth": 36,
    "ref/family/clockwork": 53,
    "ref/family/demodand": 68,
    "ref/family/div": 82,
    "ref/family/dragon-imperial": 92,
    "ref/family/dragon-imperial-forest": 94,
    "ref/family/dragon-imperial-sea": 96,
    "ref/family/dragon-imperial-sky": 98,
    "ref/family/dragon-imperial-sovereign": 100,
    "ref/family/dragon-imperial-underworld": 102,
    "ref/family/kami": 159,
    "ref/family/kyton": 170,
    "ref/family/kyton-demagogue": 170,
    "ref/family/leshy": 176,
    "ref/family/linnorm": 181,
    "ref/family/oni": 205,
    "ref/family/rakshasa": 224,
    "ref/family/sphinx": 250,
    "ref/family/thriae": 263,
    "ref/template/animal-lord": 14,
    "ref/template/fey-creature": 116,
    "ref/template/foo-creature": 120,
    "ref/template/graveknight": 138,
    "ref/template/penanggalen": 216,
    "ref/template/jiang-shi": 277,
    # Example creatures that print inside the template section.
    "ref/carried/bestiary-3/monsters/animal-lord": 14,
    "ref/carried/bestiary-3/monsters/fey-creature": 116,
    "ref/carried/bestiary-3/monsters/foo-creature/foo-dog": 120,
    "ref/carried/bestiary-3/monsters/foo-creature/foo-lion": 120,
    "ref/carried/bestiary-3/monsters/graveknight": 138,
    "ref/carried/bestiary-3/monsters/penanggalen/penanggalen": 216,
    "ref/carried/bestiary-3/monsters/vampire/jiang-shi": 277,
    # The dataset source line for this block counts one page less than print.
    "ref/monster/asura-adhukait": 22,
    "ref/monster/asura-aghasura": 23,
    "ref/monster/asura-asurendra": 24,
    "ref/monster/asura-tripurasura": 26,
    "ref/monster/asura-upasunda": 27,
    "ref/monster/atomie": 28,
    "ref/monster/axe-beak": 29,
    "ref/monster/azruverda": 30,
    "ref/monster/baku": 31,
    # A source-line typo and two shifted Thriae pages.
    "ref/monster/shedu": 243,
    "ref/monster/thriae-thriae-queen": 264,
    "ref/monster/thriae-thriae-seer": 266,
    "ref/monster/thriae-thriae-soldier": 267,
    # Entries that print in a section but carry no page of their own.
    "ref/carried/bestiary-3/monsters/marsupial/kangaroo": 191,
    "ref/carried/bestiary-3/monsters/marsupial/thylacine": 191,
    "ref/carried/bestiary-3/monsters/swarm/parasite/rot-grub": 215,
    "ref/carried/bestiary-3/monsters/parasite/giant-rot-grub": 215,
    "ref/carried/bestiary/rules/universal-monster-rules/capsize": 293,
    "ref/carried/bestiary/rules/universal-monster-rules/water-dependency": 301,
}

DRAGON_STAT_BLOCK_PAGES: dict[str, int] = {
    "ref/carried/bestiary-3/monsters/dragon/imperial/forest/dragon/imperial/forest": 94,
    "ref/carried/bestiary-3/monsters/dragon/imperial/sea/dragon/imperial/sea": 96,
    "ref/carried/bestiary-3/monsters/dragon/imperial/sky/dragon/imperial/sky": 98,
    "ref/carried/bestiary-3/monsters/dragon/imperial/sovereign/dragon/imperial/sovereign": 100,
    "ref/carried/bestiary-3/monsters/dragon/imperial/underworld/dragon/imperial/underworld": 102,
}
PAGE_OVERRIDES.update(DRAGON_STAT_BLOCK_PAGES)


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    found = BESTIARY_3_PAGE.search(path.read_text(encoding="utf-8"))
    return int(found.group(1)) if found else None


def node_page(corpus: Path, node: dict) -> int | None:
    page = page_of(corpus, node.get("entry"))
    if page is not None:
        return page
    pages = [page_of(corpus, child.get("entry")) for child in node.get("children", [])]
    known = [value for value in pages if value is not None]
    return min(known) if known else None


def page_key(corpus: Path, node: dict):
    page = node_page(corpus, node)
    return page if page is not None else 10**9


# ---------------------------------------------------------------------------
# The transform


# Printed family headings.  The family entry becomes the heading; the listed
# monsters sit under it in printed page order.
FAMILY_GROUPS: dict[str, list[str]] = {
    "ref/family/asura": [
        "ref/monster/asura-adhukait",
        "ref/monster/asura-aghasura",
        "ref/monster/asura-asurendra",
        "ref/monster/asura-tripurasura",
        "ref/monster/asura-upasunda",
    ],
    "ref/family/behemoth": [
        "ref/monster/behemoth-tempest-behemoth",
        "ref/monster/behemoth-thalassic-behemoth",
        "ref/monster/behemoth-thunder-behemoth",
    ],
    "ref/family/clockwork": [
        "ref/monster/clockwork-clockwork-goliath",
        "ref/monster/clockwork-clockwork-leviathan",
        "ref/monster/clockwork-clockwork-servant",
        "ref/monster/clockwork-clockwork-soldier",
        "ref/monster/clockwork-clockwork-spy",
    ],
    "ref/family/demodand": [
        "ref/monster/demodand-shaggy-demodand",
        "ref/monster/demodand-slimy-demodand",
        "ref/monster/demodand-tarry-demodand",
    ],
    "ref/family/div": [
        "ref/monster/div-aghash",
        "ref/monster/div-akvan",
        "ref/monster/div-doru",
        "ref/monster/div-ghawwas",
        "ref/monster/div-pairaka",
        "ref/monster/div-sepid",
        "ref/monster/div-shira",
    ],
    "ref/family/kami": [
        "ref/monster/kami-jinushigami",
        "ref/monster/kami-kodama",
        "ref/monster/kami-shikigami",
        "ref/monster/kami-toshigami",
        "ref/monster/kami-zuishin",
    ],
    "ref/family/kyton": [
        "ref/family/kyton-demagogue",
        "ref/monster/kyton-augur",
        "ref/monster/kyton-eremite",
        "ref/monster/kyton-interlocutor",
    ],
    "ref/family/leshy": [
        "ref/monster/leshy-fungus-leshy",
        "ref/monster/leshy-gourd-leshy",
        "ref/monster/leshy-leaf-leshy",
        "ref/monster/leshy-seaweed-leshy",
    ],
    "ref/family/linnorm": [
        "ref/monster/linnorm-cairn-linnorm",
        "ref/monster/linnorm-fjord-linnorm",
        "ref/monster/linnorm-taiga-linnorm",
        "ref/monster/linnorm-tor-linnorm",
    ],
    "ref/family/oni": [
        "ref/monster/oni-fire-yai",
        "ref/monster/oni-ice-yai",
        "ref/monster/oni-kuwa",
        "ref/monster/oni-spirit-oni",
        "ref/monster/oni-void-yai",
        "ref/monster/oni-water-yai",
    ],
    "ref/family/rakshasa": [
        "ref/monster/rakshasa-dandasuka",
        "ref/monster/rakshasa-rakshasa-maharaja",
        "ref/monster/rakshasa-marai",
        "ref/monster/rakshasa-raktavarna",
        "ref/monster/rakshasa-tataka",
    ],
    "ref/family/sphinx": [
        "ref/monster/sphinx-androsphinx",
        "ref/monster/sphinx-criosphinx",
        "ref/monster/sphinx-hieracosphinx",
    ],
    "ref/family/thriae": [
        "ref/monster/thriae-thriae-queen",
        "ref/monster/thriae-thriae-seer",
        "ref/monster/thriae-thriae-soldier",
    ],
}

# Imperial dragons: one heading per colour, each holding its stat block and the
# printed age variants.
DRAGON_IMPERIAL_GROUP = "ref/family/dragon-imperial"
DRAGON_IMPERIAL = [
    (
        "ref/family/dragon-imperial-forest",
        "forest",
    ),
    (
        "ref/family/dragon-imperial-sea",
        "sea",
    ),
    (
        "ref/family/dragon-imperial-sky",
        "sky",
    ),
    (
        "ref/family/dragon-imperial-sovereign",
        "sovereign",
    ),
    (
        "ref/family/dragon-imperial-underworld",
        "underworld",
    ),
]

# The Bestiary 3 prints six additional familiars as one section on pp. 112-113.
FAMILIAR_GROUP = "Familiar"
FAMILIARS = [
    "ref/monster/familiar-flying-squirrel",
    "ref/monster/familiar-fox",
    "ref/monster/familiar-goat",
    "ref/monster/familiar-otter",
    "ref/monster/familiar-pig",
    "ref/monster/familiar-raccoon",
]

# Template headings.  The printed book gives each template a heading and one
# or more example creatures; the examples sit under the heading.
TEMPLATE_GROUPS: dict[str, list[str]] = {
    "ref/template/animal-lord": ["ref/carried/bestiary-3/monsters/animal-lord"],
    "ref/template/fey-creature": ["ref/carried/bestiary-3/monsters/fey-creature"],
    "ref/template/foo-creature": [
        "ref/carried/bestiary-3/monsters/foo-creature/foo-dog",
        "ref/carried/bestiary-3/monsters/foo-creature/foo-lion",
    ],
    "ref/template/graveknight": ["ref/carried/bestiary-3/monsters/graveknight"],
    "ref/template/penanggalen": [
        "ref/carried/bestiary-3/monsters/penanggalen/penanggalen"
    ],
    "ref/template/jiang-shi": ["ref/carried/bestiary-3/monsters/vampire/jiang-shi"],
}

# A printed Construction subsection inside a construct entry.
CONSTRUCTION_TARGETS: dict[str, str] = {
    "ref/monster/caryatid-column": "ref/carried/bestiary-3/monsters/caryatid-column/construction/caryatid-column",
    "ref/monster/clockwork-clockwork-goliath": "ref/carried/bestiary-3/monsters/clockwork/clockwork-goliath/construction/clockwork-goliath",
    "ref/monster/clockwork-clockwork-leviathan": "ref/carried/bestiary-3/monsters/clockwork/clockwork-leviathan/construction/clockwork-leviathan",
    "ref/monster/clockwork-clockwork-servant": "ref/carried/bestiary-3/monsters/clockwork/clockwork-servant/construction/clockwork-servant",
    "ref/monster/clockwork-clockwork-soldier": "ref/carried/bestiary-3/monsters/clockwork/clockwork-soldier/construction/clockwork-soldier",
    "ref/monster/clockwork-clockwork-spy": "ref/carried/bestiary-3/monsters/clockwork/clockwork-spy/construction/clockwork-spy",
    "ref/monster/golem-bone-golem": "ref/carried/bestiary-3/monsters/golem/bone/construction/bone-golem",
    "ref/monster/golem-brass-golem": "ref/carried/bestiary-3/monsters/golem/brass/construction/brass-golem",
    "ref/monster/golem-cannon-golem": "ref/carried/bestiary-3/monsters/golem/cannon/construction/cannon-golem",
    "ref/monster/golem-fossil-golem": "ref/carried/bestiary-3/monsters/golem/fossil/construction/fossil-golem",
    "ref/monster/graven-guardian": "ref/carried/bestiary-3/monsters/graven-guardian/construction/graven-guardian",
    "ref/monster/taotieh": "ref/carried/bestiary-3/monsters/taotieh/construction/taotieh",
    "ref/monster/terra-cotta-soldier": "ref/carried/bestiary-3/monsters/terra-cotta-soldier/construction/terra-cotta-soldier",
    "ref/monster/tophet": "ref/carried/bestiary-3/monsters/tophet/construction/tophet",
    "ref/monster/tupilaq": "ref/carried/bestiary-3/monsters/tupilaq/construction/tupilaq",
}

# A printed "Growing a <kind> Leshy" subsection inside a leshy entry.
LESHY_GROWING: dict[str, tuple[str, str]] = {
    "ref/monster/leshy-fungus-leshy": (
        "Growing a Fungus Leshy",
        "ref/carried/bestiary-3/monsters/leshy/growing-a-fungus-leshy/fungus-leshy",
    ),
    "ref/monster/leshy-gourd-leshy": (
        "Growing a Gourd Leshy",
        "ref/carried/bestiary-3/monsters/leshy/growing-a-gourd-leshy/gourd-leshy",
    ),
    "ref/monster/leshy-leaf-leshy": (
        "Growing a Leaf Leshy",
        "ref/carried/bestiary-3/monsters/leshy/growing-a-leaf-leshy/leaf-leshy",
    ),
    "ref/monster/leshy-seaweed-leshy": (
        "Growing a Seaweed Leshy",
        "ref/carried/bestiary-3/monsters/leshy/growing-a-seaweed-leshy/seaweed-leshy",
    ),
}

# Dataset splits that print inside another entry.
VARIANT_TARGETS: dict[str, str] = {
    "ref/monster/kangaroo": "ref/carried/bestiary-3/monsters/marsupial/kangaroo",
    "ref/monster/thylacine": "ref/carried/bestiary-3/monsters/marsupial/thylacine",
}

# Monster abilities that print inside the monster's entry.
ABILITY_TARGETS: dict[str, str] = {
    "ref/affliction/asura-asurendra/poison-ex": "ref/monster/asura-asurendra",
    "ref/affliction/cerberi/cerberuss-jaws-su": "ref/monster/cerberi",
    "ref/affliction/div-pairaka/bubonic-plague": "ref/monster/div-pairaka",
    "ref/affliction/div-pairaka/shakes": "ref/monster/div-pairaka",
    "ref/affliction/spider-giant-crab-spider/poison-ex": "ref/monster/spider-giant-crab-spider",
    "ref/affliction/spider-ogre-spider/poison-ex": "ref/monster/spider-ogre-spider",
    "ref/affliction/wolf-in-sheeps-clothing/wolf-in-sheeps-clothing-egg": "ref/monster/wolf-in-sheeps-clothing",
}

# Universal Monster Rules splits print inside the rule they belong to.
UMR_TARGETS: dict[str, str] = {
    "ref/rule/umr-entrap/entrap": "ref/umr/entrap",
    "ref/rule/umr-heat/heat": "ref/umr/heat",
    "ref/rule/umr-jet/jet": "ref/umr/jet",
    "ref/rule/umr-multiweapon-mastery/multiweapon-mastery": "ref/umr/multiweapon-mastery",
    "ref/rule/umr-split/split": "ref/umr/split",
    "ref/rule/umr-water-breathing/water-breathing": "ref/umr/water-breathing",
    "ref/rule/umr-whirlwind/whirlwind": "ref/umr/whirlwind",
}

# Printed appendices that have corpus entries.  Appendices 1, 2, 4, 5, 6 and
# 9-13 have no Bestiary 3 entries in the corpus, so they get no group.
UNIVERSAL_MONSTER_RULES = [
    "ref/umr/entrap",
    "ref/umr/heat",
    "ref/umr/jet",
    "ref/umr/multiweapon-mastery",
    "ref/umr/split",
    "ref/umr/water-breathing",
    "ref/umr/whirlwind",
    "ref/carried/bestiary/rules/universal-monster-rules/capsize",
    "ref/carried/bestiary/rules/universal-monster-rules/water-dependency",
]

ANIMAL_COMPANIONS = [
    "ref/companion/antelope",
    "ref/companion/archelon",
    "ref/companion/axe-beak",
    "ref/companion/baluchitherium",
    "ref/companion/basilosaurus",
    "ref/companion/dimetrodon",
    "ref/companion/elk",
    "ref/companion/vulture-giant",
    "ref/companion/iguanodon",
    "ref/companion/kangaroo",
    "ref/companion/lizard-giant-chameleon",
    "ref/companion/lizard-giant-gecko",
    "ref/companion/megalania",
    "ref/companion/pachycephalosaurus",
    "ref/companion/spinosaurus",
    "ref/companion/thylacine",
]

SUBTYPES = [
    "ref/subtype/adlet",
    "ref/subtype/air",
    "ref/subtype/aquatic",
    "ref/subtype/archon",
    "ref/subtype/asura",
    "ref/subtype/augmented",
    "ref/subtype/behemoth",
    "ref/subtype/catfolk",
    "ref/subtype/chaotic",
    "ref/subtype/clockwork",
    "ref/subtype/cold",
    "ref/subtype/daemon",
    "ref/subtype/demodand",
    "ref/subtype/demon",
    "ref/subtype/devil",
    "ref/subtype/div",
    "ref/subtype/earth",
    "ref/subtype/elemental",
    "ref/subtype/evil",
    "ref/subtype/extraplanar",
    "ref/subtype/fire",
    "ref/subtype/giant",
    "ref/subtype/good",
    "ref/subtype/human",
    "ref/subtype/incorporeal",
    "ref/subtype/kami",
    "ref/subtype/kyton",
    "ref/subtype/lawful",
    "ref/subtype/leshy",
    "ref/subtype/native",
    "ref/subtype/oni",
    "ref/subtype/rakshasa",
    "ref/subtype/ratfolk",
    "ref/subtype/reptilian",
    "ref/subtype/sasquatch",
    "ref/subtype/shapechanger",
    "ref/subtype/swarm",
    "ref/subtype/vanara",
    "ref/subtype/vishkanya",
    "ref/subtype/water",
]


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None, str]]:
    before: dict[str, str] = {}

    def walk_paths(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            before[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_paths(child, here)

    for top in book["chapters"]:
        walk_paths(top, [])

    monsters = chapter(book, "Monsters A to Z")
    residual = optional_chapter(book, "New entries (page order)")

    umr = chapter_or_heading(book, "Universal Monster Rules")
    companions = chapter_or_heading(book, "Animal Companions")
    by_type = chapter_or_heading(book, "Monsters by Type")
    subtypes = group_heading(book, by_type, "Creature SubTypes")
    attach(by_type, subtypes)

    # 1. Dataset splits move under the entry that prints them.
    for entry_id, parent_id in ABILITY_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing ability parent {parent_id}")
        move(book, entry_id, parent)

    for entry_id, parent_id in VARIANT_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing variant parent {parent_id}")
        move(book, entry_id, parent)

    for entry_id, parent_id in UMR_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing rule parent {parent_id}")
        move(book, entry_id, parent)

    # 2. Printed Construction and Growing a Leshy subsections.
    for parent_id, entry_id in CONSTRUCTION_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing construction parent {parent_id}")
        heading = group_heading(book, parent, "Construction")
        attach(heading, extract(book, heading, entry_id))
        attach(parent, heading)

    for parent_id, (title, entry_id) in LESHY_GROWING.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing leshy parent {parent_id}")
        heading = group_heading(book, parent, title)
        attach(heading, extract(book, heading, entry_id))
        attach(parent, heading)

    # 3. Printed family headings.
    for family_id, member_ids in FAMILY_GROUPS.items():
        group = detach(book, family_id)
        for member_id in member_ids:
            attach(group, extract(book, group, member_id))
        sort_children(group, lambda child: page_key(corpus, child))
        attach(monsters, group)

    # Imperial dragons: heading -> colour -> stat block + age variants.
    dragon = detach(book, DRAGON_IMPERIAL_GROUP)
    for colour_id, colour in DRAGON_IMPERIAL:
        colour_node = extract(book, dragon, colour_id)
        base = (
            "ref/carried/bestiary-3/monsters/dragon/imperial/"
            f"{colour}/dragon/imperial/{colour}"
        )
        attach(colour_node, extract(book, colour_node, base))
        for age in ("young", "adult", "ancient"):
            attach(
                colour_node,
                extract(
                    book,
                    colour_node,
                    f"ref/monster/dragon-imperial-{colour}-{age}-{colour}-dragon",
                ),
            )
        sort_children(colour_node, lambda child: page_key(corpus, child))
        attach(dragon, colour_node)
    sort_children(dragon, lambda child: page_key(corpus, child))
    attach(monsters, dragon)

    # 4. The printed Familiar section.
    familiar = group_heading(book, monsters, FAMILIAR_GROUP)
    for entry_id in FAMILIARS:
        attach(familiar, extract(book, familiar, entry_id))
    sort_children(familiar, lambda child: page_key(corpus, child))
    attach(monsters, familiar)

    # 5. Template headings with their example creatures.
    for template_id, example_ids in TEMPLATE_GROUPS.items():
        group = detach(book, template_id)
        for example_id in example_ids:
            attach(group, extract(book, group, example_id))
        attach(monsters, group)

    # 6. Appendices that have corpus entries.
    for entry_id in UNIVERSAL_MONSTER_RULES:
        attach(umr, extract(book, umr, entry_id))
    sort_children(umr, lambda child: (page_key(corpus, child), title_key(child)))

    for entry_id in ANIMAL_COMPANIONS:
        attach(companions, extract(book, companions, entry_id))
    sort_children(
        companions, lambda child: (page_key(corpus, child), title_key(child))
    )

    for entry_id in SUBTYPES:
        attach(subtypes, extract(book, subtypes, entry_id))
    sort_children(subtypes, title_key)

    # 7. Put Monsters A to Z in printed page order.  A stable sort keeps the
    # dataset order of nodes that share a printed page.
    sort_children(monsters, lambda child: page_key(corpus, child))

    if residual is not None:
        if residual.get("children"):
            raise SystemExit(
                "unexpected residual entries: "
                + ", ".join(child.get("entry", child["title"]) for child in residual["children"])
            )
        book["chapters"].remove(residual)

    # Only the printed chapters and appendices that hold entries remain.
    book["chapters"] = [monsters, umr, companions, by_type]

    after: dict[str, str] = {}

    def walk_after(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            after[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_after(child, here)

    for top in book["chapters"]:
        walk_after(top, [])

    if set(before) != set(after):
        print(f"BEFORE COUNT {len(before)} AFTER COUNT {len(after)}", file=sys.stderr)
        lost = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        print(f"LOST {lost}", file=sys.stderr)
        print(f"ADDED {added}", file=sys.stderr)
        raise SystemExit("entry set changed")

    moves = []
    for entry_id in sorted(before):
        page = page_of(corpus, entry_id)
        title = after[entry_id].split(" > ")[-1]
        if before[entry_id] != after[entry_id]:
            moves.append((title, before[entry_id], after[entry_id], page, "move"))
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
    moves = transform(book, args.corpus)
    text = dump_books(books)

    for title, old, new, page, kind in moves:
        print(f"{kind}\t{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print("books.toml is not in the Bestiary 3 nested-placement shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {len(moves)} Bestiary 3 moves", file=sys.stderr)
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} Bestiary 3 moves)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
