// Share sandbox programs in the URL hash: "v1:" + base64url(deflate-raw(utf8)).
// Pure except for CompressionStream/DecompressionStream (browser and Node ≥ 18).

const PREFIX = 'v1:';
const MAX_SOURCE = 65536;

export function toBase64Url(bytes) {
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function fromBase64Url(text) {
  if (!/^[A-Za-z0-9_-]*$/.test(text)) throw new Error('bad base64url');
  const pad = text.length % 4 === 0 ? '' : '='.repeat(4 - (text.length % 4));
  const bin = atob(text.replace(/-/g, '+').replace(/_/g, '/') + pad);
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

async function pipe(bytes, stream) {
  const out = new Response(new Blob([bytes]).stream().pipeThrough(stream));
  return new Uint8Array(await out.arrayBuffer());
}

export async function encodeSource(source) {
  const bytes = new TextEncoder().encode(source);
  return PREFIX + toBase64Url(await pipe(bytes, new CompressionStream('deflate-raw')));
}

/** Returns the source text or null if the code is not a valid v1 share code. */
export async function decodeSource(code) {
  if (typeof code !== 'string' || !code.startsWith(PREFIX)) return null;
  try {
    const bytes = await pipe(fromBase64Url(code.slice(PREFIX.length)), new DecompressionStream('deflate-raw'));
    if (bytes.length > MAX_SOURCE) return null;
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}
