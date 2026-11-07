import { test, installedTest, expect, loadCorpus } from "./fixtures";
import { bookTrees, firstEntry, hasBlock, headingAnchors, installLibrary, openBookGroups, openQuickSearch, quickField, searchIcon, seedHome, waitResults } from "./helpers";

// Runs in the mobile-chromium project (390 px wide). Layout rules live in docs/decisions.md.
// Every screen names a target the compiled corpus actually holds; the two libraries differ
// in ids, sources, and counts, so none of these may be a literal entry id.
const corpus = loadCorpus();
const entries = [...corpus.entries.values()];
const statSpell = firstEntry("a spell with a stat block", (entry) => entry.kind === "spell" && hasBlock(entry, "stat_block"));
const tableEntry = firstEntry("an entry with a table", (entry) => hasBlock(entry, "table"));
const headingEntry = firstEntry("an entry with a heading", (entry) => headingAnchors(entry).length > 0);
const creature = firstEntry("a creature with a stat block", (entry) => entry.kind === "creature" && hasBlock(entry, "stat_block"));
const source = bookTrees()[0].source;
const classLevels = new Map<string, number>();
for (const entry of entries) {
  for (const level of entry.fields?.levels ?? []) classLevels.set(level.class, (classLevels.get(level.class) ?? 0) + 1);
}
const [listClass, listCount] = [...classLevels.entries()].sort((a, b) => b[1] - a[1])[0];
const listSpell = entries.find((entry) => entry.kind === "spell" &&
  entry.fields?.levels?.some((level: any) => level.class === listClass))!;
const listLevel = listSpell.fields.levels.find((level: any) => level.class === listClass).level;

const screens: [string, string][] = [
  ["#/", "#home-title"],
  ["#/search?q=fire", ".results .hit"],
  ["#/browse", "a.cat"],
  ["#/browse/kind/spell", ".entry-list li"],
  ["#/books", ".book-directory-row"],
  ["#/books/active", ".preset-row"],
  [`#/browse/source/${encodeURIComponent(source)}`, ".toc"],
  [`#/spells/${encodeURIComponent(listClass)}/${listLevel}`, ".level-group"],
  ["#/feats/chains", ".chain-head"],
  [`#/entry/${statSpell.id}`, ".stat-block"],
  [`#/entry/${tableEntry.id}`, ".rules-table"],
  [`#/entry/${headingEntry.id}`, ".rich h2"],
  [`#/entry/${creature.id}`, ".stat-block"],
  ["#/bookmarks", "#home-title"],
  ["#/settings", "[data-testid=library-facts]"],
  ["#/legal", ".legal-text"],
];

installedTest("Mobile viewport shows no horizontal page scroll on key screens", async ({ page }) => {
  expect(page.viewportSize()?.width).toBe(390);
  await installLibrary(page);
  const wide: string[] = [];
  for (const [hash, ready] of screens) {
    await page.goto(hash);
    if (hash.startsWith("#/search")) await waitResults(page, "fire");
    await expect(page.locator(ready).first(), hash).toBeVisible();
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    if (overflow > 0) wide.push(`${hash} overflows by ${overflow}px`);
  }
  expect(wide).toEqual([]);
});

test("Book rows, the book page contents tree and the groups fit the mobile viewport", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await openBookGroups(page);
  const book = bookTrees().find((candidate) => candidate.chapters.some((chapter) => (chapter.children ?? []).length > 0));
  if (!book) throw new Error("The compiled corpus has no book with contents.");
  const overflow = () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  const row = page.locator(`.book-directory-item[data-book-id="${book.source}"]`);
  const link = row.locator("a.book-directory-row");
  // The row is a link and nothing else: no checkbox on Books.
  await expect(row.locator("input")).toHaveCount(0);
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.height).toBeGreaterThanOrEqual(56);
  expect(linkBox.x + linkBox.width).toBeLessThanOrEqual(390);
  expect(await overflow()).toBeLessThanOrEqual(0);

  await link.click();
  const tree = page.getByRole("navigation", { name: new RegExp(`Contents of `) });
  await expect(tree).toBeVisible();
  await tree.locator(".toc-top > li > .toc-details > summary").first().click();
  expect(await overflow()).toBeLessThanOrEqual(0);
  await page.goBack();
  await expect(page.locator(".book-tier").first()).toBeVisible();

  // The book page has no breadcrumb, its first screen holds the title, the meta line, the Active
  // row and the Contents heading, and nothing overflows with a chapter open.
  await expect(page.locator("main .crumbs")).toHaveCount(0);
  await expect(page.getByTestId("source-count")).toBeVisible();
  const heading = page.getByRole("heading", { level: 2, name: "Contents", exact: true });
  await expect(heading).toBeVisible();
  expect((await heading.boundingBox())!.y).toBeLessThan(844);
  expect(await overflow()).toBeLessThanOrEqual(0);
});

