import { test, expect, loadCorpus, GENERATED } from "./fixtures";
import { installLibrary, waitControlled } from "./helpers";
import fs from "node:fs";
import path from "node:path";

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
  await page.locator(".entry-rel").getByRole("link", { name: trait.title, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${traitId.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("heading", { name: trait.title, level: 1 })).toBeVisible();
  await expect(page.getByRole("heading", { name: /racial trait of/i })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: race.title, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${race.id.replaceAll("/", "\\/")}$`));

  const classEntry = entries.find((e) => e.kind === "class" &&
    entries.some((a) => a.kind === "archetype" && related(a, "archetype").some((r: any) => r.target === e.id)))!;
  const archetype = entries.find((a) => a.kind === "archetype" && related(a, "archetype").some((r: any) => r.target === classEntry.id))!;
  await page.goto(`#/entry/${classEntry.id}`);
  await expect(page.locator(".entry-rel").getByRole("heading", { name: /Archetypes/ })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: archetype.title, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${archetype.id.replaceAll("/", "\\/")}$`));
  await expect(page.getByRole("heading", { name: /An archetype of/ })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: classEntry.title, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`#/entry/${classEntry.id.replaceAll("/", "\\/")}$`));
});

test("spell lists filter by class, level, and query; search supports source filters", async ({ page }) => {
  await page.goto("#/spells/Wizard");
  await expect(page.getByRole("heading", { name: "Wizard spells", level: 1 })).toBeVisible();
  const fire = corpus.entries.get("core-rulebook/spells/fireball")!;
  await page.locator("#list-filter").fill("fireball");
  await expect(page.locator(".entry-list").getByRole("link", { name: "Fireball", exact: true })).toBeVisible();
  const matching = entries.filter((e) => e.kind === "spell" && e.title.toLowerCase().includes("fireball") &&
    e.fields.levels.some((l: any) => l.class === "Wizard"));
  await expect(page.locator(".entry-list li")).toHaveCount(matching.length);
  await page.locator("#list-filter").fill("no-such-spell");
  await expect(page.getByRole("status").filter({ hasText: /No spell on this list matches/ })).toBeVisible();

  const fireballLevel = fire.fields.levels.find((l: any) => l.class === "Wizard").level;
  await page.goto(`#/spells/Wizard/${fireballLevel}`);
  await expect(page.getByRole("heading", { name: `Wizard spells, level ${fireballLevel}`, level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list").getByRole("link", { name: "Fireball", exact: true })).toBeVisible();
  await expect(page.locator(".level-group")).toHaveAttribute("data-level", String(fireballLevel));

  await page.goto("#/search?q=fire&kind=spell&source=core-rulebook");
  await expect(page.locator(".results-region")).toHaveAttribute("data-query", "fire");
  await expect(page.locator(".results .hit").first()).toBeVisible();
  await expect(page.locator(".results .hit-title").filter({ hasText: /^Fireball$/ })).toBeVisible();
  const resultHrefs = await page.locator(".results .hit-link").evaluateAll((nodes) => nodes.map((n) => (n as HTMLAnchorElement).hash));
  expect(resultHrefs.length).toBeGreaterThan(0);
  for (const href of resultHrefs) {
    const id = href.replace(/^#\/entry\//, "").split("?")[0];
    expect(corpus.catalog.entries.find((e) => e.id === id)?.source).toBe("core-rulebook");
  }
});

test("feat pages preserve printed prerequisites and label alternatives honestly", async ({ page }) => {
  const dependedOn = new Set(entries.flatMap((e) => related(e, "prerequisite").map((r: any) => r.target)));
  const feat = entries.find((e) => e.kind === "feat" && e.fields.prerequisites &&
    related(e, "prerequisite").some((r: any) => r.label === "or") && dependedOn.has(e.id))!;
  await page.goto(`#/entry/${feat.id}`);
  await expect(page.locator("#rel-prereq")).toHaveText("Requirements");
  await expect(page.locator(".prereq-text")).toContainText(feat.fields.prerequisites);
  await expect(page.getByText("Feats it names", { exact: true })).toBeVisible();
  const expected = related(feat, "prerequisite");
  for (const edge of expected) {
    const target = corpus.entries.get(edge.target)!;
    const link = page.locator(".rel-prereq").getByRole("link", { name: target.title, exact: true });
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

test("category hub exposes requested reference areas and dedicated lenses", async ({ page }) => {
  await page.goto("#/browse");
  const names = ["Rules", "Spells", "Feats", "Classes", "Prestige classes", "Races", "Traits", "Items", "Creatures"];
  for (const name of names) await expect(page.getByRole("link", { name: new RegExp(name) }).first()).toBeVisible();
  await page.goto("#/browse/kind/creature");
  await expect(page.getByRole("heading", { name: "Creatures", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li").first()).toBeVisible();
  await page.goto("#/browse/kind/trait");
  await expect(page.getByRole("heading", { name: "Traits", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-list li").first()).toBeVisible();
  for (const [slug, title] of [["item", "Items"], ["prestige_class", "Prestige classes"], ["rule", "Rules"]]) {
    await page.goto(`#/browse/kind/${slug}`);
    await expect(page.getByRole("heading", { name: title, level: 1 })).toBeVisible();
    await expect(page.locator(slug === "rule" ? ".tree" : ".entry-list li").first()).toBeVisible();
  }
  await page.goto("#/feats/chains");
  await expect(page.getByRole("heading", { name: "Feat chains", level: 1 })).toBeVisible();
  await expect(page.locator(".chain-heads .chain-head").first()).toBeVisible();
});

test("book contents keep their curated order and synchronize selection with history", async ({ page }) => {
  const manifest = JSON.parse(fs.readFileSync(path.join(GENERATED, corpus.version, "books.json"), "utf8"));
  const book = manifest.books.find((b: any) => b.source === "core-rulebook");
  expect(book).toBeTruthy();
  const first = book.chapters.find((n: any) => n.children.length > 0);
  await page.goto("#/browse/books");
  await page.getByRole("link", { name: /Core Rulebook/ }).click();
  await expect(page.getByRole("navigation", { name: /Contents of Core Rulebook/ })).toBeVisible();
  await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();
  const tocTitles = await page.locator(".toc-top > .toc-branch > .toc-details > summary .toc-title").allInnerTexts();
  expect(tocTitles).toEqual(book.chapters.map((n: any) => n.title));
  const chapter = page.locator(".toc-details").filter({ has: page.locator("summary", { hasText: first.title }) }).first();
  await chapter.locator("summary").click();
  await expect.poll(() => new URL(page.url()).hash).toContain("#/browse/book/core-rulebook?c=");
  const selectedUrl = page.url();
  await page.goBack();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/books");
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
