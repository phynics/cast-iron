#!/usr/bin/env python3
"""Check public IDs and class categories against a previous content/ directory.

Usage: python3 scripts/check-library-upgrade.py /path/to/old/content [new/content]
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
missing = sorted(set(old) - set(new))
changed_titles = sorted(key for key in old.keys() & new.keys() if old[key]['title'] != new[key]['title'])
wrong_categories = []
for key, entry in new.items():
    if entry['kind'] not in {'class', 'prestige_class'}:
        continue
    expected = 'prestige_class' if entry['fields'].get('class_type') == 'prestige' else 'class'
    if entry['kind'] != expected:
        wrong_categories.append(key)
report = {
    'old_schema': old_manifest['schema_version'], 'new_schema': new_manifest['schema_version'],
    'old_entries': len(old), 'new_entries': len(new), 'missing_ids': missing,
    'changed_titles': changed_titles, 'incorrect_class_categories': wrong_categories,
}
print(json.dumps(report, indent=2, sort_keys=True))
raise SystemExit(bool(missing or changed_titles or wrong_categories))