test("Books and Active books fit 390 px with full-size touch targets", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  const groups = page.locator(".book-tier");
  await expect(groups.first()).toBeVisible();
  await expect(groups.first().getByRole("heading", { level: 2 })).toContainText("Rulebooks");
  const adventures = page.locator('.book-tier[data-tier="Adventures"]');
  await expect(adventures.getByRole("button")).toHaveAttribute("aria-expanded", "false");
  // Every group heading is a 44 px target that stays inside the screen.
  for (const toggle of await page.locator(".tier-toggle").all()) {
    const box = (await toggle.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  const overflow = async () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

  // Books has no group controls and no Only Rulebooks; the link to Active books is a 44 px target.
  await expect(page.getByRole("button", { name: /Activate all|Deactivate all|Only Rulebooks/ })).toHaveCount(0);
  const link = page.getByRole("link", { name: "Active books", exact: true });
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.height).toBeGreaterThanOrEqual(44);
  expect(linkBox.x + linkBox.width).toBeLessThanOrEqual(390);
  expect(await overflow()).toBeLessThanOrEqual(0);

  await link.click();
  await expect(page).toHaveURL(/#\/books\/active$/);
  await expect(page.getByRole("heading", { level: 1, name: "Active books", exact: true })).toBeVisible();
  const targets = [
    page.getByRole("button", { name: "Done", exact: true }),
    ...(await page.locator(".preset-row").all()),
  ];
  expect(targets.length).toBeGreaterThan(1);
  for (const target of targets) {
    const box = (await target.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  // A checkbox row is a whole-row label at least 56 px tall, and the box itself is not tiny.
  const row = page.locator(".toggle-row").first();
  expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(56);
  const box = (await row.getByRole("checkbox").boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(20);
  expect(await overflow()).toBeLessThanOrEqual(0);
  await adventures.getByRole("button").click();
  await expect(adventures.locator(".book-directory-item").first()).toBeVisible();
  expect(await overflow()).toBeLessThanOrEqual(0);
});

test("Mobile header Install button is icon-only, a full touch target, and crowds nothing", async ({ page, context }) => {
  await context.addInitScript({
    content: `document.addEventListener("DOMContentLoaded", () => {
      const event = new Event("beforeinstallprompt", { cancelable: true });
      event.prompt = async () => {};
      event.userChoice = Promise.resolve({ outcome: "dismissed" });
      window.dispatchEvent(event);
    });`,
  });
  await installLibrary(page);
  const button = page.locator("header.app-header").getByRole("button", { name: "Install Cast Iron as an app" });
  await expect(button).toBeVisible();
  await expect(button.locator(".header-action-label")).toBeHidden();
  const box = (await button.boundingBox())!;
  // Device pixel ratios round a 44px target slightly below 44 in the layout viewport.
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
  const search = (await searchIcon(page).boundingBox())!;
  expect(search.x + search.width).toBeLessThanOrEqual(box.x);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Mobile entry header is compact: one metadata line and an icon star", async ({ page }) => {
  await installLibrary(page);
  await page.goto(`#/entry/${statSpell.id}`);
  await expect(page.locator(".stat-block").first()).toBeVisible();
  const top = await page.locator(".stat-block").first().evaluate((el) => el.getBoundingClientRect().top);
  // Title, a short metadata line and the stat block fit in the first 300 px of an 844 px screen.
  // (The old header, with pills and a full-row Star button, put the block near 380 px.) The bound
  // leaves room for a two-line title and a wrapped metadata line.
  expect(top).toBeLessThan(300);
  const star = (await page.locator(".bookmark-toggle").boundingBox())!;
  expect(Math.round(star.width)).toBe(44);
  expect(Math.round(star.height)).toBe(44);
  // A long book title may wrap once; wrapped pills used to take three rows.
  const meta = page.locator(".entry-meta");
  const lineHeight = await meta.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
  expect((await meta.boundingBox())!.height).toBeLessThan(lineHeight * 2.6);
});

test("Publication groups fit 390 px with full-size targets", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/search");
  await page.locator("#search-input").fill("fire");
  await waitResults(page, "fire");
  await page.getByRole("button", { name: /^Filters/ }).click();
  const group = page.locator('.filters .filter-group:has(legend:text-is("Publication"))');
  await expect(group.locator(".publication-group").first()).toBeVisible();
  await expect(group.locator(".publication-group").first()).toHaveAttribute("data-tier", "Rulebooks");
  const adventures = group.locator('details.publication-group[data-tier="Adventures"]');
  await expect(adventures).toHaveCount(1);
  await expect(adventures).not.toHaveAttribute("open", "");
  // Every tier that the budget skipped is a closed heading: full-size, inside the viewport.
  const headings = group.locator("details.publication-group > summary");
  expect(await headings.count()).toBeGreaterThanOrEqual(3);
  for (const heading of await headings.all()) {
    await expect(heading).toBeVisible();
    const box = (await heading.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  const summary = adventures.locator("summary");
  const summaryBox = (await summary.boundingBox())!;
  expect(summaryBox.height).toBeGreaterThanOrEqual(44);
  expect(summaryBox.x + summaryBox.width).toBeLessThanOrEqual(390);
  // Group headings and summaries are plain labels, never pills.
  const radii = await group.locator(".publication-group-title").evaluateAll((els) =>
    els.map((el) => getComputedStyle(el).borderRadius),
  );
  expect(radii.length).toBeGreaterThan(1);
  expect(radii.every((radius) => radius !== "999px")).toBe(true);
  const overflowClosed = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(overflowClosed).toBeLessThanOrEqual(0);
  await summary.click();
  await expect(adventures).toHaveAttribute("open", "");
  const chips = adventures.locator(".chip");
  expect(await chips.count()).toBeGreaterThan(0);
  for (const chip of await chips.all()) {
    const box = (await chip.boundingBox())!;
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
  const overflow = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Mobile Quick search is a full-screen layer with 44 px controls", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/books");
  await expect(page.locator(".book-directory-row").first()).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 500));
  const before = await page.evaluate(() => Math.round(window.scrollY));
  expect(before).toBeGreaterThan(200);

  const icon = (await searchIcon(page).boundingBox())!;
  expect(Math.round(icon.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(icon.height)).toBeGreaterThanOrEqual(44);

  const layer = await openQuickSearch(page, { keyboard: true });
  // The layer animates in; measure its resting position, not a frame of the expand.
  await layer.evaluate((el) => Promise.all(el.getAnimations({ subtree: true }).map((a) => a.finished)));
  const box = (await layer.boundingBox())!;
  const viewport = page.viewportSize()!;
  // Sub-pixel offsets round to -0, which `toBe(0)` rejects under Object.is; the layer only
  // has to sit flush with the viewport.
  expect(Math.abs(box.x)).toBeLessThan(1);
  expect(Math.abs(box.y)).toBeLessThan(1);
  expect(Math.round(box.width)).toBe(viewport.width);
  expect(Math.round(box.height)).toBe(viewport.height);
  const field = (await quickField(page).boundingBox())!;
  const cancel = (await layer.getByRole("button", { name: "Cancel" }).boundingBox())!;
  expect(Math.round(field.height)).toBeGreaterThanOrEqual(44);
  expect(Math.round(cancel.height)).toBeGreaterThanOrEqual(44);
  expect(cancel.x + cancel.width).toBeLessThanOrEqual(viewport.width);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);

  await layer.getByRole("button", { name: "Cancel" }).click();
  await expect(layer).toBeHidden();
  expect(await page.evaluate(() => Math.round(window.scrollY))).toBe(before);
  await expect(searchIcon(page)).toBeFocused();
});

installedTest("Mobile Quick search rows are full-size touch targets", async ({ page }) => {
  await installLibrary(page);
  const layer = await openQuickSearch(page);
  await quickField(page).fill("fireb");
  const options = layer.getByRole("option");
  await expect(options).not.toHaveCount(1);
  await expect(options.last()).toHaveText("See all results for “fireb”");
  const viewport = page.viewportSize()!;
  for (const option of await options.all()) {
    const box = (await option.boundingBox())!;
    expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  }
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  // The empty field lists recent searches with 44 px remove buttons.
  await options.last().click();
  await waitResults(page, "fireb");
  await page.goto("#/");
  await openQuickSearch(page);
  const remove = layer.getByRole("button", { name: /^Remove “fireb” from recent searches$/ });
  const box = (await remove.boundingBox())!;
  expect(Math.round(box.width)).toBeGreaterThanOrEqual(44);
  expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
});

test("Spell lists at 390 px: Wizard is one tap from the hub, and rows show school, facts and rulebooks first", async ({ page }) => {
  await installLibrary(page);
  const overflow = () =>
    page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

  // The hub opens with the class picker: Wizard sits inside the first screen, not below 30 cards.
  await page.goto("#/spells");
  const wizard = page.locator(".class-picker").first().getByRole("link", { name: /^Wizard spells,/ });
  await expect(wizard).toBeVisible();
  const box = (await wizard.boundingBox())!;
  expect(box.height).toBeGreaterThanOrEqual(44);
  expect(box.y + box.height, "Wizard is inside the first screen").toBeLessThanOrEqual(844);
  expect(await overflow()).toBeLessThanOrEqual(0);
  await wizard.click();
  await expect(page.getByRole("heading", { level: 1, name: "Wizard spells" })).toBeVisible();
  // The class page is a navigation rectangle on the list page, not on the hub.
  const classPage = page.getByRole("link", { name: "Wizard class page" });
  await expect(classPage).toHaveClass(/jump/);
  expect((await classPage.boundingBox())!.height).toBeGreaterThanOrEqual(44);

  // Level 5: two-line rows with the school, the facts and the book. Expectations come from the corpus.
  await page.goto("#/spells/Wizard/5");
  const rows = page.locator(".spell-rows .spell-row");
  await expect(rows.first()).toBeVisible();
  await expect(page.locator(".spell-list")).toHaveAttribute("data-density", "wide");
  const total = await rows.count();
  expect(total).toBeGreaterThan(10);
  for (const index of [0, 1, 2, 3, 4]) {
    const row = rows.nth(index);
    await expect(row.locator(".spell-school"), `row ${index} school`).not.toHaveText("");
    await expect(row.locator(".spell-row-meta"), `row ${index} facts`).not.toHaveText("");
  }
  const cone = rows.filter({ has: page.getByRole("link", { name: "Cone of Cold", exact: true }) }).first();
  await expect(cone.locator(".spell-school")).toHaveText("Evocation");
  await expect(cone.locator(".spell-row-meta")).toContainText("1 standard action");
  await expect(cone.locator(".spell-row-meta")).toContainText("Reflex half");
  await expect(cone.locator(".spell-row-meta")).toContainText("Core Rulebook");
  // The name never wraps and a row is at least 44 px tall.
  const heights = await rows.evaluateAll((nodes) => nodes.map((n) => n.getBoundingClientRect().height));
  expect(Math.min(...heights)).toBeGreaterThanOrEqual(44);
  const names = await page.locator(".spell-name").evaluateAll((nodes) =>
    nodes.map((n) => n.getClientRects().length === 1 && n.getBoundingClientRect().height < 30));
  expect(names.every(Boolean)).toBe(true);
  expect(await overflow()).toBeLessThanOrEqual(0);

  // A Core spell comes before a spell that only an adventure prints.
  const category = new Map<string, string>(corpus.catalog.sources.map((s: any) => [s.id, s.category]));
  const stubs = new Map(corpus.catalog.entries.map((stub) => [stub.id, stub]));
  const wizardFive = entries.filter((entry) => entry.kind === "spell" &&
    entry.fields?.levels?.some((level: any) => level.class.toLowerCase() === "wizard" && level.level === 5));
  const adventure = wizardFive.find((entry) =>
    ["Adventure Path", "Module"].includes(category.get(stubs.get(entry.id)!.source) ?? ""));
  expect(adventure, "an adventure-only Wizard 5 spell in the corpus").toBeTruthy();
  const order = await page.locator(".spell-rows a.spell-name").evaluateAll((nodes) => nodes.map((n) => (n as HTMLAnchorElement).hash));
  const at = (id: string) => order.indexOf(`#/entry/${id}`);
  expect(at("ref/spell/cone-of-cold")).toBeGreaterThanOrEqual(0);
  expect(at(adventure!.id)).toBeGreaterThan(at("ref/spell/cone-of-cold"));
  await expect(page.locator(".tier-head").first()).toHaveText("Rulebooks");

  // The school filter narrows the list; the chips live in a disclosure so the rows stay near the top.
  const filter = page.locator("details.school-filter");
  await filter.locator("summary").click();
  // The counts are this level's: together they add up to the rows on the page, and the chosen
  // school's count is the number of rows left.
  const counts = (await filter.locator(".chip-count").allInnerTexts()).map(Number);
  expect(counts.reduce((a, b) => a + b, 0)).toBe(total);
  const evocation = filter.getByRole("checkbox", { name: /^Evocation,/ });
  const evocationCount = Number((await evocation.getAttribute("aria-label"))!.match(/(\d+) spells?$/)![1]);
  await evocation.check();
  await expect(rows.first()).toBeVisible();
  const schools = await page.locator(".spell-school").allInnerTexts();
  expect(schools.length).toBe(evocationCount);
  expect(new Set(schools)).toEqual(new Set(["Evocation"]));
  expect(await overflow()).toBeLessThanOrEqual(0);
});

const homeSpells = entries.filter((entry) => entry.kind === "spell")
  .filter((entry, i, all) => all.findIndex((c) => c.title === entry.title) === i)
  .map((entry) => ({ id: entry.id as string, title: entry.title as string }));

installedTest("Home at 390 px: Categories starts in the first viewport with no stars and few recents", async ({ page }) => {
  await installLibrary(page);
  await seedHome(page, { recents: homeSpells.slice(0, 3) });
  const top = await page.locator("#by-kind").evaluate((el) => el.getBoundingClientRect().top);
  expect(top).toBeLessThan(page.viewportSize()!.height);
  await expect(page.getByRole("heading", { name: "Starred" })).toHaveCount(0);
  await expect(page.locator(".home-recent > li")).toHaveCount(3);
});

installedTest("Home at 390 px: twelve recents take five rows and a Show more control", async ({ page }) => {
  await installLibrary(page);
  await seedHome(page, { recents: homeSpells.slice(0, 12) });
  const rows = page.locator(".home-recent > li");
  await expect(rows).toHaveCount(5);
  const more = page.getByRole("button", { name: /^Show more/ });
  await expect(more).toBeVisible();
  const section = (await page.locator("section[aria-labelledby=home-recent]").boundingBox())!;
  const last = (await rows.last().boundingBox())!;
  const moreBox = (await more.boundingBox())!;
  // Heading, five one-line rows and the control: no taller than that plus a little spacing.
  expect(section.height).toBeLessThan(44 + 5 * 52 + 44 + 24);
  expect(moreBox.y).toBeGreaterThan(last.y);
  for (const row of await rows.all()) {
    const title = row.locator(".home-row-title");
    const lineHeight = await title.evaluate((el) => parseFloat(getComputedStyle(el).lineHeight));
    expect((await title.boundingBox())!.height).toBeLessThanOrEqual(lineHeight + 1);
    expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Feats list at 390 px: title, count, filters and a sticky 48 px letter bar, with rows after them", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const main = page.locator("main");
  await expect(main.getByRole("heading", { level: 1, name: "Feats" })).toBeVisible();
  await expect(main.locator(".list-count")).toHaveText(/^[\d,]+ feats$/);
  await expect(main.locator("nav.crumbs")).toHaveCount(0);
  await expect(main.locator(".lede-line")).toHaveCount(0);
  const nav = main.getByRole("navigation", { name: "Jump to letter" });
  await expect(nav).toBeVisible();
  expect(Math.round((await nav.boundingBox())!.height)).toBe(48);
  // The bar sticks under the 60 px masthead once the page scrolls past it, and nothing else
  // below the masthead sticks.
  await page.evaluate(() => window.scrollTo(0, 1200));
  const header = (await page.locator("header.app-header").boundingBox())!;
  // (The masthead is 60 px plus its 1 px rule; the bar sits at 60 px, under the rule.)
  await expect.poll(async () => Math.abs((await nav.boundingBox())!.y - (header.y + header.height))).toBeLessThanOrEqual(1);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Feats at 390 px: the first feat row is in the first viewport, and the type menu works beside an open panel", async ({ page }) => {
  await installLibrary(page);
  await page.goto("#/browse/kind/feat");
  const main = page.locator("main");
  const button = main.getByRole("button", { name: /^Type: / });
  await expect(button).toBeEnabled();
  const viewport = page.viewportSize()!;
  const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
  const first = main.locator(".letter-group .entry-list a").first();
  await expect(first).toBeVisible();
  const y = (await first.boundingBox())!.y;
  console.log(`first feat row y at 390 px: ${Math.round(y)}`);
  expect(y).toBeLessThan(tabBar.y - 44);
  expect(y).toBeLessThan(viewport.height / 2);
  // The filter row is one row: the menu and the Feat chains link share a line, inside the width.
  const link = main.locator(".filter-row").getByRole("link", { name: "Feat chains" });
  const menu = (await button.boundingBox())!;
  const chains = (await link.boundingBox())!;
  expect(Math.abs(menu.y - chains.y)).toBeLessThan(8);
  expect(chains.x + chains.width).toBeLessThanOrEqual(viewport.width);
  expect(Math.round(menu.height)).toBeGreaterThanOrEqual(44);

  // The open panel is in the page, at most half the viewport, and its rows are 44 px.
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  const box = (await panel.boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(viewport.height * 0.5 + 1);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  const rows = await panel.locator(".choice-row").evaluateAll((els) => els.slice(0, 8).map((el) => el.getBoundingClientRect().height));
  expect(rows.filter((height) => height < 43.5)).toEqual([]);
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  // Choosing a type closes it, filters, and leaves the button focused.
  await panel.getByRole("searchbox").fill("comb");
  await panel.getByRole("link", { name: /^Combat, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/feat?facet=feat_type:Combat");
  await expect(main.getByRole("button", { name: "Type: Combat" })).toBeFocused();
  await expect(main.getByRole("button", { name: "Type: Combat" })).toHaveAttribute("aria-expanded", "false");
});

installedTest("Scoped Quick search at 390 px: the scope pill is a 44 px target and fits, also for a long kind name", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  for (const [hash, name] of [["#/browse/kind/feat", "Feats"], ["#/browse/kind/prestige_class", "Prestige classes"]] as const) {
    await page.goto(hash);
    const layer = await openQuickSearch(page);
    const pill = layer.getByRole("button", { name: `Remove scope: ${name}`, exact: true });
    await expect(pill).toBeVisible();
    await expect(quickField(page)).toHaveAttribute("placeholder", `Search ${name.toLowerCase()}`);
    const box = (await pill.boundingBox())!;
    expect(Math.round(box.height)).toBeGreaterThanOrEqual(44);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    await pill.click();
    await expect(pill).toHaveCount(0);
    await expect(quickField(page)).toBeFocused();
    await layer.getByRole("button", { name: "Cancel" }).click();
    await expect(layer).toBeHidden();
  }
});

installedTest("Every other kind page at 390 px shows its first row in the first viewport, with no horizontal scroll", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  const main = page.locator("main");
  const kinds: [string, string][] = [
    ["spell", "Spells"],
    ["trait", "Traits"],
    ["item", "Items"],
    ["archetype", "Archetypes"],
    ["class", "Classes"],
  ];
  const report: string[] = [];
  for (const [kind, title] of kinds) {
    await page.goto(`#/browse/kind/${kind}`);
    await expect(main.getByRole("heading", { level: 1, name: title })).toBeVisible();
    const menu = main.locator(".filter-btn");
    if (kind !== "class") await expect(menu).toBeEnabled();
    else await expect(menu).toHaveCount(0);
    const first = main.locator(".entry-list a").first();
    await expect(first).toBeVisible();
    const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
    const y = (await first.boundingBox())!.y;
    report.push(`${kind} ${Math.round(y)}`);
    expect(y, `${kind} first row`).toBeLessThan(tabBar.y - 44);
    expect(y, `${kind} first row`).toBeLessThan(viewport.height / 2);
    const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
    expect(overflow, `${kind} overflow`).toBeLessThanOrEqual(0);
    // A menu is one 44 px button; the pages beside it share its line.
    if (kind !== "class") expect(Math.round((await menu.boundingBox())!.height)).toBeGreaterThanOrEqual(44);
  }
  console.log(`first row y at 390 px: ${report.join(", ")}`);
});

installedTest("Items at 390 px: the category menu opens in the page, filters, and Back and Escape return", async ({ page }) => {
  await installLibrary(page);
  const viewport = page.viewportSize()!;
  await page.goto("#/browse/kind/item");
  const main = page.locator("main");
  const button = main.getByRole("button", { name: /^Category: / });
  await expect(button).toBeEnabled();
  await button.click();
  const panel = page.locator(`#${await button.getAttribute("aria-controls")}`);
  await expect(panel).toBeVisible();
  const box = (await panel.boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(viewport.height * 0.5 + 1);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  const rows = await panel.locator(".choice-row").evaluateAll((els) => els.slice(0, 8).map((el) => el.getBoundingClientRect().height));
  expect(rows.filter((height) => height < 43.5)).toEqual([]);
  // Escape closes it and returns focus to the button.
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
  await expect(button).toBeFocused();
  expect(new URL(page.url()).hash).toBe("#/browse/kind/item");
  // A choice filters, and Back undoes it.
  await button.click();
  await panel.getByRole("searchbox").fill("rods");
  await panel.getByRole("link", { name: /^Rods, / }).click();
  await expect.poll(() => new URL(page.url()).hash).toBe("#/browse/kind/item?facet=item_category:Rods");
  await expect(main.getByRole("button", { name: "Category: Rods" })).toBeFocused();
  const first = main.locator(".entry-list a").first();
  await expect(first).toBeVisible();
  const tabBar = (await page.getByRole("navigation", { name: "Primary" }).boundingBox())!;
  const y = (await first.boundingBox())!.y;
  console.log(`first item row y at 390 px, filtered to Rods: ${Math.round(y)}`);
  expect(y).toBeLessThan(tabBar.y - 44);
  await page.goBack();
  await expect(main.getByRole("button", { name: "Category: All categories" })).toBeVisible();
  const overflow = await page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});

installedTest("Classes at 390 px: the group headings are one line and the last group is reachable by scrolling", async ({ page }) => {
  await installLibrary(page);
  const types = new Set(
    entries
      .filter((entry) => entry.kind === "class")
      .map((entry) => entry.fields?.class_type)
      .filter((type): type is string => Boolean(type)),
  );
  const ungrouped = entries.some((entry) => entry.kind === "class" && !entry.fields?.class_type);
  await page.goto("#/browse/kind/class");
  const main = page.locator("main");
  const heads = main.locator("h2.letter-head");
  await expect(heads).toHaveCount(types.size + (ungrouped ? 1 : 0));
  const heights = await heads.evaluateAll((els) => els.map((el) => el.getBoundingClientRect().height));
  expect(Math.max(...heights)).toBeLessThan(48);
  const rows = await main.locator(".entry-list li").evaluateAll((els) => els.map((el) => el.getBoundingClientRect().height));
  expect(Math.min(...rows)).toBeGreaterThanOrEqual(44);
  const last = heads.nth((await heads.count()) - 1);
  await last.scrollIntoViewIfNeeded();
  await expect(last).toBeInViewport();
});
