// Playground server: a static file server for the in-browser WASM playground.
//
// The VM now runs entirely in the browser — the Rust core is compiled to
// WebAssembly and executes in-page. So the server's only job is to serve the
// frontend (`./public`) and the wasm-bindgen package (`./pkg`): no WebSocket,
// no server-side VM, no per-connection state. Execution, completion, and
// diagnostics all happen in the page through the shared Rust language core,
// which is exactly what lets the same engine power a future LSP or native GUI.
//
// Bun's HTML import runs the bundler, so the `<script type="module">` in
// index.html gets transpiled from TypeScript to JavaScript on the fly.
//
// Run with: `bun playground/server.ts`  (then open http://localhost:3000)

import { join, normalize } from "node:path";

const ROOT = import.meta.dir; // playground/
const PORT = Number(process.env.PORT ?? 3000);

const indexHtml = await Bun.file(join(ROOT, "public", "index.html")).text();

const server = Bun.serve({
  port: PORT,
  maxRequestBodySize: 4 * 1024 * 1024,
  routes: {
    "/": new Response(indexHtml, { headers: { "content-type": "text/html; charset=utf-8" } }),
  },
  fetch(req) {
    const url = new URL(req.url);
    return serveStatic(url.pathname);
  },
});

async function serveStatic(pathname: string): Promise<Response> {
  let rel: string;
  try {
    rel = decodeURIComponent(pathname);
  } catch {
    return new Response("Bad path", { status: 400 });
  }

  let base: string;
  let sub: string;
  if (rel.startsWith("/pkg/")) {
    base = join(ROOT, "pkg");
    sub = rel.slice("/pkg/".length);
  } else {
    base = join(ROOT, "public");
    sub = rel.replace(/^\/+/, "");
  }

  const file = normalize(join(base, sub));
  if (file !== base && !file.startsWith(base + "/")) {
    return new Response("Forbidden", { status: 403 });
  }

  const bunFile = Bun.file(file);
  if (!(await bunFile.exists())) {
    return new Response("Not found", { status: 404 });
  }
  return new Response(bunFile);
}

console.log(`napi-vm playground → http://localhost:${server.port}`);
