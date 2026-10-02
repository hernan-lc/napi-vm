# @napi-vm/plugin-host

Launch and supervise trusted JavaScript process plugins under Node or Bun.

Requires Node.js 22 or newer. Package version: `0.1.0`; wire protocol: `1.0`.

Trusted process plugins execute with the launching user’s OS privileges. They are **not sandboxed**. The protocol token authenticates the session; it does not restrict filesystem, network or process access.

```js
import { TrustedPluginHost } from "@napi-vm/plugin-host";
const host = new TrustedPluginHost();
await host.shutdown();
```

The JavaScript host supports JavaScript plugins under Node and Bun, not native executable plugins. Production loads verify package integrity by default. The host owns its direct child only; process-tree containment requires OS facilities. Native executable loading is available through the independent Rust host crate.

See the [trusted plugin documentation](https://github.com/nglmercer/napi-vm/tree/main/docs/trusted-plugins) for contracts, lifecycle and platform limits. Licensed under MIT; see LICENSE.
