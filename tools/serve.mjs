// Serves dist/ under http://127.0.0.1:4860/stapelwerk/ (like GitHub Pages). No dependencies.
// Environment: PORT (default 4860; 0 picks a free port), HOST (default 127.0.0.1),
// BASE (default /stapelwerk/; the browser tests also mount the site elsewhere, e.g. /a/b/c/).
import { createReadStream, existsSync, statSync } from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { ROOT } from './lib.mjs';

const PORT = Number(process.env.PORT ?? 4860);
const HOST = process.env.HOST ?? '127.0.0.1';
const BASE = `/${(process.env.BASE ?? '/stapelwerk/').split('/').filter(Boolean).join('/')}/`.replace(/^\/\/$/, '/');
const DIST = path.join(ROOT, 'dist');

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.gif': 'image/gif',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
  '.txt': 'text/plain; charset=utf-8',
  '.lvl': 'text/plain; charset=utf-8',
  '.md': 'text/plain; charset=utf-8',
};

if (!existsSync(DIST)) {
  console.error('serve: dist/ does not exist, run `npm run build` first');
  process.exit(1);
}

function send(res, status, text, headers = {}) {
  res.writeHead(status, { 'Content-Type': 'text/plain; charset=utf-8', ...headers });
  res.end(text);
}

const server = http.createServer((req, res) => {
  if (req.method !== 'GET' && req.method !== 'HEAD') return send(res, 405, 'method not allowed\n', { Allow: 'GET, HEAD' });
  let pathname;
  try {
    pathname = decodeURIComponent(new URL(req.url ?? '/', 'http://localhost').pathname);
  } catch {
    return send(res, 400, 'bad request\n');
  }
  if (BASE !== '/' && (pathname === '/' || pathname === BASE.slice(0, -1))) return send(res, 302, '', { Location: BASE });
  if (!pathname.startsWith(BASE)) return send(res, 404, 'not found\n');
  let file = path.resolve(DIST, '.' + path.sep + pathname.slice(BASE.length));
  if (file !== DIST && !file.startsWith(DIST + path.sep)) return send(res, 403, 'forbidden\n');
  try {
    if (statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!statSync(file).isFile()) return send(res, 404, 'not found\n');
  } catch {
    return send(res, 404, 'not found\n');
  }
  res.writeHead(200, {
    'Content-Type': MIME[path.extname(file).toLowerCase()] ?? 'application/octet-stream',
    'Cache-Control': 'no-cache',
    'X-Content-Type-Options': 'nosniff',
  });
  if (req.method === 'HEAD') return res.end();
  createReadStream(file).pipe(res);
});

server.listen(PORT, HOST, () => {
  console.log(`serve: http://${HOST}:${server.address().port}${BASE}`);
});
