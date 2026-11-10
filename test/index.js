// Entry point for `node --test test/`: since Node 21, a directory argument is not searched for
// test files but loaded as a module, which resolves to this file. It imports every *.test.mjs in
// test/unit, test/wasm and test/site (browser tests run separately: npm run test:browser).
// `npm test` (tools/test.mjs) passes the files directly and does not use this file.
import { existsSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));

function collect(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .sort()
    .flatMap((name) => {
      const p = path.join(dir, name);
      if (statSync(p).isDirectory()) return collect(p);
      return name.endsWith('.test.mjs') ? [p] : [];
    });
}

for (const file of ['unit', 'wasm', 'site'].flatMap((d) => collect(path.join(here, d)))) {
  await import(pathToFileURL(file).href);
}
