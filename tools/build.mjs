// Builds the static site into dist/:
//   dist/wasm/stapelwerk.wasm  (cargo release build for wasm32, paths remapped)
//   dist/...                   (copy of web/, if it exists)
//   dist/levels/               (`stapelwerk pack-levels levels dist/levels`: packed .lvl + index.json)
//   dist/en/index.html         (English shell generated from web/index.html)
//   dist/sw.js                 (web/sw.template.js with precache list and content version)
import { createHash } from 'node:crypto';
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { ROOT, WASM_DIST, WASM_MAX_BYTES, WASM_RAW, WASM_TARGET, cargoHint, run } from './lib.mjs';

const dist = path.join(ROOT, 'dist');

function fail(msg) {
  console.error(`build: ${msg}`);
  process.exit(1);
}

// 1. WebAssembly. Remap absolute paths so no local directory names end up in the binary.
//    The repository root comes last because it lies inside HOME and the last matching prefix wins.
const flags = [];
if (process.env.CARGO_ENCODED_RUSTFLAGS) flags.push(...process.env.CARGO_ENCODED_RUSTFLAGS.split('\x1f').filter(Boolean));
else if (process.env.RUSTFLAGS) flags.push(...process.env.RUSTFLAGS.split(' ').filter(Boolean));
flags.push(`--remap-path-prefix=${os.homedir()}=~`, `--remap-path-prefix=${ROOT}=.`);
const env = { ...process.env, CARGO_ENCODED_RUSTFLAGS: flags.join('\x1f') };
delete env.RUSTFLAGS;

console.log('build: cargo build --release -p stapelwerk-wasm --target ' + WASM_TARGET);
const code = run('cargo', ['build', '--release', '--locked', '-p', 'stapelwerk-wasm', '--target', WASM_TARGET], { env });
if (code === 127) fail(cargoHint());
if (code !== 0) fail('cargo build failed');

const wasm = readFileSync(WASM_RAW);
if (wasm.length > WASM_MAX_BYTES) fail(`stapelwerk.wasm is ${wasm.length} bytes, limit ${WASM_MAX_BYTES}`);
for (const secret of [os.homedir(), ROOT, '/Users/', '/home/']) {
  if (secret.length > 1 && wasm.includes(Buffer.from(secret))) fail(`stapelwerk.wasm contains a local path (${secret.startsWith('/') && secret.length < 8 ? secret : 'home or repository directory'})`);
}

// 2. Fresh dist/.
rmSync(dist, { recursive: true, force: true });
mkdirSync(path.dirname(WASM_DIST), { recursive: true });

// 3. Static web files (owned by the UI; may not exist yet).
const web = path.join(ROOT, 'web');
if (existsSync(web)) {
  cpSync(web, dist, { recursive: true, filter: (src) => !path.basename(src).startsWith('.') || src === web });
  console.log('build: copied web/');
} else {
  console.log('build: web/ does not exist yet, skipped');
}
copyFileSync(WASM_RAW, WASM_DIST);
console.log(`build: dist/wasm/stapelwerk.wasm (${wasm.length} bytes)`);

// 4. Levels.
if (existsSync(path.join(ROOT, 'levels', 'manifest.txt'))) {
  const c = run('cargo', ['run', '-q', '--release', '--locked', '-p', 'stapelwerk-cli', '--', 'pack-levels', 'levels', path.join('dist', 'levels')]);
  if (c === 127) fail(cargoHint());
  if (c !== 0) fail('pack-levels failed');
  if (!existsSync(path.join(dist, 'levels', 'index.json'))) fail('pack-levels wrote no dist/levels/index.json');
  console.log('build: packed levels into dist/levels/');
} else {
  console.log('build: levels/ does not exist yet, skipped');
}

// 5. English shell dist/en/index.html, generated from the German one.
const shellPath = path.join(dist, 'index.html');
if (existsSync(shellPath)) {
  const en = englishShell(readFileSync(shellPath, 'utf8'));
  mkdirSync(path.join(dist, 'en'), { recursive: true });
  writeFileSync(path.join(dist, 'en', 'index.html'), en);
  console.log('build: dist/en/index.html');
}

// 6. Service worker with precache list and content version.
const swTemplate = path.join(dist, 'sw.template.js');
if (existsSync(swTemplate)) {
  const template = readFileSync(swTemplate, 'utf8');
  rmSync(swTemplate);
  const files = listFiles(dist).filter((rel) => rel !== 'sw.js');
  const hash = createHash('sha256');
  hash.update(template);
  for (const rel of files) {
    hash.update(`\0${rel}\0`);
    hash.update(readFileSync(path.join(dist, rel)));
  }
  const version = hash.digest('hex').slice(0, 12);
  const sw = template
    .replace("'__STAPELWERK_VERSION__'", JSON.stringify(version))
    .replace('__STAPELWERK_PRECACHE__', JSON.stringify(files, null, 2));
  if (sw.includes('__STAPELWERK_')) fail('sw.template.js: placeholder left after substitution');
  writeFileSync(path.join(dist, 'sw.js'), sw);
  console.log(`build: dist/sw.js (version ${version}, ${files.length} files precached)`);
}

