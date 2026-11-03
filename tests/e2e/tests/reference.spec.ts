import { test, expect, loadCorpus } from "./fixtures";
import { openBookContents, installLibrary, spans, tableRows, tablesOf, waitControlled } from "./helpers";

const corpus = loadCorpus();
const count = (pred: (s: (typeof corpus.catalog.entries)[number]) => boolean) => corpus.catalog.entries.filter(pred).length;

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Book routes, legacy routes, and separate class categories work offline", async ({ page, context }) => {
  expect(corpus.catalog.sources).toHaveLength(16);
  await waitControlled(page);
  await context.setOffline(true);
  await page.goto("#/browse/kind/class");
  await expect(page.getByRole("heading", { name: "Classes", level: 1, exact: true })).toBeVisible();
  await expect(page.locator(".entry-list li")).toHaveCount(count(s => s.kind === "class"));
  await expect(page.locator(".entry-list li").filter({ hasText: /^Fighter/ })).toHaveCount(1);
  await expect(page.locator(".entry-list li").filter({ hasText: /Arcane Archer/ })).toHaveCount(0);
  await page.goto("#/browse/kind/prestige_class");
  await expect(page.getByRole("heading", { name: "Prestige classes", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li")).toHaveCount(count(s => s.kind === "prestige_class"));
  await expect(page.locator(".entry-list li").filter({ hasText: /^Technomancer/ })).toHaveCount(1);
  // The removed standalone Rules category resolves to the Books directory.
  await page.goto("#/browse/kind/rule");
  await expect(page.getByRole("heading", { name: "Books", level: 1 })).toBeVisible();
  await expect(page.locator(".book-directory-row")).toHaveCount(16);
  await page.goto("#/browse/kind/prestige_class");
  await page.getByRole("link", { name: "Arcane Archer", exact: true }).click();
  await expect(page).toHaveURL(/#\/entry\/core-rulebook\/classes\/arcane-archer$/);
  await expect(page.getByRole("heading", { name: "Arcane Archer", level: 1 })).toBeVisible();
  await page.goto("#/search");
  await page.locator(".filters .chip", { hasText: /^Prestige classes/ }).click();
  await expect(page.locator(".results .hit")).toHaveCount(count(s => s.kind === "prestige_class"));
  await expect(page.locator(".results")).toContainText("Technomancer");
  await expect(page.locator(".results")).not.toContainText("Fighter");
  // With schema 3, the source route is the book doorway and opens the original-order contents.
  await page.goto("#/browse/source/ultimate-magic");
  await expect(page.getByRole("heading", { name: "Ultimate Magic", level: 1 })).toBeVisible();
  await expect(page.getByRole("navigation", { name: /Contents of Ultimate Magic/ })).toBeVisible();
  await page.locator(".toc-top > .toc-branch > .toc-details > summary").filter({ hasText: "Spellcasting Class Options" }).click();
  await expect(page.locator(".toc")).toContainText("Magus");
});

test("Browse is category-only and Books is the publication directory", async ({ page }) => {
  const nav = page.getByRole("navigation", { name: "Primary" });
  await nav.getByRole("link", { name: "Browse" }).click();
  await expect(page.getByRole("heading", { name: "Browse", level: 1 })).toBeVisible();

  // Every category tile shows the count that the compiled catalog has.
  const tiles = page.locator("section[aria-labelledby=by-kind] a.cat");
  for (const tile of await tiles.all()) {
    const name = (await tile.locator(".cat-name").innerText()).trim();
    const shown = Number(await tile.locator(".cat-count").innerText());
    const kind = new URL(await tile.evaluate((a: HTMLAnchorElement) => a.href)).hash.split("/").pop()!;
    expect(shown, name).toBe(count((s) => s.kind === kind));
  }

  // The Spells category lists every spell, and the letter jump scrolls to its group.
  await tiles.filter({ has: page.locator(".cat-name", { hasText: /^Spells$/ }) }).click();
  const rows = page.locator(".entry-list li");
  await expect(rows).toHaveCount(count((s) => s.kind === "spell"));
  await page.locator(".letter-btn", { hasText: /^F$/ }).click();
  await expect
    .poll(() => page.evaluate(() => document.getElementById("letter-F")!.getBoundingClientRect().top))
    .toBeLessThan(250);

  // Rules have no standalone Browse category. The former route redirects to Books.
  await page.goto("#/browse/kind/rule");
  await expect(page.getByRole("heading", { name: "Books", level: 1 })).toBeVisible();
  await expect(page.locator(".book-directory-row")).toHaveCount(corpus.catalog.sources.length);
  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Browse" }).click();
  await expect(page.locator("section[aria-labelledby=by-kind] .cat-name", { hasText: /^Rules$/ })).toHaveCount(0);

  // Each installed publication has an expandable row and a direct contents link.
  await page.goto("#/books");
  await page.locator(".book-directory-row").filter({ has: page.locator("summary", { hasText: "Core Rulebook" }) }).locator("summary").click();
  await expect(page.getByRole("link", { name: "Open Core Rulebook contents", exact: true })).toBeVisible();
  await expect(page.locator(".book-directory-chapters li").first()).toBeVisible();
  await page.getByRole("link", { name: "Open Core Rulebook contents", exact: true }).click();
  await expect(page.getByRole("navigation", { name: /Contents of Core Rulebook/ })).toBeVisible();

  // Legacy source routes still reach the book contents when directly bookmarked.
  for (const source of corpus.catalog.sources) {
    await page.goto(`#/browse/source/${source.id}`);
    await expect(page.getByRole("heading", { level: 1, name: source.title })).toBeVisible();
    await expect(page.locator(".toc")).toBeVisible();
    await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();
  }
});

test("Rules text search opens the matched heading and article contents link to real headings", async ({ page }) => {
  await waitControlled(page);
  await page.context().setOffline(true);
  const id = "core-rulebook/rules/combat/special-attacks";
  const entry = corpus.entries.get(id)!;
  const headings = entry.body.filter((block: any) => block.t === "heading" && block.anchor);
  const headingText = (heading: any) => (heading.content ?? []).map((inline: any) => inline.v ?? "").join("");
  expect(headings.length).toBeGreaterThan(0);
  await page.goto("#/search?q=grapple&kind=rule");
  await expect(page.locator(".results .hit-link").first()).toBeVisible();
  const href = await page.locator(".results .hit-link").first().getAttribute("href");
  expect(href).toContain("/special-attacks?a=");
  await page.locator(".results .hit-link").first().click();
  await expect(page.getByRole("heading", { level: 1, name: "Special Attacks" })).toBeVisible();
  await expect(page.locator(".article-contents")).toBeVisible();
  const contents = page.locator(".article-contents");
  await contents.locator("summary").click();
  const anchorIds = headings.map((heading: any) => heading.anchor);
  const tocHrefs = await contents.getByRole("link").evaluateAll((links) => links.map((link) => (link as HTMLAnchorElement).hash));
  for (const anchor of anchorIds) expect(tocHrefs).toContain(`#/entry/${id}?a=${anchor}`);
  const grapple = headings.find((heading: any) => /grapple/i.test(headingText(heading)));
  expect(grapple).toBeTruthy();
  const grappleLink = contents.getByRole("link", { name: new RegExp(headingText(grapple), "i") }).first();
  await grappleLink.click();
  await expect.poll(() => page.evaluate((anchor) => location.hash.includes(`a=${anchor}`), grapple.anchor)).toBe(true);
  await expect.poll(() => page.evaluate((anchor) => {
    const top = document.getElementById(anchor)?.getBoundingClientRect().top ?? -1;
    return top >= 0 && top < innerHeight / 2;
  }, grapple.anchor)).toBe(true);
});

test("Rule entries preserve complex formatting: stat block", async ({ page }) => {
  const fireball = corpus.entries.get("core-rulebook/spells/fireball")!;
  const statBlock = fireball.body.find((b) => b.t === "stat_block");
  await page.goto("#/entry/core-rulebook/spells/fireball");
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  const labels = await page.locator(".stat-block .stat-row dt").allInnerTexts();
  expect(labels.map((l) => l.trim())).toEqual(statBlock.rows.map((r: any) => r.label));
  await expect(page.locator(".stat-block")).toContainText("evocation [fire]");
  await expect(page.locator(".rich")).toContainText("1d6 fire damage per level");
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
  const id = "core-rulebook/rules/combat/actions-in-combat";
  const headings = corpus.entries.get(id)!.body.filter((b) => b.t === "heading" && b.anchor);
  expect(new Set(headings.map((h: any) => h.level)).size).toBeGreaterThan(1);
  await page.goto(`#/entry/${id}`);
  await expect(page.locator(".rich")).toBeVisible();
  const ids = await page.locator(".rich :is(h2,h3,h4,h5,h6)[id]").evaluateAll((els) => els.map((e) => e.id));
  expect(ids.filter((i) => headings.some((h: any) => h.anchor === i))).toEqual(headings.map((h: any) => h.anchor));
  // The deepest anchor of the longest entry opens scrolled into view.
  const cleric = corpus.entries.get("core-rulebook/classes/cleric")!;
  const last = [...cleric.body].reverse().find((b) => b.t === "heading" && b.anchor).anchor;
  await page.goto(`#/entry/core-rulebook/classes/cleric?a=${last}`);
  await expect
    .poll(() => page.evaluate((a) => document.getElementById(a)?.getBoundingClientRect().top ?? -1, last), { timeout: 10_000 })
    .toBeGreaterThanOrEqual(0);
  expect(await page.evaluate((a) => document.getElementById(a)!.getBoundingClientRect().top < innerHeight / 2, last)).toBe(true);
});

test("Cross-reference navigation works", async ({ page }) => {
  await page.goto("#/entry/core-rulebook/spells/summon-monster-ii");
  await expect(page.getByRole("heading", { level: 1, name: /Summon Monster II/ })).toBeVisible();
  const link = page.locator(".rich a.xref[href*='bestiary']").first();
  const text = (await link.innerText()).trim();
  const target = new URL(await link.evaluate((a: HTMLAnchorElement) => a.href)).hash;
  await link.click();
  await expect.poll(() => new URL(page.url()).hash).toBe(target);
  const targetId = target.replace(/^#\/entry\//, "").split("?")[0];
  expect(corpus.entries.has(targetId), `${text} -> ${targetId}`).toBe(true);
  await expect(page.getByRole("heading", { level: 1, name: corpus.entries.get(targetId)!.title })).toBeVisible();
  await expect(page.locator(".stat-block").first()).toBeVisible();

  // A link with an anchor lands on that heading.
  await page.goto("#/entry/core-rulebook/spells/summon-monster-iii");
  await page.locator(".rich a.xref[href*='a=monitor-lizard']").first().click();
  await expect.poll(() => page.evaluate(() => {
    const top = document.getElementById("monitor-lizard")?.getBoundingClientRect().top ?? -1;
    return top >= 0 && top < innerHeight / 2;
  })).toBe(true);
});

test("Back/forward navigation works", async ({ page }) => {
  await page.goto("#/entry/core-rulebook/spells/summon-monster-ii");
  await expect(page.getByRole("heading", { level: 1, name: /Summon Monster II/ })).toBeVisible();
  const origin = page.url();
  await page.locator(".rich a.xref[href*='bestiary']").first().click();
  await expect.poll(() => page.url()).not.toBe(origin);
  await expect(page.locator("h1").first()).not.toHaveText(/Summon Monster II/);
  const second = page.url();
  const title = await page.locator("h1").first().innerText();

  await page.goBack();
  await expect(page.getByRole("heading", { level: 1, name: /Summon Monster II/ })).toBeVisible();
  expect(page.url()).toBe(origin);
  await page.goForward();
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
  expect(page.url()).toBe(second);

  // Search state is part of the history: Back undoes a filter, never a keystroke.
  await page.goto("#/search");
  await expect(page.locator(".search-page")).toBeVisible();
  await page.locator("#search-input").fill("fire");
  await expect(page.locator(".results-region")).toHaveAttribute("data-query", "fire");
  await page.locator(".filters .chip", { hasText: /^Spells/ }).click();
  await expect.poll(() => page.url()).toContain("kind=spell");
  await page.goBack();
  await expect.poll(() => page.url()).not.toContain("kind=spell");
  expect(page.url()).toContain("q=fire");
  await page.goForward();
  await expect.poll(() => page.url()).toContain("kind=spell");
});

test("Cross-reference navigation works: deep link opens an entry in a fresh page", async ({ context }) => {
  const fresh = await context.newPage();
  await fresh.goto("#/entry/core-rulebook/spells/fireball");
  await expect(fresh.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  await expect(fresh).toHaveTitle("Fireball · Cast Iron");
  await expect(fresh.locator(".entry-foot .copyright")).toContainText("Paizo");
  await fresh.goto("#/entry/no-such/entry");
  await expect(fresh.getByText("No entry no-such/entry exists")).toBeVisible();
  await fresh.goto("#/garbage");
  await expect(fresh.getByRole("heading", { name: "Page not found" })).toBeVisible();
});

test("Entry reading puts core rules first and exposes real prestige requirement links", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const id of ["core-rulebook/classes/fighter", "core-rulebook/races/dwarves", "core-rulebook/feats/improved-trip"]) {
    await page.goto(`#/entry/${id}`);
    await expect(page.locator(".entry > .rich")).toBeVisible();
    const positions = await page.evaluate(() => ({
      primary: document.querySelector('.entry > .rich')!.getBoundingClientRect().top,
      related: document.getElementById('entry-related')!.getBoundingClientRect().top,
    }));
    expect(positions.primary).toBeLessThan(844);
    expect(positions.primary).toBeLessThan(positions.related);
  }
  await page.goto("#/entry/core-rulebook/classes/arcane-archer");
  const requirements = page.locator(".rel-requirements");
  for (const title of ["Point-Blank Shot", "Precise Shot", "Weapon Focus"]) {
    await expect(requirements.getByRole("link", { name: title, exact: true })).toBeVisible();
  }
  await expect(page.locator(".crumbs li").last()).toHaveText("Prestige classes");
  await page.locator(".entry-meta").getByRole("link", { name: "Core Rulebook", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Core Rulebook", level: 1 })).toBeVisible();
});
