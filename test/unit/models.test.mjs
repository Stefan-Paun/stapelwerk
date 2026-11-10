// Pure row models (hexdump, stack) and the share codec.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { hexRows, diffBytes } from '../../web/js/hexdump.js';
import { stackRows, frameDimensions } from '../../web/js/stackmodel.js';
import { encodeSource, decodeSource, toBase64Url, fromBase64Url } from '../../web/js/share.js';
import { readWord, visibleChar, segmentOf } from '../../web/js/machine.js';

test('hexdump rows: bytes, words, changes, labels', () => {
  const mem = new Uint8Array(65536);
  mem.set([0x78, 0x56, 0x34, 0x12], 0x4000);
  const rows = hexRows(mem, { from: 0x4000, to: 0x4020, changed: new Set([0x4001]), labels: new Map([[0x4010, 'werte']]) });
  assert.equal(rows.length, 2);
  assert.equal(rows[0].segment, 'data');
  assert.equal(rows[0].cells[1].value, 0x56);
  assert.equal(rows[0].cells[1].changed, true);
  assert.equal(rows[1].label, 'werte');
  const words = hexRows(mem, { from: 0x4000, to: 0x4010, mode: 'word' });
  assert.equal(words[0].cells.length, 4);
  assert.equal(words[0].cells[0].value, 0x12345678);
  assert.equal(readWord(mem, 0x4000), 0x12345678);
});

test('diffBytes finds changed addresses only', () => {
  const a = new Uint8Array(16);
  const b = a.slice();
  b[3] = 1; b[9] = 7;
  assert.deepEqual([...diffBytes(a, b, 0, 16)], [3, 9]);
});

test('stack rows annotate a frame', () => {
  const mem = new Uint8Array(65536);
  const put = (addr, v) => { for (let k = 0; k < 4; k++) mem[addr + k] = (v >>> (8 * k)) & 0xff; };
  // caller pushed ret 0x0110, callee: push fp(0), mov fp, sp, sub sp, 4
  put(0xeffc, 0x0110);
  put(0xeff8, 0);
  put(0xeff4, 5);
  const frames = [{ fp: 0xeff8, ret: 0x0110, func: 'fak', size: 4 }];
  const { rows } = stackRows(mem, { sp: 0xeff4, fp: 0xeff8 }, frames, { changed: new Set([0xeff4]) });
  assert.deepEqual(rows.map((r) => r.kind), ['local', 'oldfp', 'ret']);
  assert.equal(rows[0].isSp, true);
  assert.equal(rows[0].changed, true);
  assert.equal(rows[1].isFp, true);
  assert.equal(rows[0].offset, -4);
  const dims = frameDimensions({ sp: 0xeff4 }, frames);
  assert.deepEqual(dims.map((d) => [d.kind, d.bytes]), [['locals', 4], ['ret', 4]]);
});

test('stack rows: invalid sp gives no rows', () => {
  const mem = new Uint8Array(65536);
  assert.equal(stackRows(mem, { sp: 0x1234, fp: 0 }, []).rows.length, 0);
  assert.equal(stackRows(mem, { sp: 0xf000, fp: 0 }, []).rows.length, 0);
});

test('share codec round-trips and rejects garbage', async () => {
  const src = '; Grüße\n        mov r0, 42\n        sys WRITE_INT\n        halt\n';
  const code = await encodeSource(src);
  assert.match(code, /^v1:[A-Za-z0-9_-]+$/);
  assert.equal(await decodeSource(code), src);
  assert.equal(await decodeSource('v1:!!!'), null);
  assert.equal(await decodeSource('v2:abc'), null);
  assert.equal(await decodeSource('v1:AAAA'), null);
  assert.deepEqual([...fromBase64Url(toBase64Url(new Uint8Array([0, 255, 62, 63])))], [0, 255, 62, 63]);
});

test('visible control characters and segments', () => {
  assert.equal(visibleChar(0), '␀');
  assert.equal(visibleChar(10), '␊');
  assert.equal(visibleChar(65), 'A');
  assert.equal(segmentOf(0xf000).name, 'screen');
  assert.equal(segmentOf(0x00ff).name, 'null');
});
