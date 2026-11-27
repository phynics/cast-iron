import { test, expect, corpusLabel, distManifest } from "./fixtures";
import { activeLibrary, cacheContents, columnEdges, contentCaches, installLibrary, waitReady } from "./helpers";
import path from "node:path";

const manifest = distManifest();
const version = manifest.content_version;
const cacheName = `cast-iron-content-${version}`;
const contentPaths = (requests: string[]) =>
  requests.filter((p) => p.startsWith("content/") && !p.endsWith("manifest.json")).map((p) => p.slice("content/".length));
/**
 * Entry shards of the built library, in manifest order, as the stored files: `entries/<kind>-NNN.json.gz`
 * since schema 8, `.json` in a library of schema 7 or older. Both libraries shard by kind.
 */
const entryShards = manifest.files.map((f) => f.path).filter((p) => /(^|\/)entries\/[a-z_]+-\d+\.json(\.gz)?$/.test(p));
const shardName = (index: number) => path.posix.basename(entryShards[index]);

test("From 48rem up the install screen starts at the brand's left edge", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Install the rules library" })).toBeVisible();
  for (const width of [1024, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    const edges = await columnEdges(page);
    expect(Math.abs(edges.brand - edges.title), `h1 vs brand at ${width}`).toBeLessThanOrEqual(1);
    expect(edges.content, `content width at ${width}`).toBeLessThanOrEqual(edges.measure + 1);
    for (const line of edges.footer) {
      expect(Math.abs(edges.brand - line.left), `footer vs brand at ${width}`).toBeLessThanOrEqual(1);
      expect(line.width, `footer line length at ${width}`).toBeLessThanOrEqual(edges.measure + 1);
    }
  }

  // The phone layout keeps its 16 px header and footer padding.
  await page.setViewportSize({ width: 390, height: 844 });
  const narrow = await columnEdges(page);
  expect(narrow.brand, "brand left at 390").toBeCloseTo(16, 0);
  expect(narrow.footer[0].left, "footer left at 390").toBeCloseTo(16, 0);
});

test("Representative rules dataset installs completely", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Install the rules library" })).toBeVisible();
  await expect(page.getByTestId("download-size")).toHaveText(/\d+(\.\d+)? [MG]B/);
  // The version is the corpus tag with a short id, never the bare content hash (#584).
  await expect(page.getByText("Library version", { exact: true })).toBeVisible();
  await expect(page.getByTestId("install-version")).toHaveText(corpusLabel(manifest));
  if (manifest.corpus_tag) await expect(page.getByTestId("install-version")).toHaveText(/^v0\./);
  await expect(page.locator(".facts")).not.toContainText(version);
  await expect(page.getByText("On iPhone?")).toBeVisible();
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

test("Installing by keyboard keeps focus on the install screen status line, then main, and announces calmly", async ({ page }) => {
  await page.goto("./");
  const download = page.getByRole("button", { name: "Download" });
  await expect(download).toBeVisible();
  // Record each distinct consecutive text of the live announcement and of the detailed line. One
  // text replacement can give several mutation records, so records are not counted.
  await page.evaluate(() => {
    const w = window as unknown as { __texts: Record<string, string[]> };
    w.__texts = { live: [], detail: [] };
    const watch = (testid: string, key: string) => {
      const el = document.querySelector(`[data-testid=${testid}]`)!;
      new MutationObserver(() => {
        const text = (el.textContent ?? "").trim();
        const seen = w.__texts[key];
        if (text !== "" && text !== seen[seen.length - 1]) seen.push(text);
      }).observe(el, { childList: true, characterData: true, subtree: true });
    };
    watch("install-announce", "live");
    watch("install-status", "detail");
  });
  const active = () =>
    page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      return { tag: el?.tagName, id: el?.id, testid: el?.getAttribute("data-testid") };
    });

  await download.focus();
  // Sample the focus every 10 ms from the keypress on, for as long as the install screen is up.
  await page.evaluate(() => {
    const w = window as unknown as { __samples: string[] };
    w.__samples = [];
    setInterval(() => {
      if (document.querySelector("[data-testid=install-status]")) {
        w.__samples.push(document.activeElement?.getAttribute("data-testid") ?? document.activeElement?.tagName ?? "none");
      }
    }, 10);
  });
  await page.keyboard.press("Enter");
  // The button is gone; the status line holds focus through the whole install, never the body.
  await expect(page.getByTestId("install-status")).toContainText(/Checking what is already stored|Downloading|Verifying|Activating|installed/);
  await waitReady(page);
  const samples = await page.evaluate(() => (window as unknown as { __samples: string[] }).__samples);
  expect(samples.length, "focus samples while installing").toBeGreaterThan(0);
  expect(samples.filter((s) => s !== "install-status" && !s.startsWith("BUTTON")), "focus held outside the status line").toEqual([]);
  await expect.poll(active).toMatchObject({ tag: "MAIN", id: "main" });

  const texts = await page.evaluate(() => (window as unknown as { __texts: Record<string, string[]> }).__texts);
  expect(texts.live.length, `live announcements: ${texts.live.join(" | ")}`).toBeGreaterThanOrEqual(2);
  expect(texts.live.length, `live announcements: ${texts.live.join(" | ")}`).toBeLessThanOrEqual(15);
  expect(texts.live.length, "live announcements against detail lines").toBeLessThan(texts.detail.length);
});

