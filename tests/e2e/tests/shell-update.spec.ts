import { test, expect, goOffline } from "./fixtures";
import { activeLibrary, cacheContents, contentCaches, corpusEntry, installLibrary, waitControlled, waitReady } from "./helpers";

const spell = corpusEntry("spell", "Fireball");
const UPDATE_NAME = "App update ready. Reload to apply";

test("App update waits for reload and keeps the installed library and Home bookmarks", async ({ page, context, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto(`#/entry/${spell.id}`);
  await page.locator(".bookmark-toggle").click();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  await page.goto("#/settings");
  const shell = page.getByTestId("shell-version");
  await expect(shell).not.toHaveText(/Checking|Not available/);
  const previousShell = (await shell.innerText()).trim();
  const library = await activeLibrary(page);
  const cachesBefore = contentCaches(await cacheContents(page));
  const nextShell = `e2e-next-${previousShell}`;

  // On a reading screen: the indicator must not push the article down.
  await page.goto(`#/entry/${spell.id}`);
  await expect(page.getByRole("heading", { name: spell.title, exact: true, level: 1 })).toBeVisible();
  const mainTop = (await page.locator("main").boundingBox())!.y;

  // Start the update through the app's own control, as a user would.
  await server.shellVersion(nextShell);
  await page.goto("#/settings");
  await page.getByRole("button", { name: "Check for updates", exact: true }).click();
  const update = page.locator("header.app-header").getByRole("button", { name: UPDATE_NAME });
  await expect(update).toBeVisible();
  await page.goto(`#/entry/${spell.id}`);
  await expect(page.getByRole("heading", { name: spell.title, exact: true, level: 1 })).toBeVisible();
  await expect(update).toBeVisible();
  await expect(update).toHaveAttribute("aria-label", UPDATE_NAME);
  await expect(update).toContainText("Update");
  await expect(page.locator("header.app-header [role=status]")).toHaveText("App update ready.");
  // No banner: the reading content stays exactly where it was.
  expect((await page.locator("main").boundingBox())!.y).toBe(mainTop);
  await expect(page.locator(".banner")).toHaveCount(0);

  // A downloaded app update must wait; reading and user data still use the current app.
  await page.goto("#/settings");
  await expect(shell).toHaveText(previousShell);
  await expect(page.getByTestId("library-version")).toHaveText(library.content_version);
  // Settings keeps its own reload row.
  const settingsRow = page.locator(".banner-inline");
  await expect(settingsRow).toContainText("App update ready.");
  await expect(settingsRow.getByRole("button", { name: "Reload", exact: true })).toBeVisible();
  expect((await activeLibrary(page)).content_version).toBe(library.content_version);
  await expect.poll(async () => Object.keys(await cacheContents(page))).toContain(`cast-iron-shell-${nextShell}`);
  expect(Object.keys(await cacheContents(page))).toContain(`cast-iron-shell-${previousShell}`);

  await update.click();
  await waitReady(page);
  await expect(page.getByTestId("shell-version")).toHaveText(nextShell);
  await expect.poll(async () => Object.keys(await cacheContents(page))).not.toContain(`cast-iron-shell-${previousShell}`);
  expect(contentCaches(await cacheContents(page))).toEqual(cachesBefore);
  expect((await activeLibrary(page)).content_version).toBe(library.content_version);
  await goOffline(context, server);
  await page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Home", exact: true }).click();
  await page.reload();
  await waitReady(page);
  await page.locator(".home-bookmarks").getByRole("link", { name: new RegExp(spell.title) }).click();
  await expect(page.getByRole("heading", { level: 1, name: spell.title, exact: true })).toBeVisible();
  await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
});
