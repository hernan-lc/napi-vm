# @napi-vm/plugin-protocol

Framed JSON protocol and strict contracts for trusted process plugins.

Requires Node.js 22 or newer. Package version: `0.1.0`; wire protocol: `1.0`.

Trusted process plugins execute with the launching user’s OS privileges. They are **not sandboxed**. The protocol token authenticates the session; it does not restrict filesystem, network or process access.

```js
import { PluginError, validate } from "@napi-vm/plugin-protocol";
```

Exports framing, request correlation, contract validation and structured errors. Cancellation is cooperative and does not undo external side effects.

See the [trusted plugin documentation](https://github.com/nglmercer/napi-vm/tree/main/docs/trusted-plugins) for contracts, lifecycle and platform limits. Licensed under MIT; see LICENSE.
