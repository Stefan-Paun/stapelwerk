// Builds the static site into dist/:
//   dist/wasm/stapelwerk.wasm  (cargo release build for wasm32, paths remapped)
//   dist/...                   (copy of web/, if it exists)
//   dist/levels/               (`stapelwerk pack-levels levels dist/levels`: packed .lvl + index.json)
// Later phases add: dist/en/index.html (from the German shell) and dist/sw.js (precache list).
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { ROOT, WASM_DIST, WASM_MAX_BYTES, WASM_RAW, WASM_TARGET, cargoHint, run } from './lib.mjs';

const dist = path.join(ROOT, 'dist');

function fail(msg) {
  console.error(`build: ${msg}`);
  process.exit(1);
}

// 1. WebAssembly. Remap absolute paths so no local directory names end up in the binary.
//    The repository root comes last because it lies inside HOME and the last matching prefix wins.
const flags = [];
if (process.env.CARGO_ENCODED_RUSTFLAGS) flags.push(...process.env.CARGO_ENCODED_RUSTFLAGS.split('\x1f').filter(Boolean));
else if (process.env.RUSTFLAGS) flags.push(...process.env.RUSTFLAGS.split(' ').filter(Boolean));
flags.push(`--remap-path-prefix=${os.homedir()}=~`, `--remap-path-prefix=${ROOT}=.`);
const env = { ...process.env, CARGO_ENCODED_RUSTFLAGS: flags.join('\x1f') };
delete env.RUSTFLAGS;

console.log('build: cargo build --release -p stapelwerk-wasm --target ' + WASM_TARGET);
const code = run('cargo', ['build', '--release', '--locked', '-p', 'stapelwerk-wasm', '--target', WASM_TARGET], { env });
if (code === 127) fail(cargoHint());
if (code !== 0) fail('cargo build failed');

const wasm = readFileSync(WASM_RAW);
if (wasm.length > WASM_MAX_BYTES) fail(`stapelwerk.wasm is ${wasm.length} bytes, limit ${WASM_MAX_BYTES}`);
for (const secret of [os.homedir(), ROOT, '/Users/', '/home/']) {
  if (secret.length > 1 && wasm.includes(Buffer.from(secret))) fail(`stapelwerk.wasm contains a local path (${secret.startsWith('/') && secret.length < 8 ? secret : 'home or repository directory'})`);
}

// 2. Fresh dist/.
rmSync(dist, { recursive: true, force: true });
mkdirSync(path.dirname(WASM_DIST), { recursive: true });

// 3. Static web files (owned by the UI; may not exist yet).
const web = path.join(ROOT, 'web');
if (existsSync(web)) {
  cpSync(web, dist, { recursive: true });
  console.log('build: copied web/');
} else {
  console.log('build: web/ does not exist yet, skipped');
}
copyFileSync(WASM_RAW, WASM_DIST);
console.log(`build: dist/wasm/stapelwerk.wasm (${wasm.length} bytes)`);

// 4. Levels.
if (existsSync(path.join(ROOT, 'levels', 'manifest.txt'))) {
  const c = run('cargo', ['run', '-q', '--release', '--locked', '-p', 'stapelwerk-cli', '--', 'pack-levels', 'levels', path.join('dist', 'levels')]);
  if (c === 127) fail(cargoHint());
  if (c !== 0) fail('pack-levels failed');
  if (!existsSync(path.join(dist, 'levels', 'index.json'))) fail('pack-levels wrote no dist/levels/index.json');
  console.log('build: packed levels into dist/levels/');
} else {
  console.log('build: levels/ does not exist yet, skipped');
}

console.log('build: done');
