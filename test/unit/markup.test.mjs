import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseMarkup, parseInline, toDom } from '../../web/js/markup.js';

// Minimal document stand-in: enough to check that toDom builds nodes, never HTML.
function fakeDocument() {
  class Node {
    constructor(type, name) { this.nodeType = type; this.tagName = name; this.children = []; this.attrs = {}; this._text = ''; }
    append(...nodes) { for (const n of nodes) { if (n.isFragment) this.children.push(...n.children); else this.children.push(n); } }
    setAttribute(k, v) { this.attrs[k] = String(v); }
    set textContent(v) { this._text = String(v); this.children = []; }
    get textContent() { return this._text + this.children.map((c) => c.textContent).join(''); }
  }
  return {
    createElement: (name) => new Node(1, name.toUpperCase()),
    createTextNode: (text) => { const n = new Node(3, '#text'); n._text = text; return n; },
    createDocumentFragment: () => { const n = new Node(11, '#fragment'); n.isFragment = true; return n; },
  };
}

test('paragraphs, headings and lists', () => {
  const blocks = parseMarkup('# Titel\n\nErster Absatz\nzweite Zeile\n\n- eins\n- zwei\n\n## Unter');
  assert.deepEqual(blocks.map((b) => b.type), ['h1', 'p', 'ul', 'h2']);
  assert.equal(blocks[1].children[0].text, 'Erster Absatz zweite Zeile');
  assert.equal(blocks[2].items.length, 2);
});

test('inline bold, italic, code', () => {
  const nodes = parseInline('Das ist **fett**, *kursiv* und `mov r0, 1`.');
  assert.deepEqual(nodes.map((n) => n.type), ['text', 'strong', 'text', 'em', 'text', 'code', 'text']);
  assert.equal(nodes[5].text, 'mov r0, 1');
});

test('code blocks keep their text verbatim', () => {
  const [pre] = parseMarkup('```c\nint x = 1 << 2; // **nicht fett**\n```');
  assert.equal(pre.type, 'pre');
  assert.equal(pre.lang, 'c');
  assert.equal(pre.text, 'int x = 1 << 2; // **nicht fett**');
});

test('only manual links become links; others become text', () => {
  const nodes = parseInline('[Stack](handbuch:stack) und [Heap](manual:heap) und [böse](javascript:alert(1)) und [x](https://example.org)');
  const links = nodes.filter((n) => n.type === 'link');
  assert.deepEqual(links.map((l) => l.topic), ['stack', 'heap']);
  const text = nodes.filter((n) => n.type === 'text').map((n) => n.text).join('');
  assert.ok(!text.includes('javascript:'));
});

test('HTML in the source stays text in the DOM', () => {
  const doc = fakeDocument();
  const frag = toDom(parseMarkup('<img src=x onerror=alert(1)> **<b>x</b>**'), {}, doc);
  const p = frag.children[0];
  assert.equal(p.tagName, 'P');
  assert.ok(p.textContent.includes('<img src=x onerror=alert(1)>'));
  const tags = [];
  const walk = (n) => { tags.push(n.tagName); n.children.forEach(walk); };
  walk(p);
  assert.ok(!tags.includes('IMG') && !tags.includes('B'));
});

test('links point at the manual route', () => {
  const doc = fakeDocument();
  const frag = toDom(parseMarkup('Siehe [Rahmen](handbuch:aufruf).'), {}, doc);
  const a = frag.children[0].children.find((c) => c.tagName === 'A');
  assert.equal(a.attrs.href, '#/manual/aufruf');
});

test('headings shift by headingLevel', () => {
  const doc = fakeDocument();
  const frag = toDom(parseMarkup('# A\n## B'), { headingLevel: 3 }, doc);
  assert.deepEqual(frag.children.map((c) => c.tagName), ['H3', 'H4']);
});

test('unclosed markers stay literal', () => {
  const nodes = parseInline('a ** b und `offen');
  assert.equal(nodes.map((n) => n.text ?? '').join(''), 'a ** b und `offen');
});
