#!/usr/bin/env python3
"""Extract factual file-to-protocol mappings without executing upstream code.

Usage: python3 scripts/extract-pf-source-routes.py SNAPSHOT OUTPUT
The source routing configuration stays in the private source cache. No upstream
renderer code is copied into Cast Iron. Review the output when updating the pin.
"""
import json
from pathlib import Path
import re
import sys


def extract(snapshot: Path) -> dict[str, str]:
    text = (snapshot / "prebuild/basic_data_groups.js").read_text()
    imports = dict(re.findall(r"import\s+(\w+)\s+from\s+'\.\./(json/[^']+)'", text))
    routes = {}
    for match in re.finditer(r'"([^"\n]+)":\s*\{(.*?)\n\s*\}', text, re.S):
        protocol = re.search(r'\blink:\s*"([^"\n]+)"', match[2])
        if protocol:
            data = re.search(r'\bdata:\s*(\w+)', match[2])
            if not data or data[1] not in imports:
                raise ValueError(f"unmapped data variable: {match[1]}")
            path = imports[data[1]]
            if path in routes:
                raise ValueError(f"duplicate file mapping: {path}")
            if not (snapshot / path).is_file():
                raise ValueError(f"missing source file: {path}")
            routes[path] = protocol[1]
    if not routes:
        raise ValueError("no routes extracted; upstream format changed")
    return dict(sorted(routes.items()))


if __name__ == "__main__":
    snapshot, output = map(Path, sys.argv[1:])
    output.write_text(json.dumps(extract(snapshot), indent=2) + "\n")
