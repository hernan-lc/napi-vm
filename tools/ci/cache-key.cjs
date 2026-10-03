const { createHash } = require('node:crypto');
const { execFileSync } = require('node:child_process');
const { appendFileSync } = require('node:fs');

// Include the compiler commit (also for nightly), native target settings and
// profile overrides. Debug PDB builds must not reuse non-debug cache entries.
const settings = Object.keys(process.env)
  .filter(key => /^(RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|CARGO_PROFILE_|CARGO_TARGET_)/.test(key))
  .sort()
  .map(key => [key, process.env[key]]);
const key = createHash('sha256')
  .update(execFileSync('rustc', ['-vV']))
  .update(JSON.stringify(settings))
  .digest('hex').slice(0, 20);
appendFileSync(process.env.GITHUB_OUTPUT, `key=${key}\n`);
