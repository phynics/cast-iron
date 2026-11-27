import { test, expect, corpusLabel, distManifest } from "./fixtures";
import {
  activeLibrary,
  buildGzippedLibrary,
  cacheContents,
  contentCaches,
  corpusEntry,
  hitTitles,
  installLibrary,
  searchFor,
  waitControlled,
  waitReady,
} from "./helpers";

// A library whose files are stored as gzip (design 2026-11-27, slice 2). The overlay gzips every
// file of dist and lists the decoded hash and size under `gzip`; the reader decodes each read.
const entry = corpusEntry("spell", "Fireball");
const v1 = distManifest().content_version;
let lib: ReturnType<typeof buildGzippedLibrary>;

test.beforeAll(() => {
  test.setTimeout(300_000);
  lib = buildGzippedLibrary();
});
test.afterAll(() => lib?.cleanup());

const shardPath = () =>
  lib.manifest.files.map((f) => f.path).find((p) => /\/entries\/[a-z_]+-\d+\.json\.gz$/.test(p))!;

async function expectEntryAndSearch(page: import("./fixtures").Page) {
  await page.goto(`#/entry/${entry.id}`);
  await expect(page.getByRole("heading", { level: 1, name: entry.title })).toBeVisible();
  await searchFor(page, entry.title);
  expect((await hitTitles(page))[0]).toBe(entry.title);
}

test("A gzip library installs, opens an entry, searches and reloads offline", async ({ openProfile, server }) => {
  test.setTimeout(300_000);
  await server.overlay(lib.dir);
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  await page.evaluate(() => navigator.serviceWorker.ready);

  // The stored files are the .gz files, under the manifest's own paths.
  expect((await activeLibrary(page)).content_version).toBe(lib.version);
  const cached = (await cacheContents(page))[`cast-iron-content-${lib.version}`];
  expect(cached).toContain(`content/${lib.version}/search.bin.gz`);
  expect(cached).not.toContain(`content/${lib.version}/search.bin`);
  expect(cached.filter((p) => p.endsWith(".gz")).length).toBe(lib.manifest.files.length);
  const stored = (await server.requests()).filter((p) => p.endsWith(".gz"));
  expect(stored.length).toBeGreaterThan(0);

  await expectEntryAndSearch(page);

  // Diagnostics records the stored size beside the decoded one.
  await page.goto("#/settings/diagnostics");
  await expect(page.getByRole("main")).toContainText(/search index: read .*stored size [\d.]+ MB/);
  await context.close();

  // A new session with no network: the same reads decode from the cache.
  await server.clearLog();
  context = await openProfile({ offline: true });
  page = await context.newPage();
  await page.goto("./");
  await waitReady(page);
  await expectEntryAndSearch(page);
  expect((await server.requests()).filter((p) => !p.startsWith("sw.js"))).toEqual([]);
});

test("A corrupted gzip file stops the update and the old library stays active", async ({ page, server, guard }) => {
  test.setTimeout(300_000);
  guard.allow(/Failed to load resource|status of 5\d\d/);
  await installLibrary(page);
  await waitControlled(page);
  expect((await activeLibrary(page)).content_version).toBe(v1);

  const bad = shardPath();
  await server.overlay(lib.dir);
  await server.hook(`content/${bad}`, "corrupt");
  await page.goto("#/settings");
  await page.getByRole("button", { name: "Check now", exact: true }).click();
  await page.getByRole("button", { name: "Download update" }).click();
  const error = page.getByRole("alert").filter({ hasText: bad.split("/").pop()! });
  await expect(error).toContainText(/checksum|hash|sha/i, { timeout: 120_000 });

  // The stored file failed its hash three times, nothing was activated, and the old library reads.
  expect((await server.requests()).filter((p) => p === `content/${bad}`).length).toBe(3);
  expect((await activeLibrary(page)).content_version).toBe(v1);
  expect(contentCaches(await cacheContents(page))).toContain(`cast-iron-content-${v1}`);
  expect((await cacheContents(page))[`cast-iron-content-${lib.version}`] ?? []).not.toContain(`content/${bad}`);
  await expectEntryAndSearch(page);
});

test("A browser without DecompressionStream reads a gzip library through the Rust fallback", async ({ page, context, server }) => {
  test.setTimeout(300_000);
  await context.addInitScript(() => {
    Object.defineProperty(window, "DecompressionStream", { value: undefined, configurable: true });
  });
  await server.overlay(lib.dir);
  await installLibrary(page);
  expect(await page.evaluate(() => typeof (window as any).DecompressionStream)).toBe("undefined");
  expect((await activeLibrary(page)).content_version).toBe(lib.version);
  await expect(page.getByRole("navigation", { name: "Primary" })).toBeVisible();

  await expectEntryAndSearch(page);
  await page.goto("#/settings");
  await expect(page.getByTestId("library-version")).toHaveText(corpusLabel(lib.manifest));
});
