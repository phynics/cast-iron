#!/usr/bin/env python3
"""Place the Ultimate Magic tree by printed section and page order.

The script edits only ``books.toml``. It moves existing book nodes; it never
changes an entry id, an entry file, or an anchor. Run it against a corpus
checkout, then run ``cargo xtask corpus-fmt`` on the same directory:

    python3 scripts/ultimate-magic-nested.py /path/to/cast-iron-data
    cargo xtask corpus-fmt /path/to/cast-iron-data

Run it again with ``--check`` to prove the tree is already in the target shape.
The printed evidence is in docs/research/book-order-evidence.md.

The dataset splits and the printed chapters keep their shape: New Familiars
stays inside Mastering Magic, the residual ``New entries (page order)`` chapter
keeps cross-book base entries and duplicate carried records, and entries that
share a printed page keep the dataset order inside that page.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

BOOK = "ultimate-magic"
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


def child_by_title(parent: dict, title: str) -> dict | None:
    for child in parent.get("children", []):
        if child.get("title") == title:
            return child
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


def ensure_heading(
    book: dict,
    parent: dict,
    title: str,
    entry_id: str | None = None,
) -> dict:
    """Return the child heading, creating it, and attach a printed hub entry.

    A heading that print names but the dataset has no entry for is a grouping
    node. When a dataset hub entry names the same printed section, that entry
    opens the section (decision 2 in the book-order issue) instead of becoming
    a second row with the same title.
    """
    node = child_by_title(parent, title)
    if node is None:
        node = {"title": title}
    if entry_id is not None:
        existing = find_node(book, entry_id)
        if existing is not None and existing is not node:
            detach(book, existing)
        node["entry"] = entry_id
    return node


def book_entries(book: dict) -> set[str]:
    placed: set[str] = set()
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry"):
                placed.add(node["entry"])
    return placed


def path_of(book: dict, entry_id: str) -> str:
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            if node.get("entry") == entry_id:
                trail = trail_of(book, node)
                return " > ".join(trail) if trail else "(missing)"
    return "(missing)"


def trail_of(book: dict, target: dict) -> list[str] | None:
    def walk_node(current: dict, trail: list[str]) -> list[str] | None:
        here = trail + [current["title"]]
        if current is target:
            return here
        for child in current.get("children", []):
            found = walk_node(child, here)
            if found is not None:
                return found
        return None

    for chapter in book.get("chapters", []):
        found = walk_node(chapter, [])
        if found is not None:
            return found
    return None


# ---------------------------------------------------------------------------
# Entry metadata and page evidence


PAGE = re.compile(r"Ultimate Magic(?: pg\.?| p\.) ?(\d+)")
ROMAN = {
    "i": 1, "ii": 2, "iii": 3, "iv": 4, "v": 5,
    "vi": 6, "vii": 7, "viii": 8, "ix": 9,
}

_FRONT: dict[str, dict] = {}
_PAGE_CACHE: dict[str, int | None] = {}


def front_matter(corpus: Path, entry_id: str) -> dict:
    if entry_id not in _FRONT:
        path = corpus / "entries" / (entry_id[len("ref/"):] + ".md")
        text = path.read_text(encoding="utf-8")
        _FRONT[entry_id] = tomllib.loads(text.split("+++", 2)[1])
    return _FRONT[entry_id]


def group_of(corpus: Path, entry_id: str) -> str | None:
    try:
        return front_matter(corpus, entry_id).get("fields", {}).get("group")
    except (OSError, tomllib.TOMLDecodeError):
        return None


def page_of(corpus: Path, entry_id: str | None) -> int | None:
    if not entry_id:
        return None
    if entry_id not in _PAGE_CACHE:
        path = corpus / "entries" / (entry_id[len("ref/"):] + ".md")
        if not path.exists():
            _PAGE_CACHE[entry_id] = None
        else:
            match = PAGE.search(path.read_text(encoding="utf-8"))
            _PAGE_CACHE[entry_id] = int(match.group(1)) if match else None
    return _PAGE_CACHE[entry_id]


def spell_key(node: dict) -> tuple[str, int]:
    """Return the key print uses in the alphabetical spell descriptions."""
    text = node["title"].lower().replace("\u2019", "'").replace("'", "")
    text = re.sub(r"[^a-z0-9 ]", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    rank = 0
    match = re.match(r"^(.*), (greater|lesser|mass|communal|major)$", text)
    if match:
        text = match.group(1)
        rank = {"greater": 1, "lesser": 2, "mass": 3, "communal": 4, "major": 5}[
            match.group(2)
        ]
    else:
        for prefix, value in (
            ("greater ", 1),
            ("lesser ", 2),
            ("mass ", 3),
            ("communal ", 4),
        ):
            if text.startswith(prefix):
                text = text[len(prefix):] + " " + prefix.strip()
                rank = value
                break
    match = re.match(r"^(.*) (i|ii|iii|iv|v|vi|vii|viii|ix)$", text)
    if match:
        text = f"{match.group(1)} {ROMAN[match.group(2)]:02d}"
    return text, rank


# ---------------------------------------------------------------------------
# Collected class-option groups


def collect_group(book: dict, corpus: Path, group: str) -> list[dict]:
    """Detach and return the nodes of one class-option group in tree order."""
    found: list[dict] = []
    for chapter in book.get("chapters", []):
        for node in walk(chapter):
            entry_id = node.get("entry")
            if entry_id and group_of(corpus, entry_id) == group:
                found.append(node)
    for node in found:
        detach(book, node)
    return found


# ---------------------------------------------------------------------------
# Spellcasters


CLASS_IDS = {
    "alchemist": "ref/carried/ultimate-magic/rules/spellcasting-class-options/alchemist",
    "bard": "ref/carried/ultimate-magic/rules/spellcasting-class-options/bard",
    "cleric": "ref/carried/ultimate-magic/rules/spellcasting-class-options/cleric",
    "druid": "ref/carried/ultimate-magic/rules/spellcasting-class-options/druid",
    "inquisitor": "ref/carried/ultimate-magic/rules/spellcasting-class-options/inquisitor",
    "magus": "ref/carried/ultimate-magic/rules/spellcasting-class-options/magus",
    "monk": "ref/carried/ultimate-magic/rules/spellcasting-class-options/monk",
    "oracle": "ref/carried/ultimate-magic/rules/spellcasting-class-options/oracle",
    "paladin": "ref/carried/ultimate-magic/rules/spellcasting-class-options/paladin",
    "ranger": "ref/carried/ultimate-magic/rules/spellcasting-class-options/ranger",
    "sorcerer": "ref/carried/ultimate-magic/rules/spellcasting-class-options/sorcerer",
    "summoner": "ref/carried/ultimate-magic/rules/spellcasting-class-options/summoner",
    "witch": "ref/carried/ultimate-magic/rules/spellcasting-class-options/witch",
    "wizard": "ref/carried/ultimate-magic/rules/spellcasting-class-options/wizard",
}

# Each class section in printed order. "option" is a printed heading that holds
# a class-option group; "entry" is one printed entry; "archetypes" is the
# dataset archetype group. A hub id opens the printed heading.
CLASS_SECTIONS: dict[str, list[tuple]] = {
    "alchemist": [
        ("option", "Discoveries", "alchemist-discovery", None),
        ("archetypes", "Alchemist Archetypes"),
    ],
    "bard": [
        ("option", "Masterpieces", "bardic-masterpiece", None),
        ("archetypes", "Bard Archetypes"),
    ],
    "cleric": [
        ("entry", "ref/ability/variant-channeling"),
        ("archetypes", "Cleric Archetypes"),
    ],
    "druid": [
        ("option", "Animal and Terrain Domains", "druid-domain", "ref/ability/druid-domains"),
        ("option", "Vermin Companions", "animal-companion", None),
        ("archetypes", "Druid Archetypes"),
    ],
    "inquisitor": [
        ("option", "Inquisitions", "inquisition", "ref/ability/inquisitions"),
        ("archetypes", "Inquisitor Archetypes"),
    ],
    "magus": [
        ("archetypes", "Magus Archetypes"),
    ],
    "monk": [
        ("heading", "Monk Vows", "ref/ability/vows"),
        ("archetypes", "Monk Archetypes", ["ref/arc-monk/qinggong-monk"]),
    ],
    "oracle": [
        ("option", "Mysteries", "oracle-mystery", None),
        ("archetypes", "Oracle Archetypes"),
    ],
    "paladin": [
        ("archetypes", "Paladin Archetypes"),
        ("option", "Oaths", "paladin-oath", "ref/ability/oaths"),
    ],
    "ranger": [
        ("keep", "Ranger Traps"),
        ("archetypes", "Ranger Archetypes"),
    ],
    "sorcerer": [
        ("option", "Bloodlines", "sorcerer-bloodline", "ref/ability/sorcerer-bloodlines"),
        ("archetypes", "Sorcerer Archetypes"),
    ],
    "summoner": [
        ("heading", "Eidolon Models", "ref/sidekick/eidolon"),
        ("option", "Evolutions", "summoner-evolution", None),
        ("archetypes", "Summoner Archetypes"),
    ],
    "witch": [
        ("witch-hexes",),
        ("option", "Witch Patron Themes", "witch-patron", None),
        ("archetypes", "Witch Archetypes"),
    ],
    "wizard": [
        ("option", "Arcane Discoveries", "arcane-discovery", None),
        ("option", "Elemental Wizard Schools", "arcane-school", "ref/ability/arcane-schools"),
        ("archetypes", "Wizard Archetypes"),
    ],
}

WITCH_HEXES = {
    "Hexes": {
        "Beast of Ill-Omen", "Child-Scent", "Feral Speech", "Nails",
        "Poison Steep", "Prehensile Hair", "Scar", "Swamp Hag",
        "Unnerve Beasts", "Water Lung",
    },
    "Major Hexes": {
        "Beast Eye", "Cook People", "Hidden Home", "Hoarfrost", "Ice Tomb",
        "Infected Wounds", "Speak in Dreams", "Witch's Brew",
    },
    "Grand Hexes": {"Dire Prophecy", "Summon Spirit", "Witch's Hut"},
}


def build_class(
    book: dict, corpus: Path, class_node: dict, sections: list[tuple]
) -> None:
    out: list[dict] = []
    for section in sections:
        kind = section[0]
        if kind == "option":
            _, title, group, hub = section
            heading = ensure_heading(book, class_node, title, hub)
            heading["children"] = collect_group(book, corpus, group)
            out.append(heading)
        elif kind == "entry":
            out.append(take(book, section[1]))
        elif kind == "heading":
            _, title, hub = section
            out.append(ensure_heading(book, class_node, title, hub))
        elif kind == "keep":
            node = child_by_title(class_node, section[1])
            if node is None:
                raise SystemExit(f"{BOOK}: missing section {section[1]}")
            out.append(node)
        elif kind == "archetypes":
            _, title, *extra = section
            node = child_by_title(class_node, title)
            if node is None:
                node = {"title": title, "children": []}
            if extra:
                node["children"] = [take(book, entry) for entry in extra[0]]
            out.append(node)
        elif kind == "witch-hexes":
            hexes = collect_group(book, corpus, "witch-hex")
            for title in ("Hexes", "Major Hexes", "Grand Hexes"):
                heading = ensure_heading(book, class_node, title)
                heading["children"] = [
                    node for node in hexes if node["title"] in WITCH_HEXES[title]
                ]
                out.append(heading)
        else:
            raise SystemExit(f"{BOOK}: bad class section {section!r}")
    class_node["children"] = out


def spellcasters(book: dict, corpus: Path) -> None:
    chapter = next(c for c in book["chapters"] if c["title"] == "Spellcasters")
    magus = next(
        c for c in chapter.get("children", []) if c.get("entry") == "ref/class/magus"
    )
    hub = next(
        c
        for c in chapter.get("children", [])
        if c.get("entry")
        == "ref/carried/ultimate-magic/rules/spellcasting-class-options"
    )
    inner = next(
        c
        for c in hub.get("children", [])
        if c.get("entry")
        == "ref/carried/ultimate-magic/rules/spellcasting-class-options/"
        "spellcasting-class-options"
    )

    arcana = collect_group(book, corpus, "magus-arcana")
    arcana_heading = ensure_heading(book, magus, "Magus Arcana")
    arcana_heading["children"] = arcana
    magus["children"] = [arcana_heading]

    for key, entry_id in CLASS_IDS.items():
        class_node = find_node(book, entry_id)
        if class_node is None:
            raise SystemExit(f"{BOOK}: missing class {entry_id}")
        build_class(book, corpus, class_node, CLASS_SECTIONS[key])

    hub["children"] = [inner] + [find_node(book, CLASS_IDS[key]) for key in CLASS_SECTIONS]
    chapter["children"] = [magus, hub]


# ---------------------------------------------------------------------------
# Mastering Magic


SPELLBLIGHTS = [
    "ref/rule/gaining-spellblights",
    "ref/rule/optional-ways-to-acquire-spellblights",
    "ref/rule/removing-spellblights",
    "ref/rule/description-of-spellblights",
    "ref/rule/minor-spellblights",
    "ref/rule/major-spellblights",
    "ref/rule/benefits-of-harmful-conditions",
]

BINDING_OUTSIDERS = [
    "ref/rule/calling-outsiders",
    "ref/rule/true-names",
    "ref/rule/dealing-with-outsiders",
    "ref/rule/anathematic-substances",
    "ref/rule/outsider-categories",
    "ref/rule/aeon",
    "ref/rule/agathion",
    "ref/rule/angels",
    "ref/rule/archons",
    "ref/rule/azata",
    "ref/rule/daemons",
    "ref/rule/demons",
    "ref/rule/devils",
    "ref/rule/elementals-and-genies",
    "ref/rule/inevitables",
    "ref/rule/other-outsiders",
    "ref/rule/proteans",
    "ref/rule/qlippoth",
]

SPELLBOOKS = [
    "ref/eq-spellbook/apprentice-chapbook-of-rul-thaven",
    "ref/eq-spellbook/defensive-primer",
    "ref/eq-spellbook/travelers-tome",
    "ref/eq-spellbook/unnamed-journal",
    "ref/eq-spellbook/book-of-harms",
    "ref/eq-spellbook/book-of-the-grave",
    "ref/eq-spellbook/grimoire-of-glittering-eyes",
    "ref/eq-spellbook/lab-journal-of-constance-inflix",
    "ref/eq-spellbook/journal-of-the-beast-within",
    "ref/eq-spellbook/journeyman-book-of-rul-thaven",
    "ref/eq-spellbook/manuscript-of-jack-were-son",
    "ref/eq-spellbook/tome-of-the-transmuter",
    "ref/eq-spellbook/arctic-call",
    "ref/eq-spellbook/grandfathers-legacy",
    "ref/eq-spellbook/insights-of-far-seeing-taernis",
    "ref/eq-spellbook/quest-eternal",
    "ref/eq-spellbook/chymists-guidebook",
    "ref/eq-spellbook/guardian-grimoire",
    "ref/eq-spellbook/mysteries-of-shadow",
    "ref/eq-spellbook/library-of-the-dancer-of-skins",
    "ref/eq-spellbook/manual-of-binding",
    "ref/eq-spellbook/master-books-of-rul-thaven",
    "ref/eq-spellbook/mastery-of-word-and-thought",
    "ref/eq-spellbook/the-formulae-of-master-gebr",
]

DESIGNING_SPELLS = [
    "ref/rule/spell-terminology",
    "ref/rule/spell-damage",
    "ref/rule/target",
    "ref/rule/range-um",
    "ref/rule/saving-throw-um",
    "ref/rule/casting-time-um",
    "ref/rule/spell-resistance-um",
    "ref/rule/components-um",
    "ref/rule/school",
    "ref/rule/bonus-type",
    "ref/rule/description-um",
    "ref/rule/depletable-statistics",
    "ref/rule/hierarchy-of-attack-effects",
    "ref/rule/multipurpose-spells",
]

DESCRIPTORS = [
    "ref/spelldef/acid",
    "ref/spelldef/air",
    "ref/spelldef/chaotic",
    "ref/spelldef/cold",
    "ref/spelldef/curse",
    "ref/spelldef/darkness",
    "ref/spelldef/death",
    "ref/spelldef/disease",
    "ref/spelldef/earth",
    "ref/spelldef/electricity",
    "ref/spelldef/emotion",
    "ref/spelldef/evil",
    "ref/spelldef/fear",
    "ref/spelldef/fire",
    "ref/spelldef/force",
    "ref/spelldef/good",
    "ref/spelldef/language-dependent",
    "ref/spelldef/lawful",
    "ref/spelldef/light",
    "ref/spelldef/mind-affecting",
    "ref/spelldef/pain",
    "ref/spelldef/poison",
    "ref/spelldef/shadow-descriptor",
    "ref/spelldef/sonic",
    "ref/spelldef/water",
]


def mastering_magic(book: dict) -> None:
    chapter = next(c for c in book["chapters"] if c["title"] == "Mastering Magic")
    inner = child_by_title(chapter, "Mastering Magic")
    if inner is None or inner.get("entry") is None:
        raise SystemExit(f"{BOOK}: missing Mastering Magic hub")

    spellblights = find_node(book, "ref/rule/spellblights")
    duels = find_node(
        book, "ref/carried/ultimate-magic/rules/mastering-magic/spell-duels"
    )
    binding = find_node(book, "ref/rule/binding-outsiders")
    building = find_node(book, "ref/rule/building-and-modifying-constructs")
    familiars = child_by_title(chapter, "New Familiars")
    spellbooks = find_node(
        book, "ref/carried/ultimate-magic/rules/mastering-magic/spellbooks"
    )
    designing = find_node(book, "ref/rule/designing-spells")
    for name, node in (
        ("Spellblights", spellblights),
        ("Spell Duels", duels),
        ("Binding Outsiders", binding),
        ("Building and Modifying Constructs", building),
        ("New Familiars", familiars),
        ("Spellbooks", spellbooks),
        ("Designing Spells", designing),
    ):
        if node is None:
            raise SystemExit(f"{BOOK}: missing Mastering Magic section {name}")

    optional = take(book, SPELLBLIGHTS[1])
    gain = take(book, SPELLBLIGHTS[0])
    minor = take(book, SPELLBLIGHTS[4])
    major = take(book, SPELLBLIGHTS[5])
    description = take(book, SPELLBLIGHTS[3])
    removing = take(book, SPELLBLIGHTS[2])
    benefits = take(book, SPELLBLIGHTS[6])
    gain["children"] = [optional]
    description["children"] = [minor, major]
    spellblights["children"] = [gain, removing, description, benefits]

    calling = take(book, BINDING_OUTSIDERS[0])
    true_names = take(book, BINDING_OUTSIDERS[1])
    anathematic = take(book, BINDING_OUTSIDERS[3])
    dealing = take(book, BINDING_OUTSIDERS[2])
    category_children = [take(book, entry) for entry in BINDING_OUTSIDERS[5:]]
    categories = take(book, BINDING_OUTSIDERS[4])
    dealing["children"] = [anathematic]
    categories["children"] = category_children
    binding["children"] = [calling, true_names, dealing, categories]

    # Building and Modifying Constructs keeps its anchored printed headings.
    inner_building = child_by_title(building, "Building and Modifying Constructs")
    if inner_building is None:
        raise SystemExit(f"{BOOK}: missing construct headings")
    construct = child_by_title(inner_building, "Construct Modifications") or child_by_title(
        building, "Construct Modifications"
    )
    animated = child_by_title(inner_building, "Animated Objects")
    complex_heading = child_by_title(construct, "Complex Modifications")
    rune_carved = child_by_title(construct, "Rune-Carved") or child_by_title(
        complex_heading, "Rune-Carved"
    )
    if None in (animated, construct, complex_heading, rune_carved):
        raise SystemExit(f"{BOOK}: missing construct headings")
    carried_object = child_by_title(animated, "Animated Object")
    complex_entry = take(book, "ref/rule/complex-modifications")
    modifications = [
        child
        for child in complex_heading.get("children", [])
        if child is not rune_carved
    ]
    animated["children"] = [take(book, "ref/rule/animated-objects"), carried_object]
    construct["children"] = [
        take(book, "ref/rule/construct-modifications"),
        take(book, "ref/rule/basic-modifications"),
        complex_heading,
    ]
    complex_heading["children"] = [complex_entry] + modifications + [rune_carved]
    inner_building["children"] = [
        animated,
        take(book, "ref/rule/building-new-constructs"),
        take(book, "ref/rule/pricing-a-new-construct"),
        take(book, "ref/rule/repairing-constructs"),
        construct,
    ]
    building["children"] = [inner_building]

    spellbooks["children"] = [take(book, entry) for entry in SPELLBOOKS]

    descriptors = ensure_heading(book, designing, "Choosing Descriptors")
    descriptors["children"] = [take(book, entry) for entry in DESCRIPTORS]
    benchmarks = take(book, "ref/rule/benchmarks")
    designing["children"] = [take(book, entry) for entry in DESIGNING_SPELLS]
    designing["children"] += [descriptors, benchmarks]

    chapter["children"] = [
        inner,
        spellblights,
        duels,
        binding,
        building,
        familiars,
        spellbooks,
        designing,
    ]


# ---------------------------------------------------------------------------
# Words of Power


WORDCASTER_FEATS = [
    "ref/feat/experimental-spellcaster",
    "ref/feat/extra-word",
    "ref/feat/meta-word-mastery",
]

EFFECT_WORDS = [
    "ref/rule/effect-word-descriptions",
    "ref/rule/acid-words",
    "ref/rule/alignment-words",
    "ref/rule/animal-words",
    "ref/rule/armor-words",
    "ref/rule/binding-words",
    "ref/rule/body-words",
    "ref/rule/change-words",
    "ref/rule/cold-words",
    "ref/rule/command-words",
    "ref/rule/concealing-words",
    "ref/rule/death-words",
    "ref/rule/destruction-words",
    "ref/rule/detection-words",
    "ref/rule/dispelling-words",
    "ref/rule/divination-words",
    "ref/rule/electricity-words",
    "ref/rule/fear-words",
    "ref/rule/fire-words",
    "ref/rule/flight-words",
    "ref/rule/force-words",
    "ref/rule/gravity-words",
    "ref/rule/healing-words",
    "ref/rule/illumination-words",
    "ref/rule/illusion-words",
    "ref/rule/language-words",
    "ref/rule/life-words",
    "ref/rule/pain-words",
    "ref/rule/power-words",
    "ref/rule/sonic-words",
    "ref/rule/summoning-words",
    "ref/rule/teleportation-words",
    "ref/rule/time-words",
    "ref/rule/wall-words",
    "ref/rule/weather-words",
    "ref/rule/wounding-words",
]

CLASS_WORD_LISTS = [
    "ref/rule/alchemist-words",
    "ref/rule/bard-words",
    "ref/rule/cleric-oracle-words",
    "ref/rule/druid-words",
    "ref/rule/inquisitor-words",
    "ref/rule/magus-words",
    "ref/rule/paladin-words",
    "ref/rule/ranger-words",
    "ref/rule/sorcerer-wizard-words",
    "ref/rule/summoner-words",
    "ref/rule/witch-words",
]


def words_of_power(book: dict) -> None:
    chapter = next(c for c in book["chapters"] if c["title"] == "Words of Power")
    wordcaster = find_node(
        book, "ref/carried/ultimate-magic/rules/words-of-power/wordcaster-feats"
    )
    effect = find_node(
        book, "ref/carried/ultimate-magic/rules/words-of-power/effect-words"
    )
    class_lists = find_node(book, "ref/rule/class-word-lists")
    for name, node in (
        ("Wordcaster Feats", wordcaster),
        ("Effect Words", effect),
        ("Class Word Lists", class_lists),
    ):
        if node is None:
            raise SystemExit(f"{BOOK}: missing Words of Power section {name}")

    wordcaster["children"] = [take(book, entry) for entry in WORDCASTER_FEATS]
    effect["children"] = [take(book, entry) for entry in EFFECT_WORDS]
    class_lists["children"] = [take(book, entry) for entry in CLASS_WORD_LISTS]

    chapter["children"] = [
        take(book, "ref/rule/becoming-a-wordcaster"),
        take(book, "ref/rule/learning-words-of-power"),
        take(book, "ref/rule/class-specific-rules"),
        take(book, "ref/rule/arranging-words-of-power"),
        take(book, "ref/rule/optional"),
        take(
            book,
            "ref/carried/ultimate-magic/rules/words-of-power/"
            "words-of-power-in-the-world",
        ),
        take(book, "ref/rule/casting-wordspells"),
        take(book, "ref/rule/magic-item-creation-um"),
        take(book, "ref/rule/examples"),
        take(book, "ref/rule/target-words"),
        wordcaster,
        effect,
        take(book, "ref/rule/meta-words"),
        class_lists,
        take(book, "ref/rule/introducing-words-of-power"),
    ]


# ---------------------------------------------------------------------------
# Feats and Spells


def feats(book: dict, corpus: Path) -> None:
    chapter = next(c for c in book["chapters"] if c["title"] == "Feats")
    hub = child_by_title(chapter, "Feats")
    if hub is None or hub.get("entry") is None:
        raise SystemExit(f"{BOOK}: missing Feats hub")
    rest = [node for node in chapter["children"] if node is not hub]
    order = {id(node): index for index, node in enumerate(rest)}
    rest.sort(
        key=lambda node: (
            page_of(corpus, node.get("entry")) or 10**9,
            order[id(node)],
        )
    )
    chapter["children"] = [hub] + rest


def base_title(title: str) -> str:
    text = title.strip()
    match = re.match(r"^(.*), (?:Greater|Lesser|Mass|Communal|Major)$", text)
    if match:
        return match.group(1)
    match = re.match(r"^(?:Greater|Lesser|Mass|Communal) (.*)$", text)
    if match:
        return match.group(1)
    return text


def spells(book: dict, corpus: Path) -> None:
    chapter = next(c for c in book["chapters"] if c["title"] == "Spells")
    nodes = chapter["children"]
    paged = sorted(
        (
            (node, page_of(corpus, node.get("entry")))
            for node in nodes
            if page_of(corpus, node.get("entry")) is not None
        ),
        key=lambda item: spell_key(item[0]),
    )

    def derived_page(node: dict) -> int:
        base = base_title(node["title"]).lower()
        for other in nodes:
            if other is not node and other["title"].lower() == base:
                page = page_of(corpus, other.get("entry"))
                if page is not None:
                    return page
        key = spell_key(node)
        for other, page in paged:
            if spell_key(other) > key:
                return page
        return 10**9

    chapter["children"] = sorted(
        nodes,
        key=lambda node: (
            page_of(corpus, node.get("entry")) or derived_page(node),
            spell_key(node),
        ),
    )


# ---------------------------------------------------------------------------
# The transform


def order_map(book: dict) -> dict[str, list[str]]:
    """Return each parent's child keys in order, for reorder reporting."""
    result: dict[str, list[str]] = {}

    def walk_order(node: dict, trail: list[str]) -> None:
        children = node.get("children", [])
        if children:
            result[" > ".join(trail + [node["title"]])] = [
                child.get("entry") or child["title"] for child in children
            ]
        for child in children:
            walk_order(child, trail + [node["title"]])

    for chapter in book.get("chapters", []):
        walk_order(chapter, [])
    return result


