# Template and CLI verification — 2026-10-01

Environment: Linux x64 / GNU libc; Node 24.19.0; Bun 1.4.2; TypeScript 6.0.3; Rust/Cargo 1.98.1. These results do not establish native macOS or Windows execution support.

## Self-contained TS and JS projects

`node packages/plugin-cli/scripts/test-templates.mjs --keep` passed for both newly created languages. Each project received five locally vendored package payloads; no dependency was manually copied into `node_modules`, and no parent `main()` call replaced an installed CLI command.

Executed in each project:

- `npm install --offline --ignore-scripts --no-audit --no-fund`
- `npm run typecheck`
- `npm run build`
- `npm run test:node`
- `npm run test:artifact -- --runtime node`
- `npm run dev -- --runtime node`, then source edit, successful rebuild/reload and Ctrl-C
- `bun install --offline --ignore-scripts`, migrating the existing npm lock
- `bun run typecheck`
- `bun test`
- `bun run build`
- `bun run test:artifact --runtime bun`
- `bun run dev --runtime bun`, then source edit, successful rebuild/reload and Ctrl-C

Bun's install reused the explicit npm installation and migrated its lock. This does not claim a fresh Bun-only cache bootstrap. Public TypeScript/type packages came from the available npm cache; unpublished `@napi-vm/*` dependencies resolved through project-local `file:` paths.

[Full developer workflow log](template-workflows-2026-10-01.log)

## Final synchronized Rust template

The standalone project was created at `/tmp/napi-rust-only-jcAX2S/plugin`. Its vendored protocol/SDK sources and Cargo manifests were byte-compared with the final repository sources before verification. The generated greeter binding also matched. Existing compilation caches were reused; no placeholder implementation replaced the native plugin.

The environment explicitly excluded Node and Bun:

```sh
PATH=/workspace/shared/ugreen-toolchain/cargo/bin:/usr/bin:/bin
CARGO_HOME=/workspace/shared/ugreen-toolchain/cargo
RUSTUP_HOME=/workspace/shared/ugreen-toolchain/rustup
CARGO_BUILD_JOBS=1
CARGO_TARGET_DIR=/workspace/scratch/461b26a0463e/napi-vm-portable-rebuild/target
```

The driver asserted that neither `command -v node` nor `command -v bun` succeeded, then executed:

```sh
cargo check --locked --manifest-path /tmp/napi-rust-only-jcAX2S/plugin/Cargo.toml
cargo test --locked --manifest-path /tmp/napi-rust-only-jcAX2S/plugin/Cargo.toml
cargo build --locked --release --manifest-path /tmp/napi-rust-only-jcAX2S/plugin/Cargo.toml
```

All three commands passed. Two unit tests cover a Unicode greeting and a generated-contract SDK harness invocation. The final optimized build completed in 5m03s after earlier interrupted compilation had populated part of the cache.

[Final Cargo output](rust-template-cargo-2026-10-01.log)

## Actual native artifact and development adapter

With the explicit optional Node CLI and prebuilt `target/debug/trusted-host-rust` adapter:

```sh
node packages/plugin-cli/src/index.mjs build "$PROJECT" --format executable
node packages/plugin-cli/src/index.mjs test "$PROJECT" --artifact --host-executable "$HOST"
node packages/plugin-cli/src/index.mjs pack "$PROJECT" --out "$PACKAGE_ROOT/package"
node packages/plugin-cli/src/index.mjs invoke "$PACKAGE_ROOT/package" \
  --host-executable "$HOST" --interface example.greeter --method greet --input '{"name":"Ana"}'
node packages/plugin-cli/src/index.mjs dev "$PROJECT" --host-executable "$HOST"
```

The artifact scenario and relocated invocation returned `{"message":"Hola, Ana"}`. Native dev reached readiness, rebuilt after editing `src/lib.rs`, reloaded successfully and completed Ctrl-C cleanup with `{"stopped":true}`. The warm source-edit build took 1m15s. The original source was restored afterward.

Native dev now checksum-packs each private launch stage before passing it to the Rust host. It retains ordinary production integrity verification rather than disabling verification through an environment variable. A focused regression checks that native stages have valid inventories before host startup.

[Artifact and invocation log](rust-template-artifact-2026-10-01.log) · [Native dev log](rust-template-dev-2026-10-01.log)

## Affected unit checks and chronology

After the native-dev packing fix:

```sh
node --test packages/plugin-codegen/test/*.test.mjs packages/plugin-cli/test/*.test.mjs
```

Result: **17 passed, 0 failed**. [Unit output](codegen-cli-2026-10-01.log)

After this CLI-only native-dev fix, the complete Node and Bun aggregates were rerun: each passes 111/111 with no skips. See [Node output](node-aggregate-2026-10-01.log) and [Bun output](bun-aggregate-2026-10-01.log). The actual native artifact/dev workflows above separately validate the executable adapter.
