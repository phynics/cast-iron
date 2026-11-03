#!/usr/bin/env python3
"""Extract book and section links from a saved legacy PRD index page.

Usage: python3 scripts/extract-legacy-sections.py PATH/TO/index.html

Only the #nav-menu navigation is read. Output is deterministic JSON; links are
reported as site navigation evidence, not as printed table-of-contents data.
"""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urljoin
import json
import sys

BASE_URL = "https://legacy.aonprd.com/"


class NavigationParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.nav_depth = 0
        self.div_depth = 0
        self.list_items = []
        self.anchor = None
        self.menus = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "div":
            self.div_depth += 1
            if attrs.get("id") == "nav-menu":
                self.nav_depth = self.div_depth
        if not self.nav_depth:
            return
        if tag == "li":
            self.list_items.append({"depth": len(self.list_items) + 1, "links": [], "children": []})
        elif tag == "a":
            self.anchor = {"href": attrs.get("href", ""), "text": []}

    def handle_data(self, data):
        if self.nav_depth and self.anchor is not None:
            self.anchor["text"].append(data)

    def handle_endtag(self, tag):
        if not self.nav_depth:
            if tag == "div":
                self.div_depth -= 1
            return
        if tag == "a" and self.anchor is not None:
            item = {"href": self.anchor["href"], "label": " ".join("".join(self.anchor["text"]).split())}
            if self.list_items:
                self.list_items[-1]["links"].append(item)
            self.anchor = None
        elif tag == "li" and self.list_items:
            item = self.list_items.pop()
            links = [link for link in item["links"] if link["label"]]
            if item["depth"] == 1:
                if links:
                    heading = links[0]
                    children = item["children"]
                    self.menus.append({"title": heading["label"], "links": children})
            elif item["depth"] >= 2 and links:
                link = links[0]
                if link["href"] and not link["href"].startswith("#"):
                    entry = {"label": link["label"], "url": urljoin(BASE_URL, link["href"])}
                    groups = [ancestor["links"][0]["label"] for ancestor in self.list_items[1:]
                              if ancestor["links"]]
                    if groups:
                        entry["groups"] = groups
                    if self.list_items:
                        self.list_items[0]["children"].append(entry)
        elif tag == "div":
            if self.div_depth == self.nav_depth:
                self.nav_depth = 0
            self.div_depth -= 1


def main(path):
    parser = NavigationParser()
    parser.feed(Path(path).read_text(encoding="utf-8"))
    if not parser.menus:
        raise SystemExit("no book menus found inside #nav-menu")
    result = {
        "source": BASE_URL,
        "evidence": "site_navigation_not_printed_contents",
        "menus": parser.menus,
    }
    print(json.dumps(result, ensure_ascii=False, indent=2) + "\n", end="")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(__doc__.splitlines()[2])
    main(sys.argv[1])
