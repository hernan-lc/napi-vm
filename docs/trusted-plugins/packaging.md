# Packaging

Build explicitly before packaging. A deployable directory has plugin.json, plugin.lock.json, contracts, compiled entry/executable and declared assets/dependencies. The lock carries SHA256 file inventory, effective interface identities and artifact target/profile/runtime/ABI/availability. Hashes prove integrity only; they are not signatures or trust approvals.

The packer copies only declared content and an explicit namespaced include list. It refuses an existing destination, unsafe paths, escaped/broken symlinks and obvious secret/cache files. It does not scan unrelated directories or implicitly install anything. Production loading checks required files before executing code.

`node tools/plugins/stage-fixtures.mjs` stages in-repo controlled examples into independent artifacts. `npm run plugins:test:packaged` copies artifacts outside the repository under paths containing spaces and Unicode, executes both declared JS runtimes, checks tampered-byte rejection and exercises packed npm consumers without workspace links.

A native executable matches a concrete OS/architecture/libc. A Bun standalone binary embeds its runtime but is still target-specific. JS source imports, native dependencies and dynamic assets need actual relocated execution tests. Do not label unexecuted targets supported.
