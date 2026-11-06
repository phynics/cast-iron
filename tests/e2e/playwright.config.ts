import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const dist = path.resolve(__dirname, "../../dist");

// The suite tests the real build. It never builds, and it fails early and clearly without one.
if (!fs.existsSync(path.join(dist, "index.html")) || !fs.existsSync(path.join(dist, "content/manifest.json"))) {
  throw new Error(
    `No built site at ${dist}.\nBuild it first from the repository root:\n  cargo xtask build-site --public-url /cast-iron/`,
  );
}

export default defineConfig({
  testDir: "./tests",
  // Each worker starts its own test server on a free port (see `fixtures.ts`), so workers do not
  // share hooks and parallel runs on one machine do not collide. CI keeps one worker per shard
  // runner; set E2E_WORKERS to change the local default.
  workers: Number(process.env.E2E_WORKERS ?? (process.env.CI ? 1 : 4)),
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  timeout: 120_000,
  expect: { timeout: 15_000 },
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    // A service worker is the thing under test; the default "allow" must not change.
    serviceWorkers: "allow",
  },
  projects: [
    { name: "chromium", testIgnore: /mobile\.spec\.ts/, use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", testIgnore: /mobile\.spec\.ts/, use: { ...devices["Desktop Firefox"] } },
    // Playwright's trace and screenshot hooks inject an inline <style> into WebKit pages of persistent
    // contexts. The page CSP refuses it, which is a false console error. So no traces for WebKit.
    { name: "webkit", testIgnore: /mobile\.spec\.ts/, use: { ...devices["Desktop Safari"], trace: "off", screenshot: "off" } },
    // Mobile layout is a viewport check, not a browser matrix entry.
    // The shortcut spec runs here too, because the header search icon stays visible at 390 px.
    { name: "mobile-chromium", testMatch: /(mobile|slash-shortcut)\.spec\.ts/, use: { ...devices["Pixel 7"], viewport: { width: 390, height: 844 } } },
  ],
});
