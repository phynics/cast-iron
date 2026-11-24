import type { Locator, Page } from "@playwright/test";
import { installedTest as test, expect, loadCorpus } from "./fixtures";
import { installLibrary } from "./helpers";

// The class summary card and the folded reference panels of a class page (#170, parts B and F).
// Expectations come from the compiled corpus where it types the value; the rest is the printed
// value the card reads (docs/decisions.md, "Class pages: summary card").

const corpus = loadCorpus();
const entryOf = (id: string) => corpus.entries.get(id)!;

const card = (page: Page) => page.locator("section.class-card");
const rowOf = (page: Page, label: string) =>
  card(page).locator(".stat-row").filter({ has: page.locator("dt", { hasText: new RegExp(`^${label}$`) }) });
const rowTexts = (page: Page) =>
  card(page).locator(".stat-row dt").allTextContents();
const overflow = (page: Page) =>
  page.evaluate(() => Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) - window.innerWidth);

async function openClass(page: Page, id: string, width: number) {
  await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
  await page.goto(`#/entry/${id}`);
  await expect(page.getByRole("heading", { level: 1, name: entryOf(id).title })).toBeVisible();
  await expect(card(page)).toBeVisible();
}

/** The printed bold-label paragraphs of the intro, before the progression table. */
const introLabels = (page: Page) =>
  page.locator(".entry > .rich").first().evaluate((rich) => {
    const labels: string[] = [];
    for (const child of Array.from(rich.children)) {
      if (child.matches(".table-wrap, details, h2, h3")) break;
      const strong = child.matches("p") ? child.querySelector(":scope > strong") : null;
      if (strong) labels.push(strong.textContent ?? "");
    }
    return labels;
  });

