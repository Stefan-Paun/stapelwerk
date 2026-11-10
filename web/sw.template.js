// Service worker template. tools/build.mjs replaces the two placeholders:
//   __STAPELWERK_VERSION__   → build version string (quoted)
//   __STAPELWERK_PRECACHE__  → JSON array of paths relative to this file
// Cache-first for the app shell, only ever touches caches named "stapelwerk-…".
const VERSION = '__STAPELWERK_VERSION__';
const PRECACHE = __STAPELWERK_PRECACHE__;
const CACHE = `stapelwerk-${VERSION}`;

self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE).then((c) => c.addAll(PRECACHE.map((p) => new URL(p, self.registration.scope).href))).then(() => self.skipWaiting()));
});

self.addEventListener('activate', (event) => {
  event.waitUntil(caches.keys().then((keys) => Promise.all(keys
    .filter((k) => k.startsWith('stapelwerk-') && k !== CACHE)
    .map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || url.origin !== self.location.origin) return;
  event.respondWith(caches.open(CACHE).then(async (cache) => {
    const hit = await cache.match(event.request, { ignoreSearch: true });
    if (hit) return hit;
    const res = await fetch(event.request);
    if (res.ok && url.pathname.startsWith(new URL(self.registration.scope).pathname)) cache.put(event.request, res.clone());
    return res;
  }));
});
