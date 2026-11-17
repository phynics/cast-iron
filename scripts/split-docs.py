#!/usr/bin/env python3
"""One-time migration of docs/decisions.md and docs/status.md into one file per entry (#462).

The script is kept so a reviewer can rerun the proof. It reads the pre-split text from git
(`PRE_SPLIT_REV`), so it works after the split too.

    python3 scripts/split-docs.py plan              # print every entry: file, date, area, status
    python3 scripts/split-docs.py write [--force]   # write docs/decisions/ and docs/status/ entries
    python3 scripts/split-docs.py verify            # prove that no text was lost

`write` rebuilds each entry's front matter from this script, so it refuses to run once the generated
index exists unless you pass `--force`. It also rewrites the relative links that moving the text into a
subdirectory broke (`ui-tasks.md` becomes `../ui-tasks.md`; `decisions.md#anchor` becomes the entry
file). `verify` fails unless every `## ` section of the original maps to exactly one file, and the body
of each file equals its section text, apart from those link targets and the one renamed duplicate
heading (both counted in the output).

File format. Front matter is `key: value` lines between `---` lines. A string and a list are JSON
(`title: "..."`, `issues: [394, 420]`); `date`, `status`, `area` and `kind` are bare words. The
first body line is the original heading with one `#` instead of `##`. Everything after it is the
original text, byte for byte. The index generators in `crates/xtask/src/docs_index.rs` read the
same format.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PRE_SPLIT_REV = "f59c34c431eda0b10b52948c49be461a9f3267de"
DECISIONS_DIR = ROOT / "docs" / "decisions"
STATUS_DIR = ROOT / "docs" / "status"

DATE_RE = re.compile(r"\d{4}-\d{2}-\d{2}")

# ---------------------------------------------------------------------------------------------
# Decisions: curated metadata. Keys are titles without the date.
# ---------------------------------------------------------------------------------------------

# A heading that occurs twice. The second one is renamed (the architecture review, M3). The key is
# the heading text and the 1-based occurrence.
# The two "Install button" sections are the same text, pasted twice. Both stay (no text is lost);
# the second is renamed and marked as superseded by the first.
DUPLICATE_FIX = {
    ("Install button, 2026-11-05", 2): "Install button (a second copy of the entry above), 2026-11-05",
}

MEMBERSHIP = "Book membership is derived from `source_ids`"
PLACEMENT = "Placement membership and retired book routes"
PLANNED = "Planned simplification after the reference walkthrough"
MANAGE = "Manage books, a flat contents list, and no book-page toggle"
DEFAULT_BUILD = "Canonical library is the default build"
UPSTREAM_REMOVED = "The upstream import is removed"
TEST_SCOPE_1109 = "Test scope: affected specs locally, the full suite once in CI"
TEST_SCOPE_1117 = "Test scope: owner rules of 2026-11-17"

# title -> (status, [titles of the entries that replace it])
SUPERSEDED = {
    "Install button (a second copy of the entry above)": ("superseded", ["Install button"]),
    # Explicit "Superseded:" list of the membership entry (#394).
    "Reference simplification implementation": ("partly-superseded", [MEMBERSHIP]),
    "A listing chapter owns an entry only as a last resort": ("superseded", [MEMBERSHIP, PLACEMENT]),
    "Inactive books in search": ("partly-superseded", [MEMBERSHIP]),
    "One flattened reading order per book": ("partly-superseded", [MEMBERSHIP]),
    "Third-party changes sit on their base": (
        "partly-superseded",
        ["The Sources line follows the visible text"],
    ),
    # Collections and star export (2026-11-03 simplification, 2026-11-16 owner decisions).
    "Reference screens": ("partly-superseded", [PLANNED, "Heading DOM ids and URL anchor keys"]),
    "Navigation after the iPhone walkthrough": (
        "partly-superseded",
        [PLANNED, "Search is a header icon that opens Quick search"],
    ),
    "Portable stars and publication choices": (
        "partly-superseded",
        ["Rule target evidence and new-source preparation", "Owner decisions from the architecture review"],
    ),
    # Books page and active books.
    "Books sorting, contents tree, and publication metadata": (
        "partly-superseded",
        ["A book row opens the book page", "Books tab and book page"],
    ),
    "A book row opens the book page": ("partly-superseded", ["Active books and the presets"]),
    "Active books and the presets": ("partly-superseded", [MANAGE]),
    "Books tab and book page": ("partly-superseded", [MANAGE]),
    "Book presets and third-party books": ("partly-superseded", ["Active books and the presets"]),
    # Canonical build and the removal of the upstream import.
    "Canonical build selection and test corpus": ("partly-superseded", [DEFAULT_BUILD]),
    "Canonical compile": ("partly-superseded", [DEFAULT_BUILD, UPSTREAM_REMOVED]),
    DEFAULT_BUILD: ("partly-superseded", [UPSTREAM_REMOVED]),
    "The pinned corpus is the source of truth for the canonical library": (
        "partly-superseded",
        [UPSTREAM_REMOVED],
    ),
    # Licensing: the section rules of the screen were removed on 2026-11-05.
    "Product Identity screen for the Pf Data corpus": (
        "partly-superseded",
        ["Paizo Product Identity kept under the Community Use Policy"],
    ),
    # CI and test scope.
    "CI scope: Rust checks on pull requests, the full pipeline daily on main": (
        "partly-superseded",
        [TEST_SCOPE_1109, TEST_SCOPE_1117],
    ),
    TEST_SCOPE_1109: ("partly-superseded", [TEST_SCOPE_1117]),
}

# title -> area, for titles the rules below place wrongly.
AREA_OVERRIDE = {
    "Quick reference page and the Table tab": "ui",
    "Class options in the app": "ui",
    "Heading DOM ids and URL anchor keys": "ui",
    "Canonical entry presentation": "ui",
    "The Sources line follows the visible text": "ui",
    "The catch-all chapter is not shown": "ui",
    "Archives of Nethys links are one quiet line": "ui",
    "Book section titles are search results": "search",
    "Client id resolution and bookmark migration": "content",
    "Pf Data canonical relations": "content",
    "Third-party revisions replace their original in place": "content",
    "Platform layer": "install-offline",
    "A listing chapter owns an entry only as a last resort": "content",
}

# Ordered (regex on the title, area). The first match wins. Areas: corpus, content, search, ui,
# install-offline, releases, process, licensing. Anything else is ui.
AREA_RULES = [
    (r"Product Identity|Licensing|Section 15|notice|Owner-copy|legal screen|Legal coverage|Community Use", "licensing"),
    (r"Two-step release|Release branch|Public POC", "releases"),
    (r"\bCI\b|Test scope|Firefox|Browser acceptance|shard|Verification lessons|Owner decisions|Local diagnostics|Size budget", "process"),
    (r"Install button|update check|update indicator|update action|WebKit stops|Recents writes|Settings names", "install-offline"),
    (r"Plain-text corpus|[Dd]rift|Pf Data (derived|migration|class)|Legacy|[Aa]lias|[Cc]utover|Carry-over|Body-evidence|Reviewed primary|Rule target|Phase 5|Full Pf Data|upstream|pinned corpus|Canonical (book index|publication registry|compile|build selection|library is the default|PSRD|converter)|Body-link pass|Feat fields|Printed citations|stored paths|Footnotes|Placement membership", "corpus"),
    (r"Reachability|membership|Attached entries|Mythic feats|Stamina|Third-party|Spell variants|publication marker|Book groups follow print|Adventure series fields|Relationships|Expanded book|GameMastery|Race groups|Feature anchors|Reverse body-link|Spell-list structure", "content"),
    (r"[Ss]earch|Did you mean|prefix completions|Quick search", "search"),
]
DEFAULT_AREA = "ui"


def git_show(path):
    return subprocess.run(
        ["git", "show", f"{PRE_SPLIT_REV}:{path}"], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def git_added_date(heading_line, path):
    """The author date of the first commit that has `heading_line` in `path` (history of the file)."""
    out = subprocess.run(
        ["git", "log", "--reverse", "--format=%as", "-S" + heading_line, "--", path],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.split()
    return out[0] if out else None


def split_sections(text):
    """Returns (preamble, [(heading_without_##, section_text)]). Section text starts at the `## ` line
    and ends before the next one, with trailing blank lines removed and one final newline."""
    lines = text.split("\n")
    heads = []
    fenced = False
    for number, line in enumerate(lines):
        if line.startswith("```"):
            fenced = not fenced
        if not fenced and line.startswith("## "):
            heads.append(number)
    preamble = "\n".join(lines[: heads[0]])
    sections = []
    for at, start in enumerate(heads):
        end = heads[at + 1] if at + 1 < len(heads) else len(lines)
        chunk = lines[start:end]
        while chunk and chunk[-1] == "":
            chunk.pop()
        sections.append((lines[start][3:], "\n".join(chunk) + "\n"))
    return preamble, sections


def heading_date(heading):
    """The date a heading names: `, DATE`, `(DATE)`, or a leading `DATE: `."""
    found = DATE_RE.findall(heading)
    return found[0] if found else None


def strip_date(heading):
    title = re.sub(r",?\s*\(?\d{4}-\d{2}-\d{2}\)?\s*$", "", heading)
    title = re.sub(r"^\d{4}-\d{2}-\d{2}:\s*", "", title)
    return title.strip()


def slugify(title, limit=64):
    ascii_title = title.lower()
    ascii_title = re.sub(r"`", "", ascii_title)
    ascii_title = re.sub(r"[^a-z0-9]+", "-", ascii_title).strip("-")
    if len(ascii_title) <= limit:
        return ascii_title
    cut = ascii_title[:limit]
    if "-" in cut:
        cut = cut[: cut.rindex("-")]
    return cut.strip("-")


def pick_area(title):
    if title in AREA_OVERRIDE:
        return AREA_OVERRIDE[title]
    for pattern, area in AREA_RULES:
        if re.search(pattern, title):
            return area
    return DEFAULT_AREA


def references(heading, body):
    """Issue and pull request numbers of this repository. Links to `cast-iron/issues/N` and
    `cast-iron/pull/N` count anywhere; a bare `#N` counts in the heading and the first paragraph;
    `PR #N` counts anywhere. A number in a link to another repository never counts."""
    issues, prs = set(), set()
    for kind, number in re.findall(r"phynics/cast-iron/(issues|pull)/(\d+)", body):
        (prs if kind == "pull" else issues).add(int(number))
    for number in re.findall(r"\bPRs?\s+#?(\d+)", body):
        prs.add(int(number))
    paragraphs = [p for p in re.split(r"\n\s*\n", body.strip("\n")) if p.strip()]
    first = paragraphs[0] if paragraphs else ""
    first = re.sub(r"(?i)\bdata\s+#\d+", "", first)
    for number in re.findall(r"(?<![\w/])#(\d+)\b", heading + "\n" + first):
        issues.add(int(number))
    issues -= prs
    return sorted(issues), sorted(prs)


def decision_entries():
    _, sections = split_sections(git_show("docs/decisions.md"))
    seen = {}
    entries = []
    for index, (heading, text) in enumerate(sections):
        seen[heading] = seen.get(heading, 0) + 1
        heading = DUPLICATE_FIX.get((heading, seen[heading]), heading)
        original = text
        text = "## " + heading + text[len("## " + sections[index][0]):]
        entries.append((index, heading, text, original))
    titles = {}
    result = []
    for index, heading, text, original in entries:
        title = strip_date(heading)
        date = heading_date(heading)
        origin = "heading"
        if not date:
            date = git_added_date("## " + sections[index][0], "docs/decisions.md")
            origin = "git history"
        stem = f"{date}-{slugify(title)}"
        titles.setdefault(title, []).append(stem)
        result.append({"index": index, "heading": heading, "text": text, "original": original,
                       "title": title, "date": date, "origin": origin, "stem": stem})
    return result, titles


def front_matter(fields):
    lines = ["---"]
    for key, value in fields:
        lines.append(f"{key}: {value if isinstance(value, str) and key in BARE else json.dumps(value, ensure_ascii=False)}")
    lines.append("---")
    return "\n".join(lines) + "\n"


BARE = {"date", "status", "area", "kind"}


def file_text(fields, text):
    """Front matter, a blank line, then the section with its heading demoted from `##` to `#`."""
    return front_matter(fields) + "\n" + text[1:]


def resolve_supersession(entries, titles):
    """Maps each entry to (status, [stems]); fails on a title that names no entry."""
    for source, (status, replacements) in SUPERSEDED.items():
        if source not in titles or len(titles[source]) != 1:
            if source != TEST_SCOPE_1117:
                raise SystemExit(f"superseded entry not found exactly once: {source!r}")
    stems = {}
    for entry in entries:
        status, replacements = SUPERSEDED.get(entry["title"], ("current", []))
        resolved = []
        for title in replacements:
            if title not in titles or len(titles[title]) != 1:
                raise SystemExit(f"replacement not found exactly once: {title!r}")
            resolved.append(titles[title][0])
        stems[entry["stem"]] = (status, sorted(resolved))
    return stems


# Entries that this change adds on top of the split, in the same format.
EXTRA_DECISIONS = {}


def plan_decisions():
    entries, titles = decision_entries()
    extra_titles = {TEST_SCOPE_1117: ["2026-11-17-" + slugify(TEST_SCOPE_1117)]}
    merged = dict(titles)
    merged.update(extra_titles)
    stems = resolve_supersession(entries, merged)
    rows = []
    for entry in entries:
        status, superseded_by = stems[entry["stem"]]
        issues, prs = references(entry["heading"], entry["text"].split("\n", 1)[1])
        rows.append(dict(entry, status=status, superseded_by=superseded_by, issues=issues, prs=prs,
                         area=pick_area(entry["title"])))
    stems_seen = set()
    for row in rows:
        if row["stem"] in stems_seen:
            raise SystemExit(f"two entries map to {row['stem']}.md")
        stems_seen.add(row["stem"])
    return rows


def decision_fields(row):
    return [
        ("title", row["title"]),
        ("date", row["date"]),
        ("status", row["status"]),
        ("superseded_by", row["superseded_by"]),
        ("issues", row["issues"]),
        ("prs", row["prs"]),
        ("area", row["area"]),
    ]


# ---------------------------------------------------------------------------------------------
# Status log
# ---------------------------------------------------------------------------------------------

def status_kind(title):
    if title.startswith("Corpus pinned"):
        return "pin"
    if title.startswith("Release `") or title.startswith("Two-step release"):
        return "release"
    return "record"


def plan_status():
    _, sections = split_sections(git_show("docs/status.md"))
    rows = []
    seen = set()
    for index, (heading, text) in enumerate(sections):
        title = strip_date(heading)
        date = heading_date(heading)
        origin = "heading"
        if not date:
            date = git_added_date("## " + heading, "docs/status.md")
            origin = "git history"
        stem = f"{date}-{slugify(title)}"
        if stem in seen:
            raise SystemExit(f"two status entries map to {stem}.md")
        seen.add(stem)
        rows.append({"index": index, "heading": heading, "text": text, "original": text, "title": title,
                     "date": date, "origin": origin, "stem": stem, "kind": status_kind(title)})
    return rows


def status_fields(row):
    return [("title", row["title"]), ("date", row["date"]), ("kind", row["kind"])]


# ---------------------------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------------------------

def cmd_plan():
    decisions = plan_decisions()
    print(f"decisions: {len(decisions)} sections")
    for row in decisions:
        flag = "" if row["origin"] == "heading" else "  [date from git]"
        print(f"{row['date']} {row['area']:<15} {row['status']:<17} {row['stem']}{flag}")
        if row["superseded_by"]:
            print(f"{'':50}-> {', '.join(row['superseded_by'])}")
    status = plan_status()
    print(f"\nstatus: {len(status)} sections")
    for row in status:
        flag = "" if row["origin"] == "heading" else "  [date from git]"
        print(f"{row['date']} {row['kind']:<8} {row['stem']}{flag}")


LINK_RE = re.compile(r"\[([^\]]*)\]\(([^)\s]+)\)")


def heading_anchor(heading):
    """The GitHub anchor of a heading."""
    anchor = re.sub(r"[^\w\- ]", "", heading.lower())
    return anchor.replace(" ", "-")


def anchor_map():
    """GitHub anchors of the old docs/decisions.md headings -> entry stems."""
    mapping = {"__titles__": {}}
    for row in plan_decisions():
        mapping[heading_anchor(split_sections(git_show("docs/decisions.md"))[1][row["index"]][0])] = row["stem"]
        mapping["__titles__"][row["title"]] = row["stem"]
    return mapping


def rewrite_links(text, directory, anchors, titles=None):
    """The text moved from docs/ into docs/decisions/ or docs/status/, so a relative link written for
    docs/ needs `../`, and a link into decisions.md names the entry file. Nothing else changes."""
    count = 0
    by_title = {title.lower(): stem for title, stem in anchors.get("__titles__", {}).items()}

    def fix(match):
        nonlocal count
        label, target = match.group(1), match.group(2)
        if re.match(r"^[a-z][a-z0-9+.-]*:", target) or target.startswith("#"):
            return match.group(0)
        file_part, _, anchor = target.partition("#")
        new = target
        if file_part == "decisions.md":
            wanted = label.lower().rstrip(".")
            starts = [stem for title, stem in by_title.items() if title.startswith(wanted)]
            stem = anchors.get(anchor) or by_title.get(wanted) or (starts[0] if len(starts) == 1 else None)
            if stem:
                new = f"{stem}.md" if directory == DECISIONS_DIR else f"../decisions/{stem}.md"
            else:
                new = "README.md" if directory == DECISIONS_DIR else "../decisions/README.md"
        elif file_part.startswith("../"):
            new = "../" + target
        elif file_part and not file_part.startswith("/") and (ROOT / "docs" / file_part).exists():
            new = "../" + target
        if new != target:
            count += 1
        return f"[{label}]({new})"

    return LINK_RE.sub(fix, text), count


def strip_link_targets(text):
    return LINK_RE.sub(lambda m: f"[{m.group(1)}](_)", text)


def cmd_write():
    if (DECISIONS_DIR / "README.md").exists() and "--force" not in sys.argv:
        raise SystemExit("the split has landed: `write` would reset the front matter edited since. "
                         "Use `verify`, or `write --force` to rebuild every entry from the pre-split text.")
    DECISIONS_DIR.mkdir(parents=True, exist_ok=True)
    STATUS_DIR.mkdir(parents=True, exist_ok=True)
    anchors = anchor_map()
    total = 0
    for rows, directory, fields in ((plan_decisions(), DECISIONS_DIR, decision_fields),
                                    (plan_status(), STATUS_DIR, status_fields)):
        for row in rows:
            text, count = rewrite_links(row["text"], directory, anchors)
            total += count
            (directory / f"{row['stem']}.md").write_text(file_text(fields(row), text))
    print(f"wrote {len(plan_decisions())} decisions and {len(plan_status())} status entries; "
          f"rewrote {total} relative links")


def parse_file(path):
    text = path.read_text()
    assert text.startswith("---\n"), f"{path}: no front matter"
    end = text.index("\n---\n", 4)
    fields = {}
    for line in text[4:end].split("\n"):
        key, _, value = line.partition(": ")
        fields[key] = value
    body = text[end + len("\n---\n"):]
    assert body.startswith("\n# "), f"{path}: the body must start with a blank line and an H1"
    return fields, "##" + body[len("\n#"):]


def verify_group(name, rows, directory, original_text, anchors):
    expected = {f"{row['stem']}.md": row for row in rows}
    on_disk = {p.name for p in directory.glob("*.md") if p.name != "README.md"}
    missing = sorted(set(expected) - on_disk)
    if missing:
        raise SystemExit(f"{name}: missing files: {missing}")
    extra = sorted(on_disk - set(expected))
    from_files = {}
    changed_links = 0
    for filename, row in expected.items():
        fields, section = parse_file(directory / filename)
        want, count = rewrite_links(row["text"], directory, anchors)
        changed_links += count
        if section != want:
            raise SystemExit(f"{name}: text of {filename} differs from section {row['index']}")
        if json.loads(fields["title"]) != row["title"] or fields["date"] != row["date"]:
            raise SystemExit(f"{name}: front matter of {filename} disagrees with its heading")
        # Undo the two documented changes (a renamed duplicate heading, the link targets).
        restored = section
        if row["heading"] != split_sections(original_text)[1][row["index"]][0]:
            restored = row["original"]
        from_files[row["index"]] = strip_link_targets(restored)
    _, sections = split_sections(original_text)
    if sorted(from_files) != list(range(len(sections))):
        raise SystemExit(f"{name}: sections and files are not one to one")
    rebuilt = "\n".join(from_files[i] for i in range(len(sections)))
    original_body = strip_link_targets("\n".join(text for _, text in sections))
    if rebuilt != original_body:
        raise SystemExit(f"{name}: the concatenated bodies differ from the original sections")
    renamed = sum(1 for row in rows if row["text"] != row["original"])
    print(f"{name}: {len(sections)} sections -> {len(expected)} files, one to one. Concatenating the bodies "
          f"in order reproduces the original sections, apart from {changed_links} relative link targets "
          f"and {renamed} renamed duplicate heading. {len(extra)} files were added after the split"
          f"{': ' + ', '.join(extra) if extra else ''}.")


def cmd_verify():
    anchors = anchor_map()
    verify_group("decisions", plan_decisions(), DECISIONS_DIR, git_show("docs/decisions.md"), anchors)
    verify_group("status", plan_status(), STATUS_DIR, git_show("docs/status.md"), anchors)
    print("ok")


def main():
    commands = {"plan": cmd_plan, "write": cmd_write, "verify": cmd_verify}
    if len(sys.argv) < 2 or sys.argv[1] not in commands or sys.argv[2:] not in ([], ["--force"]):
        raise SystemExit(__doc__)
    commands[sys.argv[1]]()


if __name__ == "__main__":
    main()
