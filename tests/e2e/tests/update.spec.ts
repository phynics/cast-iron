import { test, expect, distManifest, loadCorpus } from "./fixtures";
import {
  activeLibrary,
  buildUpdatedLibrary,
  cacheContents,
  contentCaches,
  corpusEntry,
  firstBookPlacement,
  installLibrary,
  openBookGroups,
  waitControlled,
} from "./helpers";

const v1 = distManifest().content_version;
const corpus = loadCorpus();
const entry = corpusEntry("spell", "Fireball");
const placedIn = firstBookPlacement(entry.id)?.source;

/** A publication the reader shows, other than the one that owns the starred entry. */
const otherSource = corpus.catalog.sources.find((source) =>
  source.entry_count > 0 && source.id !== placedIn);

test("Content update keeps user records", async ({ page, context, server }) => {
  // The canonical library is about 220 MB; building the replacement copy and verifying it
  // take longer than the default test budget on a local disk.
  test.setTimeout(300_000);
  expect(otherSource, "a second publication in the compiled library").toBeTruthy();
  const marker = "UPDATED-LIBRARY-MARKER";
  const next = buildUpdatedLibrary(marker);
  try {
    expect(next.version).not.toBe(v1);
    await installLibrary(page);
    await waitControlled(page);

    // User records under v1: a star and a disabled publication.
    await page.goto(`#/entry/${entry.id}`);
    await page.locator(".bookmark-toggle").click();
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
    await page.goto("#/books");
    await openBookGroups(page);
    await page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true }).uncheck();
    await expect(page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true })).toBeEnabled();

    // No update yet: the check says so.
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check for updates", exact: true }).click();
    await expect(page.getByTestId("update-status")).toContainText("up to date");

    // A second page opened under v1. It stays on the old library while a replacement downloads;
    // opening it first keeps the check independent of page-load time on a warm local server.
    const reader = await context.newPage();
    await reader.goto(`#/entry/${entry.id}`);
    await expect(reader.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    await reader.goto("#/legal");
    await expect(reader.locator("#community-use + .legal-text")).not.toContainText(marker);
    expect((await activeLibrary(reader)).content_version).toBe(v1);

    // The server now offers v2. One of its files is slow, so the update is in flight while the
    // open second page is checked below.
    const slow = next.manifest.files.find((file) =>
      file.path.includes("/entries/") && file.path.endsWith(".json"))!.path;
    await server.overlay(next.dir);
    // Keep v2 visibly in flight long enough that every page agrees on v1. The canonical
    // library's first entries shard is about 400 KB, so 1500 ms per 16 KB chunk holds
    // activation for roughly 40 s.
    await server.hook(`content/${slow}`, "slow", { ms: 1500 });
    await page.getByRole("button", { name: "Check for updates", exact: true }).click();
    await expect(page.getByText("Newer library available")).toBeVisible();
    await expect(page.locator(".install-panel .facts")).toContainText(next.version);
    await page.getByRole("button", { name: "Download update" }).click();
    await expect(page.getByTestId("install-status")).toContainText(/Downloading/);

    // While v2 downloads, the open page still reads v1 and shows the old legal text.
    expect((await activeLibrary(reader)).content_version).toBe(v1);
    await expect(reader.locator("#community-use + .legal-text")).not.toContainText(marker);
    await expect.poll(async () => contentCaches(await cacheContents(page)).sort()).toEqual([`cast-iron-content-${v1}`, `cast-iron-content-${next.version}`].sort());
    // A page opened now, with v2 still downloading, must also start on v1 (AGENTS.md rule 7:
    // a partly installed library never becomes active). The slow file keeps v2 in flight, and
    // the active pointer read on the fresh page is itself the proof: it would name v2 only
    // after activation. Do not add a later cache check here; activation may legitimately
    // finish while this page is being checked.
    const fresh = await context.newPage();
    await fresh.goto(`#/entry/${entry.id}`);
    await expect(fresh.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
    expect((await activeLibrary(fresh)).content_version).toBe(v1);
    await fresh.goto("#/legal");
    await expect(fresh.locator("#community-use + .legal-text")).not.toContainText(marker);
    await fresh.close();
    await reader.close();

    // Activation: the new version is active, the old cache is gone, the records are intact.
    await expect(page.getByTestId("library-version")).toHaveText(next.version, { timeout: 120_000 });
    expect((await activeLibrary(page)).content_version).toBe(next.version);
    await expect.poll(async () => contentCaches(await cacheContents(page))).toEqual([`cast-iron-content-${next.version}`]);
    const cached = new Set((await cacheContents(page))[`cast-iron-content-${next.version}`]);
    expect(next.manifest.files.every((f) => cached.has(`content/${f.path}`))).toBe(true);

    await page.goto("#/legal");
    await expect(page.locator("#community-use + .legal-text")).toContainText(marker);
    await page.goto("#/bookmarks");
    await expect(page.locator(".home-bookmarks")).toContainText(entry.title);
    await expect(page.locator(".home-bookmarks")).not.toContainText("not in this library");
    await page.goto("#/books");
    await openBookGroups(page);
    await expect(page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true })).not.toBeChecked();
    await page.goto(`#/entry/${entry.id}`);
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  } finally {
    next.cleanup();
  }
});
