import { installedTest as test, expect, loadCorpus } from "./fixtures";
import {
  bodyHeadings,
  bodyLinks,
  bodyText,
  bookEntryOrder,
  bookTrees,
  corpusEntry,
  firstEntry,
  firstBookPlacement,
  listedChapterTitles,
  groupLetter,
  hasBlock,
  headingAnchors,
  headingKeys,
  inlineText,
  letterId,
  openBookEntry,
  pathToEntry,
  showAllResults,
  spans,
  tableRows,
  tablesOf,
  installLibrary,
  openBookGroups,
  waitControlled,
  waitResults,
} from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const count = (pred: (s: (typeof corpus.catalog.entries)[number]) => boolean) => corpus.catalog.entries.filter(pred).length;
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const uniqueTitles = (stubs: typeof corpus.catalog.entries) =>
  [...new Set(stubs.map((stub) => stub.title))];

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Book routes, legacy routes, and separate class categories work offline", async ({ page, context }) => {
  const classes = corpus.catalog.entries.filter((stub) => stub.kind === "class");
  const prestige = corpus.catalog.entries.filter((stub) => stub.kind === "prestige_class");
  expect(classes.length).toBeGreaterThan(0);
  expect(prestige.length).toBeGreaterThan(0);
  const oneClass = classes.find((stub) => classes.filter((other) => other.title === stub.title).length === 1)!;
  const onePrestige = prestige.find((stub) => prestige.filter((other) => other.title === stub.title).length === 1)!;
  await waitControlled(page);
  await context.setOffline(true);
  await page.goto("#/browse/kind/class");
  await expect(page.getByRole("heading", { name: "Classes", level: 1, exact: true })).toBeVisible();
  await expect(page.locator('.entry-list a[href^="#/entry/"]')).toHaveCount(classes.length);
  await expect(page.locator(".entry-list li").filter({ hasText: new RegExp(`^${escape(oneClass.title)}`) })).toHaveCount(1);
  await expect(page.locator(".entry-list li").filter({ hasText: new RegExp(`^${escape(onePrestige.title)}`) })).toHaveCount(0);
  await page.goto("#/browse/kind/prestige_class");
  await expect(page.getByRole("heading", { name: "Prestige classes", level: 1 })).toBeVisible();
  await expect(page.locator('.entry-list a[href^="#/entry/"]')).toHaveCount(prestige.length);
  await expect(page.locator(".entry-list li").filter({ hasText: new RegExp(`^${escape(onePrestige.title)}`) })).toHaveCount(1);
  // The removed standalone Rules category resolves to the Books directory.
  await page.goto("#/browse/kind/rule");
  await expect(page.getByRole("heading", { name: "Books", level: 1 })).toBeVisible();
  // Closed groups build their rows when they open, so open every group before counting.
  await openBookGroups(page);
  await expect(page.locator(".book-directory-row")).toHaveCount(corpus.catalog.sources.length);
  await page.goto("#/browse/kind/prestige_class");
  await page.getByRole("link", { name: onePrestige.title, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${escape(onePrestige.id)}$`));
  await expect(page.getByRole("heading", { name: onePrestige.title, level: 1 })).toBeVisible();

  // The Prestige classes filter returns only that kind, grouped by title.
  await page.goto("#/search");
  await page.locator(".filters .chip", { hasText: /^Prestige classes/ }).click();
  await showAllResults(page);
  const grouped = uniqueTitles(prestige).length;
  await expect(page.locator(".results .hit")).toHaveCount(grouped);
  await expect(page.locator(".results")).toContainText(onePrestige.title);
  const hrefs = await page.locator(".results .hit-link").evaluateAll((nodes) =>
    nodes.map((node) => (node as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]));
  expect(hrefs.length).toBeGreaterThan(0);
  for (const id of hrefs) expect(corpus.entries.get(id)?.kind, id).toBe("prestige_class");

  // With schema 3, the source route is the book doorway and opens the original-order contents.
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => chapter.children?.length))!;
  await page.goto(`#/browse/source/${encodeURIComponent(book.source)}`);
  await expect(page.getByRole("heading", { name: book.title, level: 1 })).toBeVisible();
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) })).toBeVisible();
  const topTitles = await page.locator(".toc-top > li").evaluateAll((items) => items.map((item) => {
    const branch = item.querySelector(".toc-title");
    const leaf = item.querySelector("a");
    return (branch?.textContent ?? leaf?.textContent ?? "").trim();
  }));
  expect(topTitles).toEqual(listedChapterTitles(book));
  await page.locator(".toc-top > .toc-branch > .toc-details > summary").first().click();
  await expect(page.locator(".toc-top > .toc-branch > .toc-details > ul > li").first()).toBeVisible();
});