test("Download integrity is verified", async ({ page, server, guard }) => {
  guard.allow(/Failed to load resource|status of 5\d\d/);
  const bad = entryShards[0];
  await server.hook(`content/${bad}`, "corrupt");

  await page.goto("./");
  await page.getByRole("button", { name: "Download" }).click();
  const error = page.getByRole("alert").filter({ hasText: shardName(0) });
  await expect(error).toContainText(/checksum|hash|sha/i, { timeout: 60_000 });
  // The failure takes focus: the Download button and the status line it used are gone or idle.
  await expect(error).toBeFocused();
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
  // This test closes the session with one request in flight. The installer aborts a silent request
  // after `STALL_WINDOW_MS` (15 s) and retries it, and a retry would leave `stalled` at 0 for a
  // moment, or for good once all three attempts have expired on a slow run. Keep the window far
  // longer than the test, so that the request is still in flight when the context closes.
  await context.addInitScript(() => {
    const setTimeoutReal = window.setTimeout.bind(window);
    window.setTimeout = ((handler: TimerHandler, ms?: number, ...args: unknown[]) =>
      setTimeoutReal(handler, ms === 15_000 ? 3_600_000 : ms, ...args)) as typeof window.setTimeout;
  });
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

test("A request that goes silent is aborted after the stall window and retried (#768)", async ({ page, server, guard }) => {
  guard.allow(/Failed to load resource|net::ERR|NetworkError|Load failed|AbortError|aborted/);
  // The installer waits `STALL_WINDOW_MS` (15 s) for bytes. Shorten that one timer for this page,
  // as the test of a browser without DecompressionStream replaces a browser built-in.
  await page.addInitScript(() => {
    const setTimeoutReal = window.setTimeout.bind(window);
    window.setTimeout = ((handler: TimerHandler, ms?: number, ...args: unknown[]) =>
      setTimeoutReal(handler, ms === 15_000 ? 1_000 : ms, ...args)) as typeof window.setTimeout;
  });
  // The first request for this file sends 1 KB and then goes silent; the retry gets the file.
  const silent = entryShards[1];
  await server.hook(`content/${silent}`, "stall", { once: true });

  await page.goto("./");
  await page.getByRole("button", { name: "Download" }).click();
  await waitReady(page);

  const attempts = (await server.requests()).filter((p) => p === `content/${silent}`).length;
  expect(attempts).toBe(2);
  // The aborted response is gone from the server's side too.
  await expect.poll(async () => (await server.state()).stalled).toBe(0);
  expect((await activeLibrary(page)).content_version).toBe(version);
  const cached = new Set((await cacheContents(page))[cacheName]);
  expect(manifest.files.every((f) => cached.has(`content/${f.path}`))).toBe(true);
});
