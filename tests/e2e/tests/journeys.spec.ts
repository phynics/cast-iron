import { installedTest as test, expect, loadCorpus } from "./fixtures";
import {
  afterRouteFrame,
  bookEntryOrder,
  bookTrees,
  corpusEntry,
  firstBookPlacement,
  installLibrary,
  openBookContents,
  openBookEntry,
  quickSearchFor,
  revealSourceChoice,
  waitControlled,
  waitResults,
} from "./helpers";

const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const related = (entry: (typeof entries)[number], kind: string) =>
  entry.relations?.filter((relation: any) => relation.kind === kind) ?? [];

test.beforeEach(async ({ page, context }) => {
  await installLibrary(page);
  await waitControlled(page);
  await context.setOffline(true);
});

test("Home search opens a matched rule section and keeps stars usable offline", async ({ page }) => {
  await expect(page.locator("#home-title")).toBeVisible();

  const primary = page.getByRole("navigation", { name: "Primary" });
  await expect(primary.getByRole("link")).toHaveText(["Home", "Books", "Settings"]);
  await expect(primary.getByRole("link", { name: /Search|Bookmark/i })).toHaveCount(0);

  // A quick lookup from a non-search page uses the header search icon and Quick search.
  await primary.getByRole("link", { name: "Home", exact: true }).click();
  await afterRouteFrame(page);
  await quickSearchFor(page, "grapple");
  await page.getByRole("group", { name: "Quick search categories" }).getByRole("button", { name: "Rules text", exact: true }).click();
  await expect.poll(() => page.url()).toContain("kind=rule");
  await expect(page.locator(".results .hit").first()).toBeVisible();
  await expect.poll(async () => {
    const targets = await page.locator(".results .hit-link").evaluateAll((nodes) =>
      nodes.map((node) => (node as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]));
    return targets.length > 0 && targets.every((id) => corpus.entries.get(id)?.kind === "rule");
  }).toBe(true);
  // A heading match opens the article at that heading. The corpus says which entry has
  // the anchored heading; the search index supplies the hit.
  const anchored = page.locator(".results .hit-link[href*='?a=']:visible").first();
  await expect(anchored).toBeVisible();
  const href = (await anchored.getAttribute("href"))!;
  await anchored.click();
  await expect(page.locator(".rich")).toContainText(/grappl/i);
  await expect(page).toHaveURL(/a=/);
  expect(href).toContain("a=");

  // Searching a spell by name from another page and starring it from the result.
  const spell = corpusEntry("spell", "Fireball");
  await primary.getByRole("link", { name: "Home", exact: true }).click();
  await afterRouteFrame(page);
  await quickSearchFor(page, spell.title);
  await page.locator(".results .hit-link").filter({ has: page.locator(".hit-title", { hasText: new RegExp(`^${spell.title}$`) }) }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();

  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await primary.getByRole("link", { name: "Home" }).click();
  const preview = page.locator(".home-bookmarks");
  await expect(preview.getByRole("link", { name: new RegExp(spell.title) })).toBeVisible();
  await preview.getByRole("link", { name: new RegExp(spell.title) }).click();
  await expect(page.getByRole("heading", { level: 1, name: spell.title })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await primary.getByRole("link", { name: "Home" }).click();
  await preview.getByRole("button", { name: `Unstar ${spell.title}` }).click();
  await expect(preview.getByRole("link", { name: new RegExp(spell.title) })).toHaveCount(0);
});

test("Home categories connect races, class options, spell levels, and reference categories", async ({ page }) => {
  const byKind = page.locator("section[aria-labelledby=by-kind]");
  const category = (name: string) => byKind.locator("a.cat").filter({
    has: page.locator(".cat-name", { hasText: new RegExp(`^${name}$`) }),
  });

  // Follow a race's own relationship to one of its racial traits.
  const raceTrait = entries.find((entry) => entry.kind === "racial_trait" && related(entry, "part").length > 0)!;
  const race = corpus.entries.get(related(raceTrait, "part")[0].target)!;
  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Home", exact: true }).click();
  await afterRouteFrame(page);
  await category("Races").click();
  const raceChoice = page.locator(`.entry-list a[href="#/entry/${race.id}"]`);
  await revealSourceChoice(raceChoice);
  await raceChoice.click();
  await expect(page.getByRole("heading", { level: 1, name: race.title })).toBeVisible();
  await page.locator(".entry-rel").getByRole("link", { name: raceTrait.title, exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: raceTrait.title })).toBeVisible();
  await expect(page.locator(".entry-rel").getByRole("heading", { name: /racial trait of/i })).toBeVisible();

  // Class options are reached through the Books navigation and the actual book contents.
  const primary = page.getByRole("navigation", { name: "Primary" });
  const classEntry = entries.find((entry) => entry.kind === "class" && firstBookPlacement(entry.id) &&
    entries.some((candidate) => candidate.kind === "archetype" &&
      related(candidate, "archetype").some((edge: any) => edge.target === entry.id)))!;
  const classBook = firstBookPlacement(classEntry.id)!;
  await primary.getByRole("link", { name: "Books" }).click();
  await openBookEntry(page, classBook, classEntry.id);
  await expect(page.getByRole("heading", { level: 1, name: classEntry.title })).toBeVisible();

  const archetype = entries.find((candidate) => candidate.kind === "archetype" &&
    related(candidate, "archetype").some((edge: any) => edge.target === classEntry.id))!;
  await primary.getByRole("link", { name: "Home", exact: true }).click();
  await afterRouteFrame(page);
  await category("Classes").click();
  await page.locator(".entry-list").getByRole("link", { name: classEntry.title, exact: true }).first().click();
  await page.locator(".entry-rel").getByRole("heading", { name: /Archetypes/ }).waitFor();
  await page.locator(".rel-disclosure > summary").click();
  const archetypeGroup = page.locator(".rel-group").filter({ has: page.getByRole("link", { name: archetype.title, exact: true }) });
  if ((await archetypeGroup.getAttribute("open")) === null) await archetypeGroup.locator("summary").click();
  await page.locator(".entry-rel").getByRole("link", { name: archetype.title, exact: true }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: archetype.title })).toBeVisible();
  await expect(page.locator(".entry-rel").getByRole("heading", { name: /An archetype of/ })).toBeVisible();

  // Use the class and level links in the spell-list hub to reach a spell entry.
  const classLevels = new Map<string, number>();
  for (const entry of entries) {
    for (const level of entry.fields?.levels ?? []) classLevels.set(level.class, (classLevels.get(level.class) ?? 0) + 1);
  }
  const listClass = [...classLevels.entries()].sort((a, b) => b[1] - a[1])[0][0];
  const listSpell = entries.find((entry) => entry.kind === "spell" &&
    entry.fields?.levels?.some((level: any) => level.class === listClass))!;
  const listLevel = listSpell.fields.levels.find((level: any) => level.class === listClass).level;
  await primary.getByRole("link", { name: "Home", exact: true }).click();
  await afterRouteFrame(page);
  await page.getByRole("link", { name: "Spell lists by class and level" }).click();
  await page.locator(".spell-class h3").getByRole("link", { name: listClass, exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: `${listClass} spells` })).toBeVisible();
  await page.locator(".level-nav").getByRole("link", { name: new RegExp(`^Level ${listLevel},`) }).click();
  await page.locator(".entry-list").getByRole("link", { name: listSpell.title, exact: true }).first().click();
  await expect(page.getByRole("heading", { level: 1, name: listSpell.title })).toBeVisible();

  // The other reference categories lead to readable entries through their displayed links.
  await primary.getByRole("link", { name: "Home", exact: true }).click();
  await afterRouteFrame(page);
  for (const [name, title] of [["Traits", "Traits"], ["Items", "Items"]]) {
    await category(name).click();
    await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
    const first = page.locator(".entry-list li a").first();
    await revealSourceChoice(first);
    await expect(first).toBeVisible();
    const entryId = (await first.getAttribute("href"))!.replace(/^#\/entry\//, "").split("?")[0];
    const entryTitle = corpus.entries.get(entryId)!.title;
    await first.click();
    await expect(page.getByRole("heading", { level: 1, name: entryTitle })).toBeVisible();
    await primary.getByRole("link", { name: "Home", exact: true }).click();
    await afterRouteFrame(page);
  }
  await expect(category("Rules")).toHaveCount(0);
});

