#!/usr/bin/env python3
"""Reproduce the counts in docs/research/relationship-data.md from the pinned dataset.

Read-only: it parses JSON and opens the SQLite databases with mode=ro. The dataset is
untrusted input, so the script never executes anything from it.

Usage: python3 -I scripts/survey-relationships.py
"""
from collections import Counter, defaultdict
from pathlib import Path
import html
import json
import re
import sqlite3

root = Path(__file__).resolve().parents[1] / '.cache/sources/psrd-data'
books = {
    'advanced_players_guide': 'apg', 'advanced_race_guide': 'arg', 'bestiary': 'b1',
    'bestiary_2': 'b2', 'bestiary_3': 'b3', 'bestiary_4': 'b4', 'core_rulebook': 'cr',
    'game_mastery_guide': 'gmg', 'monster_codex': 'mc', 'mythic_adventures': 'ma',
    'npc_codex': 'npc', 'technology_guide': 'tech', 'ultimate_campaign': 'ucampaign',
    'ultimate_combat': 'uc', 'ultimate_equipment': 'ue', 'ultimate_magic': 'um',
}


def records(family):
    for path in sorted(root.glob(f'*/{family}/**/*.json')):
        yield path.relative_to(root).parts[0], json.loads(path.read_text())


def walk(node):
    yield node
    for child in node.get('sections') or []:
        yield from walk(child)


def plain(text):
    return re.sub(r'\s+', ' ', html.unescape(re.sub(r'<[^>]+>', ' ', text or ''))).strip()


print('== a. families: typed records per book, and copies inside the selected rules trees')
selected_urls = Counter()
for path in sorted(root.glob('*/rules/*.json')):
    for node in walk(json.loads(path.read_text())):
        selected_urls[(node.get('type'), node.get('url'))] += 1
for family in ['racial_trait', 'class_archetype', 'trait', 'drawback', 'affliction', 'animal_companion',
               'mythic_spell', 'haunt', 'vehicle', 'settlement', 'resource', 'army', 'kingdom_resource']:
    per_book = Counter(book for book, _ in records(family))
    in_tree = sum(1 for (kind, _), n in selected_urls.items() if kind == family)
    print(f'{family}: {sum(per_book.values())} typed {dict(per_book)}; {in_tree} urls in selected rules trees')

print('== a. racial traits: parent by url')
races = {record['url'] for _, record in records('race')}
by_parent = Counter()
for book, record in records('racial_trait'):
    parts = record['url'].split('/')
    ancestor = any('/'.join(parts[:i]) in races for i in range(len(parts) - 1, 2, -1))
    by_parent[(book, 'race record above it in the url' if ancestor else 'no race record above it')] += 1
print(dict(by_parent))
print('alternate:', sum('/Alternate Racial Traits/' in r['url'] for _, r in records('racial_trait')))

print('== a. archetypes: subtype names a class')
classes = {r['name'].lower() for _, r in records('class')}
subtypes = Counter(r.get('subtype') for _, r in records('class_archetype'))
print('archetypes', sum(subtypes.values()), 'subtypes not a class name:', [s for s in subtypes if s not in classes])
print('not below an "... Archetypes" url segment:',
      [r['url'] for _, r in records('class_archetype') if not r['url'].split('/')[-2].lower().endswith('archetypes')])

print('== b. feat prerequisites')
feats = [r for _, r in records('feat')]
with_section = body_only = description_only = 0
for feat in feats:
    section = next((s for s in feat.get('sections', []) if (s.get('name') or '').strip().lower() in ('prerequisites', 'prerequisite')), None)
    if section:
        with_section += 1
        if plain(section.get('body')):
            body_only += 1
        elif plain(section.get('description')):
            description_only += 1
print(f'{len(feats)} feats, {with_section} with a prerequisites section: {body_only} wording in body, {description_only} in description only')

print('== c. spell levels')
pairs = Counter()
spells = 0
for _, spell in records('spell'):
    spells += 1
    for level in spell['levels']:
        pairs[level['class']] += 1
print(spells, 'spells,', sum(pairs.values()), 'pairs,', len(pairs), 'class names')
print(dict(sorted(pairs.items(), key=lambda item: -item[1])))
database_pairs = 0
for short in ('apg', 'arg', 'cr', 'ma', 'mc', 'tech', 'uc', 'um'):
    con = sqlite3.connect(f'file:{root}/book-{short}.db?mode=ro', uri=True)
    database_pairs += con.execute("select count(*) from spell_lists l join sections s on s.section_id=l.section_id where s.type='spell'").fetchone()[0]
print('spell_lists rows in the databases:', database_pairs)

print('== e. chapter order in the databases: children of the root, then of Rules')
for directory, short in books.items():
    con = sqlite3.connect(f'file:{root}/book-{short}.db?mode=ro', uri=True)
    top = con.execute('select section_id from sections where parent_id is null').fetchone()[0]
    names = [n for (n,) in con.execute('select name from sections where parent_id=? order by lft', (top,))]
    rules = con.execute('select section_id from sections where parent_id=? and name=?', (top, 'Rules')).fetchone()
    rule_names = [n for (n,) in con.execute('select name from sections where parent_id=? order by lft', (rules[0],))] if rules else []
    print(f'{directory}: root {names}; Rules {rule_names}')
