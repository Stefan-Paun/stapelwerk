// npm test: cargo test --workspace, then node --test over test/unit, test/wasm and test/site.
// Browser tests run separately (npm run test:browser). test/wasm needs a built .wasm
// (npm run build, or cargo build --release -p stapelwerk-wasm --target wasm32-unknown-unknown).
import path from 'node:path';
import { ROOT, cargoHint, findFiles, run } from './lib.mjs';

const cargo = run('cargo', ['test', '--workspace']);
if (cargo === 127) {
  console.error(cargoHint());
  process.exit(1);
}
if (cargo !== 0) process.exit(cargo);

const files = ['unit', 'wasm', 'site'].flatMap((d) => findFiles(path.join(ROOT, 'test', d), '.test.mjs'));
if (files.length === 0) {
  console.log('test: no Node tests found');
  process.exit(0);
}
process.exit(run(process.execPath, ['--test', ...files]));
