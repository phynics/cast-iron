import { test, expect, distManifest } from "./fixtures";
import { activeLibrary, cacheContents, contentCaches, installLibrary, waitReady } from "./helpers";
import path from "node:path";

const manifest = distManifest();
const version = manifest.content_version;
const cacheName = `cast-iron-content-${version}`;
const contentPaths = (requests: string[]) =>
  requests.filter((p) => p.startsWith("content/") && !p.endsWith("manifest.json")).map((p) => p.slice("content/".length));
/** Entry shards of the built library, in manifest order. Both libraries shard by kind. */
const entryShards = manifest.files.map((f) => f.path).filter((p) => /(^|\/)entries\/[a-z_]+-\d+\.json$/.test(p));
const shardName = (index: number) => path.posix.basename(entryShards[index]);

test("Representative rules dataset installs completely", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Install the rules library" })).toBeVisible();
  await expect(page.getByTestId("download-size")).toHaveText(/\d+(\.\d+)? [MG]B/);
  await expect(page.locator(".facts")).toContainText(version);
  await page.getByRole("button", { name: "Download" }).click();
  await expect(page.getByTestId("install-status")).toContainText(/Downloading|Verifying|Checking what is already stored|Activating|installed/);
  await waitReady(page);

  const active = await activeLibrary(page);
  expect(active.content_version).toBe(version);
  expect(active.entry_count).toBe(manifest.entry_count);

  const caches = await cacheContents(page);
  expect(contentCaches(caches)).toEqual([cacheName]);
  const cached = new Set(caches[cacheName]);
  const missing = manifest.files.filter((f) => !cached.has(`content/${f.path}`)).map((f) => f.path);
  expect(missing).toEqual([]);
  expect(cached.has(`content/${version}/manifest.json`)).toBe(true);
});

test("Download integrity is verified", async ({ page, server, guard }) => {
  guard.allow(/Failed to load resource|status of 5\d\d/);
  const bad = entryShards[0];
  await server.hook(`content/${bad}`, "corrupt");

  await page.goto("./");
  await page.getByRole("button", { name: "Download" }).click();
  const error = page.getByRole("alert").filter({ hasText: shardName(0) });
  await expect(error).toContainText(/checksum|hash|sha/i, { timeout: 60_000 });
  await expect(page.getByRole("button", { name: "Retry" })).toBeVisible();

  // Three attempts, no activation, no corrupted entry in the cache.
  const attempts = (await server.requests()).filter((p) => p === `content/${bad}`).length;
  expect(attempts).toBe(3);
  expect(await activeLibrary(page)).toBeNull();
  await expect(page.getByRole("navigation", { name: "Primary" })).toHaveCount(0);
  const before = await cacheContents(page);
  expect(before[cacheName] ?? []).not.toContain(`content/${bad}`);

  // A fixed server and Retry finish the job, and only files that are not yet verified are fetched.
  await server.unhook(`content/${bad}`);
  await server.clearLog();
  await page.getByRole("button", { name: "Retry" }).click();
  await waitReady(page);
  const have = new Set(before[cacheName] ?? []);
  const expected = manifest.files.map((f) => f.path).filter((p) => !have.has(`content/${p}`));
  expect(expected).toContain(bad);
  const fetched = contentPaths(await server.requests());
  expect(fetched.sort()).toEqual(expected.sort());
  expect((await activeLibrary(page)).content_version).toBe(version);
});

test("Interrupted installation can recover safely", async ({ openProfile, server, guard }) => {
  guard.allow(/Failed to load resource|net::ERR|NetworkError|Load failed/);
  const slow = entryShards[1];
  await server.hook(`content/${slow}`, "stall");

  let context = await openProfile();
  let page = await context.newPage();
  await page.goto("./");
  await page.getByRole("button", { name: "Download" }).click();
  // Every file but the stalled one arrives; the stalled response is in flight when the session ends.
  const expectedCached = manifest.files.length - 1;
  await expect
    .poll(async () => ((await cacheContents(page))[cacheName] ?? []).length, { timeout: 90_000 })
    .toBe(expectedCached);
  await expect.poll(async () => (await server.state()).stalled).toBeGreaterThan(0);
  expect(await activeLibrary(page)).toBeNull();
  await context.close();

  // New session: nothing is ready, and the app says the install did not finish.
  await server.unhook(`content/${slow}`);
  context = await openProfile();
  page = await context.newPage();
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Install the rules library" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary" })).toHaveCount(0);
  await expect(page.locator(".notice")).toContainText(/did not finish/);
  expect(await activeLibrary(page)).toBeNull();
  const verified = (await cacheContents(page))[cacheName] ?? [];
  expect(verified).not.toContain(`content/${slow}`);
  expect(verified.length).toBeGreaterThan(5);

  // Resume downloads only what is missing.
  await server.clearLog();
  await page.getByRole("button", { name: "Download" }).click();
  await waitReady(page);
  const fetched = contentPaths(await server.requests());
  expect(fetched).toEqual([slow]);
  expect((await activeLibrary(page)).content_version).toBe(version);
  const done = new Set((await cacheContents(page))[cacheName]);
  expect(manifest.files.every((f) => done.has(`content/${f.path}`))).toBe(true);
});
