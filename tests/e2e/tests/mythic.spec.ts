import { installedTest as test, expect, type Page } from "./fixtures";
import {
  bookTrees,
  hitTitles,
  installLibrary,
  openBookGroups,
  openQuickSearch,
  quickField,
  quickOptions,
  waitControlled,
  waitHomeIdle,
  waitReady,
  waitResults,
  writeMeta,
} from "./helpers";

const CORE = "core-rulebook";
const MYTHIC_BOOK = "mythic-adventures";
const BASE = "ref/feat/power-attack";
const MYTHIC = "ref/carried/mythic-adventures/feats/power-attack/mythic";

const allBooks = bookTrees().map((book) => book.source);

/** Writes the active set directly, then reloads. Turning off 400 books one row at a time is
 * not what this spec is about; the one book it turns on afterwards still goes through Active
 * books, as a reader would. */
async function onlyBooks(page: Page, active: string[]) {
  await writeMeta(
    page,
    "disabled_books",
    allBooks.filter((id) => !active.includes(id)),
  );
  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
}

test("Core only: a base feat shows no Mythic section, row or result", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE]);

  await page.goto(`#/entry/${BASE}`);
  await expect(page.getByRole("heading", { name: "Power Attack", level: 1 })).toBeVisible();
  await expect(page.locator(".rel-addon")).toHaveCount(0);
  await expect(page.locator(".entry-rel").getByRole("heading", { name: "Mythic", level: 2 })).toHaveCount(0);

  await page.goto("#/browse/kind/feat?l=P");
  await expect(page.locator(`a[href="#/entry/${BASE}"]`)).toBeVisible();
  await expect(page.locator('.entry-list a[href$="/mythic"]')).toHaveCount(0);

  await page.goto("#/search?q=power%20attack");
  await waitResults(page, "power attack");
  const titles = await hitTitles(page);
  expect(titles.some((title) => title.endsWith(", Mythic"))).toBe(false);

  // Typing the same words in Quick search offers no mythic suggestion either.
  await page.goto("#/");
  await openQuickSearch(page);
  await quickField(page).fill("power attack");
  await expect(quickOptions(page).last()).toHaveText("See all results for “power attack”");
  await expect(
    quickOptions(page).filter({ has: page.locator(".quick-search-name", { hasText: "Power Attack, Mythic" }) }),
  ).toHaveCount(0);
});

test("Mythic Adventures active: the base stacks one expanded version", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE]);

  // Turn the book on through Active books, as a reader does.
  await page.goto("#/books/active");
  await waitReady(page);
  await openBookGroups(page);
  const toggle = page
    .locator(`.book-directory-item[data-book-id="${MYTHIC_BOOK}"]`)
    .getByRole("checkbox", { name: "Active Mythic Adventures", exact: true });
  await toggle.check();
  await expect(toggle).toBeChecked();

  await page.goto(`#/entry/${BASE}`);
  const section = page.locator(".rel-addon");
  await expect(section.getByRole("heading", { name: "Mythic", level: 2 })).toBeVisible();
  await expect(section.getByRole("heading", { name: "Power Attack, Mythic", level: 3 })).toBeVisible();
  await expect(section.locator(".sources-line")).toHaveText("Sources: Mythic Adventures");
  await expect(section.locator(".rich")).toContainText("mythic power");
  // The stacked body carries no second stat block or left rule.
  await expect(section.locator(".stat-block")).toHaveCount(0);

  const link = section.getByRole("link", { name: "Power Attack, Mythic", exact: true });
  expect((await link.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  await link.click();
  await expect(page.getByTestId("entry-title")).toHaveText("Power Attack, Mythic");
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: "Mythic version of Power Attack", exact: true })).toBeVisible();
  await expect(meta).not.toContainText("Inactive");

  // The add-on is not a chain follower: the base's Leads to panel omits it.
  await page.goto(`#/entry/${BASE}`);
  await expect(page.locator("#rel-leads").getByRole("link", { name: "Power Attack, Mythic", exact: true })).toHaveCount(0);

  // In a feat list the add-on is its own row directly after its base, with its own label.
  await page.goto("#/browse/kind/feat?l=P");
  const hrefs = await page
    .locator(".entry-list > li > a")
    .evaluateAll((links) => links.map((link) => (link as HTMLAnchorElement).hash));
  const baseAt = hrefs.indexOf(`#/entry/${BASE}`);
  expect(baseAt).toBeGreaterThanOrEqual(0);
  expect(hrefs[baseAt + 1]).toBe(`#/entry/${MYTHIC}`);
  const row = page.locator(`.entry-list > li`).filter({ has: page.locator(`a[href="#/entry/${MYTHIC}"]`) });
  await expect(row.locator(".row-source")).toHaveText("Mythic · Mythic Adventures");

  // Search ranks the base first and labels the mythic row.
  await page.goto("#/search?q=dodge");
  await waitResults(page, "dodge");
  const titles = await hitTitles(page);
  const dodge = titles.indexOf("Dodge");
  const mythic = titles.indexOf("Dodge, Mythic");
  expect(dodge).toBeGreaterThanOrEqual(0);
  expect(mythic).toBeGreaterThan(dodge);
  const mythicRow = page
    .locator(".results .hit")
    .filter({ has: page.locator(".hit-title", { hasText: "Dodge, Mythic" }) });
  await expect(mythicRow.locator(".hit-meta")).toHaveText("Mythic · Mythic Adventures");

  // Quick search names the same label without leaving the page.
  await page.goto("#/");
  await openQuickSearch(page);
  await quickField(page).fill("power attack");
  const suggestion = quickOptions(page).filter({
    has: page.locator(".quick-search-name", { hasText: "Power Attack, Mythic" }),
  });
  await expect(suggestion).toHaveCount(1);
  await expect(suggestion.locator(".quick-search-meta")).toHaveText("Mythic · Mythic Adventures");
});

