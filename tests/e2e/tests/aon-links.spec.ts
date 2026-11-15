import { installedTest as test, expect, loadCorpus } from "./fixtures";

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;

/** A compiled record, by id, with the checks the pin must satisfy before the page can link. */
function storedEntry(id: string, kind: string) {
  const entry = corpus.entries.get(id);
  if (!entry) throw new Error(`The compiled corpus has no ${id}.`);
  if (entry.kind !== kind) throw new Error(`${id} is a ${entry.kind}, not a ${kind}.`);
  return entry;
}

const POWER_ATTACK = storedEntry("ref/feat/power-attack", "feat");
const FIREBALL = storedEntry("ref/spell/fireball", "spell");
const GOBLIN = storedEntry("ref/monster/goblin", "creature");
const CASTING_CONDUIT = storedEntry("ref/feat/casting-conduit", "feat");
const CORE_RULEBOOK = sources.find((row) => row.id === "core-rulebook");
const ARTIFACTS_AND_LEGENDS = sources.find((row) => row.id === "artifacts-and-legends");

test("The pin carries the harvested paths these tests need", () => {
  for (const entry of [POWER_ATTACK, FIREBALL, GOBLIN]) {
    expect(entry.aon, `${entry.id} has an AoN path`).toBeTruthy();
  }
  expect(CASTING_CONDUIT.aon, "Casting Conduit has no AoN path").toBeFalsy();
  expect(CORE_RULEBOOK?.aon, "Core Rulebook has a source page").toBeTruthy();
  expect(ARTIFACTS_AND_LEGENDS?.aon, "Artifacts and Legends has no source page").toBeFalsy();
});

test("A feat and a spell link to their stored Archives of Nethys page", async ({ page, guard }) => {
  for (const entry of [POWER_ATTACK, FIREBALL]) {
    await page.goto(`#/entry/${entry.id}`);
    await expect(page.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();

    const link = page.getByTestId("aon-link");
    await expect(link).toHaveCount(1);
    await expect(link).toContainText("View on Archives of Nethys");
    // The stored path is already percent-encoded: the URL is the fixed base plus that path.
    await expect(link).toHaveAttribute("href", `https://aonprd.com/${entry.aon}`);
    await expect(link).toHaveAttribute("target", "_blank");
    await expect(link).toHaveAttribute("rel", "noopener noreferrer");
    await expect(link).toHaveAccessibleName(/View on Archives of Nethys \(opens in a new tab\)/);
  }
  // Browsing the pages made no request to the reference site; the link waits for the tap.
  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

test("A creature links and an entry without a stored path shows no link", async ({ page }) => {
  await page.goto(`#/entry/${GOBLIN.id}`);
  await expect(page.getByRole("heading", { level: 1, name: GOBLIN.title })).toBeVisible();
  await expect(page.getByTestId("aon-link")).toHaveAttribute(
    "href",
    `https://aonprd.com/${GOBLIN.aon}`,
  );

  await page.goto(`#/entry/${CASTING_CONDUIT.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CASTING_CONDUIT.title })).toBeVisible();
  await expect(page.getByTestId("aon-link")).toHaveCount(0);
});

test("A book with a source page links and one without shows no link", async ({ page }) => {
  await page.goto(`#/browse/book/${CORE_RULEBOOK.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CORE_RULEBOOK.title })).toBeVisible();
  const link = page.getByTestId("aon-link");
  await expect(link).toHaveCount(1);
  await expect(link).toContainText("View on Archives of Nethys");
  await expect(link).toHaveAttribute("href", `https://aonprd.com/${CORE_RULEBOOK.aon}`);
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", "noopener noreferrer");

  await page.goto(`#/browse/book/${ARTIFACTS_AND_LEGENDS.id}`);
  await expect(
    page.getByRole("heading", { level: 1, name: ARTIFACTS_AND_LEGENDS.title }),
  ).toBeVisible();
  await expect(page.getByTestId("aon-link")).toHaveCount(0);
});
