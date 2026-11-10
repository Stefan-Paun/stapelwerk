// npm run check: runs everything CI runs, in the same order, and prints a summary.
// Exit code 0 only if every step passed (skipped steps are reported, not failed).
import { existsSync, readFileSync, statSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { ROOT, WASM_DIST, WASM_MAX_BYTES, capture, cargoHint, findFiles, run } from './lib.mjs';

const results = [];

function step(name, fn) {
  console.log(`\n=== ${name}`);
  let r;
  try {
    r = fn();
  } catch (e) {
    r = { ok: false, note: e.message };
  }
  if (typeof r === 'number') r = r === 0 ? { ok: true } : { ok: false, note: r === 127 ? cargoHint() : `exit code ${r}` };
  results.push({ name, ...r });
  if (r.skip) console.log(`skipped: ${r.skip}`);
  else if (!r.ok) console.log(`FAILED${r.note ? `: ${r.note}` : ''}`);
}

step('cargo fmt --check', () => run('cargo', ['fmt', '--all', '--check']));
step('cargo clippy -D warnings', () =>
  run('cargo', ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']),
);
step('cargo test', () => run('cargo', ['test', '--workspace', '--locked']));
step('build (wasm + dist)', () => run(process.execPath, [path.join('tools', 'build.mjs')]));

step('check-levels --strict --mutate', () =>
  run('cargo', ['run', '-q', '--release', '--locked', '-p', 'stapelwerk-cli', '--', 'check-levels', '--strict', '--mutate', 'levels']),
);

step('isa-table matches docs/isa-table.md', () => {
  const r = capture('cargo', ['run', '-q', '--locked', '-p', 'stapelwerk-cli', '--', 'isa-table']);
  if (r.code !== 0) return { ok: false, note: r.stderr.trim() || `exit code ${r.code}` };
  const doc = readFileSync(path.join(ROOT, 'docs', 'isa-table.md'), 'utf8');
  return r.stdout === doc
    ? { ok: true }
    : { ok: false, note: 'stale; run: cargo run -q -p stapelwerk-cli -- isa-table > docs/isa-table.md' };
});

step('zero third-party crates (Cargo.lock)', () => {
  const lock = readFileSync(path.join(ROOT, 'Cargo.lock'), 'utf8');
  const n = lock.split('\n').filter((l) => l.startsWith('source = ')).length;
  return n === 0 ? { ok: true } : { ok: false, note: `${n} 'source = ' lines` };
});

step('no npm dependencies (package.json)', () => {
  const pkg = JSON.parse(readFileSync(path.join(ROOT, 'package.json'), 'utf8'));
  const keys = ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies'].filter((k) => pkg[k]);
  return keys.length === 0 ? { ok: true } : { ok: false, note: `package.json has ${keys.join(', ')}` };
});

step(`wasm size <= ${WASM_MAX_BYTES / 1024} KiB`, () => {
  if (!existsSync(WASM_DIST)) return { ok: false, note: 'dist/wasm/stapelwerk.wasm missing' };
  const size = statSync(WASM_DIST).size;
  console.log(`${size} bytes`);
  return size <= WASM_MAX_BYTES ? { ok: true } : { ok: false, note: `${size} bytes` };
});

step('no private paths or e-mail addresses', () => {
  // Only files that belong to the repository: tracked plus untracked-but-not-ignored.
  const listed = capture('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard']);
  if (listed.code !== 0) return { skip: 'git is not available' };
  const textExt = /\.(rs|toml|md|txt|mjs|js|json|yml|yaml|html|css|svg|lvl|s|webmanifest)$/i;
  const home = os.homedir();
  const userDir = /\/(Users|home)\/[A-Za-z0-9._-]+\//;
  const email = /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}/g;
  const hits = [];
  for (const rel of listed.stdout.split('\0').filter(Boolean).sort()) {
    const name = path.basename(rel);
    if (!(textExt.test(name) || name.startsWith('.git') || name === 'LICENSE')) continue;
    const p = path.join(ROOT, rel);
    if (!existsSync(p)) continue;
    const text = readFileSync(p, 'utf8');
    if (text.includes(home) || userDir.test(text)) hits.push(`${rel}: absolute home path`);
    for (const m of text.matchAll(email)) {
      // Ignore asset names with a high-DPI suffix (name, at sign, 2x, .png).
      if (/\.(png|svg|jpe?g|gif|webp|avif|woff2?|css|m?js|json|html)$/i.test(m[0])) continue;
      hits.push(`${rel}: e-mail-like text "${m[0]}"`);
      break;
    }
  }
  if (hits.length) console.log(hits.join('\n'));
  return hits.length === 0 ? { ok: true } : { ok: false, note: `${hits.length} finding(s)` };
});

step('node --test (unit, wasm, site)', () => {
  const files = ['unit', 'wasm', 'site'].flatMap((d) => findFiles(path.join(ROOT, 'test', d), '.test.mjs'));
  if (files.length === 0) return { skip: 'no Node tests' };
  return run(process.execPath, ['--test', ...files]);
});

step('browser tests', () => {
  if (findFiles(path.join(ROOT, 'test', 'browser'), '.test.mjs').length === 0) return { skip: 'no browser tests yet' };
  return run(process.execPath, [path.join('tools', 'test-browser.mjs')]);
});

console.log('\n=== Summary');
let failed = 0;
for (const r of results) {
  const tag = r.skip ? 'SKIP' : r.ok ? ' OK ' : 'FAIL';
  if (!r.skip && !r.ok) failed++;
  console.log(`[${tag}] ${r.name}${r.skip ? ` (${r.skip})` : !r.ok && r.note ? ` (${r.note})` : ''}`);
}
console.log(failed === 0 ? '\nAll checks passed.' : `\n${failed} check(s) failed.`);
process.exit(failed === 0 ? 0 : 1);
