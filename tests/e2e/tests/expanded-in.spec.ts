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
 * An entry that stays visible apart from the entry under test: a spell with a unique title and
 * one member book that is neither the handbook nor a book of Summon Monster III. Turning that
 * book off hides it, which is the `via` entry's hidden state.
 */
const VIA = stubs.find(
  (stub) =>
    stub.kind === "spell" &&
    members(stub).length === 1 &&
    members(stub)[0] !== HANDBOOK &&
    !members(SUMMON_III).includes(members(stub)[0]) &&
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
  expect(VIA, "a spell that a second book can hide").toBeTruthy();
  const handbook = titleOf(HANDBOOK);
  const viaBook = members(VIA)[0];

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

    await test.step("A hidden via entry leaves the page number plain", async () => {
      await disableBooks(page, [viaBook]);
      await open(SUMMON_III.id);
      await expect(line).toContainText(`Expanded in: ${handbook} p. 18`);
      await expect(page.getByTestId("expanded-via")).toHaveCount(0);
      await expect(line.locator(`a[href="#/entry/${VIA.id}"]`)).toHaveCount(0);
      await expect(page.locator(".expanded-credit")).not.toHaveAttribute("data-inactive", "true");
    });

    await test.step("A credit from an inactive book is dimmed, labelled, and never links a hidden entry", async () => {
      await disableBooks(page, [viaBook, HANDBOOK]);
      await open(SUMMON_III.id);
      const credit = page.locator(".expanded-credit");
      await expect(credit).toHaveAttribute("data-inactive", "true");
      await expect(credit.locator(".tag")).toHaveText("Inactive");
      await expect(line).toContainText(`Expanded in: ${handbook} p. 18 · Inactive`);
      expect(await credit.evaluate((node) => getComputedStyle(node).opacity)).toBe("0.85");
      await expect(page.getByTestId("expanded-via")).toHaveCount(0);
      // The book page still opens for an inactive book.
      await expect(line.getByRole("link", { name: handbook, exact: true })).toHaveAttribute(
        "href",
        `#/browse/book/${HANDBOOK}`,
      );
      // The via entry comes back as soon as its book is on, even with the handbook off.
      await disableBooks(page, [HANDBOOK]);
      await open(SUMMON_III.id);
      await expect(page.getByTestId("expanded-via")).toHaveAttribute("href", `#/entry/${VIA.id}`);
      await expect(page.locator(".expanded-credit")).toHaveAttribute("data-inactive", "true");
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
