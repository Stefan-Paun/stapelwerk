// Differential test native vs. WebAssembly (docs/STATUS.md, test 10).
//
//   node tools/differential.mjs [DIR] [--seed S] [--count N]
//
// Without DIR it first runs `stapelwerk fuzz-gen --seed S --count N --out target/fuzz-diff`
// (defaults: seed 1, N = 50). With DIR it uses the programs and digests.txt already in DIR (CI:
// fuzz-gen with N = 300 writes target/fuzz). Every program is then run through web/js/vm.js on
// the built module exactly like the native run: test mode (input closed and empty), RAND seeded
// with the program's seed, the same cycle budget. Status, step count and the state digest every
// 1000 steps and at the end must match the native values. STAPELWERK_WASM selects the module;
// otherwise the newer of dist/wasm/stapelwerk.wasm and the cargo output is used.
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { ROOT, WASM_DIST, WASM_RAW, cargoHint, run } from './lib.mjs';

const DIGEST_EVERY = 1000;

function parseArgs(argv) {
  const opts = { dir: null, seed: 1, count: 50 };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--seed') opts.seed = Number(argv[++i]);
    else if (a === '--count') opts.count = Number(argv[++i]);
    else if (a.startsWith('-')) throw new Error(`unknown option ${a}`);
    else opts.dir = a;
  }
  if (!Number.isSafeInteger(opts.seed) || opts.seed < 0) throw new Error('--seed must be a non-negative integer');
  if (!Number.isSafeInteger(opts.count) || opts.count < 1) throw new Error('--count must be a positive integer');
  return opts;
}

function wasmPath() {
  if (process.env.STAPELWERK_WASM) return process.env.STAPELWERK_WASM;
  const found = [WASM_DIST, WASM_RAW].filter((p) => existsSync(p)).sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  if (found.length === 0) throw new Error('no stapelwerk.wasm found; run `npm run build` first');
  return found[0];
}

/** digests.txt line: `file rand_seed budget status steps step:digest…`. */
function parseDigests(text) {
  return text.split('\n').filter((l) => l && !l.startsWith('#')).map((line) => {
    const [file, seed, budget, status, steps, ...pairs] = line.split(' ');
    return {
      file,
      seed: Number(seed),
      budget: Number(budget),
      status: Number(status),
      steps: Number(steps),
      digests: pairs.map((p) => {
        const [step, digest] = p.split(':');
        return { step: Number(step), digest };
      }),
    };
  });
}

/** A one-test level that reproduces the native fuzz configuration. */
function levelText(seed, budget) {
  return [
    '[level]', 'id = fuzz', 'version = 1', 'chapter = 0', 'order = 1', 'kind = normal',
    'title.de = Fuzz', 'title.en = Fuzz', 'card.de = Fuzz', 'card.en = Fuzz',
    `budget = ${budget}`, `seed = ${seed}`, '', '[test]', 'name = fuzz', '',
  ].join('\n');
}

/** Runs one program like `fuzz::run_digests`; returns {status, steps, digests}. */
function runWasm(vm, entry, source) {
  const level = vm.loadLevel(levelText(entry.seed, entry.budget));
  if (!level) throw new Error(`level rejected: ${JSON.stringify(vm.levelDiagnostics)}`);
  const s = vm.newSession(level);
  try {
    const asm = s.assemble(source);
    if (!asm.ok) throw new Error(`does not assemble: ${JSON.stringify(asm.diagnostics[0])}`);
    s.reset(0);
    const digests = [{ step: 0, digest: s.digest() }];
    let status;
    for (;;) {
      status = s.step(DIGEST_EVERY);
      if (status !== 0) break;
      digests.push({ step: s.state().steps, digest: s.digest() });
    }
    const steps = s.state().steps;
    const last = { step: steps, digest: s.digest() };
    const prev = digests[digests.length - 1];
    if (prev.step !== last.step || prev.digest !== last.digest) digests.push(last);
    return { status, steps, digests };
  } finally {
    s.free();
    level.free();
  }
}

function compare(native, wasm) {
  if (native.status !== wasm.status) return `status ${wasm.status}, native ${native.status}`;
  if (native.steps !== wasm.steps) return `steps ${wasm.steps}, native ${native.steps}`;
  const n = Math.max(native.digests.length, wasm.digests.length);
  for (let i = 0; i < n; i++) {
    const a = native.digests[i];
    const b = wasm.digests[i];
    if (!a || !b || a.step !== b.step || a.digest !== b.digest) {
      return `digest #${i}: wasm ${b ? `${b.step}:${b.digest}` : 'missing'}, native ${a ? `${a.step}:${a.digest}` : 'missing'}`;
    }
  }
  return null;
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  let dir = opts.dir ? path.resolve(opts.dir) : path.join(ROOT, 'target', 'fuzz-diff');
  if (!opts.dir) {
    console.log(`differential: fuzz-gen --seed ${opts.seed} --count ${opts.count}`);
    const code = run('cargo', ['run', '-q', '--release', '--locked', '-p', 'stapelwerk-cli', '--', 'fuzz-gen', '--seed', String(opts.seed), '--count', String(opts.count), '--out', dir]);
    if (code === 127) throw new Error(cargoHint());
    if (code !== 0) throw new Error(`fuzz-gen failed (exit code ${code})`);
  }
  const digestsFile = path.join(dir, 'digests.txt');
  if (!existsSync(digestsFile)) throw new Error(`${digestsFile} missing; run stapelwerk fuzz-gen first`);
  const entries = parseDigests(readFileSync(digestsFile, 'utf8'));
  if (entries.length === 0) throw new Error('digests.txt lists no programs');

  const { loadVm } = await import(pathToFileURL(path.join(ROOT, 'web', 'js', 'vm.js')).href);
  const wasm = wasmPath();
  const vm = await loadVm(readFileSync(wasm));
  console.log(`differential: ${entries.length} program(s) from ${path.relative(ROOT, dir) || '.'} against ${path.relative(ROOT, wasm)}`);

  const failures = [];
  const byStatus = new Map();
  let steps = 0;
  let checkpoints = 0;
  const t0 = performance.now();
  for (const entry of entries) {
    let problem;
    try {
      const result = runWasm(vm, entry, readFileSync(path.join(dir, entry.file), 'utf8'));
      problem = compare(entry, result);
      steps += result.steps;
      checkpoints += result.digests.length;
    } catch (err) {
      problem = String(err?.message ?? err);
    }
    byStatus.set(entry.status, (byStatus.get(entry.status) ?? 0) + 1);
    if (problem) failures.push(`${entry.file}: ${problem}`);
  }
  const ms = performance.now() - t0;
  const statuses = [...byStatus].sort((a, b) => a[0] - b[0]).map(([s, n]) => `${n}× status ${s}`).join(', ');
  console.log(`differential: ${steps} steps, ${checkpoints} digests compared in ${ms.toFixed(0)} ms (${statuses})`);
  if (failures.length) {
    console.error(`differential: ${failures.length} mismatch(es)`);
    for (const f of failures.slice(0, 20)) console.error(`  ${f}`);
    process.exit(1);
  }
  console.log('differential: native and WebAssembly agree');
}

main().catch((err) => {
  console.error(`differential: ${err?.message ?? err}`);
  process.exit(2);
});
