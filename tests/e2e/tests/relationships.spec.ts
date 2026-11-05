import { test, expect, loadCorpus } from "./fixtures";
import { bookTrees, installLibrary, openBookContents, waitControlled } from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const related = (entry: (typeof entries)[number], kind: string) =>
  entry.relations?.filter((r: any) => r.kind === kind) ?? [];

test.beforeEach(async ({ page, context }) => {
  await installLibrary(page);
  await waitControlled(page);
  await context.setOffline(true);
});

test("race racial traits and class archetypes link in both directions", async ({ page }) => {
  const trait = entries.find((e) => e.kind === "racial_trait" && related(e, "part").length > 0)!;
  const traitId = trait.id;
  const race = corpus.entries.get(related(trait, "part")[0].target)!;
  await page.goto(`#/entry/${race.id}`);
  await expect(page.getByRole("heading", { name: race.title, level: 1 })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Racial traits/ })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: trait.title, exact: true }).first().click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${traitId.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("heading", { name: trait.title, level: 1 })).toBeVisible();
  await expect(page.getByRole("heading", { name: /racial trait of/i })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: race.title, exact: true }).first().click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${race.id.replaceAll("/", "\\/")}$`));

  const classEntry = entries.find((e) => e.kind === "class" &&
    entries.some((a) => a.kind === "archetype" && related(a, "archetype").some((r: any) => r.target === e.id)))!;
  const archetype = entries.find((a) => a.kind === "archetype" && related(a, "archetype").some((r: any) => r.target === classEntry.id))!;
  await page.goto(`#/entry/${classEntry.id}`);
  await expect(page.locator(".entry-rel").getByRole("heading", { name: /Archetypes/ })).toBeVisible();
  await page.locator(".rel-disclosure > summary").click();
  const archetypeGroup = page.locator(".rel-group").filter({ has: page.getByRole("link", { name: archetype.title, exact: true }) });
  if ((await archetypeGroup.getAttribute("open")) === null) await archetypeGroup.locator("summary").click();
  await page.locator(".entry-rel").getByRole("link", { name: archetype.title, exact: true }).first().click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${archetype.id.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("heading", { name: /An archetype of/ })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: classEntry.title, exact: true }).first().click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${classEntry.id.replaceAll("/", "\\/")}$`));
});

test("spell lists filter by class, level, and query; search supports source filters", async ({ page }) => {
  const classLevels = new Map<string, number>();
  for (const entry of entries) {
    for (const level of entry.fields?.levels ?? []) classLevels.set(level.class, (classLevels.get(level.class) ?? 0) + 1);
  }
  const listClass = [...classLevels.entries()].sort((a, b) => b[1] - a[1])[0][0];
  const spell = entries.find((entry) => entry.kind === "spell" &&
    entry.fields?.levels?.some((level: any) => level.class === listClass))!;
  const level = spell.fields.levels.find((level: any) => level.class === listClass).level;
  await page.goto(`#/spells/${encodeURIComponent(listClass)}`);
  await expect(page.getByRole("heading", { name: `${listClass} spells`, level: 1 })).toBeVisible();
  await page.locator("#list-filter").fill(spell.title);
  await expect(page.locator(".entry-list").getByRole("link", { name: spell.title, exact: true }).first()).toBeVisible();
  const matching = entries.filter((entry) => entry.kind === "spell" &&
    entry.title.toLowerCase().includes(spell.title.toLowerCase()) &&
    entry.fields?.levels?.some((level: any) => level.class === listClass));
  await expect(page.locator('.entry-list a[href^="#/entry/"]')).toHaveCount(matching.length);
  await page.locator("#list-filter").fill("no-such-spell");
  await expect(page.getByRole("status").filter({ hasText: /No spell on this list matches/ })).toBeVisible();

  await page.goto(`#/spells/${encodeURIComponent(listClass)}/${level}`);
  await expect(page.getByRole("heading", { name: `${listClass} spells, level ${level}`, level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list").getByRole("link", { name: spell.title, exact: true }).first()).toBeVisible();
  await expect(page.locator(".level-group")).toHaveAttribute("data-level", String(level));

  const stub = corpus.catalog.entries.find((candidate) => candidate.id === spell.id)!;
  await page.goto(`#/search?q=${encodeURIComponent(spell.title)}&kind=spell&source=${encodeURIComponent(stub.source)}`);
  await expect(page.locator(".results-region")).toHaveAttribute("data-query", spell.title);
  await expect(page.locator(".results .hit").first()).toBeVisible();
  await expect(page.locator(".results .hit-title").filter({ hasText: new RegExp(`^${spell.title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) }).first()).toBeVisible();
  const resultHrefs = await page.locator(".results .hit-link").evaluateAll((nodes) => nodes.map((n) => (n as HTMLAnchorElement).hash));
  expect(resultHrefs.length).toBeGreaterThan(0);
  for (const href of resultHrefs) {
    const id = href.replace(/^#\/entry\//, "").split("?")[0];
    expect(corpus.catalog.entries.find((e) => e.id === id)?.source).toBe(stub.source);
  }
});

test("feat pages preserve printed prerequisites and label alternatives honestly", async ({ page }) => {
  const dependedOn = new Set(entries.flatMap((e) => related(e, "prerequisite").map((r: any) => r.target)));
  const feat = entries.find((e) => e.kind === "feat" && e.fields.prerequisites &&
    related(e, "prerequisite").some((r: any) => r.label === "or") && dependedOn.has(e.id))!;
  expect(feat, "a feat with an either-or prerequisite that other feats name").toBeTruthy();
  await page.goto(`#/entry/${feat.id}`);
  await expect(page.locator("#rel-prereq")).toHaveText("Requirements");
  await expect(page.locator(".prereq-text")).toContainText(feat.fields.prerequisites);
  await expect(page.getByText("Feats it names", { exact: true })).toBeVisible();
  const expected = related(feat, "prerequisite");
  for (const edge of expected) {
    const target = corpus.entries.get(edge.target)!;
    const link = page.locator(".rel-prereq").getByRole("link", { name: target.title, exact: true }).first();
    await expect(link).toBeVisible();
    if (edge.label === "or") await expect(link.locator("xpath=..")).toContainText("or");
  }
  await expect(page.getByRole("heading", { name: /Leads to/ })).toBeVisible();

  const chain = page.locator(".entry-rel .chain").first();
  if (await chain.count()) {
    const labels = await chain.getByRole("link").allInnerTexts();
    expect(labels.length).toBeGreaterThan(0);
  }
});

test("every feat chain Open the feat link has a 24 px hit area", async ({ page }) => {
  await page.goto("#/feats/chains");
  const heads = page.locator(".chain-head details");
  await expect(heads.first()).toBeVisible();
  const headCount = await heads.count();
  expect(headCount).toBeGreaterThan(0);

  // Every link lives inside its chain's disclosure. Open them all, then measure each one.
  await page.evaluate(() => {
    for (const details of document.querySelectorAll<HTMLDetailsElement>(".chain-head details")) details.open = true;
  });
  const heights = await page
    .locator(".chain-open a")
    .evaluateAll((links) => links.map((link) => link.getBoundingClientRect().height));
  expect(heights).toHaveLength(headCount);
  expect(heights.filter((height) => height < 24)).toEqual([]);
});

test("category hub exposes requested reference areas and dedicated lenses", async ({ page }) => {
  await page.goto("#/browse");
  const names = ["Spells", "Feats", "Classes", "Prestige classes", "Races", "Traits", "Items", "Creatures"];
  for (const name of names) await expect(page.getByRole("link", { name: new RegExp(name) }).first()).toBeVisible();
  await page.goto("#/browse/kind/creature");
  await expect(page.getByRole("heading", { name: "Creatures", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li").first()).toBeVisible();
  await page.goto("#/browse/kind/trait");
  await expect(page.getByRole("heading", { name: "Traits", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li").first()).toBeVisible();
  for (const [slug, title] of [["item", "Items"], ["prestige_class", "Prestige classes"]]) {
    await page.goto(`#/browse/kind/${slug}`);
    await expect(page.getByRole("heading", { name: title, level: 1 })).toBeVisible();
    await expect(page.locator(slug === "rule" ? ".tree" : ".entry-list li").first()).toBeVisible();
  }
  await page.goto("#/feats/chains");
  await expect(page.getByRole("heading", { name: "Feat chains", level: 1 })).toBeVisible();
  await expect(page.locator(".chain-heads .chain-head").first()).toBeVisible();
});

test("book contents keep their order and synchronize selection with history", async ({ page }) => {
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => chapter.children?.length))!;
  const first = book.chapters.find((chapter) => chapter.children?.length > 0)!;
  await page.goto("#/browse/books");
  await openBookContents(page, book.title);
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${book.title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`) })).toBeVisible();
  await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();
  const tocTitles = await page.locator(".toc-top > li").evaluateAll((items) => items.map((item) => {
    const branch = item.querySelector(".toc-title");
    const leaf = item.querySelector("a");
    return (branch?.textContent ?? leaf?.textContent ?? "").trim();
  }));
  expect(tocTitles).toEqual(book.chapters.map((chapter) => chapter.title));
  const chapter = page.locator(".toc-details").filter({ has: page.locator("summary", { hasText: first.title }) }).first();
  await chapter.locator("summary").click();
  await expect.poll(() => new URL(page.url()).hash).toContain(`#/browse/book/${book.source}?c=`);
  const selectedUrl = page.url();
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/books");
  await page.goForward();
  await expect.poll(() => page.url()).toBe(selectedUrl);

  const walk = (nodes: any[]): any[] => nodes.flatMap((n) => [n, ...walk(n.children ?? [])]);
  const entryId = walk(first.children).find((n: any) => n.entry)?.entry;
  if (entryId) {
    await page.goto(`#/entry/${entryId}`);
    await expect(page.getByTestId("book-bar")).toBeVisible();
    const next = page.locator('.book-steps a[rel="next"]');
    if (await next.count()) {
      const href = await next.getAttribute("href");
      await next.click();
      await expect(page).toHaveURL(new RegExp(href!.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
      await page.goBack();
      await expect(page).toHaveURL(new RegExp(`#/entry/${entryId.replaceAll("/", "\\/")}$`));
    }
  }
});