test("Books navigation opens a rule and keeps previous and next reading usable", async ({ page }) => {
  // A placed rule with a following entry: the reader's book bar needs both neighbours.
  const rule = entries.find((entry) => {
    if (entry.kind !== "rule") return false;
    const book = firstBookPlacement(entry.id);
    if (!book) return false;
    const order = bookEntryOrder(book);
    return order.indexOf(entry.id) < order.length - 1;
  })!;
  const book = firstBookPlacement(rule.id)!;

  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Books" }).click();
  await openBookContents(page, book.title);
  await expect(page.getByRole("navigation", { name: new RegExp(`Contents of ${book.title}`) })).toBeVisible();
  await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();

  await openBookEntry(page, book, rule.id);
  await expect(page.getByRole("heading", { level: 1, name: rule.title })).toBeVisible();
  await expect(page.getByTestId("book-bar")).toBeVisible();

  const next = page.locator('.book-steps a[rel="next"]');
  await expect(next).toBeVisible();
  const nextTitle = (await next.locator(".step-title").innerText()).trim();
  await next.click();
  await expect(page.getByRole("heading", { level: 1, name: nextTitle })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole("heading", { level: 1, name: rule.title })).toBeVisible();
  await expect(page.getByTestId("book-bar")).toBeVisible();
});
