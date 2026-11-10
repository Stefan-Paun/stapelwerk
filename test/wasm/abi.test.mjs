// WebAssembly ABI checks on the raw module (docs/ABI.md). Needs a built module: `npm run build`,
// or `cargo build --release -p stapelwerk-wasm --target wasm32-unknown-unknown`.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { ROOT, wasmPath } from './helpers.mjs';

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

test('module is at most 300 KiB', (t) => {
  t.diagnostic(`${path.relative(ROOT, file)}: ${bytes.length} bytes (${(bytes.length / 1024).toFixed(1)} KiB)`);
  assert.ok(bytes.length <= 300 * 1024, `${bytes.length} bytes`);
});

test('module contains no local paths', () => {
  for (const needle of ['/Users/', '/home/']) {
    assert.equal(bytes.includes(Buffer.from(needle)), false, `found ${needle}`);
  }
});

test('abi_version() is 1', async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  assert.equal(instance.exports.abi_version(), 1);
});

test('exports memory', () => {
  assert.ok(WebAssembly.Module.exports(module).some((e) => e.name === 'memory' && e.kind === 'memory'));
});

test('every other export is a function', () => {
  for (const e of WebAssembly.Module.exports(module)) {
    if (e.name === 'memory' || e.name.startsWith('__')) continue;
    assert.equal(e.kind, 'function', e.name);
  }
});

test('docs/abi-exports.txt has no duplicates', () => {
  assert.equal(new Set(documented).size, documented.length);
});

test('every export is documented in docs/abi-exports.txt', () => {
  const undocumented = actual.filter((n) => !documented.includes(n));
  assert.deepEqual(undocumented, []);
});

test('exports match docs/abi-exports.txt exactly', () => {
  assert.deepEqual([...actual].sort(), [...documented].sort());
});

test('raw calls: handle errors and the output buffer', async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const x = instance.exports;
  const ERR = 0xffffffff;
  const text = (len) => new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.out_ptr() >>> 0, len >>> 0));
  assert.equal(x.sess_step(12345, 1) >>> 0, ERR);
  assert.equal(x.sess_view(0) >>> 0, ERR);
  assert.equal(text(x.sess_fault(12345)), '{"error":"abi.bad_handle"}');
  assert.equal(x.out_len() >>> 0, 26);
  assert.equal(x.buf_alloc(0), 0);
  const s = x.sess_new(0) >>> 0;
  assert.ok(s !== 0 && s !== ERR);
  assert.equal(text(x.level_info(s)), '{"error":"abi.bad_handle"}', 'a session is not a level');
  assert.equal(x.sess_reset(s, -1) >>> 0, 6, 'NOT_ASSEMBLED');
  assert.equal(x.sess_reset(s, 0) >>> 0, ERR, 'no test 0 in the sandbox');
  assert.equal(text(x.sess_run_tests(s)), '{"error":"abi.not_assembled"}');
  assert.equal(x.sess_assemble(s, 999_999_999, 4) >>> 0, ERR, 'not an input buffer');
  assert.equal(text(x.out_len()), '{"error":"abi.bad_arg"}');
  const view = new Uint32Array(x.memory.buffer, x.sess_view(s) >>> 0, 24);
  assert.equal(view[0], 6);
  assert.equal(view[22] | 0, -1, 'test_index');
  x.sess_free(s);
  assert.equal(x.sess_step(s, 1) >>> 0, ERR, 'freed handle');
  const s2 = x.sess_new(0) >>> 0;
  assert.equal(s2 & 0xffff, s & 0xffff, 'slot reused');
  assert.notEqual(s2, s, 'new generation');
  assert.equal(x.sess_step(s, 1) >>> 0, ERR);
  assert.equal(text(x.segments()).slice(0, 8), '[{"name"');
});
