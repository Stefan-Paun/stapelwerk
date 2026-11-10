// Shared by the WebAssembly tests: locating the built module and the sample level.
// STAPELWERK_WASM=<path> selects a specific file; otherwise the newest of dist/ and target/ is used.
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const CANDIDATES = [
  path.join(ROOT, 'dist', 'wasm', 'stapelwerk.wasm'),
  path.join(ROOT, 'target', 'wasm32-unknown-unknown', 'release', 'stapelwerk_wasm.wasm'),
];

export function wasmPath() {
  if (process.env.STAPELWERK_WASM) return process.env.STAPELWERK_WASM;
  const found = CANDIDATES.filter((p) => existsSync(p)).sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  if (found.length === 0) {
    throw new Error('no stapelwerk.wasm found; run `npm run build` first');
  }
  return found[0];
}

export const LEVEL_FILE = path.join(ROOT, 'levels', '2-2-bis-zur-null.lvl');
export const PACKED_LEVEL_FILE = path.join(ROOT, 'dist', 'levels', '2-2-bis-zur-null.lvl');

/** Raw text of section `[name]` of a `.lvl` file (up to the next section header). */
export function lvlSection(text, name) {
  const lines = text.split('\n');
  const start = lines.indexOf(`[${name}]`);
  if (start < 0) return null;
  const out = [];
  for (const line of lines.slice(start + 1)) {
    if (/^\[[a-z][a-z0-9_.-]*\]$/i.test(line)) break;
    out.push(line);
  }
  return out.join('\n').trimEnd() + '\n';
}

export const levelText = () => readFileSync(LEVEL_FILE, 'utf8');
export const referenceSource = () => lvlSection(levelText(), 'reference');
