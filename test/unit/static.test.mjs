// Static guards over the web sources: no HTML-string APIs, no dynamic code,
// no inline scripts or styles, CSP first, no private paths or e-mail addresses.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const web = fileURLToPath(new URL('../../web/', import.meta.url));

function files(dir, exts) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = path.join(dir, name);
    if (statSync(p).isDirectory()) out.push(...files(p, exts));
    else if (exts.some((e) => name.endsWith(e))) out.push(p);
  }
  return out;
}

test('no HTML strings or dynamic code in web/js', () => {
  const banned = [/\binnerHTML\b/, /\bouterHTML\b/, /\binsertAdjacentHTML\b/, /document\.write/, /\beval\s*\(/, /new\s+Function\b/];
  const hits = [];
  for (const f of files(path.join(web, 'js'), ['.js', '.mjs'])) {
    const src = readFileSync(f, 'utf8');
    for (const re of banned) if (re.test(src)) hits.push(`${path.relative(web, f)}: ${re}`);
  }
  assert.deepEqual(hits, []);
});

test('index.html: CSP meta before any script, no inline script or style', () => {
  const html = readFileSync(path.join(web, 'index.html'), 'utf8');
  const csp = html.indexOf('http-equiv="Content-Security-Policy"');
  const firstScript = html.indexOf('<script');
  assert.ok(csp > 0 && (firstScript < 0 || csp < firstScript));
  assert.match(html, /default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self';/);
  for (const m of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)) {
    assert.match(m[1], /\bsrc=/, 'script without src');
    assert.equal(m[2].trim(), '', 'inline script body');
  }
  assert.ok(!/<style\b/i.test(html), 'inline <style>');
  assert.ok(!/\sstyle=/i.test(html), 'style attribute');
  assert.ok(!/\son[a-z]+=/i.test(html), 'inline event handler');
});

test('canonical, hreflang and og use the public URL', () => {
  const html = readFileSync(path.join(web, 'index.html'), 'utf8');
  assert.match(html, /rel="canonical" href="https:\/\/stefanpaun\.page\/stapelwerk\/"/);
  assert.match(html, /hreflang="en" href="https:\/\/stefanpaun\.page\/stapelwerk\/en\/"/);
  assert.match(html, /hreflang="x-default" href="https:\/\/stefanpaun\.page\/stapelwerk\/"/);
  assert.match(html, /property="og:url" content="https:\/\/stefanpaun\.page\/stapelwerk\/"/);
});

test('no private paths or e-mail addresses in web/', () => {
  const hits = [];
  for (const f of files(web, ['.js', '.mjs', '.html', '.css', '.json', '.webmanifest', '.svg'])) {
    const src = readFileSync(f, 'utf8');
    if (/\/Users\/|\/home\/[a-z]/.test(src)) hits.push(`${path.relative(web, f)}: path`);
    if (/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/.test(src)) hits.push(`${path.relative(web, f)}: e-mail`);
  }
  assert.deepEqual(hits, []);
});

test('no external URLs loaded by CSS or JS', () => {
  const hits = [];
  for (const f of files(web, ['.js', '.css'])) {
    const src = readFileSync(f, 'utf8');
    for (const m of src.matchAll(/(?:url\(|import\(|fetch\()\s*['"]?(https?:)?\/\//g)) hits.push(`${path.relative(web, f)}: ${m[0]}`);
  }
  assert.deepEqual(hits, []);
});

test('fonts are self-hosted woff2 with their OFL texts', () => {
  const fonts = readdirSync(path.join(web, 'fonts'));
  assert.ok(fonts.some((f) => f.startsWith('SofiaSans') && f.endsWith('.woff2')));
  assert.ok(fonts.some((f) => f.startsWith('IBMPlexMono') && f.endsWith('.woff2')));
  for (const lic of ['OFL-SofiaSans.txt', 'OFL-IBMPlexMono.txt']) {
    assert.match(readFileSync(path.join(web, 'fonts', lic), 'utf8'), /SIL OPEN FONT LICENSE Version 1\.1/);
  }
});

test('dark tokens are identical for prefers-color-scheme and [data-theme=dark]', () => {
  const css = readFileSync(path.join(web, 'css/tokens.css'), 'utf8');
  const grab = (start) => {
    const i = css.indexOf(start);
    const open = css.indexOf('{', css.indexOf('{', i) === css.indexOf('{', i) ? i : i);
    const body = css.slice(open + 1, css.indexOf('}', open));
    return Object.fromEntries([...body.matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
  };
  const media = grab(":root:not([data-theme='light'])");
  const attr = grab(":root[data-theme='dark']");
  assert.ok(Object.keys(media).length > 10);
  assert.deepEqual(media, attr);
});
