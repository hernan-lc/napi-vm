import { globSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const runner = process.argv[2];
if (!['node', 'bun'].includes(runner)) {
  throw new Error('Usage: node tools/plugins/test-js.mjs node|bun');
}

// npm uses cmd.exe on Windows, which passes wildcards literally. Resolve the
// same suites here and pass explicit paths to both runtimes without a shell.
const patterns = [
  'packages/plugin-*/test/*.test.mjs',
  'tools/plugins/test/*.test.mjs',
  'examples/trusted/*/test/*.test.mjs',
];
const files = patterns.flatMap(pattern => {
  const matches = globSync(pattern);
  if (matches.length === 0) throw new Error(`No plugin tests found: ${pattern}`);
  // Bun treats ./ paths as file paths instead of substring filters.
  return matches.map(file => `./${file.replaceAll('\\', '/')}`);
}).sort();
const result = spawnSync(runner === 'node' ? process.execPath : 'bun',
  runner === 'node' ? ['--test', ...files] : ['test', '--timeout', '30000', ...files],
  { stdio: 'inherit', shell: false });
if (result.error) throw result.error;
process.exit(result.status ?? 1);
