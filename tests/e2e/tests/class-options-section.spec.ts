import fs from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { GENERATED, installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// The Options section of a class page (#739): a stop between Features and Archetypes when the class
// has an option group, one chip per group, the group's options as rows with a name field and a
// book menu, and a row under the feature that grants the choice. Expectations come from the
// compiled corpus (docs/decisions/2026-11-27-class-options-section.md).

const corpus = loadCorpus();
const groups: { slug: string; label: string; singular: string; classes: string[]; entries: string[] }[] = JSON.parse(
  fs.readFileSync(path.join(GENERATED, corpus.version, "option-groups.json"), "utf8"),
).groups;
const stubs = new Map<string, any>(corpus.catalog.entries.map((stub: any) => [stub.id, stub]));

const ROGUE = "ref/class/rogue";
const WIZARD = "ref/class/wizard";
const FIGHTER = "ref/class/fighter";
const talents = groups.find((group) => group.slug === "rogue-talent")!;

const switcher = (page: Page) => page.getByRole("navigation", { name: "Sections" });
const stop = (page: Page, name: string) => switcher(page).getByRole("link", { name, exact: true });
const chips = (page: Page) => page.getByRole("navigation", { name: "Option groups" });
const rows = (page: Page) => page.locator(".opt-rows > li");
const countLine = (page: Page) => page.locator(".opt-list .list-count");

async function openClass(page: Page, id: string, query = "", width = 1280) {
  await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
  await installLibrary(page);
  await page.goto(`#/entry/${id}${query}`);
  await expect(page.getByRole("heading", { level: 1, name: corpus.entries.get(id)!.title })).toBeVisible();
}

test("Rogue has an Options stop with a chip for rogue talents and its options as rows", async ({ page }) => {
  await openClass(page, ROGUE);
  await expect(switcher(page).getByRole("link")).toHaveText(["Overview", "Features", "Options", "Archetypes"]);
  await expect(stop(page, "Options")).toHaveAttribute("href", `#/entry/${ROGUE}?s=options`);
  await expect(page.locator('h2[data-class-section="options"]')).toHaveText("Options");

  // One chip, the group's label and its count, marked as the one that is shown.
  const chip = chips(page).getByRole("link");
  await expect(chip).toHaveCount(1);
  await expect(chip).toHaveText(`${talents.label}${talents.entries.length}`);
  await expect(chip).toHaveAttribute("aria-current", "true");

  // The list: a count line, then a row for each option, A to Z, each opening its own page.
  await expect(countLine(page)).toHaveText(`${talents.entries.length} rogue talents`);
  await expect(rows(page)).toHaveCount(talents.entries.length);
  const titles = talents.entries.map((id) => stubs.get(id).title as string);
  const first = [...titles].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()))[0];
  await expect(rows(page).first().locator("a .opt-name")).toHaveText(first);
  // Each row names its book on a muted line.
  await expect(rows(page).first().locator(".arch-meta")).not.toBeEmpty();

  // An option with a printed ability tag shows it, in italics, after the name.
  await expect(page.locator(".opt-rows .opt-tag").first()).toHaveText(/^\((Ex|Su|Sp)/);
  await expect(page.locator(".opt-rows .opt-tag").first()).toHaveCSS("font-style", "italic");

  // Typing narrows the list in place, and the text is the address's (`find=`).
  await page.locator("#opt-find").fill("bleed");
  const wanted = titles.filter((title) => title.toLowerCase().includes("bleed")).length;
  expect(wanted).toBeGreaterThan(0);
  await expect(rows(page)).toHaveCount(wanted);
  await expect(countLine(page)).toContainText(`${wanted} of ${talents.entries.length} rogue talents`);
  await expect.poll(() => page.evaluate(() => location.hash)).toContain("find=bleed");
  await expect.poll(() => page.evaluate(() => location.hash)).toContain("group=rogue-talent");

  // The book menu narrows by one book, and its count is the library's.
  const sources = new Map<string, number>();
  for (const id of talents.entries) sources.set(stubs.get(id).source, (sources.get(stubs.get(id).source) ?? 0) + 1);
  await countLine(page).getByRole("button", { name: "Show all" }).click();
  await expect(rows(page)).toHaveCount(talents.entries.length);
  await page.getByRole("button", { name: "Book: All books" }).click();
  const [book, expected] = [...sources.entries()].find(([, n]) => n > 1)!;
  const bookTitle = corpus.catalog.sources.find((source: any) => source.id === book).title as string;
  await page.getByRole("link", { name: `${bookTitle}, ${expected}`, exact: true }).click();
  await expect(rows(page)).toHaveCount(expected);
  await expect.poll(() => page.evaluate(() => location.hash)).toContain(`source=${book}`);

  // A row opens the option's page.
  await countLine(page).getByRole("button", { name: "Show all" }).click();
  const row = rows(page).first();
  const name = (await row.locator(".opt-name").textContent())!;
  await row.locator("a").click();
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
});

