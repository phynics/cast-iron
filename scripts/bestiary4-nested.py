#!/usr/bin/env python3
"""Place the Bestiary 4 tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/bestiary4-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md. The tree
helpers are shared with ``scripts/bestiary-nested.py``.
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
import tomllib
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location(
    "bestiary_nested", _HERE / "bestiary-nested.py"
)
assert _SPEC is not None and _SPEC.loader is not None
_COMMON = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_COMMON)

attach = _COMMON.attach
chapter = _COMMON.chapter
detach = _COMMON.detach
dump_books = _COMMON.dump_books
extract = _COMMON.extract
find_node = _COMMON.find_node
move = _COMMON.move
parent_of = _COMMON.parent_of
sort_children = _COMMON.sort_children

BOOK = "bestiary-4"
BOOKS_FILE = "books.toml"

# ---------------------------------------------------------------------------
# Page evidence

BESTIARY4_PAGE = re.compile(r"Bestiary 4\s*(?:pg\.?|p\.?)\s*(\d+)")
BESTIARY4_SLASH_PAGE = re.compile(r"Bestiary 4/(\d+)")

# Printed heading pages that do not appear on the source line, split stat
# blocks that carry no source line, and cross-book stat blocks that print in
# Bestiary 4.
PAGE_OVERRIDES: dict[str, int] = {
    # Printed section intros inside Monsters A to Z.
    "ref/family/beheaded": 17,
    "ref/family/colossus": 35,
    "ref/family/demon-lord": 45,
    "ref/family/dragon-outer": 64,
    "ref/family/dragon-outer-lunar": 66,
    "ref/family/dragon-outer-solar": 68,
    "ref/family/dragon-outer-time": 70,
    "ref/family/dragon-outer-void": 72,
    "ref/family/dragon-outer-vortex": 74,
    "ref/family/empyreal-lord": 87,
    "ref/family/fleshwarp": 101,
    "ref/family/formian": 108,
    "ref/family/great-old-one": 135,
    "ref/family/kaiju": 165,
    "ref/family/lurking-ray": 186,
    "ref/family/psychopomp": 217,
    # Templates that print inside a monster entry.
    "ref/template/colour-blighted": 39,
    "ref/template/nightmare-creature": 204,
    "ref/template/pod-spawned": 20,
    # Carried stat blocks with no source line.
    "ref/carried/bestiary-4/monsters/barometz/growing-a-barometz/barometz": 16,
    "ref/carried/bestiary-4/monsters/bodythief/pod-spawned-creature-cr-0/1/or-2/pod-spawned-guard-captain": 21,
    "ref/carried/bestiary-4/monsters/broken-soul": 24,
    "ref/carried/bestiary-4/monsters/cephalophore/construction/cephalophore": 27,
    "ref/carried/bestiary-4/monsters/clockwork-dragon/clockwork-dragon/construction/clockwork-dragon": 31,
    "ref/carried/bestiary-4/monsters/clockwork-dragon/clockwork-mage/construction/clockwork-mage": 32,
    "ref/carried/bestiary-4/monsters/clockwork-dragon/clockwork-steed/construction/clockwork-steed": 33,
    "ref/carried/bestiary-4/monsters/colossus/flesh/construction/flesh-colossus": 35,
    "ref/carried/bestiary-4/monsters/colossus/iron/construction/iron-colossus": 36,
    "ref/carried/bestiary-4/monsters/colossus/stone/construction/stone-colossus": 37,
    "ref/carried/bestiary-4/monsters/devilbound-creature": 56,
    "ref/carried/bestiary-4/monsters/devil/host": 53,
    "ref/carried/bestiary-4/monsters/divine-guardian": 60,
    "ref/carried/bestiary-4/monsters/dragon/outer/lunar/dragon/outer/lunar": 66,
    "ref/carried/bestiary-4/monsters/dragon/outer/lunar/young": 66,
    "ref/carried/bestiary-4/monsters/dragon/outer/lunar/adult": 67,
    "ref/carried/bestiary-4/monsters/dragon/outer/lunar/ancient": 67,
    "ref/carried/bestiary-4/monsters/dragon/outer/solar/dragon/outer/solar": 68,
    "ref/carried/bestiary-4/monsters/dragon/outer/solar/young": 68,
    "ref/carried/bestiary-4/monsters/dragon/outer/solar/adult": 69,
    "ref/carried/bestiary-4/monsters/dragon/outer/solar/ancient": 69,
    "ref/carried/bestiary-4/monsters/dragon/outer/time/dragon/outer/time": 70,
    "ref/carried/bestiary-4/monsters/dragon/outer/time/young": 70,
    "ref/carried/bestiary-4/monsters/dragon/outer/time/adult": 71,
    "ref/carried/bestiary-4/monsters/dragon/outer/time/ancient": 71,
    "ref/carried/bestiary-4/monsters/dragon/outer/void/dragon/outer/void": 72,
    "ref/carried/bestiary-4/monsters/dragon/outer/void/young": 72,
    "ref/carried/bestiary-4/monsters/dragon/outer/void/adult": 73,
    "ref/carried/bestiary-4/monsters/dragon/outer/void/ancient": 73,
    "ref/carried/bestiary-4/monsters/dragon/outer/vortex/dragon/outer/vortex": 74,
    "ref/carried/bestiary-4/monsters/dragon/outer/vortex/young": 74,
    "ref/carried/bestiary-4/monsters/dragon/outer/vortex/adult": 75,
    "ref/carried/bestiary-4/monsters/dragon/outer/vortex/ancient": 75,
    "ref/carried/bestiary-4/monsters/ectoplasmic-creature": 82,
    "ref/carried/bestiary-4/monsters/fungal-creature": 116,
    "ref/carried/bestiary-4/monsters/juggernaut/construction/juggernaut": 162,
    "ref/carried/bestiary-4/monsters/living-wall/construction/living-wall": 182,
    "ref/carried/bestiary-4/monsters/lycanthrope/werebat/werebat-human-form": 188,
    "ref/carried/bestiary-4/monsters/lycanthrope/werebat/werebat-hybrid-form": 188,
    "ref/carried/bestiary-4/monsters/lycanthrope/werecrocodile/werecrocodile-human-form": 189,
    "ref/carried/bestiary-4/monsters/lycanthrope/werecrocodile/werecrocodile-hybrid-form": 189,
    "ref/carried/bestiary-4/monsters/lycanthrope/wereshark/wereshark-human-form": 190,
    "ref/carried/bestiary-4/monsters/lycanthrope/wereshark/wereshark-hybrid-form": 190,
    "ref/carried/bestiary-4/monsters/mummified-creature": 196,
    "ref/carried/bestiary-4/monsters/nightmare-creature": 204,
    "ref/carried/bestiary-4/monsters/shadow-creature": 238,
    "ref/carried/bestiary-4/monsters/skinstitch/construction/skinstitch": 246,
    "ref/carried/bestiary-4/monsters/soulbound-shell/construction/soulbound-shell": 249,
    "ref/carried/bestiary-4/monsters/spider/scarlet": 252,
    "ref/carried/bestiary-4/monsters/tiberolith/construction/tiberolith": 259,
    "ref/carried/bestiary-4/monsters/vampire/nosferatu": 268,
    "ref/carried/bestiary-4/monsters/wickerman/construction/wickerman": 277,
    "ref/carried/bestiary-4/monsters/zombie-lord": 286,
    "ref/carried/ultimate-magic/monsters/new-familiars/small-and-vermin-familiars/familiar/scorpion/greensting": 232,
    # Duplicate stat blocks from the second dataset.  They repeat the carried
    # age variant and take its printed page.
    "ref/monster/dragon-outer-solar-ancient-solar-dragon": 69,
    "ref/monster/dragon-outer-time-adult-time-dragon": 71,
    "ref/monster/dragon-outer-time-ancient-time-dragon": 71,
    "ref/monster/dragon-outer-void-adult-void-dragon": 73,
    "ref/monster/dragon-outer-void-ancient-void-dragon": 73,
    "ref/monster/dragon-outer-vortex-adult-vortex-dragon": 75,
    "ref/monster/dragon-outer-vortex-ancient-vortex-dragon": 75,
}


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id in PAGE_OVERRIDES:
        return PAGE_OVERRIDES[entry_id]
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    text = path.read_text(encoding="utf-8")
    found = BESTIARY4_PAGE.search(text) or BESTIARY4_SLASH_PAGE.search(text)
    return int(found.group(1)) if found else None


def node_page(corpus: Path, node: dict) -> int | None:
    page = page_of(corpus, node.get("entry"))
    if page is not None:
        return page
    pages = [page_of(corpus, child.get("entry")) for child in node.get("children", [])]
    known = [value for value in pages if value is not None]
    return min(known) if known else None


def node_order(node: dict, order: dict[str, int]) -> int:
    values = [
        order.get(child.get("entry", ""), 10**9) for child in node.get("children", [])
    ]
    own = order.get(node.get("entry") or "", 10**9)
    return min([own] + values)


def page_key(corpus: Path, order: dict[str, int], node: dict):
    page = node_page(corpus, node)
    heading = 0 if node.get("entry") in FAMILY_GROUPS or not node.get("entry") else 1
    return (
        page if page is not None else 10**9,
        heading,
        node_order(node, order),
    )


# ---------------------------------------------------------------------------
# The transform


# Printed family intros inside Monsters A to Z.  The family entry becomes the
# heading; the listed monsters sit under it in printed page order.
FAMILY_GROUPS: dict[str, list[str]] = {
    "ref/family/beheaded": ["ref/monster/beheaded"],
    "ref/family/colossus": [
        "ref/monster/colossus-flesh-colossus-mythic",
        "ref/monster/colossus-iron-colossus-mythic",
        "ref/monster/colossus-stone-colossus-mythic",
    ],
    "ref/family/demon-lord": [
        "ref/monster/demon-lord-dagon",
        "ref/monster/demon-lord-kostchtchie",
        "ref/monster/demon-lord-pazuzu",
    ],
    "ref/family/empyreal-lord": [
        "ref/monster/empyreal-lord-cernunnos",
        "ref/monster/empyreal-lord-korada",
        "ref/monster/empyreal-lord-vildeis",
    ],
    "ref/family/fleshwarp": [
        "ref/monster/fleshwarp-ghonhatine",
        "ref/monster/fleshwarp-grothlut",
        "ref/monster/fleshwarp-halsora",
        "ref/monster/fleshwarp-irnakurse",
    ],
    "ref/family/formian": [
        "ref/monster/formian-formian-myrmarch",
        "ref/monster/formian-formian-queen",
        "ref/monster/formian-formian-taskmaster",
        "ref/monster/formian-formian-warrior",
        "ref/monster/formian-formian-worker",
    ],
    "ref/family/great-old-one": [
        "ref/monster/great-old-one-bokrug",
        "ref/monster/great-old-one-cthulhu",
        "ref/monster/great-old-one-hastur",
    ],
    "ref/family/kaiju": [
        "ref/monster/kaiju-agyra",
        "ref/monster/kaiju-bezravnis",
        "ref/monster/kaiju-mogaru",
    ],
    "ref/family/lurking-ray": [
        "ref/monster/lurking-ray-executioners-hood",
        "ref/monster/lurking-ray-lurker-above",
        "ref/monster/lurking-ray-trapper",
    ],
    "ref/family/psychopomp": [
        "ref/monster/psychopomp-catrina",
        "ref/monster/psychopomp-morrigna",
        "ref/monster/psychopomp-nosoi",
        "ref/monster/psychopomp-vanth",
        "ref/monster/psychopomp-yamaraj",
    ],
}

# Outer dragons: the family heading holds one heading per printed dragon, and
# each dragon holds its base stat block and the printed age variants.
DRAGON_GROUP = "ref/family/dragon-outer"
DRAGON_COLOURS = [
    ("ref/family/dragon-outer-lunar", "lunar"),
    ("ref/family/dragon-outer-solar", "solar"),
    ("ref/family/dragon-outer-time", "time"),
    ("ref/family/dragon-outer-void", "void"),
    ("ref/family/dragon-outer-vortex", "vortex"),
]

DRAGON_DUPLICATES = {
    "ref/monster/dragon-outer-solar-ancient-solar-dragon": "solar/ancient",
    "ref/monster/dragon-outer-time-adult-time-dragon": "time/adult",
    "ref/monster/dragon-outer-time-ancient-time-dragon": "time/ancient",
    "ref/monster/dragon-outer-void-adult-void-dragon": "void/adult",
    "ref/monster/dragon-outer-void-ancient-void-dragon": "void/ancient",
    "ref/monster/dragon-outer-vortex-adult-vortex-dragon": "vortex/adult",
    "ref/monster/dragon-outer-vortex-ancient-vortex-dragon": "vortex/ancient",
}

# Template rules print inside the creature entry that heads the section.
TEMPLATE_EXAMPLES: dict[str, str] = {
    "ref/template/broken-soul": "ref/carried/bestiary-4/monsters/broken-soul",
    "ref/template/colour-blighted": "ref/monster/colour-out-of-space",
    "ref/template/devilbound": "ref/carried/bestiary-4/monsters/devilbound-creature",
    "ref/template/divine-guardian": "ref/carried/bestiary-4/monsters/divine-guardian",
    "ref/template/ectoplasmic": "ref/carried/bestiary-4/monsters/ectoplasmic-creature",
    "ref/template/fungal-creature": "ref/carried/bestiary-4/monsters/fungal-creature",
    "ref/template/mummified-creature": "ref/carried/bestiary-4/monsters/mummified-creature",
    "ref/template/nightmare-creature": "ref/carried/bestiary-4/monsters/nightmare-creature",
    "ref/template/shadow": "ref/carried/bestiary-4/monsters/shadow-creature",
    "ref/template/shadow-lord": "ref/carried/bestiary-4/monsters/shadow-creature",
    "ref/template/nosferatu": "ref/carried/bestiary-4/monsters/vampire/nosferatu",
    "ref/template/zombie-lord": "ref/carried/bestiary-4/monsters/zombie-lord",
}

# Construct stat blocks that print their Construction rules inside the entry.
CONSTRUCTIONS: dict[str, str] = {
    "ref/monster/clockwork-clockwork-dragon": "ref/carried/bestiary-4/monsters/clockwork-dragon/clockwork-dragon/construction/clockwork-dragon",
    "ref/monster/clockwork-clockwork-mage": "ref/carried/bestiary-4/monsters/clockwork-dragon/clockwork-mage/construction/clockwork-mage",
    "ref/monster/clockwork-clockwork-steed": "ref/carried/bestiary-4/monsters/clockwork-dragon/clockwork-steed/construction/clockwork-steed",
    "ref/monster/colossus-flesh-colossus-mythic": "ref/carried/bestiary-4/monsters/colossus/flesh/construction/flesh-colossus",
    "ref/monster/colossus-iron-colossus-mythic": "ref/carried/bestiary-4/monsters/colossus/iron/construction/iron-colossus",
    "ref/monster/colossus-stone-colossus-mythic": "ref/carried/bestiary-4/monsters/colossus/stone/construction/stone-colossus",
}

# Dataset splits and duplicate stat blocks that print inside one entry.
SPLITS: dict[str, str] = {
    "ref/monster/devil-host-devil-greater-magaav": "ref/carried/bestiary-4/monsters/devil/host",
    "ref/monster/rukh-juvenile-rukh": "ref/monster/rukh",
    "ref/monster/seps-juvenile-seps": "ref/monster/seps",
    "ref/monster/shadow-creature-shadow-roper": "ref/carried/bestiary-4/monsters/shadow-creature",
}

# A disease that prints inside the Junk Golem entry and has no parent field.
DISEASE_TARGETS: dict[str, str] = {
    "ref/disease/tetanus": "ref/monster/golem-junk-golem",
}

# Artifacts that print inside a named monster entry.
ITEM_TARGETS: dict[str, str] = {
    "ref/magic-artifact/cicatrix": "ref/monster/empyreal-lord-vildeis",
    "ref/magic-artifact/scepter-of-shibaxet": "ref/monster/demon-lord-pazuzu",
}

# Printed headings that the dataset splits into stat blocks with no heading
# entry of their own.  The heading is synthesized, like the Bestiary 2
# Merrow and lycanthrope headings.
SYNTHESIZED_GROUPS: list[tuple[str, list[str]]] = [
    (
        "Familiars",
        [
            "ref/monster/familiar-archaeopteryx",
            "ref/monster/familiar-dodo",
            "ref/monster/familiar-platypus",
            "ref/monster/familiar-sloth",
            "ref/monster/familiar-tuatara",
        ],
    ),
    (
        "Lycanthrope, Werebat",
        [
            "ref/carried/bestiary-4/monsters/lycanthrope/werebat/werebat-human-form",
            "ref/carried/bestiary-4/monsters/lycanthrope/werebat/werebat-hybrid-form",
        ],
    ),
    (
        "Lycanthrope, Werecrocodile",
        [
            "ref/carried/bestiary-4/monsters/lycanthrope/werecrocodile/werecrocodile-human-form",
            "ref/carried/bestiary-4/monsters/lycanthrope/werecrocodile/werecrocodile-hybrid-form",
        ],
    ),
    (
        "Lycanthrope, Wereshark",
        [
            "ref/carried/bestiary-4/monsters/lycanthrope/wereshark/wereshark-human-form",
            "ref/carried/bestiary-4/monsters/lycanthrope/wereshark/wereshark-hybrid-form",
        ],
    ),
    (
        "Phantom Armor",
        [
            "ref/monster/phantom-armor-guardian",
            "ref/monster/phantom-armor-giant",
        ],
    ),
]

# Standalone monsters that the residual collected but print inside Monsters
# A to Z.
RESIDUAL_MONSTERS = [
    "ref/monster/flea-giant-flea",
    "ref/monster/flea-mammoth-flea",
    "ref/monster/golem-coral-golem",
    "ref/monster/golem-junk-golem",
    "ref/monster/golem-wax-golem",
    "ref/monster/seahorse-giant-seahorse",
    "ref/monster/seahorse-killer-seahorse",
    "ref/monster/titan-fomorian-titan-mythic",
]


# Residual entries whose printed home is an appendix that this book tree does
# not carry, plus cross-book hubs.  Everything else must leave the residual.
RESIDUAL_PREFIXES = (
    "ref/companion/",
    "ref/domain/",
    "ref/rule/",
    "ref/subtype/",
    "ref/umr/",
)
RESIDUAL_EXACT = {"ref/template/hellbound-creature"}


def find_book(books: dict) -> dict:
    for book in books["books"]:
        if book["source"] == BOOK:
            return book
    raise SystemExit(f"unknown book source: {BOOK}")


def ensure_group(parent: dict, title: str) -> dict:
    for child in parent.get("children", []):
        if child.get("title") == title and not child.get("entry"):
            return child
    node = {"title": title, "children": []}
    attach(parent, node)
    return node


def ensure_construction(node: dict) -> dict:
    for child in node.get("children", []):
        if child.get("title") == "Construction" and not child.get("entry"):
            return child
    construction = {"title": "Construction", "children": []}
    attach(node, construction)
    return construction


def entry_parent(corpus: Path, entry_id: str) -> str | None:
    path = corpus / "entries" / (entry_id[len("ref/") :] + ".md")
    if not path.exists():
        return None
    text = path.read_text(encoding="utf-8")
    found = re.search(r'^parent = "([^"]+)"', text, re.M)
    return found.group(1) if found else None


def walk_entries(node: dict, trail: list[str], out: dict[str, str]) -> None:
    here = trail + [node["title"]]
    if node.get("entry"):
        out[node["entry"]] = " > ".join(here)
    for child in node.get("children", []):
        walk_entries(child, here, out)


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None, str]]:
    before: dict[str, str] = {}
    order: dict[str, int] = {}
    for top in book["chapters"]:
        walk_entries(top, [], before)

    def walk_order(node: dict) -> None:
        if node.get("entry"):
            order.setdefault(node["entry"], len(order))
        for child in node.get("children", []):
            walk_order(child)

    for top in book["chapters"]:
        walk_order(top)

    monsters = chapter(book, "Monsters A to Z")
    before_seq = [
        child["entry"] for child in monsters.get("children", []) if child.get("entry")
    ]

    # 1. Family intros and their printed monsters.
    for family_id, member_ids in FAMILY_GROUPS.items():
        family = find_node(book, family_id)
        if family is None:
            raise SystemExit(f"missing family {family_id}")
        for member_id in member_ids:
            move(book, member_id, family)
        family = detach(book, family_id)
        sort_children(family, lambda child: page_key(corpus, order, child))
        attach(monsters, family)

    # 2. Outer dragons: family -> printed dragon -> base and age variants.
    dragon = find_node(book, DRAGON_GROUP)
    if dragon is None:
        raise SystemExit(f"missing family {DRAGON_GROUP}")
    for family_id, colour in DRAGON_COLOURS:
        colour_node = find_node(book, family_id)
        if colour_node is None:
            raise SystemExit(f"missing dragon {family_id}")
        base = (
            f"ref/carried/bestiary-4/monsters/dragon/outer/{colour}"
            f"/dragon/outer/{colour}"
        )
        attach(colour_node, extract(book, colour_node, base))
        for age in ("young", "adult", "ancient"):
            attach(
                colour_node,
                extract(
                    book,
                    colour_node,
                    f"ref/carried/bestiary-4/monsters/dragon/outer/{colour}/{age}",
                ),
            )
        sort_children(colour_node, lambda child: page_key(corpus, order, child))
        colour_node = detach(book, family_id)
        attach(dragon, colour_node)
    dragon = detach(book, DRAGON_GROUP)
    sort_children(dragon, lambda child: page_key(corpus, order, child))
    attach(monsters, dragon)

    # 3. Standalone monsters that the residual collected.
    for entry_id in RESIDUAL_MONSTERS:
        move(book, entry_id, monsters)

    # 4. Template rules sit under the creature that heads the printed entry.
    for template_id, creature_id in TEMPLATE_EXAMPLES.items():
        creature = find_node(book, creature_id)
        if creature is None:
            raise SystemExit(f"missing template example {creature_id}")
        move(book, template_id, creature)

    bodythief = find_node(book, "ref/monster/bodythief")
    if bodythief is None:
        raise SystemExit("missing Bodythief")
    pod_spawned = ensure_group(
        bodythief, "Pod-Spawned Creature (CR 0, \u20131, or \u20132)"
    )
    move(book, "ref/template/pod-spawned", pod_spawned)

    # 4. Construction rules sit under their construct.
    for monster_id, construction_id in CONSTRUCTIONS.items():
        monster = find_node(book, monster_id)
        if monster is None:
            raise SystemExit(f"missing construct {monster_id}")
        move(book, construction_id, ensure_construction(monster))

    # 5. Dataset splits and duplicate stat blocks.
    for child_id, parent_id in SPLITS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing split parent {parent_id}")
        move(book, child_id, parent)

    for duplicate_id, age_path in DRAGON_DUPLICATES.items():
        parent_id = (
            f"ref/carried/bestiary-4/monsters/dragon/outer/{age_path}"
        )
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing dragon duplicate parent {parent_id}")
        move(book, duplicate_id, parent)

    # 6. Afflictions and diseases print inside their monster entry.
    for entry_id, path in list(before.items()):
        if not entry_id.startswith(("ref/affliction/", "ref/disease/")):
            continue
        parent_id = entry_parent(corpus, entry_id)
        if parent_id is None:
            continue
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing affliction parent {parent_id}")
        move(book, entry_id, parent)

    for entry_id, parent_id in DISEASE_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing disease parent {parent_id}")
        move(book, entry_id, parent)

    # 7. Artifacts print inside their monster entry.
    for entry_id, parent_id in ITEM_TARGETS.items():
        parent = find_node(book, parent_id)
        if parent is None:
            raise SystemExit(f"missing item parent {parent_id}")
        move(book, entry_id, parent)

    # 8. Printed headings that the dataset splits with no heading entry.
    for title, member_ids in SYNTHESIZED_GROUPS:
        group = ensure_group(monsters, title)
        for member_id in member_ids:
            move(book, member_id, group)
        sort_children(group, lambda child: page_key(corpus, order, child))

    # 9. Monsters A to Z in printed page order.  A stable sort keeps the
    # dataset order of nodes that share a printed page, and places a group
    # heading before its first member on the same page.
    sort_children(monsters, lambda child: page_key(corpus, order, child))

    after: dict[str, str] = {}
    for top in book["chapters"]:
        walk_entries(top, [], after)

    if set(before) != set(after):
        print(f"BEFORE COUNT {len(before)} AFTER COUNT {len(after)}", file=sys.stderr)
        lost = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        print(f"LOST {lost}", file=sys.stderr)
        print(f"ADDED {added}", file=sys.stderr)
        raise SystemExit("entry set changed")

    residual = chapter(book, "New entries (page order)")
    residual_ids: set[str] = set()

    def walk_ids(node: dict) -> None:
        if node.get("entry"):
            residual_ids.add(node["entry"])
        for child in node.get("children", []):
            walk_ids(child)

    walk_ids(residual)
    unexpected = sorted(
        entry_id
        for entry_id in residual_ids
        if entry_id not in RESIDUAL_EXACT
        and not entry_id.startswith(RESIDUAL_PREFIXES)
    )
    if unexpected:
        print(f"UNEXPECTED RESIDUAL {unexpected}", file=sys.stderr)
        raise SystemExit("residual chapter holds a moved entry")

    after_seq = [
        child["entry"] for child in monsters.get("children", []) if child.get("entry")
    ]

    common = set(before_seq) & set(after_seq)
    before_rank = {
        entry_id: index for index, entry_id in enumerate(before_seq) if entry_id in common
    }
    after_rank = {
        entry_id: index for index, entry_id in enumerate(after_seq) if entry_id in common
    }
    reordered: set[str] = set()
    kept = [entry_id for entry_id in before_seq if entry_id in common]
    for i, left in enumerate(kept):
        for right in kept[i + 1 :]:
            if (before_rank[left] < before_rank[right]) != (
                after_rank[left] < after_rank[right]
            ):
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
            print(
                "books.toml is not in the Bestiary 4 nested-placement shape",
                file=sys.stderr,
            )
            return 1
        print(f"books.toml is canonical for {len(moves)} Bestiary 4 moves", file=sys.stderr)
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(f"wrote {books_path} ({len(moves)} Bestiary 4 moves)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
