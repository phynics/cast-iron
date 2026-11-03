#!/usr/bin/env python3
"""Regenerate explicit roots from the pinned, reviewed 16-book snapshot.

This inventories files only. The compiler remains responsible for pin hashes,
normalization, rejection reports, and validation. Run cargo xtask content after it.
"""
from pathlib import Path
import hashlib
import json
import tomllib

root = Path(__file__).resolve().parents[1]
cache = root / '.cache/sources/psrd-data'
config = root / 'sources/psrd-data/source.toml'
old = config.read_text()
books = {
    'advanced_players_guide': "Advanced Player's Guide",
    'advanced_race_guide': 'Advanced Race Guide',
    'bestiary': 'Bestiary', 'bestiary_2': 'Bestiary 2',
    'bestiary_3': 'Bestiary 3', 'bestiary_4': 'Bestiary 4',
    'core_rulebook': 'Core Rulebook', 'game_mastery_guide': 'Game Mastery Guide',
    'monster_codex': 'Monster Codex', 'mythic_adventures': 'Mythic Adventures',
    'npc_codex': 'NPC Codex', 'technology_guide': 'Technology Guide',
    'ultimate_campaign': 'Ultimate Campaign', 'ultimate_combat': 'Ultimate Combat',
    'ultimate_equipment': 'Ultimate Equipment', 'ultimate_magic': 'Ultimate Magic',
}
known = {
    'spell', 'feat', 'creature', 'item', 'class', 'race', 'skill', 'affliction', 'trap',
    # Schema 3: relationship families. `drawback` becomes a trait category, and
    # `class_archetype` becomes the archetype kind. See docs/research/relationship-data.md.
    'trait', 'drawback', 'racial_trait', 'class_archetype',
}
actual = {p.name for p in cache.iterdir() if p.is_dir()}
if actual != set(books):
    raise SystemExit(f'Book scope changed; review before selecting: {actual ^ set(books)}')
typed, rules = [], []
for book in sorted(books):
    typed.extend(f'{book}/{kind}' for kind in sorted(known) if (cache / book / kind).is_dir())
    rules.extend(str(p.relative_to(cache)) for p in sorted((cache / book / 'rules').glob('*.json')))
files = sorted({p for directory in typed for p in (cache / directory).rglob('*.json')} | {cache / p for p in rules})
lines = sorted(f'{p.relative_to(cache)}:{hashlib.sha256(p.read_bytes()).hexdigest()}\n' for p in files)
digest = hashlib.sha256(''.join(lines).encode()).hexdigest()
header = old[:old.index('# Typed records:')]
if 'prestige_classes =' not in header:
    header += 'prestige_classes = ["pfsrd://Technology Guide/Rules/Technology in the world/Technomancer"]\n\n'
import re
header = re.sub(r'input_digest = "[a-f0-9]*"', f'input_digest = "{digest}"', header)
header = header.replace('POC selection', 'reviewed book selection')
output = header + '# Typed records: recursively selected directories. Regenerate with scripts/expand-sources.py.\n'
for key, values in [('typed', typed), ('rules', rules)]:
    output += key + ' = [\n' + ''.join(f'  {json.dumps(value)},\n' for value in values) + ']\n\n'
output += '[dataset.books]\n' + ''.join(f'{book} = {json.dumps(title)}\n' for book, title in sorted(books.items()))
assert tomllib.loads(output)['dataset']['input_digest'] == digest
config.write_text(output)
print(f'{len(books)} books, {len(typed)} typed roots, {len(rules)} rules trees, {len(files)} selected files')