test("Reviewed primary-source passage appears with its attribution in the rules", async ({ page, context }) => {
  // The passage is content, not a route: find the entry the compiler gave the reviewed text.
  const passage = firstEntry("the reviewed magic passage", (entry) =>
    bodyText(entry).includes("From creating a wisp of light"));
  const sourceLine = "PRPG Core Rulebook pg 206";
  expect(bodyText(passage)).toContain(sourceLine);
  const casting = firstEntry("the Casting Spells rule", (entry) =>
    entry.kind === "rule" && entry.title === "Casting Spells");
  const book = firstBookPlacement(passage.id);
  expect(book, "the passage is placed in a book").toBeTruthy();
  expect(pathToEntry(book!.chapters, casting.id), "Casting Spells is in the same book").not.toBeNull();

  await waitControlled(page);
  await context.setOffline(true);
  await openBookEntry(page, book!, passage.id);
  await expect(page.getByRole("heading", { level: 1, name: passage.title })).toBeVisible();
  await expect(page.getByText("From creating a wisp of light", { exact: false })).toBeVisible();
  await expect(page.getByText(sourceLine, { exact: false })).toBeVisible();
  await openBookEntry(page, book!, casting.id);
  await expect(page.getByRole("heading", { level: 1, name: casting.title })).toBeVisible();
});

