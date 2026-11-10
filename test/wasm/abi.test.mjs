// WebAssembly ABI checks (docs/ABI.md). Needs a built module: `npm run build`, or
// `cargo build --release -p stapelwerk-wasm --target wasm32-unknown-unknown`.
// STAPELWERK_WASM=<path> selects a specific file; otherwise the newest of dist/ and target/ is used.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const CANDIDATES = [
  path.join(ROOT, 'dist', 'wasm', 'stapelwerk.wasm'),
  path.join(ROOT, 'target', 'wasm32-unknown-unknown', 'release', 'stapelwerk_wasm.wasm'),
];

function wasmPath() {
  if (process.env.STAPELWERK_WASM) return process.env.STAPELWERK_WASM;
  const found = CANDIDATES.filter((p) => existsSync(p)).sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  if (found.length === 0) {
    throw new Error('no stapelwerk.wasm found; run `npm run build` first');
  }
  return found[0];
}

const file = wasmPath();
const bytes = readFileSync(file);
const module = new WebAssembly.Module(bytes);
const documented = readFileSync(path.join(ROOT, 'docs', 'abi-exports.txt'), 'utf8')
  .split('\n')
  .map((l) => l.trim())
  .filter(Boolean);
// Linker-generated globals such as __data_end are not part of the ABI.
const actual = WebAssembly.Module.exports(module)
  .map((e) => e.name)
  .filter((n) => !n.startsWith('__'));

test('module has no imports', () => {
  assert.deepEqual(WebAssembly.Module.imports(module), []);
});

test('module is at most 300 KiB', () => {
  assert.ok(bytes.length <= 300 * 1024, `${bytes.length} bytes`);
});

test('abi_version() is 1', async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  assert.equal(instance.exports.abi_version(), 1);
});

test('exports memory', () => {
  assert.ok(WebAssembly.Module.exports(module).some((e) => e.name === 'memory' && e.kind === 'memory'));
});

test('docs/abi-exports.txt has no duplicates', () => {
  assert.equal(new Set(documented).size, documented.length);
});

test('every export is documented in docs/abi-exports.txt', () => {
  const undocumented = actual.filter((n) => !documented.includes(n));
  assert.deepEqual(undocumented, []);
});

// Enable (remove `skip`) when crates/wasm implements the full export list.
test('exports match docs/abi-exports.txt exactly', { skip: 'full ABI arrives in a later phase' }, () => {
  assert.deepEqual([...actual].sort(), [...documented].sort());
});
