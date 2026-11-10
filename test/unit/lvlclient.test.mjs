// Level client: overlapping loads of the same level share one handle.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createLevelClient } from '../../web/js/lvlclient.js';

function fakeVm() {
  const vm = {
    mock: false,
    loads: 0,
    loadLevel(text) {
      vm.loads += 1;
      return { info: () => ({ id: text.trim(), chapter: 0, order: 1, tests: [] }), section: () => '' };
    },
  };
  return vm;
}

function fakeFetch() {
  return async (url) => {
    const u = String(url);
    // Resolve on a later turn so two calls really overlap.
    await new Promise((r) => setTimeout(r, 1));
    if (u.endsWith('levels/index.json')) return { ok: true, json: async () => ({ levels: [{ id: 'a', chapter: 0, order: 1 }] }) };
    if (u.endsWith('levels/a.lvl')) return { ok: true, text: async () => 'a' };
    return { ok: false, status: 404 };
  };
}

test('two overlapping load(id) calls create one level handle', async () => {
  const vm = fakeVm();
  const client = createLevelClient(vm, { base: 'http://x/', fetchFn: fakeFetch() });
  const [a, b] = await Promise.all([client.load('a'), client.load('a')]);
  assert.equal(vm.loads, 1);
  assert.equal(a, b);
  assert.equal(await client.load('a'), a);
  assert.equal(vm.loads, 1);
});

test('an unknown level is not cached as a failure forever', async () => {
  const vm = fakeVm();
  const client = createLevelClient(vm, { base: 'http://x/', fetchFn: fakeFetch() });
  assert.equal(await client.load('nope'), null);
  assert.equal(await client.load('nope'), null);
  assert.equal(vm.loads, 0);
});
