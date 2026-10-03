// setup-node's Unix distributions include headers beside bin/node. Selecting
// them explicitly prevents macOS fixture tests from silently skipping.
const {existsSync, appendFileSync} = require('node:fs');
const {dirname, resolve, join} = require('node:path');
const include = resolve(dirname(process.execPath), '../include/node');
if (!existsSync(join(include, 'node_api.h'))) {
  throw new Error(`Node headers missing from the selected toolchain: ${include}`);
}
if (!process.env.GITHUB_ENV) throw new Error('GITHUB_ENV is required');
appendFileSync(process.env.GITHUB_ENV, `NODE_INCLUDE_DIR=${include}\n`);
console.log(`Node fixture headers: ${include}`);
