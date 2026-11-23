import { installedTest as test, expect, type Page } from "./fixtures";
import {
  bookSources,
  hitTitles,
  installLibrary,
  openQuickSearch,
  quickSearch,
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

const allBooks = bookSources();

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

test("Elephant active: Quick search names the revising book, and Core alone names Core", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const dodgeLine = async () => {
    await page.goto("#/");
    await waitHomeIdle(page);
    const layer = await openQuickSearch(page);
    await layer.getByRole("combobox", { name: "Search the rules" }).fill("Dodge");
    const row = quickSearch(page)
      .locator('[role="option"]')
      .filter({ has: page.locator(".quick-search-name", { hasText: /^Dodge$/ }) });
    await expect(row).toHaveCount(1);
    return row.locator(".quick-search-meta");
  };
  // The effective text is the revision's, so the row names its book, not the base feat's.
  await onlyBooks(page, [CORE, ELEPHANT]);
  await expect(await dodgeLine()).toContainText("The Elephant in the Room");
  // With the revision's book off, the original feat names its own book again.
  await onlyBooks(page, [CORE]);
  await expect(await dodgeLine()).toContainText("Core Rulebook");
});

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
  // Each changed stat row carries the quiet `changed` label, and the metadata line names the
  // book that revised the feat.
  await expect(page.locator(".entry > .rich .changed-tag")).toHaveCount(1);
  const benefitLabel = page.locator(".entry > .rich strong", { hasText: /^Benefit:/ });
  await expect(benefitLabel.locator("xpath=following-sibling::*[1]")).toHaveClass(
    /changed-tag/,
  );
  const meta = page.locator(".entry-meta");
  await expect(meta).toContainText("The Elephant in the Room");
  await expect(meta).not.toContainText("Core Rulebook");
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

test("A revision's own page names its base and marks it Inactive when the base's book is off", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [ELEPHANT]);

  await page.goto(`#/entry/${DODGE_REVISION}`);
  await expect(page.getByTestId("entry-title")).toHaveText("Dodge");
  const meta = page.locator(".entry-meta");
  const base = meta.getByRole("link", { name: "Revision of Dodge", exact: true });
  await expect(base).toBeVisible();
  await expect(base).toHaveAttribute("href", `#/entry/${DODGE}`);
  await expect(meta.locator(".tag")).toHaveText("Inactive");
});

test("A merged notice marks an inactive merge target", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [ELEPHANT]);

  await page.goto(`#/entry/${MOBILITY}`);
  const merged = page.locator(".revision-notice");
  await expect(merged).toContainText("Merged by The Elephant in the Room");
  await expect(merged.getByRole("link", { name: "Dodge", exact: true })).toHaveAttribute(
    "href",
    `#/entry/${DODGE}`,
  );
  await expect(merged.locator(".tag")).toHaveText("Inactive");
});

test("The feat chain follows a revised feat's prerequisites", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE]);

  // Point-Blank Shot is printed as a chain head: it has dependents and no prerequisite.
  await page.goto("#/feats/chains");
  const heads = page.locator(".chain-head .chain-name");
  await expect(heads.filter({ hasText: /^Point-Blank Shot$/ })).toHaveCount(1);
  await expect(heads.filter({ hasText: /^Precise Shot$/ })).toHaveCount(0);

  // The revision gives Point-Blank Shot Precise Shot as a prerequisite, and the revised
  // Precise Shot drops its own. The effective graph moves the head, so no chain starts from
  // the printed version.
  await onlyBooks(page, [CORE, ELEPHANT]);
  await page.goto("#/feats/chains");
  await expect(heads.filter({ hasText: /^Point-Blank Shot$/ })).toHaveCount(0);
  await expect(heads.filter({ hasText: /^Precise Shot$/ })).toHaveCount(1);
});