test("Wizard's summary card holds the class basics at 390 px", async ({ page }) => {
  const width = 390;
  await installLibrary(page);
  const wizard = entryOf("ref/class/wizard");
  await openClass(page, "ref/class/wizard", width);

  expect(await rowTexts(page)).toEqual([
    "Hit Die",
    "Base attack",
    "Good saves",
    "Skill ranks",
    "Class skills",
    "Alignment",
    "Starting wealth",
    "Spellcasting",
  ]);
  await expect(rowOf(page, "Hit Die").locator("dd")).toHaveText(wizard.fields.hit_die);
  await expect(rowOf(page, "Base attack").locator("dd")).toContainText("Poor");
  await expect(rowOf(page, "Good saves").locator("dd")).toHaveText("Will");
  await expect(rowOf(page, "Skill ranks").locator("dd")).toContainText(wizard.fields.skill_ranks);
  await expect(rowOf(page, "Alignment").locator("dd")).toHaveText("Any");
  await expect(rowOf(page, "Starting wealth").locator("dd")).toContainText("2d6");
  await expect(rowOf(page, "Spellcasting").locator("dd")).toContainText("Arcane, prepared · Intelligence");
  const list = rowOf(page, "Spellcasting").getByRole("link", { name: "Wizard spell list" });
  await expect(list).toHaveAttribute("href", "#/spells/Wizard");
  expect((await list.boundingBox())!.height).toBeGreaterThanOrEqual(44);

  // The card is a stat block: it keeps the left accent rule.
  expect(await card(page).evaluate((el) => getComputedStyle(el).borderLeftWidth)).toBe("4px");

  // Class skills are compact and open in place.
  const skills = rowOf(page, "Class skills").locator("details.class-skills");
  const summary = skills.locator("summary");
  await expect(summary).toHaveText(/^\d+ skills$/);
  expect((await summary.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  await expect(skills.getByText("Spellcraft")).toBeHidden();
  await summary.click();
  await expect(skills.getByText("Spellcraft")).toBeVisible();
  await expect(skills.locator("li").first()).toContainText("Fly");

  // The card replaces the printed basics: none of them shows again in the body.
  const labels = await introLabels(page);
  for (const gone of ["Hit Die:", "Alignment:", "Starting Wealth:", "Skill Points per Level:"]) {
    expect(labels, gone).not.toContain(gone);
  }
  expect(labels.filter((label) => label.startsWith("Class Skills"))).toEqual([]);
  expect(labels).toContain("Role:");
  // The card does not move the page sideways.
  expect(await overflow(page)).toBeLessThanOrEqual(0);
  // At 390 px the whole card is in the first viewport.
  if (width === 390) expect((await card(page).boundingBox())!.y + (await card(page).boundingBox())!.height).toBeLessThan(844);
});

test("Fighter's summary card has no spellcasting row at 390 px", async ({ page }) => {
  const width = 390;
  await installLibrary(page);
  const fighter = entryOf("ref/class/fighter");
  await openClass(page, "ref/class/fighter", width);

  expect(await rowTexts(page)).toEqual([
    "Hit Die",
    "Base attack",
    "Good saves",
    "Skill ranks",
    "Class skills",
    "Alignment",
    "Starting wealth",
  ]);
  await expect(rowOf(page, "Hit Die").locator("dd")).toHaveText(fighter.fields.hit_die);
  await expect(rowOf(page, "Base attack").locator("dd")).toContainText("Good");
  await expect(rowOf(page, "Good saves").locator("dd")).toHaveText("Fortitude");
  await expect(rowOf(page, "Starting wealth").locator("dd")).toContainText("5d6");
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});

test("Wizard's summary card holds the class basics at 1280 px", async ({ page }) => {
  const width = 1280;
  await installLibrary(page);
  const wizard = entryOf("ref/class/wizard");
  await openClass(page, "ref/class/wizard", width);

  expect(await rowTexts(page)).toEqual([
    "Hit Die",
    "Base attack",
    "Good saves",
    "Skill ranks",
    "Class skills",
    "Alignment",
    "Starting wealth",
    "Spellcasting",
  ]);
  await expect(rowOf(page, "Hit Die").locator("dd")).toHaveText(wizard.fields.hit_die);
  await expect(rowOf(page, "Base attack").locator("dd")).toContainText("Poor");
  await expect(rowOf(page, "Good saves").locator("dd")).toHaveText("Will");
  await expect(rowOf(page, "Skill ranks").locator("dd")).toContainText(wizard.fields.skill_ranks);
  await expect(rowOf(page, "Alignment").locator("dd")).toHaveText("Any");
  await expect(rowOf(page, "Starting wealth").locator("dd")).toContainText("2d6");
  await expect(rowOf(page, "Spellcasting").locator("dd")).toContainText("Arcane, prepared · Intelligence");
  const list = rowOf(page, "Spellcasting").getByRole("link", { name: "Wizard spell list" });
  await expect(list).toHaveAttribute("href", "#/spells/Wizard");
  expect((await list.boundingBox())!.height).toBeGreaterThanOrEqual(44);

  // The card is a stat block: it keeps the left accent rule.
  expect(await card(page).evaluate((el) => getComputedStyle(el).borderLeftWidth)).toBe("4px");

  // Class skills are compact and open in place.
  const skills = rowOf(page, "Class skills").locator("details.class-skills");
  const summary = skills.locator("summary");
  await expect(summary).toHaveText(/^\d+ skills$/);
  expect((await summary.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  await expect(skills.getByText("Spellcraft")).toBeHidden();
  await summary.click();
  await expect(skills.getByText("Spellcraft")).toBeVisible();
  await expect(skills.locator("li").first()).toContainText("Fly");

  // The card replaces the printed basics: none of them shows again in the body.
  const labels = await introLabels(page);
  for (const gone of ["Hit Die:", "Alignment:", "Starting Wealth:", "Skill Points per Level:"]) {
    expect(labels, gone).not.toContain(gone);
  }
  expect(labels.filter((label) => label.startsWith("Class Skills"))).toEqual([]);
  expect(labels).toContain("Role:");
  // The card does not move the page sideways.
  expect(await overflow(page)).toBeLessThanOrEqual(0);
  // At 390 px the whole card is in the first viewport.
  if (width === 390) expect((await card(page).boundingBox())!.y + (await card(page).boundingBox())!.height).toBeLessThan(844);
});

test("Fighter's summary card has no spellcasting row at 1280 px", async ({ page }) => {
  const width = 1280;
  await installLibrary(page);
  const fighter = entryOf("ref/class/fighter");
  await openClass(page, "ref/class/fighter", width);

  expect(await rowTexts(page)).toEqual([
    "Hit Die",
    "Base attack",
    "Good saves",
    "Skill ranks",
    "Class skills",
    "Alignment",
    "Starting wealth",
  ]);
  await expect(rowOf(page, "Hit Die").locator("dd")).toHaveText(fighter.fields.hit_die);
  await expect(rowOf(page, "Base attack").locator("dd")).toContainText("Good");
  await expect(rowOf(page, "Good saves").locator("dd")).toHaveText("Fortitude");
  await expect(rowOf(page, "Starting wealth").locator("dd")).toContainText("5d6");
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});


test("A hybrid, a prestige class and an NPC class show only the rows they have a value for", async ({ page }) => {
  await installLibrary(page);

  // Arcanist: no printed `Spell list:` block, so the tradition, style and key ability come from
  // its Spells feature, and the list link ends the row.
  await openClass(page, "ref/class/arcanist", 390);
  await expect(rowOf(page, "Spellcasting").locator("dd")).toContainText("Arcane, prepared · Intelligence");
  await expect(rowOf(page, "Spellcasting").getByRole("link", { name: "Arcanist spell list" })).toBeVisible();
  await expect(rowOf(page, "Base attack").locator("dd")).toContainText("Poor");

  // Eldritch Knight: Hit Die, attack, saves and skill ranks. Its alignment is a paragraph of
  // prose, so it stays in the body and the card has no Alignment row.
  await openClass(page, "ref/pclass/eldritch-knight", 390);
  expect(await rowTexts(page)).toEqual(["Hit Die", "Base attack", "Good saves", "Skill ranks"]);
  await expect(rowOf(page, "Hit Die").locator("dd")).toHaveText("d10");
  await expect(rowOf(page, "Good saves").locator("dd")).toHaveText("Fortitude");
  expect(await introLabels(page)).toContain("Alignment:");
  expect(await introLabels(page)).not.toContain("Hit Die:");
  // The body's headings were stored as h3; the shallowest of a body renders as h2 (#12, L3).
  await expect(page.getByRole("heading", { level: 2, name: "Class Skills" })).toBeVisible();

  // Adept (NPC): printed in sections, so the card keeps what the table and the typed fields say.
  await openClass(page, "ref/npcclass/adept", 390);
  expect(await rowTexts(page)).toEqual(["Hit Die", "Base attack", "Good saves", "Skill ranks", "Spellcasting"]);
  await expect(rowOf(page, "Good saves").locator("dd")).toHaveText("Will");
  await expect(rowOf(page, "Spellcasting").locator("dd")).toContainText("Divine, prepared · Wisdom");
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});

// The casters without a printed `Spell list:` block read their row from the Spells feature. A part
// the text does not state plainly stays out: the Investigator has no spellcasting section. The
// Bloodrager's save DC names Charisma from corpus v0.1.25 (#435).
async function spellcastingRow(page: Page, id: string, name: string, line: string | null) {
  await installLibrary(page);
  await openClass(page, id, 390);
  const dd = rowOf(page, "Spellcasting").locator("dd");
  await expect(dd.getByRole("link", { name: `${name} spell list` })).toBeVisible();
  if (line) await expect(dd).toContainText(line);
  else await expect(dd).toHaveText(`${name} spell list`);
}

test("Paladin's summary card Spellcasting row reads Divine, prepared · Charisma", async ({ page }) => {
  await spellcastingRow(page, "ref/class/paladin", "Paladin", "Divine, prepared · Charisma");
});

test("Antipaladin's summary card Spellcasting row reads Divine, prepared · Charisma", async ({ page }) => {
  await spellcastingRow(page, "ref/class/antipaladin", "Antipaladin", "Divine, prepared · Charisma");
});

test("Ranger's summary card Spellcasting row reads Divine, prepared · Wisdom", async ({ page }) => {
  await spellcastingRow(page, "ref/class/ranger", "Ranger", "Divine, prepared · Wisdom");
});

test("Bloodrager's summary card Spellcasting row reads Arcane, spontaneous · Charisma", async ({ page }) => {
  await spellcastingRow(page, "ref/class/bloodrager", "Bloodrager", "Arcane, spontaneous · Charisma");
});

test("Alchemist's summary card Spellcasting row reads Prepared · Intelligence", async ({ page }) => {
  await spellcastingRow(page, "ref/class/alchemist", "Alchemist", "Prepared · Intelligence");
});

test("Investigator's summary card Spellcasting row reads the list link only", async ({ page }) => {
  await spellcastingRow(page, "ref/class/investigator", "Investigator", null);
});


async function foldCount(fold: Locator) {
  const text = (await fold.locator("> summary .rel-count").textContent())!;
  return Number(text.replace(/,/g, ""));
}

test("References and Referenced by fold with their counts on a class page at 390 px", async ({ page }) => {
  const width = 390;
  await installLibrary(page);
  await openClass(page, "ref/class/wizard", width);

  const refs = page.locator(".entry-refs");
  const references = refs.locator("details.ref-fold").filter({ hasText: /^References/ });
  const referencedBy = refs.locator("details.ref-fold#entry-backlinks");
  await expect(references).toHaveCount(1);
  await expect(referencedBy).toHaveCount(1);
  await references.scrollIntoViewIfNeeded();

  // The panels end the page on every width: below the body, the related panels and the
  // sources line, above the footer, and inside the text column (no side panel).
  const bottom = async (selector: string) => {
    const box = (await page.locator(selector).first().boundingBox())!;
    return box.y + box.height;
  };
  const top = async (locator: Locator) => (await locator.boundingBox())!.y;
  expect(await top(references)).toBeGreaterThanOrEqual(await bottom(".entry > .rich"));
  expect(await top(references)).toBeGreaterThanOrEqual(await bottom("#entry-related"));
  expect(await top(referencedBy)).toBeGreaterThan(await top(references));
  expect(await bottom(".entry-refs")).toBeLessThanOrEqual(await top(page.locator(".entry-foot")));
  const refsBox = (await page.locator(".entry-refs").boundingBox())!;
  const bodyBox = (await page.locator(".entry > .rich").boundingBox())!;
  expect(refsBox.x).toBe(bodyBox.x);

  // Both are closed, and each summary is a 44 px target that names its count.
  for (const fold of [references, referencedBy]) {
    expect(await fold.evaluate((el) => (el as HTMLDetailsElement).open)).toBe(false);
    expect((await fold.locator("> summary").boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }
  await expect(references.locator("> summary")).toHaveText(/^References\s*[\d,]+$/);
  await expect(referencedBy.locator("> summary")).toHaveText(/^Referenced by\s*[\d,]+$/);
  await expect(references.locator("ul.ref-list")).toBeHidden();
  await expect(referencedBy.locator("details.backlink-group").first()).toBeHidden();

  // Opening shows the rows, and the count is the row count.
  await references.locator("> summary").click();
  const total = await foldCount(references);
  expect(total).toBeGreaterThan(0);
  await expect(references.locator("ul.ref-list > li")).toHaveCount(total);
  await referencedBy.locator("> summary").click();
  const groups = await referencedBy.locator("details.backlink-group > summary .rel-count").allTextContents();
  expect(groups.reduce((sum, n) => sum + Number(n.replace(/,/g, "")), 0)).toBe(await foldCount(referencedBy));
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});

test("References and Referenced by fold with their counts on a class page at 1280 px", async ({ page }) => {
  const width = 1280;
  await installLibrary(page);
  await openClass(page, "ref/class/wizard", width);

  const refs = page.locator(".entry-refs");
  const references = refs.locator("details.ref-fold").filter({ hasText: /^References/ });
  const referencedBy = refs.locator("details.ref-fold#entry-backlinks");
  await expect(references).toHaveCount(1);
  await expect(referencedBy).toHaveCount(1);
  await references.scrollIntoViewIfNeeded();

  // The panels end the page on every width: below the body, the related panels and the
  // sources line, above the footer, and inside the text column (no side panel).
  const bottom = async (selector: string) => {
    const box = (await page.locator(selector).first().boundingBox())!;
    return box.y + box.height;
  };
  const top = async (locator: Locator) => (await locator.boundingBox())!.y;
  expect(await top(references)).toBeGreaterThanOrEqual(await bottom(".entry > .rich"));
  expect(await top(references)).toBeGreaterThanOrEqual(await bottom("#entry-related"));
  expect(await top(referencedBy)).toBeGreaterThan(await top(references));
  expect(await bottom(".entry-refs")).toBeLessThanOrEqual(await top(page.locator(".entry-foot")));
  const refsBox = (await page.locator(".entry-refs").boundingBox())!;
  const bodyBox = (await page.locator(".entry > .rich").boundingBox())!;
  expect(refsBox.x).toBe(bodyBox.x);

  // Both are closed, and each summary is a 44 px target that names its count.
  for (const fold of [references, referencedBy]) {
    expect(await fold.evaluate((el) => (el as HTMLDetailsElement).open)).toBe(false);
    expect((await fold.locator("> summary").boundingBox())!.height).toBeGreaterThanOrEqual(44);
  }
  await expect(references.locator("> summary")).toHaveText(/^References\s*[\d,]+$/);
  await expect(referencedBy.locator("> summary")).toHaveText(/^Referenced by\s*[\d,]+$/);
  await expect(references.locator("ul.ref-list")).toBeHidden();
  await expect(referencedBy.locator("details.backlink-group").first()).toBeHidden();

  // Opening shows the rows, and the count is the row count.
  await references.locator("> summary").click();
  const total = await foldCount(references);
  expect(total).toBeGreaterThan(0);
  await expect(references.locator("ul.ref-list > li")).toHaveCount(total);
  await referencedBy.locator("> summary").click();
  const groups = await referencedBy.locator("details.backlink-group > summary .rel-count").allTextContents();
  expect(groups.reduce((sum, n) => sum + Number(n.replace(/,/g, "")), 0)).toBe(await foldCount(referencedBy));
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});

test("Other entries show References and Referenced by open in the side panel from 64 rem", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await installLibrary(page);
  const spell = [...corpus.entries.values()].find((entry) => entry.kind === "spell" && entry.title === "Fireball")!;
  await page.goto(`#/entry/${spell.id}`);
  await expect(page.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
  const folds = page.locator(".entry-refs details.ref-fold");
  await expect(folds.filter({ hasText: /^Referenced by/ })).toHaveCount(1);
  for (const fold of await folds.all()) await expect(fold).toHaveAttribute("open", "");
});