test("Mythic active, base inactive: the add-on stays listed and names the base", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [MYTHIC_BOOK]);

  await page.goto(`#/entry/${MYTHIC}`);
  await expect(page.getByTestId("entry-title")).toHaveText("Power Attack, Mythic");
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: "Mythic version of Power Attack", exact: true })).toBeVisible();
  await expect(meta.locator("span.tag")).toHaveText("Inactive");

  await page.goto("#/browse/kind/feat?l=P");
  await expect(page.locator(`a[href="#/entry/${MYTHIC}"]`)).toBeVisible();
  await expect(page.locator(`a[href="#/entry/${BASE}"]`)).toHaveCount(0);
  const row = page.locator(`.entry-list > li`).filter({ has: page.locator(`a[href="#/entry/${MYTHIC}"]`) });
  await expect(row.locator(".row-source")).toHaveText("Mythic · Mythic Adventures");

  await page.goto("#/search?q=power%20attack");
  await waitResults(page, "power attack");
  const mythicRow = page
    .locator(".results .hit")
    .filter({ has: page.locator(".hit-title", { hasText: "Power Attack, Mythic" }) });
  await expect(mythicRow.locator(".hit-meta")).toHaveText("Mythic · Mythic Adventures");
});

test("A base spell stacks its mythic version only while Mythic Adventures is active", async ({ page }) => {
  const spell = "ref/spell/magic-missile";
  const mythic = `${spell}/mythic`;
  await installLibrary(page);
  await waitControlled(page);

  await onlyBooks(page, [CORE]);
  await page.goto(`#/entry/${spell}`);
  await expect(page.getByRole("heading", { name: "Magic Missile", level: 1 })).toBeVisible();
  await expect(page.locator(".entry-rel").getByRole("heading", { name: "Mythic", level: 2 })).toHaveCount(0);

  // onlyBooks waits for Home's idle signal before it reloads, so go back to Home first.
  await page.goto("#/");
  await onlyBooks(page, [CORE, MYTHIC_BOOK]);
  await page.goto(`#/entry/${spell}`);
  const rel = page.locator(".entry-rel");
  await expect(rel.getByRole("heading", { name: "Mythic", level: 2 })).toBeVisible();
  await expect(rel.locator(`a[href="#/entry/${mythic}"]`)).toBeVisible();

  // The mythic version's own page names its base.
  await page.goto(`#/entry/${mythic}`);
  await expect(page.getByRole("link", { name: "Mythic version of Magic Missile", exact: true })).toBeVisible();
});
