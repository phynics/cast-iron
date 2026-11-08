import { installedTest as test, expect, loadCorpus, type Page } from "./fixtures";
import {
  bookTrees,
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
const ULTIMATE_MAGIC = "ultimate-magic";
const CURE = "ref/spell/cure-light-wounds";
const MASS_CURE = "ref/spell/cure-light-wounds-mass";
const MASS_MODERATE = "ref/spell/cure-moderate-wounds-mass";
const RESTORATION = "ref/spell/restoration";
const RESTORE_EIDOLON = "ref/spell/restore-eidolon";

const corpus = loadCorpus();
const allBooks = bookTrees().map((book) => book.source);

function stubOf(id: string) {
  const stub = corpus.catalog.entries.find((entry) => entry.id === id);
  if (!stub) throw new Error(`The compiled catalog has no entry ${id}.`);
  return stub;
}

function bookTitleOf(id: string) {
  const source = corpus.catalog.sources.find((candidate) => candidate.id === stubOf(id).source);
  if (!source) throw new Error(`The compiled catalog has no source ${stubOf(id).source}.`);
  return source.title;
}

/** The muted line a variants row shows, built the way the page builds it from `lists.json`. */
function levelsLine(id: string) {
  const entry = corpus.entries.get(id);
  if (!entry) throw new Error(`The compiled corpus has no entry ${id}.`);
  const rows = [...(entry.fields.levels as { class: string; level: number }[])].sort(
    (a, b) => {
      const left = a.class.toLowerCase();
      const right = b.class.toLowerCase();
      if (left !== right) return left < right ? -1 : 1;
      return a.level - b.level;
    },
  );
  const levels = rows
    .map(({ class: name, level }) => (level === 0 ? `${name} Cantrips` : `${name} ${level}`))
    .join(", ");
  return `${levels} · ${bookTitleOf(id)}`;
}

/** Writes the active set directly, then reloads. Turning off 400 books one row at a time is
 * not what this spec is about. */
async function onlyBooks(page: Page, active: string[]) {
  await writeMeta(
    page,
    "disabled_books",
    allBooks.filter((id) => !active.includes(id)),
  );
  await page.goto("./");
  await waitReady(page);
  await waitHomeIdle(page);
}

test("Cure Light Wounds lists its direct variant with levels and book", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);

  await page.goto(`#/entry/${CURE}`);
  const section = page.locator(".rel-variants");
  await expect(section).toHaveCount(1);
  await expect(section.getByRole("heading", { name: "Variants", level: 2 })).toBeVisible();

  const row = section
    .locator(".rel-list li")
    .filter({ has: page.locator(`a[href="#/entry/${MASS_CURE}"]`) });
  await expect(row).toHaveCount(1);
  await expect(row.locator(".rel-meta")).toHaveText(levelsLine(MASS_CURE));

  // The title keeps a full touch target, and the row fits 390 px without a sideways scroll.
  const link = row.getByRole("link", { name: "Mass Cure Light Wounds", exact: true });
  expect((await link.boundingBox())!.height).toBeGreaterThanOrEqual(44);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

test("a variant names only its direct base, and its own text still links it", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);

  await page.goto(`#/entry/${MASS_MODERATE}`);
  const meta = page.locator(".entry-meta");
  await expect(
    meta.getByRole("link", { name: "Variant of Mass Cure Light Wounds", exact: true }),
  ).toBeVisible();
  // A chain is walked one page at a time: the direct base only, never the chain root.
  await expect(
    meta.getByRole("link", { name: "Variant of Cure Light Wounds", exact: true }),
  ).toHaveCount(0);
  await expect(meta.locator("span.tag")).toHaveCount(0);

  // The body keeps the words that name the base, linked.
  await expect(
    page.locator(`.rich a[href="#/entry/${MASS_CURE}"]`).first(),
  ).toHaveText("mass cure light wounds");

  // Search and Quick search keep the normal spell row: no variant label was asked for.
  await page.goto(`#/search?q=${encodeURIComponent("mass cure moderate wounds")}`);
  await waitResults(page, "mass cure moderate wounds");
  const hit = page
    .locator(".results .hit")
    .filter({ has: page.locator(".hit-title", { hasText: "Mass Cure Moderate Wounds" }) });
  await expect(hit.locator(".hit-meta")).toHaveText("Spell · Core Rulebook");

  await page.goto("#/");
  await openQuickSearch(page);
  await quickField(page).fill("mass cure moderate wounds");
  const suggestion = quickOptions(page).filter({
    has: page.locator(".quick-search-name", { hasText: "Mass Cure Moderate Wounds" }),
  });
  await expect(suggestion).toHaveCount(1);
  await expect(suggestion.locator(".quick-search-meta")).toHaveText("Spell · Core Rulebook");
});

test("a variant whose book is inactive leaves the base's Variants row empty", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await onlyBooks(page, [CORE]);

  await page.goto(`#/entry/${RESTORATION}`);
  await expect(page.getByRole("heading", { name: "Restoration", level: 1 })).toBeVisible();
  await expect(page.locator(".rel-variants")).toHaveCount(0);
  await expect(page.locator(`a[href="#/entry/${RESTORE_EIDOLON}"]`)).toHaveCount(0);

  // With the variant's own book on, the row appears and reads that book and its levels.
  await onlyBooks(page, [CORE, ULTIMATE_MAGIC]);
  await page.goto(`#/entry/${RESTORATION}`);
  const row = page
    .locator(".rel-variants .rel-list li")
    .filter({ has: page.locator(`a[href="#/entry/${RESTORE_EIDOLON}"]`) });
  await expect(row).toHaveCount(1);
  await expect(row.locator(".rel-meta")).toHaveText(levelsLine(RESTORE_EIDOLON));

  // The base's book never hides the variant's own page; it names the inactive base instead.
  await onlyBooks(page, [ULTIMATE_MAGIC]);
  await page.goto(`#/entry/${RESTORE_EIDOLON}`);
  const meta = page.locator(".entry-meta");
  await expect(
    meta.getByRole("link", { name: "Variant of Restoration", exact: true }),
  ).toBeVisible();
  await expect(meta.locator("span.tag")).toHaveText("Inactive");
});
