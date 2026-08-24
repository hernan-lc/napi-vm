# Plugins

`plugins/` is a capability-based plugin host built on top of the VM. It lives
entirely outside the Rust interpreter: the core knows nothing about manifests,
globs, plugin roots or filesystems.

```text
plugin.json
     │
     ▼
PluginHost ── validates manifest, compiles permissions, resolves paths,
     │        installs napi:fs / napi:path, manages the lifecycle
     ▼
  napi-vm
     │
     ▼
 plugin.js
```

```bash
bun examples/plugins.ts
```

## Manifest

```json
{
  "name": "example-plugin",
  "version": "1.0.0",
  "apiVersion": 1,
  "entry": "./plugin.js",
  "permissions": {
    "fs": {
      "read": ["./config.json", "./assets/**"],
      "write": "./cache/**"
    },
    "path": true
  }
}
```

A manifest is validated when the plugin loads — name, version, `apiVersion`,
entry (which must stay inside the plugin directory) and every permission
pattern. Malformed patterns fail at load time, not on the first filesystem
call.

| Value | Meaning |
|-------|---------|
| missing / `false` | denied |
| `true` | same as `"*"` |
| `"*"` | requests unrestricted access |
| `"./assets/**"` | a plugin-relative subtree |
| `"/usr/share/app/**"` | an absolute path (host policy still decides) |
| `["./a", "./b/**"]` | any of several patterns |

Glob syntax is deliberately small: `*` matches within one path segment, `**`
matches zero or more segments. Everything else is literal.

## The manifest is a request, not a grant

```text
requested permissions  ∩  host policy  =  effective permissions
```

```ts
const host = new PluginHost({
  policy: {
    fs: {
      absoluteRead: false,   // may the plugin read outside its own root?
      absoluteWrite: false,  // may it write outside? dangerous
      deny: ["/etc/**"],     // always refused
      allow: ["/srv/data/**"], // when present, out-of-root paths must match
    },
  },
});
```

A plugin asking for `"read": "*"` gets unrestricted reads *inside its own
directory*; anything beyond that needs `absoluteRead`.

## Path resolution

`"./"` is always the plugin root — never `process.cwd()`. Every privileged call
resolves its path before checking anything:

```text
guest path → fold separators → resolve . and .. → resolve against plugin root
  → canonicalize (follow symlinks) → host policy → manifest permission → I/O
```

Because the check happens on the canonical path, `./cache/../../secret.txt` and
a `cache/outside -> /etc` symlink are both refused, and an in-root symlink is
matched by where it really points.

Guest paths are POSIX on every host; the host converts to native paths when it
performs I/O.

## Guest API

```js
import { readText, writeText, exists } from "napi:fs";
import { join, normalize, dirname, basename, extname } from "napi:path";
```

`napi:fs` is always registered — registering it grants nothing, since each
function checks its own path. `napi:path` is pure computation and is registered
only when the manifest asks for `"path": true`.

Denied calls raise a catchable error carrying no host paths:

```js
try {
  readText("./secret.txt");
} catch (error) {
  error.name;    // "PermissionDenied"
  error.message; // 'fs.read is not permitted for "./secret.txt"'
}
```

Guests never see `require`, `process`, `node:fs`, `Bun` or `Deno`.

## Entry module

The default export may be an object or a class; both are normalized to a single
instance.

```js
import { readText, writeText } from "napi:fs";
import { join } from "napi:path";

export default class ExamplePlugin {
  onLoad(context) {
    this.config = JSON.parse(readText("./config.json"));
    writeText(join("./cache", "status.json"), JSON.stringify({ plugin: context.name }));
  }

  onUnload(context) {
    return { config: this.config }; // serializable state, survives a reload
  }

  onReload(context, previousState) {
    this.config = previousState ? previousState.config : JSON.parse(readText("./config.json"));
  }
}
```

`context` carries `name` and `version` only — the plugin root is host-side
information and is deliberately withheld. `onUnload` also receives
`reason: "unload" | "reload"`. All three hooks are optional; a plugin without
`onReload` falls back to `onLoad`.

## Host API

```ts
const host = new PluginHost({ policy });

const plugin = host.load("./examples/plugins/example-plugin");
plugin.loadResult;      // whatever onLoad returned
plugin.status;          // "loaded" | "error"

host.reload("example-plugin"); // fresh VM, state handed to onReload
host.unload("example-plugin"); // returns onUnload's state
host.get("example-plugin");
host.list();
host.unloadAll();
```

Reload never mutates a half-loaded environment: the old instance gets
`onUnload({ reason: "reload" })`, its serializable return value is kept, the VM
is discarded, and everything is rebuilt from disk — so edited source *and*
edited permissions both take effect.

Unload calls `onUnload`, then detaches the capabilities: the modules and the
bridge globals are removed from the VM. A lifecycle hook that throws revokes
them immediately as well — an errored plugin never keeps a live `napi:fs` — and
a failed `onUnload` still unloads the plugin. The registry keeps the errored
entry so `reload()` can rebuild it from disk.

## Swapping the filesystem

Everything the host touches goes through one small interface, so the same
permission logic can sit on Node, Bun, Deno or a Rust backend without the
guest-facing API changing:

```ts
new PluginHost({
  policy,
  fs: { realpath, readText, writeText, exists },
});
```

## Layout

```text
plugins/
  index.ts                  public surface
  plugin-host.ts            load / reload / unload
  manifest.ts               plugin.json types and validation
  permissions.ts            compilation, policy intersection, enforcement
  path-rules.ts             guest path normalization and glob matching
  filesystem-capability.ts  napi:fs
  path-capability.ts        napi:path
  host-filesystem.ts        the swappable backend
  lifecycle.ts              guest-side bootstrap and hook wrappers
```

The capability installers use `exposeFunction` + `registerModule` so the host
runs against any published napi-vm build; `vm.registerHostModule()` (see the
[API reference](api.md)) is the newer core shortcut for the same shape.

Security regressions live in `tests/plugins/` — permissions, traversal,
symlink escapes, policy intersection, lifecycle and reload.
