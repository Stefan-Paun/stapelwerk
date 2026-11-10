import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createProgress, validate, STORAGE_KEY } from '../../web/js/progress.js';

function memoryStorage(initial = {}) {
  const map = new Map(Object.entries(initial));
  return { getItem: (k) => (map.has(k) ? map.get(k) : null), setItem: (k, v) => map.set(k, String(v)), removeItem: (k) => map.delete(k), map };
}

const LEVELS = [
  { id: '0-1-a', chapter: 0, order: 1, kind: 'tutorial' },
  { id: '0-2-b', chapter: 0, order: 2, kind: 'normal' },
  { id: '1-1-c', chapter: 1, order: 1, kind: 'normal' },
  { id: '1-2-d', chapter: 1, order: 2, kind: 'bonus' },
  { id: '2-1-e', chapter: 2, order: 1, kind: 'normal' },
];

test('empty storage loads empty progress', () => {
  const p = createProgress(memoryStorage());
  p.load();
  assert.equal(p.level('0-1-a').solved, false);
  assert.equal(p.damaged, false);
});

test('damaged storage is ignored and flagged', () => {
  for (const bad of ['{nope', '[]', '{"v":99,"levels":{}}', '{"v":1,"levels":[]}']) {
    const p = createProgress(memoryStorage({ [STORAGE_KEY]: bad }));
    p.load();
    assert.equal(p.damaged, true, bad);
    assert.deepEqual(p.data.levels, {});
  }
});

test('storage that throws does not break anything', () => {
  const throwing = { getItem() { throw new Error('blocked'); }, setItem() { throw new Error('blocked'); }, removeItem() { throw new Error('blocked'); } };
  const p = createProgress(throwing);
  p.load();
  p.saveCode('0-1-a', 'halt');
  p.clear();
  assert.equal(p.level('0-1-a').solved, false);
});

test('markSolved keeps bests and badges', () => {
  const s = memoryStorage();
  const p = createProgress(s);
  p.load();
  p.markSolved('0-2-b', { cycles_avg: 50, size: 10 }, { cycles: 40, size: 10 });
  let r = p.level('0-2-b');
  assert.deepEqual(r.best, { cycles: 50, size: 10 });
  assert.deepEqual(r.badges, { compact: true, fast: false });
  p.markSolved('0-2-b', { cycles_avg: 39, size: 12 }, { cycles: 40, size: 10 });
  r = p.level('0-2-b');
  assert.deepEqual(r.best, { cycles: 39, size: 10 });
  assert.deepEqual(r.badges, { compact: true, fast: true });
  const again = createProgress(s);
  again.load();
  assert.equal(again.level('0-2-b').solved, true);
});

test('unlock rules: chapters in order, first level opens the rest, bonus not required', () => {
  const p = createProgress(memoryStorage());
  p.load();
  assert.equal(p.isUnlocked(LEVELS, '0-1-a'), true);
  assert.equal(p.isUnlocked(LEVELS, '0-2-b'), false);
  assert.equal(p.isUnlocked(LEVELS, '1-1-c'), false);
  p.markSolved('0-1-a', { cycles_avg: 1, size: 1 }, null);
  assert.equal(p.isUnlocked(LEVELS, '0-2-b'), true);
  assert.equal(p.isUnlocked(LEVELS, '1-1-c'), false);
  p.markSolved('0-2-b', { cycles_avg: 1, size: 1 }, null);
  assert.equal(p.isUnlocked(LEVELS, '1-1-c'), true);
  assert.equal(p.isUnlocked(LEVELS, '1-2-d'), false);
  p.markSolved('1-1-c', { cycles_avg: 1, size: 1 }, null);
  assert.equal(p.chapterComplete(LEVELS, 1), true, 'bonus level not required');
  assert.equal(p.isUnlocked(LEVELS, '2-1-e'), true);
  assert.equal(p.isUnlocked(LEVELS, 'unknown'), false);
});

test('unlock all', () => {
  const p = createProgress(memoryStorage());
  p.load();
  p.setUnlockAll(true);
  assert.ok(LEVELS.every((l) => p.isUnlocked(LEVELS, l.id)));
});

test('three solution slots, tutorial checklist', () => {
  const p = createProgress(memoryStorage());
  p.load();
  p.saveSlot('0-1-a', 0, 'a');
  p.saveSlot('0-1-a', 2, 'c');
  p.saveSlot('0-1-a', 3, 'x');
  assert.deepEqual(p.level('0-1-a').slots, ['a', null, 'c']);
  for (const item of ['step', 'breakpoint', 'run']) p.markTutorial('0-1-a', item);
  p.markTutorial('0-1-a', 'bogus');
  assert.equal(p.tutorialDone('0-1-a'), false);
  p.markTutorial('0-1-a', 'back');
  assert.equal(p.tutorialDone('0-1-a'), true);
});

test('export and import round-trip; import validates', () => {
  const p = createProgress(memoryStorage());
  p.load();
  p.markSolved('0-1-a', { cycles_avg: 5, size: 3 }, { cycles: 5, size: 3 });
  p.saveCode('0-1-a', 'halt');
  const json = p.exportJson();
  const q = createProgress(memoryStorage());
  q.load();
  assert.deepEqual(q.importJson(json), { ok: true });
  assert.equal(q.level('0-1-a').code, 'halt');
  assert.equal(q.importJson('not json').ok, false);
  assert.equal(q.importJson('{"v":2,"levels":{}}').ok, false);
  const r = validate({ v: 1, levels: { 'BAD ID': { solved: true }, '0-1-a': { solved: 'yes', best: { cycles: -1, size: 2 }, code: 5 } } });
  assert.equal(r.ok, true);
  assert.deepEqual(Object.keys(r.data.levels), ['0-1-a']);
  assert.equal(r.data.levels['0-1-a'].solved, false);
  assert.equal(r.data.levels['0-1-a'].best, null);
  assert.equal(r.data.levels['0-1-a'].code, null);
});

test('clear removes everything', () => {
  const s = memoryStorage();
  const p = createProgress(s);
  p.load();
  p.saveCode('0-1-a', 'x');
  p.clear();
  assert.equal(s.getItem(STORAGE_KEY), null);
  assert.equal(p.level('0-1-a').code, null);
});
