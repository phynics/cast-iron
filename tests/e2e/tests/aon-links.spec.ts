import { installedTest as test, expect, loadCorpus, type Page } from "./fixtures";

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

/**
 * Nothing but the link's own anchor may name the reference site. A `link` hint (preconnect,
 * dns-prefetch, prefetch), an `img`, a `script`, or an `iframe` would either make a request or
 * tell the browser to open a connection before the tap; neither is visible in the request log
 * for every kind of hint, so this checks the markup directly.
 */
async function noAonResources(page: Page) {
  const named = await page.evaluate(() =>
    [...document.querySelectorAll("link, img, script, iframe")].flatMap((element) =>
      [...element.attributes]
        .filter((attribute) => /aonprd\.com/i.test(attribute.value))
        .map(
          (attribute) => `${element.tagName.toLowerCase()}[${attribute.name}]=${attribute.value}`,
        ),
    ),
  );
  expect(named, "a preconnect hint, image, script or frame names aonprd.com").toEqual([]);
}

/**
 * The compact line keeps a text box shorter than the 44 px target. Scan outward from that box
 * with `elementFromPoint`, as a tap would, to measure the box the anchor really reaches; the
 * extra height comes from `.meta-link::after`, so the layout does not move.
 */
async function hitHeight(page: Page, selector: string) {
  return page.evaluate((sel) => {
    const link = document.querySelector(sel) as HTMLElement;
    link.scrollIntoView({ block: "center" });
    const rect = link.getBoundingClientRect();
    const x = rect.x + rect.width / 2;
    const hits = (y: number) => {
      const element = document.elementFromPoint(x, y);
      return element !== null && (element === link || link.contains(element));
    };
    let top = Math.round(rect.top);
    let bottom = Math.round(rect.bottom);
    while (top > 0 && hits(top - 1)) top -= 1;
    while (bottom < window.innerHeight - 1 && hits(bottom + 1)) bottom += 1;
    return { box: Math.round(rect.height), hit: bottom - top + 1 };
  }, selector);
}

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
    await noAonResources(page);

    const link = page.getByTestId("aon-link");
    await expect(link).toHaveCount(1);
    await expect(link).toContainText("View on Archives of Nethys");
    // The stored path is already percent-encoded: the URL is the fixed base plus that path.
    await expect(link).toHaveAttribute("href", `https://aonprd.com/${entry.aon}`);
    await expect(link).toHaveAttribute("target", "_blank");
    await expect(link).toHaveAttribute("rel", "noopener noreferrer");
    await expect(link).toHaveAccessibleName(/View on Archives of Nethys \(opens in a new tab\)/);
  }

  // The link closes the entry's own text: directly after the typed Sources line, before the
  // related panels.
  await page.goto(`#/entry/${POWER_ATTACK.id}`);
  await expect(page.locator(".entry > .sources-line + .aon-line + #entry-related")).toHaveCount(1);

  // The line reads as secondary: the same weight as the citation line it follows.
  const weights = await page.evaluate(() => ({
    sources: getComputedStyle(document.querySelector(".entry > .sources-line")!).fontWeight,
    aon: getComputedStyle(document.querySelector(".entry > .aon-line a")!).fontWeight,
  }));
  expect(weights.aon, "the trail link is no heavier than the citation line").toBe(weights.sources);

  // The line stays compact and still reaches the 44 px target at a phone width.
  await page.setViewportSize({ width: 390, height: 844 });
  const target = await hitHeight(page, ".entry > .aon-line a");
  expect(target.box, "the visible text box stays compact").toBeLessThan(44);
  expect(target.hit, "the hit area reaches the 44 px target").toBeGreaterThanOrEqual(44);

  // Browsing the pages made no request to the reference site; the link waits for the tap.
  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

test("A creature links and an entry without a stored path shows no link", async ({ page, guard }) => {
  await page.goto(`#/entry/${GOBLIN.id}`);
  await expect(page.getByRole("heading", { level: 1, name: GOBLIN.title })).toBeVisible();
  await noAonResources(page);
  await expect(page.getByTestId("aon-link")).toHaveAttribute(
    "href",
    `https://aonprd.com/${GOBLIN.aon}`,
  );

  await page.goto(`#/entry/${CASTING_CONDUIT.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CASTING_CONDUIT.title })).toBeVisible();
  await noAonResources(page);
  await expect(page.getByTestId("aon-link")).toHaveCount(0);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});

test("A book with a source page links and one without shows no link", async ({ page, guard }) => {
  await page.goto(`#/browse/book/${CORE_RULEBOOK.id}`);
  await expect(page.getByRole("heading", { level: 1, name: CORE_RULEBOOK.title })).toBeVisible();
  await noAonResources(page);

  const link = page.getByTestId("aon-link");
  await expect(link).toHaveCount(1);
  await expect(link).toContainText("View on Archives of Nethys");
  await expect(link).toHaveAttribute("href", `https://aonprd.com/${CORE_RULEBOOK.aon}`);
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", "noopener noreferrer");
  await expect(link).toHaveAccessibleName(/View on Archives of Nethys \(opens in a new tab\)/);

  await page.goto(`#/browse/book/${ARTIFACTS_AND_LEGENDS.id}`);
  await expect(
    page.getByRole("heading", { level: 1, name: ARTIFACTS_AND_LEGENDS.title }),
  ).toBeVisible();
  await noAonResources(page);
  await expect(page.getByTestId("aon-link")).toHaveCount(0);

  expect(guard.problems(), "console errors or third-party requests").toEqual([]);
});
