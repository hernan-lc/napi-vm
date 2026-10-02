# @napi-vm/plugin-cli

Author, validate, build and package trusted process plugins.

Requires Node.js 22 or newer. Package version: `0.1.0`; wire protocol: `1.0`.

Trusted process plugins execute with the launching user’s OS privileges. They are **not sandboxed**. The protocol token authenticates the session; it does not restrict filesystem, network or process access.

```sh
napi-vm-plugin create ./my-plugin --language ts
cd my-plugin
npm install
npm run typecheck
npm run build
npm run test:artifact -- --runtime node
```

Scaffolds TS, JS and Rust templates with licensed local SDK assets. Use `napi-vm-plugin help` for validation, inspection, code generation, packaging and development commands. The CLI is authoring tooling: production Rust hosts loading packaged native executables do not require Node, Bun, npm or this package.

See the [trusted plugin documentation](https://github.com/nglmercer/napi-vm/tree/main/docs/trusted-plugins) for contracts, lifecycle and platform limits. Licensed under MIT; see LICENSE.
