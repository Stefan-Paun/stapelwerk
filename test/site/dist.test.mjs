// Static checks over the built site in dist/ (run `npm run build` first): relative paths,
// CSP before every script, no inline scripts, the English shell, the service worker's
// precache list and no local paths anywhere (also not inside the .wasm bytes).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const DIST = path.join(ROOT, 'dist');
const SITE = 'https://stefanpaun.page/stapelwerk/';
const built = existsSync(path.join(DIST, 'index.html')) && existsSync(path.join(DIST, 'sw.js'));
const skip = built ? false : 'dist/ is not built (npm run build)';

function files(dir = DIST, prefix = '') {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const p = path.join(dir, name);
    if (statSync(p).isDirectory()) out.push(...files(p, `${prefix}${name}/`));
    else out.push(`${prefix}${name}`);
  }
  return out;
}
const read = (rel) => readFileSync(path.join(DIST, rel), 'utf8');
const shells = ['index.html', 'en/index.html'];

/** Every URL in href/src/content attributes that points somewhere (not text content). */
function urls(html) {
  const out = [];
  for (const m of html.matchAll(/<(link|script|img|a|meta)\b([^>]*)>/g)) {
    const attrs = m[2];
    for (const a of attrs.matchAll(/\b(href|src)="([^"]*)"/g)) out.push({ tag: m[1], attrs, url: a[2] });
    if (m[1] === 'meta' && /property="og:(url|image)"/.test(attrs)) out.push({ tag: 'meta', attrs, url: attrs.match(/content="([^"]*)"/)[1] });
  }
  return out;
}

/** The precache list from dist/sw.js, read by running the top of the script in a sandbox. */
function swInfo() {
  const src = read('sw.js');
  const head = src.slice(0, src.indexOf('const PREFIX'));
  const ctx = {};
  vm.runInNewContext(`${head.replace(/^const /gm, 'var ')}`, ctx);
  return { version: ctx.VERSION, precache: ctx.PRECACHE, src };
}

test('only relative URLs, except canonical, hreflang and Open Graph', { skip }, () => {
  for (const shell of shells) {
    const dir = path.posix.dirname(shell);
    for (const { tag, attrs, url } of urls(read(shell))) {
      if (url.startsWith('#')) continue;
      if (/^https?:/.test(url)) {
        const allowed = (tag === 'link' && /rel="(canonical|alternate)"/.test(attrs)) || tag === 'meta';
        assert.ok(allowed, `${shell}: absolute URL ${url} in <${tag}>`);
        assert.ok(url.startsWith(SITE), `${shell}: ${url} is not on ${SITE}`);
        continue;
      }
      assert.ok(!url.startsWith('/'), `${shell}: root-relative URL ${url}`);
      assert.ok(!/^[a-z][a-z0-9+.-]*:/i.test(url), `${shell}: URL with scheme ${url}`);
      const target = path.posix.normalize(path.posix.join(dir, url));
      if (tag === 'a') continue; // links to the other language and hash routes
      assert.ok(existsSync(path.join(DIST, target)), `${shell}: ${url} → ${target} missing`);
    }
  }
});

test('CSS and JS reference only relative, existing files', { skip }, () => {
  for (const rel of files().filter((f) => f.endsWith('.css'))) {
    for (const m of read(rel).matchAll(/url\(\s*['"]?([^'")]+)['"]?\s*\)/g)) {
      if (m[1].startsWith('data:')) continue;
      assert.ok(!/^(\/|[a-z]+:)/i.test(m[1]), `${rel}: ${m[1]}`);
      assert.ok(existsSync(path.join(DIST, path.posix.dirname(rel), m[1])), `${rel}: ${m[1]} missing`);
    }
  }
  for (const rel of files().filter((f) => f.endsWith('.js') && f !== 'sw.js')) {
    for (const m of read(rel).matchAll(/(?:import\s[^'"]*|import\(\s*)['"]([^'"]+)['"]/g)) {
      // node: imports only run under Node (vm.js reads the .wasm file there), never in the browser.
      assert.ok(m[1].startsWith('./') || m[1].startsWith('../') || m[1].startsWith('node:'), `${rel}: import ${m[1]} is not relative`);
    }
  }
});

test('CSP meta comes before every script; no inline scripts, styles or handlers', { skip }, () => {
  for (const shell of shells) {
    const html = read(shell);
    const csp = html.indexOf('http-equiv="Content-Security-Policy"');
    assert.ok(csp > 0, `${shell}: no CSP`);
    for (const m of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)) {
      assert.ok(m.index > csp, `${shell}: script before CSP`);
      assert.match(m[1], /\bsrc="/, `${shell}: script without src`);
      assert.equal(m[2].trim(), '', `${shell}: inline script`);
    }
    assert.match(html, /content="default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; manifest-src 'self'; worker-src 'self'; base-uri 'none'; form-action 'none'"/);
    assert.ok(!/<style\b/i.test(html), `${shell}: <style>`);
    assert.ok(!/\sstyle="/i.test(html), `${shell}: style attribute`);
    assert.ok(!/\son[a-z]+="/i.test(html), `${shell}: inline event handler`);
    assert.ok(!/="\s*javascript:/i.test(html), `${shell}: javascript: URL`);
  }
});

test('en/index.html is the English shell with correct links', { skip }, () => {
  const en = read('en/index.html');
  const de = read('index.html');
  assert.match(en, /<html lang="en">/);
  assert.match(de, /<html lang="de">/);
  assert.match(en, /<title>Stapelwerk · Assembly puzzles on a CPU of its own<\/title>/);
  assert.match(en, /<meta name="description" content="A learning game in the browser:/);
  assert.match(en, /<meta property="og:title" content="Stapelwerk · Assembly puzzles/);
  assert.match(en, /<meta property="og:locale" content="en_GB">/);
  assert.match(en, new RegExp(`<link rel="canonical" href="${SITE}en/">`));
  assert.match(en, new RegExp(`<meta property="og:url" content="${SITE}en/">`));
  assert.match(de, new RegExp(`<link rel="canonical" href="${SITE}">`));
  for (const html of [de, en]) {
    assert.match(html, new RegExp(`<link rel="alternate" hreflang="de" href="${SITE}">`));
    assert.match(html, new RegExp(`<link rel="alternate" hreflang="en" href="${SITE}en/">`));
    assert.match(html, new RegExp(`<link rel="alternate" hreflang="x-default" href="${SITE}">`));
    assert.match(html, new RegExp(`<meta property="og:image" content="${SITE}og.png">`));
  }
  assert.match(en, /<script type="module" src="\.\.\/js\/main\.js"><\/script>/);
  assert.match(en, /<link rel="stylesheet" href="\.\.\/css\/app\.css">/);
  assert.match(en, /<a class="lang-link" href="\.\.\/" hreflang="de" lang="de">Deutsch<\/a>/);
  assert.match(de, /<a class="lang-link" href="en\/" hreflang="en" lang="en">English<\/a>/);
  // Apart from the language link, nothing German is left in the English shell.
  assert.ok(!/[äöüß]|Kapitelplan|Handbuch|Sandkasten/.test(en.replace(/<a class="lang-link"[^>]*>Deutsch<\/a>/, '')));
});

test('og.png is a 1200×630 PNG', { skip }, () => {
  const png = readFileSync(path.join(DIST, 'og.png'));
  assert.equal(png.subarray(1, 4).toString('latin1'), 'PNG');
  assert.equal(png.readUInt32BE(16), 1200);
  assert.equal(png.readUInt32BE(20), 630);
});

test('sw.js: precache list is complete, exists and has a content version', { skip }, () => {
  const { version, precache, src } = swInfo();
  assert.match(version, /^[0-9a-f]{12}$/);
  assert.ok(!src.includes('__STAPELWERK_'), 'placeholder left');
  const all = files().filter((f) => f !== 'sw.js' && !path.posix.basename(f).startsWith('.'));
  assert.deepEqual([...precache].sort(), all, 'precache list must be every file in dist except sw.js');
  for (const rel of precache) assert.ok(existsSync(path.join(DIST, rel)), `missing ${rel}`);
  for (const must of ['index.html', 'en/index.html', 'wasm/stapelwerk.wasm', 'levels/index.json', 'levels/2-2-bis-zur-null.lvl', 'fonts/SofiaSans-wght.woff2', 'js/main.js', 'css/app.css', 'og.png']) {
    assert.ok(precache.includes(must), `precache lacks ${must}`);
  }
  assert.equal(new Set(precache).size, precache.length, 'duplicates in precache');
  assert.ok(!existsSync(path.join(DIST, 'sw.template.js')), 'sw.template.js must not ship');
  assert.match(src, /const PREFIX = 'stapelwerk-'/);
  assert.match(src, /k\.startsWith\(PREFIX\) && k !== CACHE/, 'deletes only its own old caches');
});

test('every level of levels/index.json is in dist and precached', { skip }, () => {
  const index = JSON.parse(read('levels/index.json'));
  const list = Array.isArray(index) ? index : index.levels;
  assert.equal(list.length, 30);
  const { precache } = swInfo();
  for (const l of list) {
    const file = l.file ?? `${l.id}.lvl`;
    assert.ok(precache.includes(`levels/${file}`), `levels/${file} not precached`);
  }
});

test('no local paths in dist (text files and .wasm bytes)', { skip }, () => {
  const needles = ['/Users/', '/home/'].map((s) => Buffer.from(s));
  const hits = [];
  for (const rel of files()) {
    const bytes = readFileSync(path.join(DIST, rel));
    for (const n of needles) if (bytes.includes(n)) hits.push(`${rel}: ${n}`);
  }
  assert.deepEqual(hits, []);
});
