import { installedTest as test, expect, type Page } from "./fixtures";
import {
  bookTrees,
  installLibrary,
  waitControlled,
  waitHomeIdle,
  waitReady,
  waitResults,
  writeMeta,
} from "./helpers";

const UNCHAINED = "pathfinder-unchained";
const PLAYER_GUIDE = "advanced-players-guide";
const ELEPHANT = "the-elephant-in-the-room-feat-taxes-in-pathfinder";
const BASE = "ref/class/unchained-monk";
const CHANGES = "ref/class/unchained-monk/teitr";
const BOOK = "The Elephant in the Room: Feat Taxes in Pathfinder";
const HEADING = `Changes in ${BOOK}`;
const PHRASE = `Changes ${BOOK} makes to Unchained Monk`;

const allBooks = bookTrees().map((book) => book.source);

/** Writes the active set directly, then reloads. Turning off 400 books one row at a time is
 * not what this spec is about. Home's idle signal must settle before the reload. */
async function onlyBooks(page: Page, active: string[]) {
  await page.goto("#/");
  await waitHomeIdle(page);
  await writeMeta(
    page,
    "disabled_books",
    allBooks.filter((id) => !active.includes(id)),
  );
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
}

test("The Elephant off: an Unchained Monk page shows no Changes section", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [UNCHAINED]);

  await page.goto(`#/entry/${BASE}`);
  await expect(page.getByRole("heading", { name: "Unchained Monk", level: 1 })).toBeVisible();
  await expect(page.locator(".rel-changes")).toHaveCount(0);
  await expect(page.locator(".entry-rel").getByRole("heading", { name: HEADING, level: 2 })).toHaveCount(0);

  // The add-on is not a search result either, because its own book is off.
  await page.goto("#/search?q=unchained%20monk");
  await waitResults(page, "unchained monk");
  await expect(
    page.locator(".results .hit").filter({ has: page.locator(".hit-title", { hasText: "Unchained Monk (The Elephant in the Room)" }) }),
  ).toHaveCount(0);
});

test("The Elephant active: the base shows one expanded Changes section", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [PLAYER_GUIDE, UNCHAINED, ELEPHANT]);

  await page.goto(`#/entry/${BASE}`);
  const section = page.locator(".rel-changes");
  await expect(section).toHaveCount(1);
  await expect(section.getByRole("heading", { name: HEADING, level: 2 })).toBeVisible();
  const link = section.getByRole("link", { name: "Unchained Monk (The Elephant in the Room)", exact: true });
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute("href", `#/entry/${CHANGES}`);
  // Expanded, like a mythic version: the body is already there while the page loads.
  const body = section.locator(".rich");
  await expect(body).toContainText("a monk can select a bonus feat");
  // The stacked body carries no second stat block or left rule, and the compiled body's own
  // two printed `Sources:` lines are the only ones in the section.
  await expect(section.locator(".stat-block")).toHaveCount(0);
  await expect(section.getByText(`Sources: ${BOOK}`, { exact: false })).toHaveCount(2);
  await expect(body).toContainText("Instead of Improved Unarmed Strike");

  // The add-on's own page names its base once, with the book inside the phrase.
  await link.click();
  await expect(page.getByTestId("entry-title")).toHaveText("Unchained Monk (The Elephant in the Room)");
  const meta = page.locator(".entry-meta");
  await expect(meta.getByRole("link", { name: PHRASE, exact: true })).toBeVisible();
  await expect(meta).not.toContainText("Inactive");

  // Search labels the row by its edge, not by its kind.
  await page.goto("#/search?q=unchained%20monk");
  await waitResults(page, "unchained monk");
  const row = page
    .locator(".results .hit")
    .filter({ has: page.locator(".hit-title", { hasText: "Unchained Monk (The Elephant in the Room)" }) });
  await expect(row).toHaveCount(1);
  await expect(row.locator(".hit-meta")).toHaveText(`Changes · ${BOOK}`);

  // Turning the book off removes the section on the next load.
  await onlyBooks(page, [PLAYER_GUIDE, UNCHAINED]);
  await page.goto(`#/entry/${BASE}`);
  await expect(page.locator(".rel-changes")).toHaveCount(0);
  await expect(page.locator(".entry-rel").getByRole("heading", { name: HEADING, level: 2 })).toHaveCount(0);
});

test("The Elephant active, the base's book off: the section still shows", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [ELEPHANT]);

  await page.goto(`#/entry/${BASE}`);
  const section = page.locator(".rel-changes");
  await expect(section).toHaveCount(1);
  await expect(section.getByRole("heading", { name: HEADING, level: 2 })).toBeVisible();
  await expect(section.getByRole("link", { name: "Unchained Monk (The Elephant in the Room)", exact: true })).toBeVisible();
});
