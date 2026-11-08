import { installedTest as test, expect, loadCorpus, type Page } from "./fixtures";
import {
  bookTrees,
  hitTitles,
  installLibrary,
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
const UNCHAINED = "pathfinder-unchained";
const BASE = "ref/feat/power-attack";
const STAMINA = "ref/feat/power-attack/stamina";
const STAMINA_SUMMARY = "Stamina · Pathfinder Unchained";

const allBooks = bookTrees().map((book) => book.source);
const staminaShard = loadCorpus().catalog.entries.find((entry) => entry.id === STAMINA)!.shard;

/** Writes the active set directly, then reloads. Turning off 400 books one row at a time is
 * not what this spec is about; the one book it turns on afterwards still goes through Active
 * books, as a reader would. */
async function onlyBooks(page: Page, active: string[]) {
  await writeMeta(
    page,
    "disabled_books",
    allBooks.filter((id) => !active.includes(id)),
  );
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
}

test("Core only: a base feat shows no Stamina row", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE]);

  await page.goto(`#/entry/${BASE}`);
  await expect(page.getByRole("heading", { name: "Power Attack", level: 1 })).toBeVisible();
  await expect(page.locator(".rel-stamina")).toHaveCount(0);
  await expect(page.locator("summary", { hasText: "Stamina ·" })).toHaveCount(0);

  await page.goto("#/search?q=power%20attack");
  await waitResults(page, "power attack");
  const titles = await hitTitles(page);
  expect(titles.some((title) => title === "Power Attack (Stamina)")).toBe(false);

  await page.goto("#/");
  await openQuickSearch(page);
  await quickField(page).fill("power attack");
  await expect(
    quickOptions(page).filter({ has: page.locator(".quick-search-name", { hasText: "Power Attack (Stamina)" }) }),
  ).toHaveCount(0);
});

test("Pathfinder Unchained active: the base folds one collapsed Stamina row", async ({ page, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE, UNCHAINED]);
  await server.clearLog();

  await page.goto(`#/entry/${BASE}`);
  const row = page.locator("details.rel-stamina");
  await expect(row).toHaveCount(1);
  const summary = row.locator("summary");
  await expect(summary).toHaveText(STAMINA_SUMMARY);
  // Collapsed by default: no body, no status line, and no shard request.
  await expect(row).not.toHaveAttribute("open", "");
  await expect(row.locator(".rich")).toHaveCount(0);
  await expect(row.locator(".status")).toHaveCount(0);
  await page.waitForTimeout(500);
  expect((await server.requests()).filter((path) => path.endsWith(staminaShard))).toEqual([]);

  // The summary is a full touch target and wraps nothing at 390 px.
  expect((await summary.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);

  // Keyboard: Enter and Space both toggle the summary.
  await summary.focus();
  await page.keyboard.press("Enter");
  await expect(row).toHaveAttribute("open", "");
  await expect(row.getByText("spend 2 stamina points")).toBeVisible();
  await page.keyboard.press("Space");
  await expect(row).not.toHaveAttribute("open", "");
  await page.keyboard.press("Space");
  await expect(row).toHaveAttribute("open", "");
  // The body stays mounted through close and reopen: one load, not one per open.
  await expect(row.locator(".rich")).toHaveCount(1);

  // The folded body carries no second stat block.
  await expect(row.locator(".stat-block")).toHaveCount(0);

  // The add-on is not a chain follower: the base's Leads to panel omits it.
  await expect(
    page.locator("#rel-leads").getByRole("link", { name: "Power Attack (Stamina)", exact: true }),
  ).toHaveCount(0);

  // The add-on's own page names the base once.
  await page.goto(`#/entry/${STAMINA}`);
  await expect(page.getByTestId("entry-title")).toHaveText("Power Attack (Stamina)");
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: "Stamina rules for Power Attack", exact: true })).toBeVisible();
  await expect(meta).not.toContainText("Inactive");

  // Search labels the row by its edge, not by its kind.
  await page.goto("#/search?q=power%20attack");
  await waitResults(page, "power attack");
  const staminaRow = page
    .locator(".results .hit")
    .filter({ has: page.locator(".hit-title", { hasText: "Power Attack (Stamina)" }) });
  await expect(staminaRow).toHaveCount(1);
  await expect(staminaRow.locator(".hit-meta")).toHaveText("Stamina · Pathfinder Unchained");

  // Quick search names the same label without leaving the page.
  await page.goto("#/");
  await openQuickSearch(page);
  await quickField(page).fill("power attack");
  const suggestion = quickOptions(page).filter({
    has: page.locator(".quick-search-name", { hasText: "Power Attack (Stamina)" }),
  });
  await expect(suggestion).toHaveCount(1);
  await expect(suggestion.locator(".quick-search-meta")).toHaveText("Stamina · Pathfinder Unchained");
});

test("Unchained active, base inactive: the add-on names its inactive base", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [UNCHAINED]);

  await page.goto(`#/entry/${STAMINA}`);
  await expect(page.getByTestId("entry-title")).toHaveText("Power Attack (Stamina)");
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: "Stamina rules for Power Attack", exact: true })).toBeVisible();
  await expect(meta.locator("span.tag")).toHaveText("Inactive");
});
