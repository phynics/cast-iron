#!/usr/bin/env python3
"""Place the Bestiary tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/bestiary-nested.py /path/to/cast-iron-data
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

BOOK = "bestiary"
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


def chapter(book: dict, title: str) -> dict:
    for node in book["chapters"]:
        if node["title"] == title:
            return node
    raise SystemExit(f"missing chapter {title}")


def section(book: dict, path: list[str]) -> dict:
    """Find a section by title path, allowing entry-bearing nodes."""
    nodes = book["chapters"]
    found = None
    for title in path:
        found = next((node for node in nodes if node["title"] == title), None)
        if found is None:
            raise SystemExit(f"missing section {' > '.join(path)}")
        nodes = found.get("children", [])
    return found


def sort_children(parent: dict, key) -> None:
    parent["children"] = sorted(parent.get("children", []), key=key)


def title_key(node: dict):
    return tuple(re.findall(r"[a-z0-9]+", node["title"].lower()))


# ---------------------------------------------------------------------------
# Page evidence


BESTIARY_PAGE = re.compile(r"Pathfinder RPG Bestiary(?: pg\.?| p\.) ?(\d+)")

# Entries whose source line names no page, or names the wrong page. Each value
# is the printed page of the entry in the Bestiary.
PAGE_OVERRIDES: dict[str, int] = {
    "ref/monster/mummy": 210,
    "ref/monster/sphinx": 257,
    "ref/carried/bestiary/monsters/sphinx": 257,
    "ref/monster/night-hag": 215,
    "ref/carried/bestiary/monsters/night-hag": 215,
    "ref/monster/ghost": 144,
    "ref/monster/half-celestial": 169,
    "ref/monster/half-dragon/dracolisk": 170,
    "ref/monster/half-fiend": 171,
    "ref/monster/lich": 188,
    "ref/monster/hell-hound/nessian": 173,
    "ref/carried/bestiary/monsters/hell-hound/nessian": 173,
    "ref/monster/horse/pony": 177,
    "ref/carried/bestiary/monsters/horse/pony": 177,
    "ref/monster/skeleton/skeleton/human-skeleton": 250,
    "ref/monster/skeletal-champion": 252,
    "ref/monster/vampire": 270,
    "ref/monster/zombie/zombie/human-zombie": 288,
    "ref/template/ghost": 144,
    "ref/template/half-celestial": 169,
    "ref/template/half-dragon": 170,
    "ref/template/half-fiend": 171,
    "ref/template/lich": 188,
    "ref/template/lycanthrope": 197,
    "ref/template/skeleton": 250,
    "ref/template/skeletal-champion": 252,
    "ref/template/vampire": 270,
    "ref/template/zombie": 288,
    "ref/family/shield-guardian": 158,
}

# Dragon stat blocks and their age variants carry no own source page.
DRAGON_COLOR_PAGE = {
    "black": 92,
    "blue": 94,
    "green": 96,
    "red": 98,
    "white": 100,
    "brass": 102,
    "bronze": 104,
    "copper": 106,
    "gold": 108,
    "silver": 110,
}


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    match = re.search(r"dragon/(chromatic|metallic)/([a-z]+)/", entry_id)
    if match:
        return DRAGON_COLOR_PAGE[match.group(2)]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    found = BESTIARY_PAGE.search(path.read_text(encoding="utf-8"))
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


# Family headings that the Bestiary prints as their own section.  The family
# entry becomes the heading; the listed monsters sit under it.
FAMILY_GROUPS: dict[str, list[str]] = {
    "ref/family/angel": [
        "ref/monster/angel-astral-deva",
        "ref/monster/angel-planetar",
        "ref/monster/angel-solar",
    ],
    "ref/family/archon": [
        "ref/monster/archon-hound-archon",
        "ref/monster/archon-lantern-archon",
        "ref/monster/archon-trumpet-archon",
    ],
    "ref/family/azata": [
        "ref/monster/azata-bralani",
        "ref/monster/azata-ghaele",
        "ref/monster/azata-lillend",
    ],
    "ref/family/demon": [
        "ref/monster/demon-babau",
        "ref/monster/demon-balor",
        "ref/monster/demon-dretch",
        "ref/monster/demon-glabrezu",
        "ref/monster/demon-hezrou",
        "ref/monster/demon-marilith",
        "ref/monster/demon-nabasu",
        "ref/monster/demon-nalfeshnee",
        "ref/monster/demon-quasit",
        "ref/monster/demon-shadow-demon",
        "ref/monster/demon-succubus",
        "ref/monster/demon-vrock",
    ],
    "ref/family/devil": [
        "ref/monster/devil-barbed-devil-hamatula",
        "ref/monster/devil-bearded-devil-barbazu",
        "ref/monster/devil-bone-devil-osyluth",
        "ref/monster/devil-erinyes",
        "ref/monster/devil-horned-devil-cornugon",
        "ref/monster/devil-ice-devil-gelugon",
        "ref/monster/devil-imp",
        "ref/monster/devil-lemure",
        "ref/monster/devil-pit-fiend",
    ],
    "ref/family/familiar": [
        "ref/monster/familiar-bat",
        "ref/monster/familiar-cat",
        "ref/monster/familiar-hawk",
        "ref/monster/familiar-lizard",
        "ref/monster/familiar-monkey",
        "ref/monster/familiar-owl",
        "ref/monster/familiar-rat",
        "ref/monster/familiar-raven",
        "ref/monster/familiar-toad",
        "ref/monster/familiar-viper",
        "ref/monster/familiar-weasel",
    ],
}

# Dragons: the true-dragon heading holds one heading per colour, and each
# colour holds its stat block and the printed age variants.
DRAGON_GROUP = "ref/family/dragon-true"
DRAGON_COLOURS = [
    ("ref/family/dragon-black", "black", "chromatic"),
    ("ref/family/dragon-blue", "blue", "chromatic"),
    ("ref/family/dragon-green", "green", "chromatic"),
    ("ref/family/dragon-red", "red", "chromatic"),
    ("ref/family/dragon-white", "white", "chromatic"),
    ("ref/family/dragon-brass", "brass", "metallic"),
    ("ref/family/dragon-bronze", "bronze", "metallic"),
    ("ref/family/dragon-copper", "copper", "metallic"),
    ("ref/family/dragon-gold", "gold", "metallic"),
    ("ref/family/dragon-silver", "silver", "metallic"),
]

ELEMENTALS = {
    "air": "ref/monster/air-elemental",
    "earth": "ref/monster/earth-elemental",
    "fire": "ref/monster/fire-elemental",
    "water": "ref/monster/water-elemental",
}

GOLEMS = [
    "ref/monster/golem-clay-golem",
    "ref/monster/golem-flesh-golem",
    "ref/monster/golem-ice-golem",
    "ref/monster/golem-iron-golem",
    "ref/monster/golem-stone-golem",
    "ref/monster/golem-wood-golem",
]

GOLEM_CONSTRUCTION = {
    "ref/monster/golem-clay-golem": "ref/carried/bestiary/monsters/golem/clay/construction/clay-golem",
    "ref/monster/golem-flesh-golem": "ref/carried/bestiary/monsters/golem/flesh/construction/flesh-golem",
    "ref/monster/golem-ice-golem": "ref/carried/bestiary/monsters/golem/ice/construction/ice-golem",
    "ref/monster/golem-iron-golem": "ref/carried/bestiary/monsters/golem/iron/construction/iron-golem",
    "ref/monster/golem-stone-golem": "ref/carried/bestiary/monsters/golem/stone/construction/stone-golem",
    "ref/monster/golem-wood-golem": "ref/carried/bestiary/monsters/golem/wood/construction/wood-golem",
}

LYCANTHROPES = [
    "ref/carried/bestiary/monsters/lycanthrope/wererat/wererat-human-form",
    "ref/carried/bestiary/monsters/lycanthrope/wererat/wererat-hybrid-form",
    "ref/carried/bestiary/monsters/lycanthrope/werewolf/werewolf",
    "ref/carried/bestiary/monsters/lycanthrope/werewolf/werewolf-hybrid-form",
]

# Template headings.  The printed Bestiary gives each template a heading and
# one or more example creatures; the examples sit under the heading.
TEMPLATE_GROUPS: dict[str, list[str]] = {
    "ref/template/ghost": ["ref/carried/bestiary/monsters/ghost"],
    "ref/template/half-celestial": ["ref/carried/bestiary/monsters/half-celestial"],
    "ref/template/half-dragon": ["ref/carried/bestiary/monsters/half-dragon/dracolisk"],
    "ref/template/half-fiend": ["ref/carried/bestiary/monsters/half-fiend"],
    "ref/template/lich": ["ref/carried/bestiary/monsters/lich"],
    "ref/template/skeleton": ["ref/carried/bestiary/monsters/skeleton/skeleton/human-skeleton"],
    "ref/template/skeletal-champion": ["ref/carried/bestiary/monsters/skeletal-champion"],
    "ref/template/vampire": ["ref/carried/bestiary/monsters/vampire"],
    "ref/template/zombie": [
        "ref/carried/bestiary/monsters/zombie/zombie/human-zombie",
        "ref/affliction/zombie/zombie-rot",
    ],
}

SIMPLE_TEMPLATE_IDS = [
    "ref/template/advanced",
    "ref/template/celestial",
    "ref/template/fiendish",
    "ref/template/giant",
    "ref/template/young",
]

TYPE_IDS = [
    "ref/type/aberration",
    "ref/type/animal",
    "ref/type/construct",
    "ref/type/dragon",
    "ref/type/fey",
    "ref/type/humanoid",
    "ref/type/magical-beast",
    "ref/type/monstrous-humanoid",
    "ref/type/ooze",
    "ref/type/outsider",
    "ref/type/plant",
    "ref/type/undead",
    "ref/type/vermin",
]

SUBTYPE_IDS = [
    "ref/subtype/air",
    "ref/subtype/angel",
    "ref/subtype/aquatic",
    "ref/subtype/archon",
    "ref/subtype/augmented",
    "ref/subtype/azata",
    "ref/subtype/chaotic",
    "ref/subtype/cold",
    "ref/subtype/demon",
    "ref/subtype/devil",
    "ref/subtype/dwarf",
    "ref/subtype/earth",
    "ref/subtype/elemental",
    "ref/subtype/elf",
    "ref/subtype/evil",
    "ref/subtype/extraplanar",
    "ref/subtype/fire",
    "ref/subtype/giant",
    "ref/subtype/gnome",
    "ref/subtype/goblinoid",
    "ref/subtype/good",
    "ref/subtype/halfling",
    "ref/subtype/human",
    "ref/subtype/lawful",
    "ref/subtype/native",
    "ref/subtype/orc",
    "ref/subtype/reptilian",
    "ref/subtype/shapechanger",
    "ref/subtype/swarm",
    "ref/subtype/water",
]

MONSTER_CREATION_STEPS = [
    "ref/rule/step-1-concept",
    "ref/rule/step-2-target-statistics",
    "ref/rule/step-3-hit-dice",
    "ref/rule/step-4-size",
    "ref/rule/step-5-abilities",
    "ref/rule/step-6-skills-and-feats",
    "ref/rule/step-7-other-statistics",
    "ref/rule/step-8-special-abilities-and-qualities",
    "ref/rule/step-9-treasure",
    "ref/rule/step-10-details",
]

ADDING_CLASS_LEVELS_STEPS = [
    "ref/rule/step-1-determine-creatures-role",
    "ref/rule/step-2-add-class-levels",
    "ref/rule/step-3-determine-cr",
]

# Animal-companion stat blocks print inside the monster's entry.
COMPANION_TARGETS: dict[str, str] = {
    "ref/companion/ankylosaurus": "ref/monster/dinosaur-ankylosaurus",
    "ref/companion/aurochs": "ref/monster/herd-animal-aurochs",
    "ref/companion/brachiosaurus": "ref/monster/dinosaur-brachiosaurus",
    "ref/companion/dire-bat": "ref/monster/bat-dire-bat",
    "ref/companion/dire-rat": "ref/monster/rat-dire-rat",
    "ref/companion/dolphin": "ref/monster/dolphin",
    "ref/companion/elasmosaurus": "ref/monster/dinosaur-elasmosaurus",
    "ref/companion/electric-eel": "ref/monster/eel-electric-eel",
    "ref/companion/elephant-mastodon": "ref/monster/elephant",
    "ref/companion/frog": "ref/monster/frog-giant-frog",
    "ref/companion/goblin-dog": "ref/monster/goblin-dog",
    "ref/companion/hyena": "ref/monster/hyena",
    "ref/companion/lizard-monitor-lizard": "ref/monster/lizard-monitor-lizard",
    "ref/companion/moray-eel": "ref/monster/eel-giant-moray-eel",
    "ref/companion/octopus": "ref/monster/octopus",
    "ref/companion/orca": "ref/monster/dolphin-orca",
    "ref/companion/pteranodon": "ref/monster/dinosaur-pteranodon",
    "ref/companion/rhinoceros": "ref/monster/rhinoceros",
    "ref/companion/roc": "ref/monster/roc",
    "ref/companion/squid": "ref/monster/squid",
    "ref/companion/stegosaurus": "ref/monster/dinosaur-stegosaurus",
    "ref/companion/triceratops": "ref/monster/dinosaur-triceratops",
    "ref/companion/tyrannosaurus": "ref/monster/dinosaur-tyrannosaurus",
}

# Dataset splits of one printed monster entry.
VARIANT_TARGETS: dict[str, str] = {
    "ref/monster/sphinx-gynosphinx": "ref/carried/bestiary/monsters/sphinx",
    "ref/monster/herd-animal-oxen": "ref/monster/herd-animal-aurochs",
    "ref/monster/yak": "ref/monster/herd-animal-aurochs",
    "ref/monster/horse-pony": "ref/monster/horse",
    "ref/monster/moldering-emperor": "ref/monster/neothelid",
    "ref/carried/bestiary/monsters/horse/pony": "ref/monster/horse",
}

# Ability and affliction splits print inside the parent monster's entry.
ABILITY_TARGETS: dict[str, str] = {
    "ref/affliction/hag-sea-hag/evil-eye-curse": "ref/monster/hag-sea-hag",
    "ref/affliction/kraken/kraken-ink": "ref/monster/kraken",
    "ref/affliction/ant-giant-ant/poison-ex": "ref/monster/ant-giant-ant",
    "ref/affliction/lizard-monitor-lizard/poison-ex": "ref/monster/lizard-monitor-lizard",
    "ref/affliction/shadow/shadow-blight": "ref/monster/shadow",
    "ref/disease/ghoul-fever": "ref/monster/ghoul",
    "ref/disease/rabies": "ref/monster/dog",
    "ref/eq-poison/black-smear": "ref/monster/dark-folk-dark-stalker",
    "ref/umr/ink-cloud": "ref/monster/octopus",
    "ref/umr/trample": "ref/monster/herd-animal-aurochs",
    "ref/rule/monster-sahuagin/sahuagin": "ref/monster/sahuagin",
}


def move(book: dict, entry_id: str, target: dict) -> None:
    node = find_node(book, entry_id)
    if node is None:
        raise SystemExit(f"missing entry {entry_id}")
    if parent_of(book, node) is target:
        return
    attach(target, detach(book, entry_id))


def reorder_entries(parent: dict, order: list[str]) -> None:
    index = {entry_id: position for position, entry_id in enumerate(order)}
    head = [child for child in parent.get("children", []) if child.get("entry") in index]
    rest = [child for child in parent.get("children", []) if child.get("entry") not in index]
    head.sort(key=lambda child: index[child["entry"]])
    parent["children"] = head + rest


def top_level_monsters(book: dict) -> dict:
    return chapter(book, "Monsters A to Z")


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None, str]]:
    before: dict[str, str] = {}
    before_seq: list[str] = []

    def walk_paths(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            before[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_paths(child, here)

    for top in book["chapters"]:
        walk_paths(top, [])

    monsters = top_level_monsters(book)
    before_seq = [child["entry"] for child in monsters.get("children", []) if child.get("entry")]

    # 1. Rules that print in the introduction and the appendices.
    intro = chapter(book, "Introduction")
    move(book, "ref/rule/monster-rules", intro)
    move(book, "ref/rule/stat-block", intro)

    pcs = chapter(book, "Monsters as PCs")
    node = find_node(book, "ref/rule/monsters-as-pcs")
    if node is None:
        raise SystemExit("missing Monsters as PCs rule entry")
    if parent_of(book, node) is not pcs:
        pcs.setdefault("children", []).insert(0, detach(book, "ref/rule/monsters-as-pcs"))

    feats = chapter(book, "Monster Feats")
    move(book, "ref/misc/damage-die-adjustments", feats)

    # 2. Dataset splits move under the entry that prints them.
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

    for entry_id, parent_id in COMPANION_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing companion parent {parent_id}")
        move(book, entry_id, parent)

    # 3. Family headings.
    for family_id, member_ids in FAMILY_GROUPS.items():
        group = detach(book, family_id)
        for member_id in member_ids:
            attach(group, extract(book, group, member_id))
        sort_children(group, lambda child: page_key(corpus, child))
        attach(monsters, group)

    # Dragons: true dragon -> colour -> stat block + age variants.
    dragon = detach(book, DRAGON_GROUP)
    for colour_id, colour, kind in DRAGON_COLOURS:
        colour_node = extract(book, dragon, colour_id)
        base = f"ref/carried/bestiary/monsters/dragon/{kind}/{colour}/dragon/{kind}/{colour}"
        attach(colour_node, extract(book, colour_node, base))
        for age in ("young", "adult", "ancient"):
            attach(
                colour_node,
                extract(book, colour_node, f"ref/carried/bestiary/monsters/dragon/{kind}/{colour}/{age}"),
            )
        attach(dragon, colour_node)
    attach(monsters, dragon)

    # Elementals: heading -> base elemental -> size variants.
    elemental = detach(book, "ref/family/elemental")
    for kind, base_id in ELEMENTALS.items():
        base = extract(book, elemental, base_id)
        for size in ("small", "medium", "large", "huge", "greater", "elder"):
            attach(base, extract(book, base, f"ref/carried/bestiary/monsters/elemental/{kind}/{size}"))
        attach(elemental, base)
    sort_children(elemental, lambda child: page_key(corpus, child))
    attach(monsters, elemental)

    # Golems: heading, the Shield Guardian subsection, then the golems.
    golem = detach(book, "ref/family/golem")
    attach(golem, extract(book, golem, "ref/family/shield-guardian"))
    for golem_id in GOLEMS:
        node = extract(book, golem, golem_id)
        construction_id = GOLEM_CONSTRUCTION[golem_id]
        construction = next(
            (child for child in node.get("children", []) if child["title"] == "Construction" and not child.get("entry")),
            None,
        )
        if construction is None:
            construction = {"title": "Construction", "children": []}
            attach(node, construction)
        attach(construction, extract(book, construction, construction_id))
        attach(golem, node)
    sort_children(golem, lambda child: page_key(corpus, child))
    attach(monsters, golem)

    # Lycanthropes: family heading -> template -> example creatures.
    lycanthrope = detach(book, "ref/family/lycanthrope")
    attach(lycanthrope, extract(book, lycanthrope, "ref/template/lycanthrope"))
    for example_id in LYCANTHROPES:
        attach(lycanthrope, extract(book, lycanthrope, example_id))
    attach(monsters, lycanthrope)

    # 4. Template headings with their example creatures.
    for template_id, example_ids in TEMPLATE_GROUPS.items():
        group = detach(book, template_id)
        for example_id in example_ids:
            attach(group, extract(book, group, example_id))
        attach(monsters, group)

    # 5. Simple templates in Appendix 2.
    simple = section(book, ["Monster Advancement", "Simple Templates"])
    for template_id in SIMPLE_TEMPLATE_IDS:
        attach(simple, extract(book, simple, template_id))
    sort_children(simple, title_key)

    # 6. Creature types and subtypes in Monsters by Type.
    types = section(book, ["Monsters by Type", "Creature Types"])
    for type_id in TYPE_IDS:
        attach(types, extract(book, types, type_id))
    sort_children(types, title_key)

    subtypes = section(book, ["Monsters by Type", "Creature SubTypes"])
    for subtype_id in SUBTYPE_IDS:
        attach(subtypes, extract(book, subtypes, subtype_id))
    sort_children(subtypes, title_key)

    # 7. Monster Creation and Adding Class Levels steps.
    building = section(book, ["Monster Creation", "Building the Monster"])
    for step_id in MONSTER_CREATION_STEPS:
        attach(building, extract(book, building, step_id))
    reorder_entries(building, MONSTER_CREATION_STEPS)

    adding = section(book, ["Monster Advancement", "Adding Class Levels"])
    for step_id in ADDING_CLASS_LEVELS_STEPS:
        attach(adding, extract(book, adding, step_id))
    reorder_entries(adding, ADDING_CLASS_LEVELS_STEPS)

    # 8. Put Monsters A to Z in printed page order.  A stable sort keeps the
    # dataset order of nodes that share a printed page.
    sort_children(monsters, lambda child: page_key(corpus, child))

    after: dict[str, str] = {}

    def walk_after(node: dict, trail: list[str]) -> None:
        here = trail + [node["title"]]
        if node.get("entry"):
            after[node["entry"]] = " > ".join(here)
        for child in node.get("children", []):
            walk_after(child, here)

    for top in book["chapters"]:
        walk_after(top, [])

    # 9. The residual chapter may only keep entries with no printed home.
    residual = chapter(book, "New entries (page order)")
    for child in residual.get("children", []):
        if child.get("entry") not in {
            "ref/carried/bestiary-2/monsters/bat/skaveling/special-abilities/disease/skaveling-ghoul-fever",
            "ref/rule/gamemastery",
            "ref/rule/rare-races",
            "ref/rule/slimes-molds-and-fungi",
        }:
            raise SystemExit(f"unexpected residual entry {child.get('entry')}")

    if set(before) != set(after):
        print(f"BEFORE COUNT {len(before)} AFTER COUNT {len(after)}", file=sys.stderr)
        lost = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        print(f"LOST {lost}", file=sys.stderr)
        print(f"ADDED {added}", file=sys.stderr)
        raise SystemExit("entry set changed")

    after_seq = [child["entry"] for child in monsters.get("children", []) if child.get("entry")]

    # A top-level node is reordered when it swaps order with another node that
    # stayed in the chapter.
    common = set(before_seq) & set(after_seq)
    before_rank = {entry_id: index for index, entry_id in enumerate(before_seq) if entry_id in common}
    after_rank = {entry_id: index for index, entry_id in enumerate(after_seq) if entry_id in common}
    reordered: set[str] = set()
    kept = [entry_id for entry_id in before_seq if entry_id in common]
    for i, left in enumerate(kept):
        for right in kept[i + 1 :]:
            if (before_rank[left] < before_rank[right]) != (after_rank[left] < after_rank[right]):
                reordered.add(left)
                reordered.add(right)

    moves = []
    for entry_id in sorted(before):
        page = page_of(corpus, entry_id)
        title = after[entry_id].split(" > ")[-1]
        if before[entry_id] != after[entry_id]:
            moves.append((title, before[entry_id], after[entry_id], page, "move"))
        elif entry_id in reordered:
            moves.append((title, before[entry_id], after[entry_id], page, "order"))
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
            print("books.toml is not in the Bestiary nested-placement shape", file=sys.stderr)
            return 1
        print(f"books.toml is canonical for {len(moves)} Bestiary moves", file=sys.stderr)
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} Bestiary moves)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
