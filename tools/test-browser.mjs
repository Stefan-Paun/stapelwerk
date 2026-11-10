// npm run test:browser: runs test/browser/*.test.mjs (own CDP client, no npm packages).
// The browser tests themselves find Chrome (CHROME_PATH or Chrome for Testing in the puppeteer
// cache) and skip clearly when none is found, unless STAPELWERK_REQUIRE_BROWSER=1.
import path from 'node:path';
import { ROOT, findFiles, run } from './lib.mjs';

const files = findFiles(path.join(ROOT, 'test', 'browser'), '.test.mjs');
if (files.length === 0) {
  console.log('test:browser: no browser tests yet (they arrive with the web UI), skipped');
  process.exit(0);
}
process.exit(run(process.execPath, ['--test', '--test-concurrency=1', ...files]));