test("Home is category-only and Books is the publication directory", async ({ page }) => {
  const nav = page.getByRole("navigation", { name: "Primary" });
  await nav.getByRole("link", { name: "Home", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Your rules reference", level: 1 })).toBeVisible();

  // Every category tile shows the count that the compiled catalog has.
  const tiles = page.locator("section[aria-labelledby=by-kind] a.cat");
  for (const tile of await tiles.all()) {
    const name = (await tile.locator(".cat-name").innerText()).trim();
    const shown = Number((await tile.locator(".cat-count").innerText()).replace(/,/g, ""));
    const kind = new URL(await tile.evaluate((a: HTMLAnchorElement) => a.href)).hash.split("/").pop()!;
    expect(shown, name).toBe(count((s) => s.kind === kind));
  }

  // The Spells category opens one letter group at a time; the jump bar moves between them.
  const spells = corpus.catalog.entries.filter((stub) => stub.kind === "spell");
  const letters = [...new Set(spells.map((stub) => groupLetter(stub.title)))].sort(
    (a, b) => Number(a === "#") - Number(b === "#") || a.localeCompare(b),
  );
  const firstLetter = letters[0];
  const firstId = letterId(firstLetter);
  const firstGroup = spells.filter((stub) => groupLetter(stub.title) === firstLetter);
  await tiles.filter({ has: page.locator(".cat-name", { hasText: /^Spells$/ }) }).click();
  const jump = page.getByRole("navigation", { name: "Jump to letter" });
  await expect(jump).toBeVisible();
  await expect(jump.getByRole("link")).toHaveCount(letters.length);
  await expect(page.locator(`#${firstId} .entry-list a[href^="#/entry/"]`)).toHaveCount(firstGroup.length);
  // Other groups stay collapsed behind their heading link.
  const targetLetter = letters.find((letter) => letter !== firstLetter)!;
  const targetId = letterId(targetLetter);
  const targetGroup = spells.filter((stub) => groupLetter(stub.title) === targetLetter);
  const collapsed = page.locator(`#${targetId} .letter-head-link`);
  await expect(collapsed).toHaveCount(1);
  await expect(collapsed).toHaveAccessibleName(new RegExp(`^${targetLetter}, ${targetGroup.length} entr`));
  await jump.getByRole("link", { name: `Jump to ${targetLetter}`, exact: true }).click();
  await expect.poll(() => new URL(page.url()).hash).toContain(`l=${encodeURIComponent(targetLetter)}`);
  await expect(page.locator(`#${targetId} .letter-head`)).toBeInViewport();
  await expect(page.locator(`#${targetId} .letter-head`)).toBeFocused();
  await expect(page.locator(`#${firstId} .letter-head-link`)).toBeVisible();

  // Rules have no standalone Browse category. The former route redirects to Books.
  await page.goto("#/browse/kind/rule");
  await expect(page.getByRole("heading", { name: "Books", level: 1 })).toBeVisible();
  // Closed groups build their rows when they open, so open every group before counting.
  await openBookGroups(page);
  await expect(page.locator(".book-directory-row")).toHaveCount(corpus.catalog.sources.length);
  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Home", exact: true }).click();
  await expect(page.locator("section[aria-labelledby=by-kind] .cat-name", { hasText: /^Rules$/ })).toHaveCount(0);

  // Each installed publication has one row that links to its book page.
  const book = bookTrees().find((candidate) => candidate.chapters.length > 0)!;
  await page.goto("#/books");
  await openBookGroups(page);
  const row = page.locator("a.book-directory-row").filter({ has: page.locator(".cat-name", { hasText: new RegExp(`^${escape(book.title)}$`) }) });
  await expect(page.locator(".book-tree-root")).toHaveCount(0);
  await row.click();
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${escape(book.title)}`) })).toBeVisible();

  // Legacy source routes still reach the book contents when directly bookmarked. One route
  // renders every source the same way, so a spread sample (first, last, and evenly spaced)
  // checks the route without opening all of the library's publications.
  // The sample also pins the edge cases a regression for one publication is most likely to hit:
  // the fewest and most top-level chapters, the most entries, the longest id, and a source
  // without a contents tree when the library has one.
  const sources = corpus.catalog.sources;
  expect(sources.length).toBeGreaterThan(0);
  const step = Math.max(1, Math.floor(sources.length / 8));
  const trees = new Map(bookTrees().map((tree) => [tree.source, tree]));
  const placed = sources.filter((source) => trees.has(source.id));
  expect(placed.length, "the library places sources in books.json").toBeGreaterThan(0);
  const extreme = (score: (id: string) => number) =>
    [...placed].sort((a, b) => score(a.id) - score(b.id) || a.id.localeCompare(b.id));
  const byChapters = extreme((id) => trees.get(id)!.chapters.length);
  const byEntries = extreme((id) => bookEntryOrder(trees.get(id)!).length);
  const longest = [...sources].sort((a, b) => b.id.length - a.id.length || a.id.localeCompare(b.id))[0];
  const sample = [...new Set([
    ...sources.filter((_, i) => i % step === 0),
    sources[sources.length - 1],
    byChapters[0],
    byChapters[byChapters.length - 1],
    byEntries[byEntries.length - 1],
    longest,
    sources.find((source) => !trees.has(source.id)),
  ].filter((source): source is (typeof sources)[number] => source !== undefined))];
  for (const source of sample) {
    await page.goto(`#/browse/source/${encodeURIComponent(source.id)}`);
    await expect(page.getByRole("heading", { level: 1, name: source.title })).toBeVisible();
    await expect(page.locator(".toc")).toBeVisible();
    await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();
  }
});

test("Rules text search opens the matched heading and article contents link to real headings", async ({ page }) => {
  await waitControlled(page);
  await page.context().setOffline(true);
  await page.goto("#/search?q=grapple&kind=rule");
  await waitResults(page, "grapple");
  await expect(page.locator(".results .hit").first()).toBeVisible();
  const anchored = page.locator(".results .hit-link[href*='?a=']:visible").first();
  await expect(anchored).toBeVisible();
  const href = (await anchored.getAttribute("href"))!;
  const id = href.replace(/^#\/entry\//, "").split("?")[0];
  const anchor = new URLSearchParams(href.split("?")[1]).get("a")!;
  const entry = corpus.entries.get(id)!;
  const headings = headingAnchors(entry);
  expect(headings.length).toBeGreaterThan(0);
  expect(headings.some((heading) => heading.anchor === anchor)).toBe(true);
  await anchored.click();
  await expect(page.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
  await expect(page.locator(".article-contents")).toBeVisible();
  const contents = page.locator(".article-contents");
  await contents.locator("summary").click();
  const tocHrefs = await contents.getByRole("link").evaluateAll((links) => links.map((link) => (link as HTMLAnchorElement).hash));
  for (const heading of headings) expect(tocHrefs).toContain(`#/entry/${id}?a=${heading.anchor}`);
  await contents.getByRole("link").filter({ hasText: new RegExp(escape(headings.find((heading) => heading.anchor === anchor)!.text), "i") }).first().click();
  await expect.poll(() => page.evaluate((value) => location.hash.includes(`a=${value}`), anchor)).toBe(true);
  await expect.poll(() => page.evaluate((value) => {
    const top = document.getElementById(`h-${value}`)?.getBoundingClientRect().top ?? -1;
    return top >= 0 && top < innerHeight / 2;
  }, anchor)).toBe(true);
});

test("A class page's article contents lists sections, not its source books", async ({ page }) => {
  const wizard = corpusEntry("class", "Wizard");
  const publicationTitles = new Set(corpus.catalog.sources.map((source: { title: string }) => source.title));
  await page.goto(`#/entry/${wizard.id}`);
  await expect(page.getByRole("heading", { level: 1, name: "Wizard" })).toBeVisible();
  const contents = page.locator(".article-contents");
  await expect(contents).toBeVisible();
  await contents.locator("summary").click();
  const labels = (await contents.getByRole("link").allInnerTexts()).map((label) => label.trim());
  expect(labels).toEqual(expect.arrayContaining([
    "Class Features and Spells Per Day",
    "Alternate Capstones",
    "Archetypes",
    "Archetypes by Feature",
  ]));
  // A source attribution heading is not a section of the article.
  expect(labels.filter((label) => publicationTitles.has(label))).toEqual([]);
});

test("Rule entries preserve complex formatting: stat block", async ({ page }) => {
  const spell = firstEntry("a spell with a stat block and a school", (entry) =>
    entry.kind === "spell" && hasBlock(entry, "stat_block") && entry.fields?.school);
  const statBlock = spell.body.find((block: any) => block.t === "stat_block");
  const paragraph = spell.body.find((block: any) => block.t === "paragraph" &&
    inlineText(block.content ?? []).length > 60);
  const expected = inlineText(paragraph.content).trim().slice(0, 60);
  await page.goto(`#/entry/${spell.id}`);
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  // A retrying assertion: one allInnerTexts() read can catch the block while it still renders.
  await expect(page.locator(".stat-block .stat-row dt")).toHaveText(statBlock.rows.map((row: any) => row.label));
  await expect(page.locator(".stat-block")).toContainText(spell.fields.school);
  await expect(page.locator(".rich")).toContainText(expected);
});

test("Rule entries preserve complex formatting: spell levels keep their groups", async ({ page }) => {
  const spell = firstEntry("a spell with several class levels", (entry) =>
    entry.kind === "spell" &&
    (entry.fields?.levels?.length ?? 0) >= 3 &&
    new Set(entry.fields.levels.map((level: any) => level.level)).size >= 2);
  const levels = [...new Set(spell.fields.levels.map((level: any) => level.level))] as number[];
  const classNames = spell.fields.levels.map((level: any) => level.class);
  await page.goto(`#/entry/${spell.id}`);
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  const row = page.locator(".stat-block .stat-row").filter({ has: page.locator("dt", { hasText: /^Level$/ }) });
  await expect(row).toHaveCount(1);
  const value = row.locator("dd");
  await expect(value.locator("br")).toHaveCount(levels.length - 1);
  const text = await value.innerText();
  for (const level of levels) expect(text).toContain(`${level === 0 ? "Cantrips" : level}: `);
  for (const name of classNames) expect(text).toContain(name);
});

test("Level 0 spells read as Cantrips on the spell page and the Wizard list", async ({ page }) => {
  const spell = firstEntry("a level-0 Wizard spell", (entry) =>
    entry.kind === "spell" && entry.fields?.levels?.some((level: any) => level.class === "Wizard" && level.level === 0));
  await page.goto(`#/entry/${spell.id}`);
  // The printed stat-block row is rendered from the typed levels, so it says Cantrips.
  const row = page.locator(".stat-block .stat-row").filter({ has: page.locator("dt", { hasText: /^Level$/ }) });
  const value = row.locator("dd");
  await expect(value).toContainText(/^Cantrips: /);
  await expect(value).not.toContainText(/\b0: /);
  await expect(value).toContainText("Wizard");
  const lists = page.locator(".entry-rel section").filter({ has: page.getByRole("heading", { name: "On spell lists" }) });
  const wizard = lists.getByRole("link", { name: "Wizard cantrip", exact: true });
  await expect(wizard).toBeVisible();
  // The route keeps the number even though the label is a word.
  await expect(wizard).toHaveAttribute("href", /#\/spells\/Wizard\/0$/);

  await page.goto("#/spells/Wizard/0");
  await expect(page.getByRole("heading", { level: 1, name: "Wizard cantrips" })).toBeVisible();
  const tab = page.locator(".level-nav").getByRole("link", { name: /^Cantrips,/ });
  await expect(tab).toHaveAttribute("aria-current", "page");
  await expect(tab.locator(".lvl-n")).toHaveText("Cantrips");
  // The group keeps its numeric value for CSS and links.
  await expect(page.locator(".level-group")).toHaveAttribute("data-level", "0");
});

test("Rule entries preserve complex formatting: creature spell lists use readable level labels", async ({ page }) => {
  const creature = firstEntry("a creature with a class/level spell list", (entry) =>
    entry.kind === "creature" && entry.body.some((block: any) => {
      if (block.t !== "paragraph" || block.content?.[0]?.t !== "strong") return false;
      const label = inlineText(block.content[0].c ?? []).replace(/:$/, "");
      return /^\d+(st|nd|rd|th)-level spells$/.test(label);
    }));
  const labelBlock = creature.body.find((block: any) => {
    if (block.t !== "paragraph" || block.content?.[0]?.t !== "strong") return false;
    const label = inlineText(block.content[0].c ?? []).replace(/:$/, "");
    return /^\d+(st|nd|rd|th)-level spells$/.test(label);
  });
  const label = inlineText(labelBlock.content[0].c).replace(/:$/, "");
  await page.goto(`#/entry/${creature.id}`);
  await expect(page.getByRole("heading", { level: 1, name: creature.title })).toBeVisible();
  await expect(page.locator(".rich")).toContainText(label);
  await expect(page.locator(".rich")).not.toContainText(/\bl[0-9]:/);
});

test("Rule entries preserve complex formatting: the largest table keeps its cells and spans", async ({ page }) => {
  let best: { id: string; index: number; table: any; cells: number } | null = null;
  for (const entry of corpus.entries.values()) {
    tablesOf(entry.body).forEach((table, index) => {
      const cells = tableRows(table).reduce((n, r) => n + r.cells.length, 0);
      if (!best || cells > best.cells) best = { id: entry.id, index, table, cells };
    });
  }
  const target = best!;
  const rows = tableRows(target.table);
  const expected = {
    cells: target.cells,
    spanning: rows.reduce((n, r) => n + r.cells.filter(spans).length, 0),
    area: rows.reduce((n, r) => n + r.cells.reduce((a: number, c: any) => a + (c.colspan ?? 1) * (c.rowspan ?? 1), 0), 0),
  };
  await page.goto(`#/entry/${target.id}`);
  const table = page.locator(".rules-table").nth(target.index);
  await expect(table).toBeVisible();
  const actual = await table.evaluate((t: HTMLTableElement) => {
    const cells = [...t.querySelectorAll("td,th")] as HTMLTableCellElement[];
    return {
      cells: cells.length,
      spanning: cells.filter((c) => c.colSpan > 1 || c.rowSpan > 1).length,
      area: cells.reduce((a, c) => a + c.colSpan * c.rowSpan, 0),
    };
  });
  expect(actual).toEqual(expected);
  // The wide table scrolls inside its own labelled region, never the page.
  const wrap = table.locator("xpath=..");
  await expect(wrap).toHaveAttribute("role", "region");
  await expect(wrap).toHaveAttribute("tabindex", "0");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test("Rule entries preserve complex formatting: nested headings with anchors", async ({ page }) => {
  const entry = entries
    .filter((candidate) => new Set(headingAnchors(candidate).map((heading) => heading.level)).size > 1)
    .sort((a, b) => headingAnchors(b).length - headingAnchors(a).length)[0];
  const headings = headingAnchors(entry);
  await page.goto(`#/entry/${entry.id}`);
  await expect(page.locator(".rich")).toBeVisible();
  const ids = await page.locator(".rich :is(h2,h3,h4,h5,h6)[id]").evaluateAll((els) => els.map((e) => e.id));
  expect(ids).toEqual(headings.map((heading) => `h-${heading.anchor}`));
  // The deepest anchor of the entry with the most anchors opens scrolled into view.
  const longest = [...entries].sort((a, b) => headingAnchors(b).length - headingAnchors(a).length)[0];
  const last = headingAnchors(longest).at(-1)!.anchor;
  await page.goto(`#/entry/${longest.id}?a=${last}`);
  await expect
    .poll(() => page.evaluate((a) => document.getElementById(`h-${a}`)?.getBoundingClientRect().top ?? -1, last), { timeout: 10_000 })
    .toBeGreaterThanOrEqual(0);
  expect(await page.evaluate((a) => document.getElementById(`h-${a}`)!.getBoundingClientRect().top < innerHeight / 2, last)).toBe(true);
});

test("Duplicate heading keys scroll to their own section and old anchor URLs still work", async ({ page }) => {
  // Find an article with repeated unanchored heading text, so the reader must give each
  // rendered heading its own suffixed key. Every key below comes from the compiled body.
  const entry = entries.find((candidate) => {
    const headings = bodyHeadings(candidate).filter((heading) => heading.text.trim());
    const texts = headings.filter((heading) => !heading.anchor).map((heading) => heading.text.trim());
    // Article contents starts at four headings; the repeated text needs a suffixed key.
    return headings.length >= 4 && texts.length >= 2 && new Set(texts).size < texts.length;
  })!;
  expect(entry, "an article with repeated heading text").toBeTruthy();
  const keyed = headingKeys(entry);
  const unanchored = bodyHeadings(entry).map((heading, i) => ({ ...heading, key: keyed[i].key }));
  const texts = unanchored.filter((heading) => !heading.anchor && heading.text.trim()).map((heading) => heading.text.trim());
  const repeated = texts.find((text) => texts.filter((other) => other === text).length > 1)!;
  const anchors = unanchored
    .filter((heading) => !heading.anchor && heading.text.trim() === repeated)
    .map((heading) => heading.key!);
  expect(anchors.length).toBeGreaterThan(1);
  expect(new Set(anchors).size).toBe(anchors.length);

  await page.goto(`#/entry/${entry.id}`);
  await expect(page.locator(".rich")).toBeVisible();
  // The rendered ids are exactly the keys the corpus implies, in order.
  const ids = await page.locator(".rich :is(h2,h3,h4,h5,h6)[id]").evaluateAll((els) => els.map((e) => e.id));
  expect(ids).toEqual(keyed.filter((heading) => heading.key !== null).map((heading) => `h-${heading.key}`));
  const contents = page.locator(".article-contents");
  await expect(contents).toBeVisible();
  const links = contents.getByRole("link").filter({ hasText: new RegExp(`^${escape(repeated)}$`) });
  await contents.locator("summary").click();
  expect(await links.count()).toBe(anchors.length);
  for (let i = 0; i < anchors.length; i++) {
    const href = (await links.nth(i).getAttribute("href"))!;
    expect(new URLSearchParams(href.split("?")[1]).get("a")).toBe(anchors[i]);
  }
  // Every duplicate scrolls into view from the contents list.
  for (const anchor of anchors) {
    if (!(await contents.evaluate((e: HTMLDetailsElement) => e.open))) await contents.locator("summary").click();
    await contents.locator(`a[href="#/entry/${entry.id}?a=${anchor}"]`).click();
    await expect(page.locator(`#h-${anchor}`)).toBeInViewport();
    expect(await page.evaluate((key) => {
      const top = document.getElementById(`h-${key}`)?.getBoundingClientRect().top ?? -1;
      return top >= 0 && top < innerHeight / 2;
    }, anchor)).toBe(true);
  }
  // A direct URL for every duplicate key lands on that heading, the first included.
  for (const anchor of anchors) {
    await page.goto(`#/entry/${entry.id}?a=${anchor}`);
    await expect(page.locator(`#h-${anchor}`)).toBeInViewport();
    await expect.poll(() => page.evaluate((key) => {
      const top = document.getElementById(`h-${key}`)?.getBoundingClientRect().top ?? -1;
      return top >= 0 && top < innerHeight / 2;
    }, anchor)).toBe(true);
  }
});

test("Cross-reference navigation works", async ({ page }) => {
  // A typed link from one entry body to another that carries a stat block.
  const pair = entries.map((from) => ({
    from,
    link: bodyLinks(from.body).find((link) => {
      const target = corpus.entries.get(link.target);
      return target && target.id !== from.id && hasBlock(target, "stat_block");
    }),
  })).find((candidate) => candidate.link)!;
  const target = corpus.entries.get(pair.link!.target)!;
  await page.goto(`#/entry/${pair.from.id}`);
  await expect(page.getByRole("heading", { level: 1, name: pair.from.title })).toBeVisible();
  const link = page.locator(`.entry > .rich a[href^="#/entry/${target.id}"]`).first();
  await expect(link).toBeVisible();
  const text = (await link.innerText()).trim();
  await link.click();
  await expect.poll(() => new URL(page.url()).hash).toContain(`#/entry/${target.id}`);
  expect(text.length).toBeGreaterThan(0);
  await expect(page.getByRole("heading", { level: 1, name: target.title })).toBeVisible();
  await expect(page.locator(".stat-block").first()).toBeVisible();

  // A link with an anchor lands on that heading.
  const anchored = entries.map((from) => ({
    from,
    link: bodyLinks(from.body).find((link) => {
      const target = corpus.entries.get(link.target);
      return target && link.anchor && headingAnchors(target).some((heading) => heading.anchor === link.anchor);
    }),
  })).find((candidate) => candidate.link)!;
  await page.goto(`#/entry/${anchored.from.id}`);
  await page.locator(`.entry > .rich a[href*="#/entry/${anchored.link!.target}?a=${anchored.link!.anchor}"]`).first().click();
  await expect.poll(() => page.evaluate((value) => {
    const top = document.getElementById(`h-${value}`)?.getBoundingClientRect().top ?? -1;
    return top >= 0 && top < innerHeight / 2;
  }, anchored.link!.anchor)).toBe(true);
});

test("Back/forward navigation works", async ({ page }) => {
  const pair = entries.map((from) => ({
    from,
    link: bodyLinks(from.body).find((link) => {
      const target = corpus.entries.get(link.target);
      return target && target.id !== from.id && hasBlock(target, "stat_block");
    }),
  })).find((candidate) => candidate.link)!;
  const target = corpus.entries.get(pair.link!.target)!;
  await page.goto(`#/entry/${pair.from.id}`);
  await expect(page.getByRole("heading", { level: 1, name: pair.from.title })).toBeVisible();
  const origin = page.url();
  await page.locator(`.entry > .rich a[href^="#/entry/${target.id}"]`).first().click();
  await expect.poll(() => page.url()).not.toBe(origin);
  await expect(page.locator("h1").first()).toHaveText(new RegExp(escape(target.title)));
  const second = page.url();
  const title = await page.locator("h1").first().innerText();

  await page.goBack();
  await expect(page.getByRole("heading", { level: 1, name: pair.from.title })).toBeVisible();
  expect(page.url()).toBe(origin);
  await page.goForward();
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
  expect(page.url()).toBe(second);

  // Search state is part of the history: Back undoes a filter, never a keystroke.
  const spell = corpusEntry("spell", "Fireball");
  await page.goto("#/search");
  await expect(page.locator(".search-page")).toBeVisible();
  await page.locator("#search-input").fill(spell.title);
  await expect(page.locator(".results-region")).toHaveAttribute("data-query", spell.title);
  await page.locator(".filters .chip", { hasText: /^Spells/ }).click();
  await expect.poll(() => page.url()).toContain("kind=spell");
  await page.goBack();
  await expect.poll(() => page.url()).not.toContain("kind=spell");
  expect(page.url()).toContain(`q=${encodeURIComponent(spell.title)}`);
  await page.goForward();
  await expect.poll(() => page.url()).toContain("kind=spell");
});

test("Cross-reference navigation works: deep link opens an entry in a fresh page", async ({ context }) => {
  const spell = corpusEntry("spell", "Fireball");
  const stub = corpus.catalog.entries.find((candidate) => candidate.id === spell.id)!;
  const source = corpus.catalog.sources.find((candidate) => candidate.id === stub.source)!;
  const fresh = await context.newPage();
  await fresh.goto(`#/entry/${spell.id}`);
  await expect(fresh.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  await expect(fresh).toHaveTitle(`${spell.title} · Cast Iron`);
  await expect(fresh.locator(".entry-foot .copyright").first()).toContainText(
    source.copyright.includes("Paizo") ? "Paizo" : source.copyright.slice(0, 30));
  await fresh.goto("#/entry/no-such/entry");
  await expect(fresh.getByText("No entry no-such/entry exists")).toBeVisible();
  await fresh.goto("#/garbage");
  await expect(fresh.getByRole("heading", { name: "Page not found" })).toBeVisible();
});

test("Entry reading puts rules first and exposes requirement links", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const kind of ["class", "race", "feat"]) {
    const entry = firstEntry(`a ${kind}`, (candidate) => candidate.kind === kind);
    await page.goto(`#/entry/${entry.id}`);
    // A hash route keeps the previous entry on screen until the new one renders. Wait for the
    // new title, or the measurement can run while the body is being replaced.
    await expect(page.getByRole("heading", { level: 1, name: entry.title, exact: true })).toBeVisible();
    await expect(page.locator(".entry > .rich")).toBeVisible();
    const positions = await page.evaluate(() => ({
      primary: document.querySelector('.entry > .rich')!.getBoundingClientRect().top,
      related: document.getElementById('entry-related')!.getBoundingClientRect().top,
    }));
    expect(positions.primary).toBeLessThan(844);
    expect(positions.primary).toBeLessThan(positions.related);
  }

  // A feat's printed requirements include named feats, and the page links each one.
  const feat = firstEntry("a feat with named prerequisite links", (entry) =>
    entry.kind === "feat" && (entry.relations ?? []).some((relation: any) => relation.kind === "prerequisite"));
  await page.goto(`#/entry/${feat.id}`);
  const requirements = page.locator(".rel-prereq");
  for (const relation of (feat.relations ?? []).filter((candidate: any) => candidate.kind === "prerequisite")) {
    const target = corpus.entries.get(relation.target);
    if (!target) continue;
    await expect(requirements.getByRole("link", { name: target.title, exact: true }).first()).toBeVisible();
  }

  // A prestige class keeps its category crumb and its publication link.
  const prestige = firstEntry("a prestige class", (entry) => entry.kind === "prestige_class");
  const stub = corpus.catalog.entries.find((candidate) => candidate.id === prestige.id)!;
  const source = corpus.catalog.sources.find((candidate) => candidate.id === stub.source)!;
  await page.goto(`#/entry/${prestige.id}`);
  await expect(page.locator(".crumbs li").last()).toHaveText("Prestige classes");
  // Books beyond the first open from the "+N more" disclosure.
  const meta = page.locator(".entry-meta");
  const bookLink = meta.getByRole("link", { name: source.title, exact: true });
  await expect(meta.getByRole("link").first()).toBeVisible();
  const more = meta.locator(".meta-more");
  if ((await more.count()) > 0) await more.locator("summary").click();
  await expect(bookLink).toBeVisible();
  await bookLink.click();
  await expect(page.getByRole("heading", { name: source.title, level: 1 })).toBeVisible();
});
