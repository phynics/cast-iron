import { test, expect, distManifest } from "./fixtures";
import { activeLibrary, buildUpdatedLibrary, cacheContents, contentCaches, installLibrary, waitControlled } from "./helpers";

const v1 = distManifest().content_version;

test("Content update keeps user records", async ({ page, context, server }) => {
  const marker = "UPDATED-LIBRARY-MARKER";
  const next = buildUpdatedLibrary(marker);
  try {
    expect(next.version).not.toBe(v1);
    await installLibrary(page);
    await waitControlled(page);

    // User records under v1: a bookmark and a collection.
    await page.goto("#/entry/core-rulebook/spells/fireball");
    await page.locator(".bookmark-toggle").click();
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
    await page.locator(".picker summary").click();
    await page.locator("#new-collection-entry").fill("Keepers");
    await page.getByRole("button", { name: "Create and add" }).click();
    await expect(page.locator(".picker .check", { hasText: "Keepers" }).locator("input")).toBeChecked();

    // No update yet: the check says so.
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check for library update" }).click();
    await expect(page.getByTestId("update-status")).toContainText("up to date");

    // The server now offers v2. One of its files is slow, so the update is in flight for a while.
    await server.overlay(next.dir);
    await server.hook(`content/${next.version}/entries/creature-000.json`, "slow", { ms: 150 });
    await page.getByRole("button", { name: "Check for library update" }).click();
    await expect(page.getByText("Newer library available")).toBeVisible();
    await expect(page.locator(".install-panel .facts")).toContainText(next.version);
    await page.getByRole("button", { name: "Download update" }).click();
    await expect(page.getByTestId("install-status")).toContainText(/Downloading/);

    // While v2 downloads, a second page still reads v1 and shows the user's bookmark.
    await expect.poll(async () => contentCaches(await cacheContents(page)).sort()).toEqual([`cast-iron-content-${v1}`, `cast-iron-content-${next.version}`].sort());
    const reader = await context.newPage();
    await reader.goto("#/entry/core-rulebook/spells/fireball");
    await expect(reader.getByRole("heading", { level: 1, name: "Fireball" })).toBeVisible();
    await reader.goto("#/legal");
    await expect(reader.locator("#community-use + .legal-text")).not.toContainText(marker);
    expect((await activeLibrary(reader)).content_version).toBe(v1);
    await reader.close();

    // Activation: the new version is active, the old cache is gone, the records are intact.
    await expect(page.getByTestId("library-version")).toHaveText(next.version, { timeout: 60_000 });
    expect((await activeLibrary(page)).content_version).toBe(next.version);
    await expect.poll(async () => contentCaches(await cacheContents(page))).toEqual([`cast-iron-content-${next.version}`]);
    const cached = new Set((await cacheContents(page))[`cast-iron-content-${next.version}`]);
    expect(next.manifest.files.every((f) => cached.has(`content/${f.path}`))).toBe(true);

    await page.goto("#/legal");
    await expect(page.locator("#community-use + .legal-text")).toContainText(marker);
    await page.goto("#/bookmarks");
    await expect(page.locator(".bm-list")).toContainText("Fireball");
    await expect(page.locator(".bm-list")).not.toContainText("not in this library");
    await expect(page.locator(".collection-list")).toContainText("Keepers");
    await page.goto("#/entry/core-rulebook/spells/fireball");
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  } finally {
    next.cleanup();
  }
});
