import { test, expect, corpusLabel, distManifest, loadCorpus } from "./fixtures";
import {
  activeLibrary,
  buildNewerManifest,
  buildUpdatedLibrary,
  cacheContents,
  contentCaches,
  corpusEntry,
  deleteMeta,
  firstBookPlacement,
  installLibrary,
  openBookGroups,
  storedMeta,
  waitControlled,
  waitHomeIdle,
  waitReady,
  writeMeta,
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
    await page.goto("#/books/manage");
    await openBookGroups(page);
    await page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true }).uncheck();
    await expect(page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true })).toBeEnabled();

    // No update yet: the check says so.
    await page.goto("#/settings");
    await page.getByRole("button", { name: "Check now", exact: true }).click();
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
    await page.getByRole("button", { name: "Check now", exact: true }).click();
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
    await expect(page.getByTestId("library-version")).toHaveText(corpusLabel(next.manifest), { timeout: 120_000 });
    expect((await activeLibrary(page)).content_version).toBe(next.version);
    await expect.poll(async () => contentCaches(await cacheContents(page))).toEqual([`cast-iron-content-${next.version}`]);
    const cached = new Set((await cacheContents(page))[`cast-iron-content-${next.version}`]);
    expect(next.manifest.files.every((f) => cached.has(`content/${f.path}`))).toBe(true);

    await page.goto("#/legal");
    await expect(page.locator("#community-use + .legal-text")).toContainText(marker);
    await page.goto("#/bookmarks");
    await expect(page.locator(".home-bookmarks")).toContainText(entry.title);
    await expect(page.locator(".home-bookmarks")).not.toContainText("not in this library");
    await page.goto("#/books/manage");
    await openBookGroups(page);
    await expect(page.getByRole("checkbox", { name: `Active ${otherSource!.title}`, exact: true })).not.toBeChecked();
    await page.goto(`#/entry/${entry.id}`);
    await expect(page.locator(".bookmark-toggle")).toHaveAttribute("aria-pressed", "true");
  } finally {
    next.cleanup();
  }
});

test("The daily check surfaces a newer library, and Settings puts Updates first", async ({ page, server }) => {
  const next = buildNewerManifest();
  try {
    await installLibrary(page);
    await waitControlled(page);
    // Forget the check the installation made, so the next start asks again at once.
    await deleteMeta(page, "update_check");
    await server.overlay(next.dir);
    await waitHomeIdle(page);
    await page.reload();
    await waitReady(page);

    // The start check finds the newer manifest, and the header says so without a tap.
    const action = page
      .locator("header.app-header")
      .getByRole("button", { name: "Library update available. Open Settings" });
    await expect(action).toBeVisible();
    await action.click();

    // Settings → Updates, focused and first, with the library row before the app row.
    await expect(page.locator("#set-updates")).toBeFocused();
    const sections = page.locator("main section.settings-section");
    await expect(sections.first()).toHaveAttribute("id", "set-updates");
    const row = page.getByTestId("library-update");
    await expect(row).toContainText(`Newer rules library: ${next.version}`);
    await expect(row).toContainText("Your stars and book choices are kept.");
    await expect(page.getByTestId("last-checked")).toContainText("Last checked:");
    await expect(page.getByRole("button", { name: "Check now", exact: true })).toBeVisible();

    // A check only reads the manifest: the active library and its cache are untouched.
    expect((await activeLibrary(page)).content_version).toBe(v1);
    expect(contentCaches(await cacheContents(page))).toEqual([`cast-iron-content-${v1}`]);
  } finally {
    next.cleanup();
  }
});

test("With the automatic check off, no manifest request happens at start", async ({ page, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  await page.goto("#/settings");
  const toggle = page.getByRole("checkbox", { name: "Check for updates automatically" });
  await expect(toggle).toBeChecked();
  await toggle.uncheck();
  await expect.poll(() => storedMeta(page, "auto_update_check")).toBe(false);

  // An enabled app would ask again as soon as the last check is forgotten.
  await deleteMeta(page, "update_check");
  await server.clearLog();
  await page.reload();
  await waitReady(page);
  // The switch renders from the stored value that the start path read before it would check.
  await expect(page.getByRole("checkbox", { name: "Check for updates automatically" })).not.toBeChecked();
  await expect.poll(() => storedMeta(page, "update_check")).toBeNull();
  expect((await server.requests()).filter((p) => p === "content/manifest.json")).toEqual([]);
});

test("A stored newer library mounts no installer until the reader asks", async ({ page, server }) => {
  await installLibrary(page);
  await waitControlled(page);
  const library = await activeLibrary(page);
  // A record from an earlier session: it knows the version and size, but holds no manifest.
  await writeMeta(page, "auto_update_check", false);
  await writeMeta(page, "update_check", {
    last_checked: Date.now(),
    known_version: "v0000000000000000",
    known_bytes: library.total_bytes,
  });
  await server.clearLog();
  await waitHomeIdle(page);
  await page.reload();
  await waitReady(page);

  // Opening Settings must not fetch the manifest just because a record names a newer library.
  await page.goto("#/settings");
  const row = page.getByTestId("library-update");
  await expect(row).toContainText("Newer rules library: v0000000000000000");
  await expect(row).toContainText("Your stars and book choices are kept.");
  await expect(row.locator(".install-panel")).toHaveCount(0);
  await expect(row.getByRole("button", { name: "Check now", exact: true })).toBeVisible();
  await expect
    .poll(async () => (await server.requests()).filter((p) => p === "content/manifest.json"))
    .toEqual([]);
  // The stored record is untouched: no check ran and wrote a new one.
  await expect
    .poll(async () => (await storedMeta(page, "update_check"))?.known_version)
    .toBe("v0000000000000000");
});

test("Offline, the automatic check asks for nothing and Settings still works", async ({ openProfile, server }) => {
  let context = await openProfile();
  let page = await context.newPage();
  await installLibrary(page);
  await waitControlled(page);
  // The install session made its check, so the offline session has a time to show.
  await expect.poll(() => storedMeta(page, "update_check")).not.toBeNull();
  await context.close();

  // A new session with no network from its first navigation.
  await server.clearLog();
  context = await openProfile({ offline: true });
  page = await context.newPage();
  // Offline hides the request from the server, so record every attempt on the page as well.
  await page.addInitScript(() => {
    const original = window.fetch.bind(window);
    (window as any).__fetches = [];
    window.fetch = (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
      (window as any).__fetches.push(url);
      return original(input, init);
    };
  });
  await page.goto("./");
  await waitReady(page);
  await expect(page.locator("#home-title")).toBeVisible();
  await page.goto("#/settings");
  await expect(page.getByRole("heading", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.getByTestId("last-checked")).toContainText("Last checked:");
  await expect
    .poll(() =>
      page.evaluate(() =>
        ((window as any).__fetches as string[]).filter((url) => url.includes("content/manifest.json")),
      ),
    )
    .toEqual([]);
  expect((await server.requests()).filter((p) => p === "content/manifest.json")).toEqual([]);
});
