import { test as base, expect, type BrowserContext, type Page } from "@playwright/test";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export { expect };
export type { BrowserContext, Page };

export const ROOT = path.resolve(__dirname, "../../..");
export const DIST = path.join(ROOT, "dist");
/**
 * Compiled corpus the test expectations come from. The default is the canonical library
 * that `cargo xtask build-site` packages from `generated/content`. Set
 * CAST_IRON_CONTENT=generated/psrd to check the same suite against the PSRD carry-over
 * build (`cargo xtask content --psrd` and `cargo xtask build-site --psrd`).
 */
export const GENERATED = path.resolve(ROOT, process.env.CAST_IRON_CONTENT ?? "generated/content");

export interface ManifestFile {
  path: string;
  sha256: string;
  bytes: number;
}
export interface Manifest {
  schema_version: number;
  content_version: string;
  total_bytes: number;
  entry_count: number;
  files: ManifestFile[];
}

/** The manifest of the built site, which is what the test server hands out. */
export function distManifest(): Manifest {
  return JSON.parse(fs.readFileSync(path.join(DIST, "content/manifest.json"), "utf8"));
}

/** Compiled entries from GENERATED, keyed by id. The expectations come from here. */
export interface Entry {
  id: string;
  kind: string;
  title: string;
  source_ids: string[];
  fields: Record<string, any>;
  body: any[];
  relations?: any[];
}
export interface Stub {
  id: string;
  kind: string;
  title: string;
  source: string;
  shard: string;
}
export interface Corpus {
  version: string;
  entries: Map<string, Entry>;
  catalog: { entries: Stub[]; sources: any[] };
}
let cachedCorpus: Corpus | undefined;
/**
 * The compiled corpus, parsed once per worker. The canonical library is about 29,000
 * entries, so a second parse per spec file would dominate the run's memory and startup.
 */
export function loadCorpus(): Corpus {
  if (cachedCorpus) return cachedCorpus;
  const manifest: Manifest = JSON.parse(fs.readFileSync(path.join(GENERATED, "manifest.json"), "utf8"));
  const built: Manifest = JSON.parse(fs.readFileSync(path.join(DIST, "content/manifest.json"), "utf8"));
  if (built.content_version !== manifest.content_version) {
    throw new Error(
      `The built site has content ${built.content_version}, but the expectations read ` +
        `${manifest.content_version} from ${path.relative(ROOT, GENERATED)}.\n` +
        `Build both from the same library, or point CAST_IRON_CONTENT at the corpus that was built.\n` +
        `  canonical (default): cargo xtask content && cargo xtask build-site\n` +
        `  PSRD:                cargo xtask content --psrd && cargo xtask build-site --psrd\n` +
        `Then run the suite with CAST_IRON_CONTENT=generated/psrd for the PSRD build.`,
    );
  }
  const dir = path.join(GENERATED, manifest.content_version, "entries");
  const entries = new Map<string, Entry>();
  for (const file of fs.readdirSync(dir).sort()) {
    for (const entry of JSON.parse(fs.readFileSync(path.join(dir, file), "utf8"))) entries.set(entry.id, entry);
  }
  const catalog = JSON.parse(fs.readFileSync(path.join(GENERATED, manifest.content_version, "catalog.json"), "utf8"));
  cachedCorpus = { version: manifest.content_version, entries, catalog };
  return cachedCorpus;
}

export interface BacklinkSourceRow {
  source: string;
  anchor?: string;
}

/**
 * The reverse body-link index from GENERATED, target id to its sources. The descriptor names
 * the shard count, so the loader follows whatever the compiler wrote. An older library has
 * no descriptor and returns an empty map.
 */
export function loadBacklinks(): Map<string, BacklinkSourceRow[]> {
  const manifest: Manifest = JSON.parse(fs.readFileSync(path.join(GENERATED, "manifest.json"), "utf8"));
  const descriptorPath = path.join(GENERATED, manifest.content_version, "backlinks.json");
  const out = new Map<string, BacklinkSourceRow[]>();
  if (!fs.existsSync(descriptorPath)) return out;
  const descriptor = JSON.parse(fs.readFileSync(descriptorPath, "utf8")) as { shards: number };
  for (let i = 0; i < descriptor.shards; i++) {
    const shardPath = path.join(
      GENERATED,
      manifest.content_version,
      "backlinks",
      `${String(i).padStart(2, "0")}.json`,
    );
    const shard = JSON.parse(fs.readFileSync(shardPath, "utf8")) as [
      string,
      [string, string | null][],
    ][];
    for (const [target, rows] of shard) {
      out.set(
        target,
        rows.map(([source, anchor]) => (anchor ? { source, anchor } : { source })),
      );
    }
  }
  return out;
}

