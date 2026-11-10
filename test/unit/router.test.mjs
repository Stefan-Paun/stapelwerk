import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseHash, formatRoute } from '../../web/js/router.js';

test('known routes', () => {
  assert.equal(parseHash('').name, 'home');
  assert.equal(parseHash('#/').name, 'home');
  assert.deepEqual(parseHash('#/level/2-2-bis-zur-null').params, { id: '2-2-bis-zur-null' });
  assert.deepEqual(parseHash('#/manual/stack').params, { topic: 'stack' });
  assert.deepEqual(parseHash('#/manual').params, { topic: null });
  assert.equal(parseHash('#/sandbox').name, 'sandbox');
  assert.equal(parseHash('#/sandbox?code=v1:abc').query.get('code'), 'v1:abc');
  assert.equal(parseHash('#/about').name, 'about');
});

test('unknown or malformed routes are notfound', () => {
  for (const h of ['#/level', '#/level/', '#/level/Böse', '#/level/a/b', '#/level/%E0%A4%A', '#/handbuch', '#/manual/<script>', '#/about/x', '#/sandbox/x', '#nope']) {
    assert.equal(parseHash(h).name, 'notfound', h);
  }
});

test('format and parse round-trip', () => {
  for (const [name, params] of [['home', {}], ['level', { id: '6-1-fakultaet' }], ['manual', { topic: 'heap' }], ['manual', {}], ['sandbox', {}], ['about', {}]]) {
    const r = parseHash(formatRoute(name, params));
    assert.equal(r.name, name);
    if (params.id) assert.equal(r.params.id, params.id);
    if (params.topic) assert.equal(r.params.topic, params.topic);
  }
  assert.equal(formatRoute('sandbox', {}, { code: 'v1:xy' }), '#/sandbox?code=v1%3Axy');
  assert.throws(() => formatRoute('nowhere'));
});