console.log('build: done');

/** Every file below `dir` as a sorted list of '/'-separated relative paths (dot files skipped). */
function listFiles(dir) {
  const out = [];
  const walk = (d, prefix) => {
    for (const name of readdirSync(d).sort()) {
      if (name.startsWith('.')) continue;
      const p = path.join(d, name);
      if (statSync(p).isDirectory()) walk(p, `${prefix}${name}/`);
      else out.push(`${prefix}${name}`);
    }
  };
  walk(dir, '');
  return out.sort();
}

/**
 * Turns the German shell into the English one: lang, title, description, Open Graph,
 * canonical, language link and static texts; relative paths gain a "../" because the
 * page lives one directory deeper. Every replacement must match, so a changed German
 * shell fails the build instead of producing a half-German English page.
 */
function englishShell(de) {
  const SITE = 'https://stefanpaun.page/stapelwerk/';
  const title = 'Stapelwerk · Assembly puzzles on a CPU of its own';
  const replacements = [
    ['<html lang="de">', '<html lang="en">'],
    [/<title>[^<]*<\/title>/, `<title>${title}</title>`],
    [/<meta name="description" content="[^"]*">/, '<meta name="description" content="A learning game in the browser: assembly puzzles on a home-made 32-bit teaching CPU, with a debugger and time travel. No account, no data, works offline.">'],
    [`<link rel="canonical" href="${SITE}">`, `<link rel="canonical" href="${SITE}en/">`],
    [/<meta property="og:title" content="[^"]*">/, `<meta property="og:title" content="${title}">`],
    [/<meta property="og:description" content="[^"]*">/, '<meta property="og:description" content="30 levels from registers to the heap, with a debugger and time travel. No account, no data, works offline.">'],
    [`<meta property="og:url" content="${SITE}">`, `<meta property="og:url" content="${SITE}en/">`],
    [/<meta property="og:image:alt" content="[^"]*">/, '<meta property="og:image:alt" content="Stapelwerk: how a computer really computes. A stack of words drawn on a drafting sheet.">'],
    ['<meta property="og:locale" content="de_AT">', '<meta property="og:locale" content="en_GB">'],
    ['<meta property="og:locale:alternate" content="en_GB">', '<meta property="og:locale:alternate" content="de_AT">'],
    ['<a class="skip" href="#main">Zum Inhalt</a>', '<a class="skip" href="#main">Skip to content</a>'],
    ['aria-label="Hauptnavigation"', 'aria-label="Main navigation"'],
    ['data-route="home">Kapitelplan</a>', 'data-route="home">Chapter plan</a>'],
    ['data-route="sandbox">Sandkasten</a>', 'data-route="sandbox">Sandbox</a>'],
    ['data-route="manual">Handbuch</a>', 'data-route="manual">Manual</a>'],
    ['data-route="about">Über</a>', 'data-route="about">About</a>'],
    ['<a class="lang-link" href="en/" hreflang="en" lang="en">English</a>', '<a class="lang-link" href="../" hreflang="de" lang="de">Deutsch</a>'],
    ['aria-label="Tastenkürzel"', 'aria-label="Keyboard shortcuts"'],
    ['aria-label="Thema wechseln"', 'aria-label="Switch theme"'],
    [/<noscript><p class="page">[^<]*<\/p><\/noscript>/, '<noscript><p class="page">Stapelwerk needs JavaScript: the machine runs as WebAssembly in your browser. No data is sent anywhere.</p></noscript>'],
  ];
  let html = de;
  for (const [from, to] of replacements) {
    const found = typeof from === 'string' ? html.includes(from) : from.test(html);
    if (!found) fail(`English shell: "${from}" not found in web/index.html`);
    html = html.replace(from, to);
  }
  // Relative URLs move one level up; absolute URLs, fragments and the language link stay.
  html = html.replace(/\b(href|src)="([^"]*)"/g, (all, attr, url) => {
    if (/^(?:[a-z][a-z0-9+.-]*:|#|\/)/i.test(url) || (attr === 'href' && url === '../')) return all;
    return `${attr}="../${url}"`;
  });
  if (/[äöüßÄÖÜ]/.test(html.replace(/<link rel="alternate"[^>]*>/g, ''))) fail('English shell still contains German text');
  return html;
}
