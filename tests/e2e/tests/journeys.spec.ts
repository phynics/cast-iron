import { test, expect, loadCorpus, GENERATED } from "./fixtures";
import { openBookContents, installLibrary, waitControlled, waitResults, revealSourceChoice } from "./helpers";
import fs from "node:fs";
import path from "node:path";

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
  await expect(primary.getByRole("link")).toHaveText(["Home", "Browse", "Books", "Settings"]);
  await expect(primary.getByRole("link", { name: /Search|Bookmark/i })).toHaveCount(0);

  // A quick lookup from a non-search page uses the persistent header search.
  await primary.getByRole("link", { name: "Browse" }).click();
  await page.locator(".header-search #search-input").fill("grapple");
  await page.locator(".header-search #search-input").press("Enter");
  await waitResults(page, "grapple");
  await page.getByRole("group", { name: "Quick search categories" }).getByRole("button", { name: "Rules text", exact: true }).click();
  await expect.poll(() => page.url()).toContain("kind=rule");
  await expect(page.locator(".results .hit").first()).toBeVisible();
  await expect.poll(async () => {
    const targets = await page.locator(".results .hit-link").evaluateAll((nodes) =>
      nodes.map((node) => (node as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]));
    return targets.length > 0 && targets.every((id) => corpus.entries.get(id)?.kind === "rule");
  }).toBe(true);
  const ruleTargets = await page.locator(".results .hit-link").evaluateAll((nodes) =>
    nodes.map((node) => (node as HTMLAnchorElement).hash.replace(/^#\/entry\//, "").split("?")[0]));
  expect(ruleTargets.slice(0, 3)).toContain("core-rulebook/rules/combat/special-attacks");
  await page.locator(".results .hit-link").first().click();
  await expect(page.locator(".rich")).toContainText(/grappl/i);
  await expect(page).toHaveURL(/special-attacks\?a=/);
  await primary.getByRole("link", { name: "Browse" }).click();
  await page.locator(".header-search #search-input").fill("fireball");
  await page.locator(".header-search #search-input").press("Enter");
  await waitResults(page, "fireball");
  await page.locator(".results .hit-link").filter({ has: page.locator(".hit-title", { hasText: /^Fireball$/ }) }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();

  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await primary.getByRole("link", { name: "Home" }).click();
  const preview = page.locator(".home-bookmarks");
  await expect(preview.getByRole("link", { name: /Fireball/ })).toBeVisible();
  await preview.getByRole("link", { name: /Fireball/ }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");

  await primary.getByRole("link", { name: "Home" }).click();
  await preview.getByRole("button", { name: "Unstar Fireball" }).click();
  await expect(preview.getByRole("link", { name: /Fireball/ })).toHaveCount(0);
});

test("Browse connects races, class options, spell levels, and reference categories", async ({ page }) => {
  const byKind = page.locator("section[aria-labelledby=by-kind]");
  const category = (name: string) => byKind.locator("a.cat").filter({
    has: page.locator(".cat-name", { hasText: new RegExp(`^${name}$`) }),
  });

  // Follow a race's own relationship to one of its racial traits.
  const raceTrait = entries.find((entry) => entry.kind === "racial_trait" && related(entry, "part").length > 0)!;
  const race = corpus.entries.get(related(raceTrait, "part")[0].target)!;
  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Browse" }).click();
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
  await primary.getByRole("link", { name: "Books" }).click();
  await openBookContents(page, "Ultimate Magic");
  const classOptions = page.locator(".toc-top > .toc-branch > .toc-details").filter({
    has: page.locator("summary", { hasText: "Spellcasting Class Options" }),
  });
  await classOptions.locator("summary").click();
  await expect(page.locator(".toc")).toContainText("Magus");
  await page.locator(".toc").getByRole("link", { name: "Magus", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Magus" })).toBeVisible();

  const classEntry = entries.find((entry) => entry.kind === "class" && entries.some((candidate) =>
    candidate.kind === "archetype" && related(candidate, "archetype").some((edge: any) => edge.target === entry.id))
  )!;
  const archetype = entries.find((candidate) => candidate.kind === "archetype" &&
    related(candidate, "archetype").some((edge: any) => edge.target === classEntry.id))!;
  await primary.getByRole("link", { name: "Browse" }).click();
  await category("Classes").click();
  await page.locator(".entry-list").getByRole("link", { name: classEntry.title, exact: true }).click();
  await page.locator(".entry-rel").getByRole("heading", { name: /Archetypes/ }).waitFor();
  await page.locator(".rel-disclosure > summary").click();
  const archetypeGroup = page.locator(".rel-group").filter({ has: page.getByRole("link", { name: archetype.title, exact: true }) });
  if ((await archetypeGroup.getAttribute("open")) === null) await archetypeGroup.locator("summary").click();
  await page.locator(".entry-rel").getByRole("link", { name: archetype.title, exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: archetype.title })).toBeVisible();
  await expect(page.locator(".entry-rel").getByRole("heading", { name: /An archetype of/ })).toBeVisible();

  // Use the class and level links in the spell-list hub to reach a spell entry.
  await primary.getByRole("link", { name: "Browse" }).click();
  await page.getByRole("link", { name: "Spell lists by class and level" }).click();
  await page.locator(".spell-class h3").getByRole("link", { name: "Wizard", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Wizard spells" })).toBeVisible();
  const fireball = corpus.entries.get("core-rulebook/spells/fireball")!;
  const wizardLevel = fireball.fields.levels.find((level: any) => level.class === "Wizard").level;
  await page.locator(".level-nav").getByRole("link", { name: new RegExp(`^Level ${wizardLevel},`) }).click();
  await page.locator(".entry-list").getByRole("link", { name: "Fireball", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();

  // The other reference categories lead to readable entries through their displayed links.
  await primary.getByRole("link", { name: "Browse" }).click();
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
    await primary.getByRole("link", { name: "Browse" }).click();
  }
  await expect(category("Rules")).toHaveCount(0);
});

test("Books navigation opens a rule and keeps previous and next reading usable", async ({ page }) => {
  const manifest = JSON.parse(fs.readFileSync(path.join(GENERATED, corpus.version, "books.json"), "utf8"));
  const book = manifest.books.find((item: any) => item.source === "core-rulebook");
  expect(book).toBeTruthy();

  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Books" }).click();
  await openBookContents(page, "Core Rulebook");
  await expect(page.getByRole("navigation", { name: /Contents of Core Rulebook/ })).toBeVisible();
  await expect(page.getByText(/draft|checked against/i).first()).toBeVisible();

  // Read the Combat subsystem from its book contents, then continue in book order.
  const combat = page.locator(".toc-top > .toc-branch > .toc-details").filter({
    has: page.locator(":scope > summary .toc-title", { hasText: /^Combat$/ }),
  });
  await combat.locator(":scope > summary").click();
  await expect(combat.getByRole("link", { name: "Actions in Combat", exact: true })).toBeVisible();
  await expect(combat.getByRole("link", { name: "Big and Little Creatures in Combat", exact: true })).toBeVisible();
  const firstRule = combat.getByRole("link", { name: "Read Combat", exact: true });
  const href = await firstRule.getAttribute("href");
  const ruleId = href!.replace(/^#\/entry\//, "").split("?")[0];
  const ruleTitle = corpus.entries.get(ruleId)!.title;
  await firstRule.click();
  await expect(page.getByRole("heading", { level: 1, name: ruleTitle })).toBeVisible();
  await expect(page.getByTestId("book-bar")).toBeVisible();

  const next = page.locator('.book-steps a[rel="next"]');
  await expect(next).toBeVisible();
  const nextTitle = (await next.locator(".step-title").innerText()).trim();
  await next.click();
  await expect(page.getByRole("heading", { level: 1, name: nextTitle })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole("heading", { level: 1, name: ruleTitle })).toBeVisible();
  await expect(page.getByTestId("book-bar")).toBeVisible();
});
