import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { capture } from '../capture.mjs';

const script = fileURLToPath(new URL('../test-js.mjs', import.meta.url));
for (const runner of ['node', 'bun']) {
  test(`portable plugin runner discovers all suites with ${runner} and propagates failures`, async t => {
    if (runner === 'bun') {
      try { await capture('bun', ['--version']); }
      catch { t.skip('Bun is not installed'); return; }
    }
    const root = await mkdtemp(join(tmpdir(), 'plugin test discovery '));
    const dirs = ['packages/plugin-fixture/test', 'tools/plugins/test', 'examples/trusted/fixture/test'];
    const env = { ...process.env };
    // The child is an independent test harness, not a recursive Node worker.
    delete env.NODE_TEST_CONTEXT;
    try {
      for (const [index, dir] of dirs.entries()) {
        await mkdir(join(root, dir), { recursive: true });
        await writeFile(join(root, dir, 'discovery.test.mjs'),
          `import test from 'node:test'; test('discovered suite ${index}', () => {});\n`);
      }
      // Launch without a shell, as Windows npm does: wildcard expansion must
      // come from the wrapper, including paths under a temporary dir with spaces.
      const result = await capture('node', [script, runner], { cwd: root, env });
      for (const index of dirs.keys()) {
        assert.match(result.stdout + result.stderr, new RegExp(`discovered suite ${index}`));
      }
      await writeFile(join(root, dirs[0], 'discovery.test.mjs'), "throw new Error('fixture failure');\n");
      await assert.rejects(capture('node', [script, runner], { cwd: root, env }), /fixture failure/);
      await rm(join(root, dirs[2], 'discovery.test.mjs'));
      await assert.rejects(capture('node', [script, runner], { cwd: root, env }), /No plugin tests found/);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
}