/** Test-only controls of `server.mjs`. */
export class ServerControl {
  constructor(private origin: string) {}
  private async post(route: string, body: object = {}) {
    const response = await fetch(`${this.origin}/__test/${route}`, { method: "POST", body: JSON.stringify(body) });
    if (!response.ok) throw new Error(`test server ${route}: HTTP ${response.status}`);
  }
  reset() {
    return this.post("reset");
  }
  /** Fault for one path under /cast-iron/: corrupt, stall, abort, slow ({ms}), or status ({status}). */
  hook(relPath: string, mode: "corrupt" | "stall" | "abort" | "slow" | "status", extra: object = {}) {
    return this.post("hook", { path: relPath, mode, ...extra });
  }
  unhook(relPath: string) {
    return this.post("hook", { path: relPath, mode: "none" });
  }
  refuse(on: boolean) {
    return this.post("refuse", { on });
  }
  overlay(dir: string | null) {
    return this.post("overlay", { dir });
  }
  shellVersion(version: string) {
    return this.post("shell", { version });
  }
  async state(): Promise<{ log: string[]; counts: Record<string, number>; stalled: number }> {
    return (await fetch(`${this.origin}/__test/state`)).json();
  }
  /** Request paths since the last clear, relative to /cast-iron/. */
  async requests(): Promise<string[]> {
    const { log } = await this.state();
    return log.map((p) => p.replace(/^\/cast-iron\//, ""));
  }
  /** Forget the request log but keep hooks, overlay and the refuse flag. */
  async clearLog() {
    await this.post("clear-log");
  }
}

const LOCAL_HOSTS = new Set(["127.0.0.1", "localhost", "[::1]"]);

class Guard {
  console: string[] = [];
  external: string[] = [];
  private allowed: RegExp[] = [];
  /** Tolerate console errors that match, for tests that cause an error on purpose. */
  allow(pattern: RegExp) {
    this.allowed.push(pattern);
  }
  watch(context: BrowserContext) {
    context.on("request", (request) => {
      const { hostname, protocol } = new URL(request.url());
      if (/^https?:$/.test(protocol) && !LOCAL_HOSTS.has(hostname)) this.external.push(request.url());
    });
    const hook = (page: Page) => {
      page.on("console", (m) => {
        if (m.type() === "error") this.console.push(`console.error: ${m.text()} (${page.url()})`);
      });
      page.on("pageerror", (e) => this.console.push(`pageerror: ${e.message}`));
    };
    context.on("page", hook);
    context.pages().forEach(hook);
    // A CSP violation is also a console error in every browser; this makes the cause explicit.
    void context.addInitScript(() => {
      document.addEventListener("securitypolicyviolation", (e) =>
        console.error(`CSP violation: ${e.violatedDirective} ${e.blockedURI}`),
      );
    });
  }
  problems(): string[] {
    const errors = this.console.filter((line) => !this.allowed.some((p) => p.test(line)));
    return [...errors, ...this.external.map((u) => `request to a non-local host: ${u}`)];
  }
}

export interface Fixtures {
  server: ServerControl;
  guard: Guard;
  /** Opens a browser context on a profile directory that survives close(), as a real browser session does. */
  openProfile: (options?: { offline?: boolean }) => Promise<BrowserContext>;
}

/**
 * One test server per worker, on a port the OS picks. Hooks, overlays and the request log
 * belong to one worker, so workers can run in parallel, and two runs on one machine never
 * fight over a port.
 */
export const test = base.extend<Fixtures & { _auto: void }, { serverOrigin: string }>({
  serverOrigin: [
    async ({}, use) => {
      const { startServer } = await import("../server.mjs");
      const { server, port } = await startServer({ dist: DIST, port: 0 });
      await use(`http://127.0.0.1:${port}`);
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    },
    { scope: "worker" },
  ],
  baseURL: async ({ serverOrigin }, use) => {
    await use(`${serverOrigin}/cast-iron/`);
  },
  server: async ({ baseURL }, use) => {
    const server = new ServerControl(new URL(baseURL!).origin);
    await server.reset();
    await use(server);
    await server.reset();
  },
  guard: async ({}, use) => {
    const guard = new Guard();
    await use(guard);
    expect(guard.problems(), "console errors, CSP violations or third-party requests").toEqual([]);
  },
  // Every context gets the guard, including the default one.
  context: async ({ context, guard }, use) => {
    guard.watch(context);
    await use(context);
  },
  openProfile: async ({ browserName, playwright, guard, baseURL, viewport, userAgent, server }, use) => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cast-iron-e2e-"));
    const opened: BrowserContext[] = [];
    await use(async (options = {}) => {
      const context = await playwright[browserName].launchPersistentContext(dir, {
        baseURL,
        viewport,
        userAgent,
        serviceWorkers: "allow",
      });
      guard.watch(context);
      if (options.offline) await goOffline(context, server);
      opened.push(context);
      return context;
    });
    for (const context of opened) await context.close().catch(() => {});
    fs.rmSync(dir, { recursive: true, force: true });
  },
});

/**
 * Offline for real: the browser is told it has no network, and the server also drops every
 * connection, so a service worker fetch cannot succeed even where the emulation does not reach it.
 */
export async function goOffline(context: BrowserContext, server: ServerControl) {
  await context.setOffline(true);
  await server.refuse(true);
}
export async function goOnline(context: BrowserContext, server: ServerControl) {
  await context.setOffline(false);
  await server.refuse(false);
}
