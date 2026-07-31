// Playground server: a static file server for the in-browser WASM playground.
//
// The VM now runs entirely in the browser — the Rust core is compiled to
// WebAssembly and executes in-page. So the server's only job is to serve the
// frontend (`./public`) and the wasm-bindgen package (`./pkg`): no WebSocket,
// no server-side VM, no per-connection state. Execution, completion, and
// diagnostics all happen in the page through the shared Rust language core,
// which is exactly what lets the same engine power a future LSP or native GUI.
//
// Run with: `bun playground/server.ts`  (then open http://localhost:3000)

import { join, normalize } from "node:path";

const ROOT = import.meta.dir; // playground/
const PORT = Number(process.env.PORT ?? 3000);

const server = Bun.serve({
  port: PORT,
  // Keep a large pasted script from blowing up server memory.
  maxRequestBodySize: 4 * 1024 * 1024,
  fetch(req) {
    const url = new URL(req.url);
    return serveStatic(url.pathname);
  },
});

/**
 * Serve a file with path-traversal protection. `/pkg/*` maps to the wasm
// package; every other path maps to `./public`.
 */
async function serveStatic(pathname: string): Promise<Response> {
  let rel: string;
  try {
    rel = decodeURIComponent(pathname);
  } catch {
    return new Response("Bad path", { status: 400 });
  }
  if (rel === "/" || rel === "") rel = "/index.html";

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
  // Bun infers Content-Type from the extension — notably `application/wasm`
  // for the module, which lets the browser stream-compile it.
  return new Response(bunFile);
}

console.log(`napi-vm playground → http://localhost:${server.port}`);