def lcs_kept(before: list[str], after: list[str]) -> set[str]:
    """Return the keys that stay in the longest common subsequence."""
    n, m = len(before), len(after)
    table = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            if before[i - 1] == after[j - 1]:
                table[i][j] = table[i - 1][j - 1] + 1
            else:
                table[i][j] = max(table[i - 1][j], table[i][j - 1])
    kept: set[str] = set()
    i, j = n, m
    while i and j:
        if before[i - 1] == after[j - 1]:
            kept.add(before[i - 1])
            i -= 1
            j -= 1
        elif table[i - 1][j] >= table[i][j - 1]:
            i -= 1
        else:
            j -= 1
    return kept


def transform(book: dict, corpus: Path) -> list[tuple[str, str, str, int | None]]:
    before = {
        node["entry"]: path_of(book, node["entry"])
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }
    before_orders = order_map(book)

    spellcasters(book, corpus)
    mastering_magic(book)
    words_of_power(book)
    feats(book, corpus)
    spells(book, corpus)

    after = {
        node["entry"]: path_of(book, node["entry"])
        for chapter in book["chapters"]
        for node in walk(chapter)
        if node.get("entry")
    }
    after_orders = order_map(book)
    if set(before) != set(after):
        missing = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        raise SystemExit(f"entry set changed: lost {missing}, added {added}")

    moves = []
    for entry_id in sorted(before):
        if before[entry_id] != after[entry_id]:
            node = find_node(book, entry_id)
            moves.append(
                (
                    node["title"] if node else entry_id,
                    before[entry_id],
                    after[entry_id],
                    page_of(corpus, entry_id),
                )
            )

    # A node can keep its parent and still change sibling order. Report those
    # separately, skipping anything already reported as a parent change.
    for parent, old_order in before_orders.items():
        new_order = after_orders.get(parent)
        if new_order is None or new_order == old_order:
            continue
        kept = lcs_kept(old_order, new_order)
        for key in old_order:
            if key in kept:
                continue
            if key not in new_order:
                continue
            entry_id = key if key.startswith("ref/") else None
            if entry_id is not None and before.get(entry_id) != after.get(entry_id):
                continue
            old_index = old_order.index(key)
            new_index = new_order.index(key)
            if old_index == new_index:
                continue
            node = find_node(book, entry_id) if entry_id else None
            title = node["title"] if node else key
            moves.append(
                (
                    title,
                    f"{parent} (position {old_index + 1})",
                    f"{parent} (position {new_index + 1})",
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
        "against the printed contents page of Ultimate Magic. Each chapter "
        "holds its printed section headings, and each entry sits in its "
        "printed section in page order. See "
        "docs/research/book-order-evidence.md. Canonical ids were substituted "
        "through the reviewed alias map; chapters and order are unchanged."
    )
    book["review"] = [
        "New Familiars is a dataset split kept as a sub-group inside the "
        "printed Mastering Magic chapter; the printed appendix is the Appendix "
        "chapter.",
        "Nested placement follows the printed section headings; entries that "
        "share a printed page keep the dataset order.",
        "Canonical ids were substituted through sources/canonical/id-map.toml. "
        "A legacy entry with no installed target is omitted and listed in "
        "coverage.",
    ]

    text = dump_books(books)
    for title, old, new, page in moves:
        print(f"{title}\t{old}\t{new}\t{page if page is not None else '-'}")

    if args.check:
        if text != original_text:
            print(
                f"books.toml is not in the nested {BOOK} shape", file=sys.stderr
            )
            return 1
        print(f"books.toml is canonical for {BOOK}; entries placed: {len(after)}")
        return 0
    if text != original_text:
        books_path.write_text(text, encoding="utf-8")
    print(
        f"wrote {books_path} ({len(moves)} moves, {len(after)} entries placed)",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
