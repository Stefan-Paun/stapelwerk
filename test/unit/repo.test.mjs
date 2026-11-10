// Repository rules that are cheap to check without building anything.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const read = (p) => readFileSync(path.join(ROOT, p), 'utf8');

test('package.json has no dependencies and requires Node >= 22', () => {
  const pkg = JSON.parse(read('package.json'));
  for (const k of ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']) {
    assert.equal(pkg[k], undefined, k);
  }
  assert.equal(pkg.private, true);
  assert.equal(pkg.type, 'module');
  assert.equal(pkg.engines.node, '>=22');
  for (const s of ['build', 'start', 'test', 'check', 'test:browser']) assert.ok(pkg.scripts[s], s);
});

test('Cargo.lock contains no third-party crates', () => {
  const lines = read('Cargo.lock').split('\n');
  assert.equal(lines.filter((l) => l.startsWith('source = ')).length, 0);
});

test('Rust toolchain is pinned', () => {
  assert.match(read('rust-toolchain.toml'), /channel = "1\.99\.1"/);
  assert.match(read('rust-toolchain.toml'), /wasm32-unknown-unknown/);
});

test('docs/ERRORS.md lists every fault key once', () => {
  const keys = read('docs/ERRORS.md')
    .split('\n')
    .map((l) => l.match(/^- `(fault\.[a-z_]+)`/))
    .filter(Boolean)
    .map((m) => m[1]);
  assert.equal(keys.length, 23);
  assert.equal(new Set(keys).size, 23);
});
