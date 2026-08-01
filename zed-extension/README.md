# napi-vm Zed launcher

This is intentionally a thin Zed extension. It starts the Node stdio server;
the server loads the native `napi-vm` addon and the workspace's optional
`.napi-vm.json` metadata manifest.

For local development, open this directory as a Zed extension project or use
Zed's local extension installation flow. In a workspace checkout the launcher
uses `lsp/server.cjs`; in a consumer project it falls back to
`node_modules/napi-vm/lsp/server.cjs`.

The JavaScript language remains Zed's normal JavaScript language, so its
Tree-sitter highlighting is preserved. This extension only contributes the
napi-vm language server.
