// zuihitsu service worker — Workbox CDN.
// Caches app shell + assets; network-first for HTML; cache-first for images/fonts.

importScripts('https://storage.googleapis.com/workbox-cdn/releases/7.0.0/workbox-sw.js');

const { registerRoute, NavigationRoute } = workbox.routing;
const { NetworkFirst, CacheFirst, StaleWhileRevalidate } = workbox.strategies;
const { ExpirationPlugin } = workbox.expiration;
const { CacheableResponsePlugin } = workbox.cacheableResponse;

workbox.precaching.cleanupOutdatedCaches();

// HTML pages — network first, fall back to cached shell.
registerRoute(
  ({ request }) => request.mode === 'navigate',
  new NetworkFirst({
    cacheName: 'zuihitsu-pages',
    networkTimeoutSeconds: 4,
    plugins: [
      new ExpirationPlugin({ maxAgeSeconds: 24 * 60 * 60, maxEntries: 40 }),
      new CacheableResponsePlugin({ statuses: [0, 200] }),
    ],
  })
);

// WASM + JS bundles — stale-while-revalidate.
registerRoute(
  ({ url }) => url.pathname.startsWith('/pkg/'),
  new StaleWhileRevalidate({
    cacheName: 'zuihitsu-pkg',
    plugins: [
      new ExpirationPlugin({ maxAgeSeconds: 7 * 24 * 60 * 60, maxEntries: 30 }),
      new CacheableResponsePlugin({ statuses: [0, 200] }),
    ],
  })
);

// Images — cache first, 30d.
registerRoute(
  ({ request }) => request.destination === 'image',
  new CacheFirst({
    cacheName: 'zuihitsu-images',
    plugins: [
      new ExpirationPlugin({ maxAgeSeconds: 30 * 24 * 60 * 60, maxEntries: 200 }),
      new CacheableResponsePlugin({ statuses: [0, 200] }),
    ],
  })
);

// Fonts — cache first, 1y.
registerRoute(
  ({ request }) => request.destination === 'font',
  new CacheFirst({
    cacheName: 'zuihitsu-fonts',
    plugins: [
      new ExpirationPlugin({ maxAgeSeconds: 365 * 24 * 60 * 60, maxEntries: 20 }),
    ],
  })
);

// Navigation fallback for SPA-style transitions.
registerRoute(
  new NavigationRoute(
    workbox.precaching.createHandlerBoundToURL('/'),
    { denylist: [/^\/api\//, /^\/sitemap\.xml/, /^\/rss\.xml/, /^\/healthz/, /^\/readyz/] }
  )
);

self.addEventListener('message', (event) => {
  if (event.data?.type === 'SKIP_WAITING') self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});
