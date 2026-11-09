import { installedTest as test, expect, type Page } from "./fixtures";
import {
  bookTrees,
  hitTitles,
  installLibrary,
  waitControlled,
  waitHomeIdle,
  waitReady,
  waitResults,
  writeMeta,
} from "./helpers";

const CORE = "core-rulebook";
const ELEPHANT = "the-elephant-in-the-room-feat-taxes-in-pathfinder";
const DODGE = "ref/feat/dodge";
const DODGE_REVISION = "ref/feat/dodge/teitr";
const POWER_ATTACK = "ref/feat/power-attack";
const MOBILITY = "ref/feat/mobility";

const allBooks = bookTrees().map((book) => book.source);

/** Writes the active set directly, then reloads. Turning off hundreds of books one row at a
 * time is not what this spec is about. */
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

test("Elephant active: revised, removed and merged feats show one effective entry", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE, ELEPHANT]);

  // Revised: the original page shows the revision's text under a notice, with the original in
  // a collapsed disclosure.
  await page.goto(`#/entry/${DODGE}`);
  await expect(page.getByRole("heading", { name: "Dodge", level: 1 })).toBeVisible();
  const dodgeNotice = page.locator(".revision-notice");
  await expect(dodgeNotice).toContainText("Revised by The Elephant in the Room");
  await expect(dodgeNotice).toContainText("Changed: Benefit.");
  await expect(page.locator(".entry > .rich")).toContainText(
    "This bonus increases to +5 against attacks of opportunity",
  );
  const original = page.locator("details.rel-original");
  await expect(original).toHaveCount(1);
  await expect(original.locator("summary")).toContainText("Original text · Core Rulebook");
  await expect(original).not.toHaveAttribute("open", "");
  await original.locator("summary").click();
  await expect(original).toHaveAttribute("open", "");
  await expect(original).not.toContainText("+5 against attacks of opportunity");

  // Removed: the printed sentence, with no revised stat text.
  await page.goto(`#/entry/${POWER_ATTACK}`);
  await expect(page.locator(".revision-notice")).toContainText(
    "Removed by The Elephant in the Room",
  );
  await expect(page.locator(".revision-sentence")).toContainText(
    "This feat has been removed",
  );

  // Merged: the notice links the feat it merges into.
  await page.goto(`#/entry/${MOBILITY}`);
  const merged = page.locator(".revision-notice");
  await expect(merged).toContainText("Merged by The Elephant in the Room");
  await expect(merged.getByRole("link", { name: "Dodge", exact: true })).toHaveAttribute(
    "href",
    `#/entry/${DODGE}`,
  );

  // A list never shows the revision as a second Dodge row, and the effective row names the
  // book that revised it.
  await page.goto("#/browse/kind/feat?l=D");
  await expect(page.locator(`a[href="#/entry/${DODGE}"]`)).toHaveCount(1);
  await expect(page.locator(`a[href="#/entry/${DODGE_REVISION}"]`)).toHaveCount(0);
  const dodgeRow = page
    .locator(".entry-list > li")
    .filter({ has: page.locator(`a[href="#/entry/${DODGE}"]`) });
  await expect(dodgeRow.locator(".row-source")).toContainText("The Elephant in the Room");

  // Search shows one base row, from the revision's book.
  await page.goto("#/search?q=dodge");
  await waitResults(page, "dodge");
  const titles = await hitTitles(page);
  expect(titles.filter((title) => title === "Dodge")).toHaveLength(1);
  const dodgeHit = page
    .locator(".results .hit")
    .filter({ has: page.locator(".hit-title", { hasText: /^Dodge$/ }) });
  await expect(dodgeHit).toHaveCount(1);
  await expect(dodgeHit.locator(".hit-meta")).toContainText("The Elephant in the Room");

  // Turning the book off restores the original feat everywhere.
  await onlyBooks(page, [CORE]);
  await page.goto(`#/entry/${DODGE}`);
  await expect(page.locator(".revision-notice")).toHaveCount(0);
  await expect(page.locator("details.rel-original")).toHaveCount(0);
  await expect(page.locator(".entry > .rich")).not.toContainText(
    "+5 against attacks of opportunity",
  );

  await page.goto(`#/entry/${POWER_ATTACK}`);
  await expect(page.locator(".revision-notice")).toHaveCount(0);
  await expect(page.locator(".entry > .rich")).toContainText("You can choose to take a -1 penalty");

  await page.goto("#/browse/kind/feat?l=P");
  await expect(page.locator(`a[href="#/entry/${POWER_ATTACK}"]`)).toBeVisible();
  await expect(page.locator(`a[href="#/entry/${POWER_ATTACK}/teitr"]`)).toHaveCount(0);
});