test("A facet count drops for a feat that an active revision removes", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE]);

  // The feat type is a filter menu (#579): its Combat row names its count and shows it.
  const combatCount = async () => {
    const trigger = page.locator(".filters").getByRole("button", { name: "Feat type: All types", exact: true });
    if ((await trigger.getAttribute("aria-expanded")) !== "true") await trigger.click();
    const row = page
      .locator(`#${await trigger.getAttribute("aria-controls")}`)
      .getByRole("checkbox", { name: /^Feat type Combat, / });
    return row.locator("xpath=..").locator(".choice-count");
  };
  await page.goto("#/search?q=finesse&kind=feat");
  await waitResults(page, "finesse");
  await expect(await combatCount()).toHaveText("1");

  // The revision removes Weapon Finesse from the effective rows, so the facet count drops by
  // that one feat even though search still finds the removed feat.
  await onlyBooks(page, [CORE, ELEPHANT]);
  await page.goto("#/search?q=finesse&kind=feat");
  await waitResults(page, "finesse");
  await expect(await combatCount()).toHaveText("0");
});

test("A revised page keeps its notice and original text above the body on a wide screen", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE, ELEPHANT]);
  // The wide entry is a grid with named areas. An unnamed child auto-places after the footer,
  // so every closing notice needs a row of its own.
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(`#/entry/${DODGE}`);
  await expect(page.getByRole("heading", { name: "Dodge", level: 1 })).toBeVisible();

  const head = page.locator(".entry > .entry-head");
  const notice = page.locator(".entry > .revision-notice");
  const disclosure = page.locator(".entry > .rel-original");
  const body = page.locator(".entry > .rich");
  const foot = page.locator(".entry > .entry-foot");
  await expect(notice).toContainText("Revised by The Elephant in the Room");
  await expect(disclosure).toBeVisible();

  const [headBox, noticeBox, disclosureBox, bodyBox, footBox] = await Promise.all(
    [head, notice, disclosure, body, foot].map(async (locator) => {
      const box = await locator.boundingBox();
      expect(box, "the element is laid out").not.toBeNull();
      return box!;
    }),
  );
  const inContentColumn = (box: { x: number }) => Math.abs(box.x - bodyBox.x) < 1;
  expect(noticeBox.y, "the notice is under the header").toBeGreaterThanOrEqual(
    headBox.y + headBox.height,
  );
  expect(inContentColumn(noticeBox), "the notice is in the content column").toBe(true);
  expect(inContentColumn(disclosureBox), "the original text is in the content column").toBe(true);
  expect(noticeBox.y + noticeBox.height, "the original text follows the notice").toBeLessThanOrEqual(
    disclosureBox.y,
  );
  expect(disclosureBox.y + disclosureBox.height, "the original text is above the body").toBeLessThanOrEqual(
    bodyBox.y,
  );
  expect(disclosureBox.y + disclosureBox.height, "the original text is above the footer").toBeLessThanOrEqual(
    footBox.y,
  );
});

test("A removed entry keeps its printed sentence in the content column on a wide screen", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE, ELEPHANT]);
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(`#/entry/${POWER_ATTACK}`);
  await expect(page.getByRole("heading", { name: "Power Attack", level: 1 })).toBeVisible();

  // A removed revision renders its printed sentence as a second `.rich`, so it needs its own
  // row; without one it takes the body's area and the two overlap.
  const sentence = page.locator(".entry > .revision-sentence");
  const body = page.locator(".entry > .rich:not(.revision-sentence)");
  const foot = page.locator(".entry > .entry-foot");
  await expect(sentence).toContainText("This feat has been removed");

  const sentenceBox = (await sentence.boundingBox())!;
  const bodyBox = (await body.boundingBox())!;
  const footBox = (await foot.boundingBox())!;
  expect(Math.abs(sentenceBox.x - bodyBox.x), "the sentence is in the content column").toBeLessThan(1);
  expect(sentenceBox.y + sentenceBox.height, "the sentence is above the body").toBeLessThanOrEqual(
    bodyBox.y,
  );
  expect(sentenceBox.y + sentenceBox.height, "the sentence is above the footer").toBeLessThanOrEqual(
    footBox.y,
  );
});