test("The feature that grants rogue talents has a row that opens the section at that group", async ({ page }) => {
  await openClass(page, ROGUE);
  const link = page.locator(".opt-link a");
  await expect(link).toHaveCount(1);
  await expect(link).toContainText(`${talents.label} · ${talents.entries.length}`);
  // The row sits under the text of the feature it belongs to.
  const heading = page.getByRole("heading", { name: /^Rogue Talents/ }).first();
  const headingBox = (await heading.boundingBox())!;
  const linkBox = (await link.boundingBox())!;
  expect(linkBox.y).toBeGreaterThan(headingBox.y);

  await link.click();
  await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${ROGUE}?s=options&group=rogue-talent`);
  await expect(stop(page, "Options")).toHaveAttribute("aria-current", "true");
  await expect(chips(page).getByRole("link")).toHaveAttribute("aria-current", "true");
  await expect(page.locator('h2[data-class-section="options"]')).toBeInViewport();
});

test("A class with two groups has a chip each, and the address names the group that is shown", async ({ page }) => {
  const wizard = groups.filter((group) => group.classes.includes(WIZARD));
  expect(wizard.length).toBe(2);
  await openClass(page, WIZARD, `?s=options&group=${wizard[1].slug}`);
  await expect(chips(page).getByRole("link")).toHaveCount(2);
  await expect(chips(page).getByRole("link", { name: new RegExp(`^${wizard[1].label}`) })).toHaveAttribute(
    "aria-current",
    "true",
  );
  await expect(rows(page)).toHaveCount(wizard[1].entries.length);

  // The other chip shows its own list, resets the filters, and is one more history entry.
  await chips(page).getByRole("link", { name: new RegExp(`^${wizard[0].label}`) }).click();
  await expect(rows(page)).toHaveCount(wizard[0].entries.length);
  await expect.poll(() => page.evaluate(() => location.hash)).toContain(`group=${wizard[0].slug}`);
  await page.goBack();
  await expect(rows(page)).toHaveCount(wizard[1].entries.length);

  // An unknown group shows the class's first.
  await page.goto(`#/entry/${WIZARD}?s=options&group=nope`);
  await expect(chips(page).getByRole("link").first()).toHaveAttribute("aria-current", "true");
});

test("A class with no option group has no Options stop and drops an address that names it", async ({ page }) => {
  expect(groups.some((group) => group.classes.includes(FIGHTER))).toBe(false);
  await openClass(page, FIGHTER, "?s=options");
  await expect(switcher(page).getByRole("link")).toHaveText(["Overview", "Features", "Archetypes"]);
  await expect(page.locator('[data-class-section="options"]')).toHaveCount(0);
  await expect(page.locator(".opt-section")).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#/entry/${FIGHTER}`);
});

test("At 390 px the chips and the list fit the screen", async ({ page }) => {
  await openClass(page, ROGUE, "?s=options&group=rogue-talent", 390);
  await expect(rows(page).first()).toBeVisible();
  const overflow = await page.evaluate(
    () => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
  // Four stops share one row, so the sticky bar keeps the height every section's scroll margin assumes.
  expect((await switcher(page).boundingBox())!.height).toBeLessThan(64);
  // The chips are one row that scrolls sideways, not a page that does.
  const chip = (await chips(page).getByRole("link").first().boundingBox())!;
  expect(chip.height).toBeGreaterThanOrEqual(44);
  expect(chip.x + chip.width).toBeLessThanOrEqual(390);
});
