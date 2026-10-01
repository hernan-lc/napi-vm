# Trusted greeter

This process has your account's OS privileges. The creator copies the five installed local SDK/tooling packages into `vendor/` and rewrites their dependencies to relative `file:` paths. No unpublished package is fetched. Install the published TypeScript/type dependencies explicitly with npm or Bun.

Bun-first: `bun install`, `bun run dev`, `bun run typecheck`, `bun test`, `bun run build`, `bun run test:artifact`.

Node: `npm install`, `npm run typecheck`, `npm run build`, `npm run test:node`, `napi-vm-plugin test . --artifact --runtime node`.

Production loading runs existing emitted JS. It never installs or builds. `napi-vm-plugin pack . --out ../greeter-package` creates a checksummed relocatable directory; dependency links stay inside the project and are materialized into the package. Loading never installs dependencies.
