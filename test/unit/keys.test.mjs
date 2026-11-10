import { test } from 'node:test';
import assert from 'node:assert/strict';
import { actionForKey, SHORTCUTS } from '../../web/js/keys.js';

test('plain letters act outside text fields', () => {
  assert.equal(actionForKey({ key: 's' }), 'step');
  assert.equal(actionForKey({ key: 'S' }), 'step');
  assert.equal(actionForKey({ key: 'z' }), 'back');
  assert.equal(actionForKey({ key: 'l' }), 'runPause');
  assert.equal(actionForKey({ key: 'r' }), 'restart');
  assert.equal(actionForKey({ key: 'b' }), 'breakpoint');
  assert.equal(actionForKey({ key: 't' }), 'tests');
  assert.equal(actionForKey({ key: '?' }), 'help');
  assert.equal(actionForKey({ key: 'x' }), null);
});

test('letters rest while typing', () => {
  for (const k of ['s', 'z', 'l', 'r', 'b', 't', '?']) assert.equal(actionForKey({ key: k }, { typing: true }), null, k);
});

test('Ctrl/⌘ combinations', () => {
  assert.equal(actionForKey({ key: 'Enter', ctrlKey: true }), 'assembleTest');
  assert.equal(actionForKey({ key: 'Enter', metaKey: true }, { typing: true }), 'assembleTest');
  assert.equal(actionForKey({ key: 's', ctrlKey: true }, { typing: true }), 'step');
  assert.equal(actionForKey({ key: 'b', metaKey: true }, { typing: true }), 'breakpoint');
  // never steal undo, reload, new tab or address bar
  for (const k of ['z', 'r', 't', 'l']) assert.equal(actionForKey({ key: k, ctrlKey: true }, { typing: true }), null, k);
  assert.equal(actionForKey({ key: 's', ctrlKey: true }), null);
  assert.equal(actionForKey({ key: 's', ctrlKey: true, shiftKey: true }, { typing: true }), null);
});

test('Escape leaves the editor; Alt and plain Enter do nothing', () => {
  assert.equal(actionForKey({ key: 'Escape' }, { typing: true }), 'leaveEditor');
  assert.equal(actionForKey({ key: 'Escape' }), null);
  assert.equal(actionForKey({ key: 's', altKey: true }), null);
  assert.equal(actionForKey({ key: 'Enter' }), null);
  assert.equal(actionForKey(null), null);
  assert.equal(actionForKey({ key: '' }), null);
});

test('shortcut table has unique keys', () => {
  assert.equal(new Set(SHORTCUTS.map((s) => s.key)).size, SHORTCUTS.length);
});
