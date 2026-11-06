import { test, expect } from "./fixtures";
import { installLibrary } from "./helpers";

/**
 * The Books tab must open without a visible stall. The page builds one row per publication, and a
 * count for each row once rescanned the whole catalog per publication (about 13 million lookups).
 * The budget is generous on purpose: it catches that class of regression, not a slow runner.
 */
const BUDGET_MS = 1500;

test.beforeEach(async ({ page }) => {
  await installLibrary(page);
});

test("the Books tab opens within a generous budget", async ({ page }, info) => {
  const samples: number[] = [];
  for (let round = 0; round < 3; round++) {
    // Leave Books for Settings so each round mounts the page again.
    await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Settings" }).click();
    await expect(page.locator("[data-testid=library-facts]")).toBeVisible();
    // Time from the click to the first book row on screen after a paint.
    const elapsed = await page.evaluate(
      () =>
        new Promise<number>((resolve) => {
          const link = [...document.querySelectorAll<HTMLAnchorElement>("nav[aria-label=Primary] a")].find(
            (a) => a.textContent?.trim() === "Books",
          )!;
          const started = performance.now();
          const done = () => requestAnimationFrame(() => requestAnimationFrame(() => resolve(performance.now() - started)));
          const watcher = new MutationObserver(() => {
            if (document.querySelector(".book-directory-item")) {
              watcher.disconnect();
              done();
            }
          });
          watcher.observe(document.body, { childList: true, subtree: true });
          link.click();
        }),
    );
    samples.push(Math.round(elapsed));
  }
  console.log(`books-open ${info.project.name}: ${samples.join(" ms, ")} ms`);
  info.annotations.push({ type: "books-open-ms", description: samples.join(",") });
  expect(Math.min(...samples)).toBeLessThan(BUDGET_MS);
});
