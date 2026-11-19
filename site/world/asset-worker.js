'use strict';

// Dedicated off-main-thread asset fetcher & SHA-256 verifier for Webizen World (WEB-WP-22).
// Performs Cache API lookup, Tier 1 local + Tier 2 CORS mirror fetch, and WebCrypto SHA-256
// verification off the UI thread, transferring the verified ArrayBuffer back zero-copy.

const EWM_CACHE_NAME = 'webizen-ewm-v2-content-addressed';
const MAX_CACHE_PUT_BYTES = 16 * 1024 * 1024;

function toLowerHex(buffer) {
  const bytes = new Uint8Array(buffer);
  let hex = '';
  for (let i = 0; i < bytes.length; i += 1) {
    hex += bytes[i].toString(16).padStart(2, '0');
  }
  return hex;
}

async function sha256Hex(arrayBuffer) {
  if (!self.crypto || !self.crypto.subtle) {
    throw new Error('WebCrypto crypto.subtle unavailable in worker context');
  }
  const digest = await self.crypto.subtle.digest('SHA-256', arrayBuffer);
  return toLowerHex(digest);
}

function raceTimeout(promise, timeoutMs) {
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (!settled) {
        settled = true;
        resolve(null);
      }
    }, timeoutMs);
    Promise.resolve(promise)
      .then((val) => {
        if (!settled) {
          settled = true;
          clearTimeout(timer);
          resolve(val);
        }
      })
      .catch(() => {
        if (!settled) {
          settled = true;
          clearTimeout(timer);
          resolve(null);
        }
      });
  });
}

async function openCacheSafe() {
  if (!self.caches || typeof self.caches.open !== 'function') {
    return null;
  }
  return raceTimeout(self.caches.open(EWM_CACHE_NAME), 400);
}

async function tryCacheVerified(logicalUrl, expectedLower, maxBytes) {
  if (!logicalUrl || !expectedLower) {
    return null;
  }
  const cache = await openCacheSafe();
  if (!cache) {
    return null;
  }
  const resp = await raceTimeout(cache.match(logicalUrl), 400);
  if (!resp || !resp.ok) {
    return null;
  }
  const buf = await raceTimeout(resp.arrayBuffer(), 800);
  if (!buf || buf.byteLength > maxBytes) {
    return null;
  }
  try {
    const actual = await sha256Hex(buf);
    if (actual === expectedLower) {
      return { buf, sha256: actual };
    }
  } catch (_) {
    return null;
  }
  raceTimeout(cache.delete(logicalUrl), 300);
  return null;
}

async function cachePutBestEffort(logicalUrl, copyBuffer) {
  if (!logicalUrl || !copyBuffer) {
    return;
  }
  const cache = await openCacheSafe();
  if (!cache) {
    return;
  }
  try {
    const resp = new Response(copyBuffer, {
      status: 200,
      headers: { 'Content-Type': 'application/octet-stream' },
    });
    await raceTimeout(cache.put(logicalUrl, resp), 1000);
  } catch (_) {
    // Private browsing or quota exceeded: non-fatal.
  }
}

async function handleFetchRequest(msg) {
  const { id, reqId, urls, logicalUrl, expectedSha256, maxBytes } = msg;
  const limit = typeof maxBytes === 'number' && maxBytes > 0 ? maxBytes : 64 * 1024 * 1024;
  const expectedLower =
    typeof expectedSha256 === 'string' ? expectedSha256.trim().toLowerCase() : '';
  const candidates = Array.isArray(urls) ? urls.filter((u) => typeof u === 'string' && u.length > 0) : [];

  if (candidates.length === 0) {
    self.postMessage({
      reqId,
      id,
      ok: false,
      error: `Asset ${id || 'unknown'}: no candidate URLs provided`,
    });
    return;
  }

  // 1. Check Cache API off the main thread first.
  const cached = await tryCacheVerified(logicalUrl, expectedLower, limit);
  if (cached) {
    self.postMessage(
      {
        reqId,
        id,
        ok: true,
        sourceUrl: `cache:${logicalUrl}`,
        sha256: cached.sha256,
        buffer: cached.buf,
      },
      [cached.buf]
    );
    return;
  }

  // 2. Try Tier 1 local URL first, then Tier 2 allowlisted CORS mirrors.
  let lastError = `No candidate succeeded for ${id || 'asset'}`;
  for (let i = 0; i < candidates.length; i += 1) {
    const url = candidates[i];
    const isRemote = url.startsWith('https://');
    try {
      const resp = await fetch(url, {
        method: 'GET',
        mode: isRemote ? 'cors' : 'same-origin',
        credentials: isRemote ? 'omit' : 'same-origin',
      });
      if (!resp.ok) {
        lastError = `HTTP ${resp.status} fetching ${url}`;
        continue;
      }
      const clHeader = resp.headers ? resp.headers.get('content-length') : null;
      if (clHeader) {
        const declared = Number(clHeader);
        if (Number.isFinite(declared) && declared > limit) {
          lastError = `Asset ${url} exceeds byte budget (${declared} > ${limit})`;
          continue;
        }
      }
      const buf = await resp.arrayBuffer();
      if (buf.byteLength > limit) {
        lastError = `Asset ${url} exceeds byte budget (${buf.byteLength} > ${limit})`;
        continue;
      }
      const actualSha = await sha256Hex(buf);
      if (expectedLower && actualSha !== expectedLower) {
        lastError = `Asset ${id || url}: SHA-256 mismatch from ${url} (expected ${expectedLower}, got ${actualSha})`;
        continue;
      }
      const cacheCopy =
        logicalUrl && self.caches && buf.byteLength <= MAX_CACHE_PUT_BYTES
          ? buf.slice(0)
          : null;
      self.postMessage(
        {
          reqId,
          id,
          ok: true,
          sourceUrl: url,
          sha256: actualSha,
          buffer: buf,
        },
        [buf]
      );
      if (cacheCopy) {
        cachePutBestEffort(logicalUrl, cacheCopy);
      }
      return;
    } catch (err) {
      lastError = err && err.message ? `${url}: ${err.message}` : `Fetch error for ${url}`;
    }
  }

  self.postMessage({
    reqId,
    id,
    ok: false,
    error: lastError,
  });
}

self.onmessage = function onWorkerMessage(event) {
  const msg = event && event.data ? event.data : null;
  if (!msg || typeof msg !== 'object') {
    return;
  }
  if (msg.type === 'ping') {
    self.postMessage({ reqId: msg.reqId, ok: true, pong: true });
    return;
  }
  handleFetchRequest(msg).catch((err) => {
    self.postMessage({
      reqId: msg.reqId,
      id: msg.id,
      ok: false,
      error: err && err.message ? err.message : String(err),
    });
  });
};
