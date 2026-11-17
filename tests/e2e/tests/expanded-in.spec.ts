import fs from "node:fs";
import { test, expect, loadCorpus, type Page } from "./fixtures";
import { buildLibraryWithExpandedIn, installLibrary, memberBooks, waitControlled, waitHomeIdle, waitReady, writeMeta } from "./helpers";

// No pinned corpus has `expanded_in` yet (#459 adds it), so the spec serves the compiled library
// with credits added to two entries and checks the entry page against it. The pinned corpus still
// lists the handbook as a member of Summon Monster III (#459 moves it to a credit), so the spec
// tests the line and leaves membership to the native tests of #458.

const corpus = loadCorpus();
const sources: any[] = corpus.catalog.sources;
const stubs: any[] = corpus.catalog.entries;
const titleOf = (id: string) => sources.find((row) => row.id === id)!.title as string;
const members = (stub: any): string[] => memberBooks(stub);

const HANDBOOK = sources.find((row) => row.title === "Monster Summoner's Handbook")?.id as string;
const SUMMON_III = stubs.find((stub) => stub.title === "Summon Monster III" && stub.kind === "spell");
const DODGE = "ref/feat/dodge";
const DODGE_REVISION = "ref/feat/dodge/teitr";

/**
 * The `via` entry, chosen so that the credit is valid under #458's check: the via entry is a
 * member of the expanding book. It also belongs to a second book that Summon Monster III does not
 * belong to, so the handbook can be off while the entry stays visible, and both books off hide it.
 * (A valid `via` can only be hidden while its expanding book is off.)
 */
const VIA = stubs.find(
  (stub) =>
    stub.id !== SUMMON_III.id &&
    members(stub).includes(HANDBOOK) &&
    members(stub).some((book) => book !== HANDBOOK && !members(SUMMON_III).includes(book)) &&
    stubs.filter((other) => other.title === stub.title).length === 1,
);

/** Where the compiled shot of the line goes, when the reviewer asks for one. */
const SHOTS = process.env.EXPANDED_IN_SHOTS;

async function disableBooks(page: Page, ids: string[]) {
  await page.goto("#/");
  await waitHomeIdle(page);
  await writeMeta(page, "disabled_books", ids);
  await page.reload();
  await waitReady(page);
  await waitHomeIdle(page);
}

