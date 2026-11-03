// Static server for the acceptance tests. Dev and test only; it is not part of the product.
//
// It serves an existing `dist/` at /cast-iron/ and adds test hooks under /__test/:
//   GET  /__test/state                   request log and counts since the last reset
//   POST /__test/clear-log               forget the request log, keep hooks
//   POST /__test/reset                   clear counts, hooks, overlay and the refuse flag
//   POST /__test/hook    {path, mode...} per-path fault: corrupt | stall | abort | slow | status
//   POST /__test/refuse  {on: bool}      destroy every request (a server that is not there)
//   POST /__test/overlay {dir}           serve content/* from `dir` first (alternative library)
// Paths in hooks are relative to the prefix, such as "content/v123/entries/spell-000.json".

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".webmanifest": "application/manifest+json",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".bin": "application/octet-stream",
};

export function startServer({ dist, port, prefix = "/cast-iron/", host = "127.0.0.1" }) {
  const root = path.resolve(dist);
  const state = { log: [], hooks: new Map(), overlay: null, refuse: false, stalled: new Set() };
  const reset = () => {
    state.log = [];
    state.hooks.clear();
    state.overlay = null;
    state.refuse = false;
    for (const res of state.stalled) res.destroy();
    state.stalled.clear();
  };

  const inside = (base, rel) => {
    const dir = path.resolve(base);
    const file = path.resolve(dir, rel);
    return file.startsWith(dir + path.sep) && fs.existsSync(file) && fs.statSync(file).isFile() ? file : null;
  };
  // The overlay (an alternative library) answers for content/ only and falls back to dist.
  const resolveFile = (rel) =>
    (state.overlay && rel.startsWith("content/") ? inside(state.overlay, rel) : null) ?? inside(root, rel);

  const readBody = (req) =>
    new Promise((resolve) => {
      const chunks = [];
      req.on("data", (c) => chunks.push(c));
      req.on("end", () => resolve(Buffer.concat(chunks).toString()));
    });

  const json = (res, value, status = 200) => {
    res.writeHead(status, { "Content-Type": "application/json", "Cache-Control": "no-store" });
    res.end(JSON.stringify(value));
  };

  async function control(req, res, pathname) {
    if (pathname === "/__test/state") {
      const counts = {};
      for (const p of state.log) counts[p] = (counts[p] ?? 0) + 1;
      return json(res, { log: state.log, counts, stalled: state.stalled.size });
    }
    if (req.method !== "POST") return json(res, { error: "POST expected" }, 405);
    const body = JSON.parse((await readBody(req)) || "{}");
    if (pathname === "/__test/reset") reset();
    else if (pathname === "/__test/clear-log") state.log = [];
    else if (pathname === "/__test/hook") {
      if (body.mode === "none") state.hooks.delete(body.path);
      else state.hooks.set(body.path, body);
    }
    else if (pathname === "/__test/refuse") state.refuse = Boolean(body.on);
    else if (pathname === "/__test/overlay") state.overlay = body.dir ? path.resolve(body.dir) : null;
    else return json(res, { error: "unknown control" }, 404);
    return json(res, { ok: true });
  }

  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, "http://localhost");
    const pathname = url.pathname;
    if (pathname.startsWith("/__test/")) return control(req, res, pathname);

    if (state.refuse) {
      state.log.push(pathname + " (refused)");
      return req.socket.destroy();
    }
    state.log.push(pathname);

    if (pathname === "/") {
      res.writeHead(302, { Location: prefix });
      return res.end();
    }
    if (!pathname.startsWith(prefix)) {
      res.writeHead(404);
      return res.end("not found");
    }
    const rel = decodeURIComponent(pathname.slice(prefix.length)) || "index.html";
    const hook = state.hooks.get(rel);
    if (hook?.mode === "abort") return req.socket.destroy();
    if (hook?.mode === "status") {
      res.writeHead(hook.status ?? 500);
      return res.end("hooked");
    }
    const file = resolveFile(rel);
    if (!file) {
      res.writeHead(404, { "Content-Type": "text/plain" });
      return res.end("not found");
    }
    let body = fs.readFileSync(file);
    if (hook?.mode === "corrupt") {
      body = Buffer.from(body);
      body[Math.min(100, body.length - 1)] ^= 0x01; // same length, different hash
    }
    res.writeHead(200, {
      "Content-Type": TYPES[path.extname(file)] ?? "application/octet-stream",
      "Content-Length": body.length,
      "Cache-Control": "no-cache",
    });
    if (hook?.mode === "stall") {
      // Send a first part, then hold the response open until the client leaves or reset().
      state.stalled.add(res);
      res.on("close", () => state.stalled.delete(res));
      return res.write(body.subarray(0, Math.min(1024, body.length)));
    }
    if (hook?.mode === "slow") {
      // Dribble the body out in 16 KB pieces, `ms` apart.
      let at = 0;
      const timer = setInterval(() => {
        if (res.destroyed || at >= body.length) {
          clearInterval(timer);
          if (!res.destroyed) res.end();
          return;
        }
        res.write(body.subarray(at, at + 16384));
        at += 16384;
      }, hook.ms ?? 50);
      res.on("close", () => clearInterval(timer));
      return;
    }
    res.end(body);
  });

  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, host, () => resolve({ server, port, reset }));
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const dist = process.env.E2E_DIST ?? path.resolve(here, "../../dist");
  const port = Number(process.env.E2E_PORT ?? 4318);
  if (!fs.existsSync(path.join(dist, "index.html")) || !fs.existsSync(path.join(dist, "content/manifest.json"))) {
    console.error(
      `\nNo built site at ${dist}.\nBuild it first from the repository root:\n  cargo xtask build-site --public-url /cast-iron/\n`,
    );
    process.exit(1);
  }
  await startServer({ dist, port });
  console.log(`serving ${dist} at http://127.0.0.1:${port}/cast-iron/`);
}
