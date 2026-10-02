# @napi-vm/plugin-codegen

Generate deterministic TypeScript and Rust bindings for trusted plugin contracts.

Requires Node.js 22 or newer. Package version: `0.1.0`; wire protocol: `1.0`.

Trusted process plugins execute with the launching user’s OS privileges. They are **not sandboxed**. The protocol token authenticates the session; it does not restrict filesystem, network or process access.

```sh
napi-vm-plugin-codegen path/to/example.interface.json generated
```

Generated bindings use `@napi-vm/plugin-protocol` and `@napi-vm/plugin-sdk`. Descriptor and schema inputs must satisfy the strict supported contract profile. Outputs contain no timestamps or checkout paths.

See the [trusted plugin documentation](https://github.com/nglmercer/napi-vm/tree/main/docs/trusted-plugins) for contracts, lifecycle and platform limits. Licensed under MIT; see LICENSE.
