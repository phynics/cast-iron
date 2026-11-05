import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const dist = path.resolve(__dirname, "../../dist");
const port = Number(process.env.E2E_PORT ?? 4318);

// The suite tests the real build. It never builds, and it fails early and clearly without one.
if (!fs.existsSync(path.join(dist, "index.html")) || !fs.existsSync(path.join(dist, "content/manifest.json"))) {
  throw new Error(
    `No built site at ${dist}.\nBuild it first from the repository root:\n  cargo xtask build-site --public-url /cast-iron/\n` +
      `For the canonical library:\n  cargo xtask content --canonical && cargo xtask build-site --canonical --public-url /cast-iron/\n` +
      `and set CAST_IRON_CONTENT=generated/canonical for the test run.`,
  );
}

export default defineConfig({
  testDir: "./tests",
  // Tests share one server and reset its hooks, so they run one at a time.
  workers: 1,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  timeout: 120_000,
  expect: { timeout: 15_000 },
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: `http://127.0.0.1:${port}/cast-iron/`,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    // A service worker is the thing under test; the default "allow" must not change.
    serviceWorkers: "allow",
  },
  webServer: {
    command: "node server.mjs",
    cwd: __dirname,
    url: `http://127.0.0.1:${port}/cast-iron/`,
    reuseExistingServer: false,
    env: { E2E_PORT: String(port), E2E_DIST: dist },
    timeout: 30_000,
  },
  projects: [
    { name: "chromium", testIgnore: /mobile\.spec\.ts/, use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", testIgnore: /mobile\.spec\.ts/, use: { ...devices["Desktop Firefox"] } },
    // Playwright's trace and screenshot hooks inject an inline <style> into WebKit pages of persistent
    // contexts. The page CSP refuses it, which is a false console error. So no traces for WebKit.
    { name: "webkit", testIgnore: /mobile\.spec\.ts/, use: { ...devices["Desktop Safari"], trace: "off", screenshot: "off" } },
    // Mobile layout is a viewport check, not a browser matrix entry.
    { name: "mobile-chromium", testMatch: /mobile\.spec\.ts/, use: { ...devices["Pixel 7"], viewport: { width: 390, height: 844 } } },
  ],
});
