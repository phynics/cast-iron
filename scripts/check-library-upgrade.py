#!/usr/bin/env python3
"""Check public IDs, class categories, and schema-3 files against a previous content/ directory.

Usage: python3 scripts/check-library-upgrade.py /path/to/old/content [new/content]

Fails when a previously published ID or title is gone, a class is in the wrong category, or
a schema-3 file (relations.json, lists.json, books.json) names an entry that does not exist.
Kind changes of old IDs are reported, not failures: bookmarks keep working by ID.
"""
from pathlib import Path
import json
import sys


def library(root):
    manifest = json.loads((root / 'manifest.json').read_text())
    directory = root / manifest['content_version']
    entries = {
        entry['id']: entry
        for shard in sorted((directory / 'entries').glob('*.json'))
        for entry in json.loads(shard.read_text())
    }
    return manifest, entries


old_manifest, old = library(Path(sys.argv[1]))
new_manifest, new = library(Path(sys.argv[2]) if len(sys.argv) > 2 else Path('dist/content'))
placed = set()
missing = sorted(set(old) - set(new))
changed_titles = sorted(key for key in old.keys() & new.keys() if old[key]['title'] != new[key]['title'])
wrong_categories = []
for key, entry in new.items():
    if entry['kind'] not in {'class', 'prestige_class'}:
        continue
    expected = 'prestige_class' if entry['fields'].get('class_type') == 'prestige' else 'class'
    if entry['kind'] != expected:
        wrong_categories.append(key)
changed_kinds = sorted(f'{key}: {old[key]["kind"]} -> {new[key]["kind"]}'
                       for key in old.keys() & new.keys() if old[key]['kind'] != new[key]['kind'])

# Schema-3 files.
dangling = []
version_dir = Path(sys.argv[2] if len(sys.argv) > 2 else 'dist/content') / new_manifest['content_version']


def load(name):
    path = version_dir / name
    return json.loads(path.read_text()) if path.exists() else None


relations, lists, books, coverage = (load(n) for n in ('relations.json', 'lists.json', 'books.json', 'coverage.json'))
if new_manifest['schema_version'] >= 3:
    for source, kind, target in (relations or {}).get('edges', []):
        for end in (source, target):
            if end not in new:
                dangling.append(f'relations.json: {end}')
    for spell, _ in (lists or {}).get('spells', []):
        if spell not in new:
            dangling.append(f'lists.json: {spell}')
    placed = set()

    def walk(node):
        if node.get('entry') and not node.get('anchor'):
            if node['entry'] not in new:
                dangling.append(f'books.json: {node["entry"]}')
            if node['entry'] in placed:
                dangling.append(f'books.json: {node["entry"]} placed twice')
            placed.add(node['entry'])
        for child in node.get('children', []):
            walk(child)

    for tree in (books or {}).get('books', []):
        for chapter in tree['chapters']:
            walk(chapter)
    unplaced = {u['entry'] for u in coverage['relations']['book_order']['unplaced']}
    for key in new:
        if (key in placed) == (key in unplaced):
            dangling.append(f'books.json: {key} is neither placed nor reported, or both')
report = {
    'old_schema': old_manifest['schema_version'], 'new_schema': new_manifest['schema_version'],
    'old_entries': len(old), 'new_entries': len(new), 'missing_ids': missing,
    'changed_titles': changed_titles, 'incorrect_class_categories': wrong_categories,
    'changed_kinds_count': len(changed_kinds), 'changed_kinds_first': changed_kinds[:10],
    'dangling_references': dangling,
    'relation_edges': sum((relations or {}).get('counts', {}).values()),
    'book_placed': len(placed) if new_manifest['schema_version'] >= 3 else None,
}
print(json.dumps(report, indent=2, sort_keys=True))
raise SystemExit(bool(missing or changed_titles or wrong_categories or dangling))
