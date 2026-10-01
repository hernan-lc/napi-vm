import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const result = spawnSync(process.execPath, [fileURLToPath(import.meta.resolve('typescript/bin/tsc')), '-p', 'tsconfig.json'], { cwd: new URL('.', import.meta.url), stdio: 'inherit', shell: false });
if (result.status !== 0) process.exit(result.status ?? 1);
