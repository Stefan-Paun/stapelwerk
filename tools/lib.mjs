// Shared helpers for the tools/*.mjs scripts. No dependencies.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const WASM_TARGET = 'wasm32-unknown-unknown';
export const WASM_RAW = path.join(ROOT, 'target', WASM_TARGET, 'release', 'stapelwerk_wasm.wasm');
export const WASM_DIST = path.join(ROOT, 'dist', 'wasm', 'stapelwerk.wasm');
export const WASM_MAX_BYTES = 300 * 1024;

/** Runs a command with inherited stdio; returns the exit code (127 if it could not start). */
export function run(cmd, args, options = {}) {
  const res = spawnSync(cmd, args, { cwd: ROOT, stdio: 'inherit', ...options });
  if (res.error) {
    console.error(`cannot start ${cmd}: ${res.error.message}`);
    return 127;
  }
  return res.status ?? 1;
}

/** Runs a command and captures stdout as UTF-8. */
export function capture(cmd, args, options = {}) {
  const res = spawnSync(cmd, args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...options });
  return { code: res.error ? 127 : (res.status ?? 1), stdout: res.stdout ?? '', stderr: res.stderr ?? '', error: res.error };
}

/** All files below `dir` whose names end with `suffix`, sorted. Missing dir → []. */
export function findFiles(dir, suffix) {
  if (!existsSync(dir)) return [];
  const out = [];
  const walk = (d) => {
    for (const name of readdirSync(d).sort()) {
      const p = path.join(d, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (name.endsWith(suffix)) out.push(p);
    }
  };
  walk(dir);
  return out;
}

/** cargo must come from rustup (rust-toolchain.toml pins the version). */
export function cargoHint() {
  return 'cargo not found: install rustup and make sure it is on PATH (with Homebrew: export PATH=/opt/homebrew/opt/rustup/bin:$PATH)';
}
