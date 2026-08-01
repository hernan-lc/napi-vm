# napi-vm Zed launcher

This is intentionally a thin Zed extension. It starts the Node stdio server;
the server loads the native `napi-vm` addon and watches the workspace's live
`.napi-vm/runtime.json` locator when a `VmSession` is running. The optional
`.napi-vm.json` metadata manifest remains a static fallback.

For local development, open this directory as a Zed extension project or use
Zed's local extension installation flow. In a workspace checkout the launcher
uses `lsp/server.cjs`; in a consumer project it falls back to
`node_modules/napi-vm/lsp/server.cjs`.

The JavaScript language remains Zed's normal JavaScript language, so its
Tree-sitter highlighting is preserved. This extension only contributes the
napi-vm language server. Start `bun examples/hotreload.ts` from the workspace
to publish live host functions and module exports for completion and hover.
