import type { Locator, Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// The class-option power layout (#537; decisions 2026-11-20-class-option-power-layout and
// 2026-11-21-class-option-power-owner-cases): a power reads as printed prose, one head per power
// on a page of several, the power folded into the entry head on a one-power page, and today's
// labeled rows for anything that is not a validated power.

const corpus = loadCorpus();

/** The text of a `.power`, one entry per paragraph after its head and meta line, in page order. */
const paragraphs = (power: Locator) => power.locator(":scope > p:not(.power-meta)").allInnerTexts();

const power = (page: Page, name: string) =>
  page.locator(".rich .power").filter({
    has: page.getByRole("heading", { name, exact: true }),
  });

async function open(page: Page, id: string, width = 1280) {
  await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
  await page.goto(`#/entry/${id}`);
  await expect(page.getByRole("heading", { level: 1, name: corpus.entries.get(id)!.title })).toBeVisible();
}

const overflow = (page: Page) =>
  page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("Abjuration lays its powers out as a head, a meta line, the effect, then one line per upgrade", async ({ page }) => {
  await open(page, "ref/arcaneschool/abjuration");

  // Resistance (Ex): a passive power. Its head holds the level, with no meta line.
  const resistance = power(page, "Resistance (Ex)");
  await expect(resistance.locator(".power-level")).toHaveText("Gained at Level 1");
  await expect(resistance.locator(".power-level")).toContainText("Level 1");
  await expect(resistance.locator(".power-meta")).toHaveCount(0);
  const lines = await paragraphs(resistance);
  // The base effect comes first, then one line per upgrade, in print order.
  expect(lines[0]).toMatch(/^You gain resistance 5 to an energy type of your choice/);
  expect(lines[1]).toMatch(/^At 11th level, this resistance increases to 10\./);
  expect(lines[2]).toMatch(/^At 20th level, this resistance changes to immunity/);
  expect(lines).toHaveLength(3);
  await expect(resistance.locator(".power-upgrade strong").first()).toHaveText("At 11th level,");
  // The labels of a passive row and of Gained are not shown.
  await expect(resistance).not.toContainText("Passive");
  await expect(resistance).not.toContainText("Gained:");

  // Protective Ward (Su): the action and the uses fold into one muted meta line.
  const ward = power(page, "Protective Ward (Su)");
  await expect(ward.locator(".power-meta")).toHaveText("Standard action·3 + Intelligence modifier times/day");
  // The dot is decorative: hidden from assistive technology.
  await expect(ward.locator(".power-meta .meta-sep")).toHaveAttribute("aria-hidden", "true");
  expect(await ward.locator(".power-meta span:not(.meta-sep)").allInnerTexts()).toEqual([
    "Standard action",
    "3 + Intelligence modifier times/day",
  ]);

  // Energy Absorption (Su) is gained at 6th level.
  await expect(power(page, "Energy Absorption (Su)").locator(".power-level")).toContainText("Level 6");

  // A power before the first body heading is h2; after `Banishment School` it is h4.
  await expect(page.getByRole("heading", { level: 2, name: "Resistance (Ex)", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { level: 4, name: "Aura of Banishment (Su)", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { level: 3, name: "Banishment School" })).toBeVisible();

  // A passive row and an action row: the passive text first, then the action as its own line.
  const mastery = power(page, "Counterspell Mastery");
  await expect(mastery.locator(".power-meta")).toContainText("Immediate action");
  const masteryLines = await paragraphs(mastery);
  expect(masteryLines[0]).toMatch(/^You gain Improved Counterspell/);
  expect(masteryLines[1]).toMatch(/^You may attempt to counterspell an opponent's spell/);

  // The power names are not in the page outline: they are not in the contents.
  await expect(page.locator(".entry-contents, nav[aria-label='On this page']").getByText("Resistance (Ex)")).toHaveCount(0);
});

test("A mystery keeps the Prereq row and shows no level where Gained is absent", async ({ page }) => {
  await open(page, "ref/mystery/flame");

  const firestorm = power(page, "Firestorm (Su)");
  await expect(firestorm.locator(".power-level")).toHaveCount(0);
  await expect(firestorm.locator(".power-leader")).toHaveCount(0);
  await expect(firestorm.locator(".power-meta")).toHaveText("Standard action·Once per day.");
  const lines = await paragraphs(firestorm);
  expect(lines[0]).toBe("You can cause fire to erupt around you."); // Flavor, without its label
  expect(lines[1]).toMatch(/^You can create one 10-foot cube of fire per oracle level/);
  // The level of the prerequisite never becomes the level of the power.
  expect(lines.at(-1)).toBe("Prereq: 11th-level oracle");

  // The base effect, then the upgrades in ascending level, then the other rows.
  const form = power(page, "Form of Flame (Su)");
  const formLines = await paragraphs(form);
  expect(formLines[0]).toMatch(/^You can assume the form of a Small fire elemental/);
  expect(formLines.slice(1, 4).map((line) => line.slice(0, 14))).toEqual([
    "At 9th level, ",
    "At 11th level,",
    "At 13th level,",
  ]);
  // Prereq and the other rows come after the upgrades, with their bold labels.
  expect(formLines.slice(4).map((line) => line.split(":")[0])).toEqual(["Prereq", "Special"]);

  // Special keeps its bold label and stays after the effect.
  const burning = power(page, "Burning Magic (Su)");
  const burningLines = await paragraphs(burning);
  expect(burningLines.at(-1)).toMatch(/^Special: Spells that do not grant a save/);
  await expect(burning.locator(":scope > p").last().locator("strong")).toHaveText("Special:");
});

test("A section label and the blocks under it render as they did before", async ({ page }) => {
  await open(page, "ref/domain/fire");

  // Fire Resistance (Ex): the base effect, then its upgrades.
  const resistance = power(page, "Fire Resistance (Ex)");
  const lines = await paragraphs(resistance);
  expect(lines[0]).toMatch(/^You gain resist fire/);
  expect(lines[1]).toMatch(/^At 12th level, your resistance increases to 20\./);
  expect(lines[2]).toMatch(/^At 20th level, you gain immunity/);

  // `Domain Spells` is a section label, not a power: a bold paragraph, then its list, no `.power`.
  const label = page.locator(".rich > p", { has: page.locator(":scope > strong", { hasText: /^Domain Spells$/ }) });
  await expect(label).toHaveCount(1);
  await expect(label.locator("xpath=ancestor::*[contains(@class,'power')]")).toHaveCount(0);
  await expect(label.locator("xpath=following-sibling::*[1]")).toHaveJSProperty("tagName", "OL");

  // `Replacement Domain Spells` of a subdomain keeps its labeled rows.
  const replacement = page
    .locator(".rich > p", { has: page.locator(":scope > strong", { hasText: /^Replacement Domain Spells$/ }) })
    .first();
  await expect(replacement).toBeVisible();
  const next = replacement.locator("xpath=following-sibling::p[1]");
  await expect(next.locator("strong")).toHaveText("2nd:");
  await expect(next).toHaveText("2nd: flaming sphere");
  await expect(page.locator(".rich .power").filter({ hasText: "Replacement Domain Spells" })).toHaveCount(0);
});

test("A one-power page folds the head into the title and shows a facts line", async ({ page }) => {
  const evil = corpus.entries.get("ref/hex/evil-eye")!;
  await open(page, evil.id);

  // The title is the entry title, never the name with its tag.
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(evil.title);
  // No power head repeats it.
  await expect(page.locator(".rich .power-head, .rich .power-name, .rich .power-meta")).toHaveCount(0);
  // The metadata line keeps kind and book; the facts line follows it and holds only text.
  const facts = page.locator(".entry-head > .entry-facts");
  await expect(facts).toHaveText("Supernatural (Su)");
  await expect(facts.getByRole("link")).toHaveCount(0);
  const order = await page.locator(".entry-head").evaluate((head) => Array.from(head.children).map((c) => c.className));
  expect(order.indexOf("entry-meta")).toBeLessThan(order.indexOf("entry-facts"));

  // The body starts with Flavor, without its label, then the effect, the upgrade and the rest.
  const lines = await paragraphs(page.locator(".rich .power"));
  expect(lines[0]).toBe("The witch can cause doubt to creep into the mind of a foe within 30 feet that she can see.");
  expect(lines[1]).toMatch(/^The target takes a -2 penalty/);
  expect(lines[2]).toMatch(/^At 8th level, the penalty increases to -4\./);
  expect(lines[3]).toMatch(/^Special: This hex lasts for a number of rounds/);
});

test("Powers wrap inside 390 px and the level stays on the name's first line", async ({ page }) => {
  await open(page, "ref/arcaneschool/abjuration", 390);
  expect(await overflow(page)).toBeLessThanOrEqual(0);
  const head = power(page, "Protective Ward (Su)").locator(".power-head");
  const [name, level] = await Promise.all([
    head.locator(".power-name").boundingBox(),
    head.locator(".power-level").boundingBox(),
  ]);
  expect(level!.x + level!.width).toBeLessThanOrEqual(390);
  expect(level!.y).toBeLessThan(name!.y + name!.height);
});
