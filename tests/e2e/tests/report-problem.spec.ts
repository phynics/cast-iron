import { installedTest as test, expect, distManifest, distShellVersion, loadCorpus } from "./fixtures";
import { installLibrary, waitControlled } from "./helpers";

// One footer control, routed by the page: an entry opens the content issue form; every other
// page opens the app form. The tests read the href and never navigate to GitHub.
const corpus = loadCorpus();
const entry = corpus.catalog.entries.find((row) => row.kind === "spell")!;

test("An entry page opens the content form with its ref id", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/entry/${entry.id}`);

  const footer = page.locator("footer.app-footer");
  const link = footer.getByRole("link", {
    name: `Report a problem with ${entry.title} on GitHub (opens in a new tab)`,
  });
  await expect(link).toHaveCount(1);
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", "noopener noreferrer");

  const url = new URL((await link.getAttribute("href"))!);
  expect(url.origin).toBe("https://github.com");
  expect(url.pathname).toBe("/phynics/cast-iron-data/issues/new");
  expect(url.searchParams.get("template")).toBe("data-problem.yml");
  expect(url.searchParams.get("entry")).toBe(entry.id);
});

test("Other pages open the app form with the route and versions", async ({ page }) => {
  await installLibrary(page);
  await waitControlled(page);
  const manifest = distManifest();
  const shell = distShellVersion();

  for (const route of ["#/settings", "#/"]) {
    await page.goto(route);
    const footer = page.locator("footer.app-footer");
    const link = footer.getByRole("link", {
      name: "Report a problem on GitHub (opens in a new tab)",
    });
    await expect(link).toHaveCount(1);
    await expect(link).toBeVisible();
    await expect(link).toHaveAttribute("target", "_blank");
    await expect(link).toHaveAttribute("rel", "noopener noreferrer");
    // The shell version arrives from the controlling service worker.
    await expect
      .poll(async () => new URL((await link.getAttribute("href"))!).searchParams.get("app-version"))
      .toBe(shell);

    const url = new URL((await link.getAttribute("href"))!);
    expect(url.pathname).toBe("/phynics/cast-iron/issues/new");
    expect(url.searchParams.get("template")).toBe("app-problem.yml");
    expect(url.searchParams.get("route")).toBe(route);
    expect(url.searchParams.get("content-version")).toBe(manifest.content_version);
    expect(url.searchParams.get("schema-version")).toBe(String(manifest.schema_version));
  }
});

test("The report control is at least 44 px at 390 px", async ({ page }) => {
  await installLibrary(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("#/settings");
  const link = page.locator("footer.app-footer .report-problem");
  await expect(link).toBeVisible();
  const box = (await link.boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(44);
  expect(box.height).toBeGreaterThanOrEqual(44);
});