test("Expanded in credits a book under the Sources line, with via and inactive states", async ({ page, server }) => {
  // The overlay and the first install of the library take longer than the default budget.
  test.setTimeout(300_000);
  expect(HANDBOOK, "the compiled library has Monster Summoner's Handbook").toBeTruthy();
  expect(SUMMON_III, "the compiled library has Summon Monster III").toBeTruthy();
  expect(VIA, "an entry of the handbook and a second book").toBeTruthy();
  const handbook = titleOf(HANDBOOK);
  const viaBook = members(VIA).find((book) => book !== HANDBOOK)!;

  const variant = buildLibraryWithExpandedIn({
    [SUMMON_III.id]: [{ publication: HANDBOOK, page: "18", via: VIA.id }],
    [DODGE]: [{ publication: HANDBOOK, page: "9" }],
  });
  try {
    await server.overlay(variant.dir);
    await installLibrary(page);
    await waitControlled(page);

    const line = page.getByTestId("expanded-line");
    const open = async (id: string) => {
      await page.goto(`#/entry/${id}`);
      await expect(page.getByTestId("entry-title")).toBeVisible();
    };

    await test.step("Summon Monster III shows the credit under its Sources line", async () => {
      await open(SUMMON_III.id);
      await expect(line).toHaveCount(1);
      await expect(line).toContainText(`Expanded in: ${handbook} p. 18`);
      await expect(line).not.toContainText("Inactive");
      // Directly after the entry's own citation line, before anything else.
      await expect(page.locator(".entry > .sources-line + .expanded-line")).toHaveCount(1);
      await expect(line.getByRole("link", { name: handbook, exact: true })).toHaveAttribute(
        "href",
        `#/browse/book/${HANDBOOK}`,
      );
      await expect(page.locator(".expanded-credit")).not.toHaveAttribute("data-inactive", "true");
    });

    await test.step("The book link opens the book page", async () => {
      await line.getByRole("link", { name: handbook, exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`#/browse/book/${HANDBOOK}`));
      await expect(page.getByRole("heading", { level: 1, name: handbook })).toBeVisible();
    });

    await test.step("The page number links the via entry while it is visible", async () => {
      await open(SUMMON_III.id);
      await expect(line.getByTestId("expanded-via")).toHaveText("18");
      await expect(line.getByTestId("expanded-via")).toHaveAttribute("href", `#/entry/${VIA.id}`);
    });

    await test.step("A credit from an inactive book is dimmed and labelled, and its via entry stays linked while visible", async () => {
      await disableBooks(page, [HANDBOOK]);
      await open(SUMMON_III.id);
      const credit = page.locator(".expanded-credit");
      await expect(credit).toHaveAttribute("data-inactive", "true");
      await expect(credit.locator(".tag")).toHaveText("Inactive");
      await expect(line).toContainText(`Expanded in: ${handbook} p. 18 · Inactive`);
      // The via entry is still visible through its second book.
      await expect(page.getByTestId("expanded-via")).toHaveAttribute("href", `#/entry/${VIA.id}`);
      // The book page still opens for an inactive book.
      await expect(line.getByRole("link", { name: handbook, exact: true })).toHaveAttribute(
        "href",
        `#/browse/book/${HANDBOOK}`,
      );
    });

    await test.step("The dimmed credit keeps 4.5:1 contrast in both themes, links included", async () => {
      for (const scheme of ["light", "dark"] as const) {
        await page.emulateMedia({ colorScheme: scheme });
        const ratios = await page.evaluate(() => {
          const channel = (value: number) => {
            const c = value / 255;
            return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
          };
          const luminance = (rgb: number[]) => 0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2]);
          const rgb = (css: string) => (css.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
          const ground = rgb(getComputedStyle(document.body).backgroundColor);
          const ratio = (element: Element) => {
            const style = getComputedStyle(element);
            const opacity = Number(style.opacity);
            const ink = rgb(style.color).map((value, at) => value * opacity + ground[at] * (1 - opacity));
            const [hi, lo] = [luminance(ink), luminance(ground)].sort((a, b) => b - a);
            return (hi + 0.05) / (lo + 0.05);
          };
          const credit = document.querySelector(".expanded-credit")!;
          return [credit, credit.querySelector("a")!, credit.querySelector(".tag")!].map(ratio);
        });
        for (const ratio of ratios) expect(ratio, `${scheme}: ${ratios.join(", ")}`).toBeGreaterThanOrEqual(4.5);
      }
      await page.emulateMedia({ colorScheme: null });
    });

    await test.step("With both books off the via entry is hidden and its number is plain", async () => {
      await disableBooks(page, [HANDBOOK, viaBook]);
      await open(SUMMON_III.id);
      await expect(page.locator(".expanded-credit")).toHaveAttribute("data-inactive", "true");
      await expect(page.getByTestId("expanded-via")).toHaveCount(0);
      await expect(line.locator(`a[href="#/entry/${VIA.id}"]`)).toHaveCount(0);
      await expect(line).toContainText(`Expanded in: ${handbook} p. 18 · Inactive`);
    });

    await test.step("A revision in force keeps the base's credit", async () => {
      await disableBooks(page, []);
      await open(DODGE);
      await expect(page.locator(".revision-notice")).toContainText("Revised by");
      await expect(page.locator(".entry > .rich")).toBeVisible();
      await expect(line).toContainText(`Expanded in: ${handbook} p. 9`);
      // The revision's own page has no credit of its own.
      await open(DODGE_REVISION);
      await expect(page.getByTestId("entry-title")).toBeVisible();
      await expect(line).toHaveCount(0);
    });

    await test.step("An entry without a credit has no line", async () => {
      await open("ref/feat/power-attack");
      await expect(page.getByTestId("entry-title")).toBeVisible();
      await expect(line).toHaveCount(0);
    });

    await test.step("The line stays inside the page at this width", async () => {
      await open(SUMMON_III.id);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
      const box = (await line.boundingBox())!;
      const viewport = page.viewportSize()!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
    });

    if (SHOTS) {
      fs.mkdirSync(SHOTS, { recursive: true });
      for (const [width, height] of [
        [390, 844],
        [1280, 900],
      ]) {
        for (const scheme of ["light", "dark"] as const) {
          for (const state of ["active", "inactive"]) {
            await page.setViewportSize({ width, height });
            await page.emulateMedia({ colorScheme: scheme });
            await disableBooks(page, state === "inactive" ? [HANDBOOK] : []);
            await open(SUMMON_III.id);
            await expect(line).toBeVisible();
            await line.scrollIntoViewIfNeeded();
            await page.screenshot({ path: `${SHOTS}/expanded-in-${width}-${scheme}-${state}.png` });
          }
        }
      }
    }
  } finally {
    variant.cleanup();
  }
});
