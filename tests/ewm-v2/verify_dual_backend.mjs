import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const PORT = Number(process.env.WEBIZEN_PORT || 8819);
const BASE_URL = `http://127.0.0.1:${PORT}`;
const EVIDENCE_DIR = path.resolve('C:/github/webizen-website/evidence/ewm-v2');
const CHROME_PATH = process.env.CHROME_PATH || 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const CDP_PORT = Number(process.env.CDP_PORT || 9229);
const HEADLESS = process.env.HEADLESS !== '0';

fs.mkdirSync(EVIDENCE_DIR, { recursive: true });

async function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function checkHttpEndpoints() {
  const favUrl = `${BASE_URL}/favicon.ico`;
  const favRes = await fetch(favUrl);
  const favBuf = await favRes.arrayBuffer();

  const url = `${BASE_URL}/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d`;
  const res200 = await fetch(url);
  const buf200 = await res200.arrayBuffer();
  const res206 = await fetch(url, { headers: { Range: 'bytes=0-1023' } });
  const buf206 = await res206.arrayBuffer();
  const resOpt = await fetch(url, { method: 'OPTIONS' });

  return {
    favicon: {
      url: favUrl,
      status: favRes.status,
      bytes: favBuf.byteLength,
      contentType: favRes.headers.get('content-type'),
    },
    intenteco: {
      url,
      full: {
        status: res200.status,
        bytes: buf200.byteLength,
        acao: res200.headers.get('access-control-allow-origin'),
        acceptRanges: res200.headers.get('accept-ranges'),
        corp: res200.headers.get('cross-origin-resource-policy'),
        coop: res200.headers.get('cross-origin-opener-policy'),
        coep: res200.headers.get('cross-origin-embedder-policy'),
      },
      range: {
        status: res206.status,
        bytes: buf206.byteLength,
        contentRange: res206.headers.get('content-range'),
        acao: res206.headers.get('access-control-allow-origin'),
        acceptRanges: res206.headers.get('accept-ranges'),
      },
      options: {
        status: resOpt.status,
        acao: resOpt.headers.get('access-control-allow-origin'),
        acam: resOpt.headers.get('access-control-allow-methods'),
        acah: resOpt.headers.get('access-control-allow-headers'),
      },
    },
  };
}

class CDPClient {
  constructor(wsUrl) {
    this.wsUrl = wsUrl;
    this.nextId = 1;
    this.pending = new Map();
    this.listeners = new Map();
  }

  async connect() {
    this.ws = new WebSocket(this.wsUrl);
    await new Promise((resolve, reject) => {
      this.ws.addEventListener('open', resolve, { once: true });
      this.ws.addEventListener('error', reject, { once: true });
    });
    this.ws.addEventListener('message', (ev) => {
      const msg = JSON.parse(ev.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message || JSON.stringify(msg.error)));
        else resolve(msg.result);
      } else if (msg.method) {
        const handlers = this.listeners.get(msg.method) || [];
        for (const h of handlers) h(msg.params);
      }
    });
  }

  on(method, fn) {
    if (!this.listeners.has(method)) this.listeners.set(method, []);
    this.listeners.get(method).push(fn);
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  close() {
    try {
      this.ws?.close();
    } catch {}
  }
}

async function waitForCdp(port, timeoutMs = 15000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (r.ok) return await r.json();
    } catch {}
    await sleep(250);
  }
  throw new Error('Timed out waiting for Chrome DevTools Protocol');
}

const EVAL_STATE_EXPR = `(() => {
  const statusEl = document.getElementById('native-world-status');
  const uiEl = document.getElementById('native-world-ui');
  const canvas = document.getElementById('native-world');
  const webizenKeys = Object.keys(window).filter(k => k.toLowerCase().includes('webizen'));
  const webizen = {};
  for (const k of webizenKeys) {
    try { webizen[k] = JSON.parse(JSON.stringify(window[k])); } catch { webizen[k] = String(window[k]); }
  }
  const buttons = Array.from(document.querySelectorAll('button')).map(b => (b.textContent || '').trim()).filter(Boolean);
  const bodyText = (document.body?.innerText || '').trim();
  const exploreLinks = Array.from(document.querySelectorAll('a')).filter(a => /explore the source demonstration/i.test(a.textContent || ''));
  let duplicateAdjacentExploreLinks = 0;
  for (const a of exploreLinks) {
    if (a.nextElementSibling && a.nextElementSibling.tagName === 'A' && /explore the source demonstration/i.test(a.nextElementSibling.textContent || '')) {
      duplicateAdjacentExploreLinks++;
    }
  }
  return {
    readyState: document.readyState,
    title: document.title,
    href: location.href,
    statusText: statusEl ? statusEl.textContent : null,
    uiHidden: uiEl ? uiEl.hidden : null,
    uiChildCount: uiEl ? uiEl.childElementCount : null,
    canvasRect: canvas ? { width: canvas.width, height: canvas.height, clientW: canvas.clientWidth, clientH: canvas.clientHeight } : null,
    hasNavigatorGpu: Boolean(navigator.gpu),
    bodyTextLen: bodyText.length,
    headingCount: document.querySelectorAll('h1,h2,h3').length,
    exploreLinkCount: exploreLinks.length,
    duplicateAdjacentExploreLinks,
    visibleButtons: buttons.slice(0, 40),
    webizen,
  };
})()`;

async function runProfile(profile) {
  const targetResp = await fetch(
    `http://127.0.0.1:${CDP_PORT}/json/new?${encodeURIComponent('about:blank')}`,
    { method: 'PUT' }
  );
  const target = await targetResp.json();
  const cdp = new CDPClient(target.webSocketDebuggerUrl);
  await cdp.connect();

  const consoleLogs = [];
  const exceptions = [];
  const failedRequests = [];
  const httpErrors = [];

  cdp.on('Runtime.consoleAPICalled', (p) => {
    const text = (p.args || []).map((a) => a.value ?? a.description ?? a.type).join(' ');
    consoleLogs.push({ type: p.type, text });
  });
  cdp.on('Runtime.exceptionThrown', (p) => {
    exceptions.push(p.exceptionDetails?.exception?.description || p.exceptionDetails?.text || 'exception');
  });
  cdp.on('Log.entryAdded', (p) => {
    if (p.entry?.level === 'error' || p.entry?.level === 'warning') {
      consoleLogs.push({ type: `log:${p.entry.level}`, text: p.entry.text, url: p.entry.url });
    }
  });
  cdp.on('Network.loadingFailed', (p) => {
    failedRequests.push({ requestId: p.requestId, errorText: p.errorText, type: p.type });
  });
  cdp.on('Network.responseReceived', (p) => {
    if (p.response && p.response.status >= 400) {
      httpErrors.push({ url: p.response.url, status: p.response.status, statusText: p.response.statusText });
    }
  });

  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Log.enable');
  await cdp.send('Network.enable');

  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: profile.width,
    height: profile.height,
    deviceScaleFactor: profile.deviceScaleFactor || 1,
    mobile: Boolean(profile.mobile),
  });
  if (profile.hasTouch) {
    await cdp.send('Emulation.setTouchEmulationEnabled', {
      enabled: true,
      maxTouchPoints: 5,
    });
  }

  const url = `${BASE_URL}${profile.path}`;
  await cdp.send('Page.navigate', { url });

  const start = Date.now();
  let state = null;
  const maxWaitMs = profile.isHtmlFallback ? 10000 : 75000;

  while (Date.now() - start < maxWaitMs) {
    await sleep(1000);
    const evalRes = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      awaitPromise: true,
      expression: EVAL_STATE_EXPR,
    });
    state = evalRes?.result?.value;
    if (profile.isHtmlFallback) {
      if (state && state.readyState === 'complete' && state.bodyTextLen > 200) break;
    } else {
      const init = state?.webizen?.__webizenNativeInit;
      const upload = state?.webizen?.__webizenNativeUpload;
      const ewmState = state?.webizen?.__webizenEwmState;
      const failed = init?.state === 'error' || init?.state === 'failed';
      const firstPaintDone = Boolean(upload?.uploaded) && init?.state === 'ready';
      const catchmentSettled = Boolean(ewmState && (ewmState.streaming_phase === 'active' || ewmState.streamed_species_count > 0 || ewmState.catchment_terrain_active));
      if (failed || (firstPaintDone && catchmentSettled)) {
        await sleep(2500);
        break;
      }
      if (firstPaintDone && Date.now() - start > 55000) {
        await sleep(3000);
        break;
      }
    }
  }

  if (!profile.isHtmlFallback) {
    // Dismiss intro loader if still visible
    await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const btn = Array.from(document.querySelectorAll('button')).find(b => /skip to the world/i.test(b.textContent || ''));
        if (btn && btn.offsetParent !== null) { btn.click(); return true; }
        return false;
      })()`,
    });
    await sleep(2000);
    const finalEval = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      awaitPromise: true,
      expression: EVAL_STATE_EXPR,
    });
    state = finalEval?.result?.value;
  }

  const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
  const pngPath = path.join(EVIDENCE_DIR, `${profile.id}.png`);
  fs.writeFileSync(pngPath, Buffer.from(shot.data, 'base64'));

  const bonusCaptures = [];
  if (profile.runBonusInteractive) {
    // Bonus 1: Click Rhizosphere X-Ray
    const rhizoClick = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const btn = Array.from(document.querySelectorAll('button')).find(b => /rhizosphere/i.test(b.textContent || ''));
        if (btn) { btn.click(); return { clicked: true, label: btn.textContent.trim() }; }
        return { clicked: false };
      })()`,
    });
    await sleep(2000);
    const rhizoShot = await cdp.send('Page.captureScreenshot', { format: 'png' });
    const rhizoPath = path.join(EVIDENCE_DIR, 'profile5-rhizosphere-xray.png');
    fs.writeFileSync(rhizoPath, Buffer.from(rhizoShot.data, 'base64'));
    const rhizoState = (await cdp.send('Runtime.evaluate', { returnByValue: true, expression: EVAL_STATE_EXPR }))?.result?.value;
    bonusCaptures.push({
      id: 'profile5-rhizosphere-xray',
      action: rhizoClick?.result?.value,
      screenshot: rhizoPath,
      statusText: rhizoState?.statusText,
      ewmState: rhizoState?.webizen?.__webizenEwmState,
    });

    // Bonus 2: Open Places first, then open Field Guide drawer (verifying Places auto-collapses)
    await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const obs = Array.from(document.querySelectorAll('button')).find(b => /^observe$/i.test((b.textContent || '').trim()));
        if (obs) obs.click();
        const ep = Array.from(document.querySelectorAll('button')).find(b => /^(explore )?places$/i.test((b.textContent || '').trim()));
        if (ep && ep.getAttribute('aria-expanded') !== 'true') ep.click();
      })()`,
    });
    await sleep(600);
    const fgClick = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const btn = Array.from(document.querySelectorAll('button')).find(b => /field guide|discovered/i.test(b.textContent || ''));
        if (btn) { btn.click(); return { clicked: true, label: btn.textContent.trim() }; }
        return { clicked: false };
      })()`,
    });
    await sleep(1500);
    const fgShot = await cdp.send('Page.captureScreenshot', { format: 'png' });
    const fgPath = path.join(EVIDENCE_DIR, 'profile6-field-guide.png');
    fs.writeFileSync(fgPath, Buffer.from(fgShot.data, 'base64'));
    const fgState = (await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const ep = Array.from(document.querySelectorAll('button')).find(b => /^(explore )?places$/i.test((b.textContent || '').trim()));
        return {
          explorePlacesExpanded: ep ? ep.getAttribute('aria-expanded') : null,
          statusText: document.getElementById('native-world-status')?.textContent || null,
        };
      })()`,
    }))?.result?.value;
    bonusCaptures.push({
      id: 'profile6-field-guide',
      action: fgClick?.result?.value,
      screenshot: fgPath,
      explorePlacesExpanded: fgState?.explorePlacesExpanded,
      statusText: fgState?.statusText,
    });

    // Bonus 3 (Profile 7): Close Field Guide, click #native-mode-drive (or press F), throttle+steer, capture Vehicle HUD
    await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const fg = Array.from(document.querySelectorAll('button')).find(b => /field guide|discovered/i.test(b.textContent || ''));
        if (fg && fg.getAttribute('aria-expanded') === 'true') fg.click();
      })()`,
    });
    await sleep(400);
    const driveClick = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const btn = document.getElementById('native-mode-drive') || Array.from(document.querySelectorAll('button')).find(b => /drive/i.test(b.textContent || ''));
        if (btn) { btn.click(); return { clicked: true, id: btn.id, label: (btn.textContent || '').trim() }; }
        return { clicked: false };
      })()`,
    });
    await sleep(600);
    // Focus canvas and dispatch short throttle (W) + steer (D) input
    await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const c = document.getElementById('native-world');
        if (c) c.focus();
      })()`,
    });
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'w', code: 'KeyW', windowsVirtualKeyCode: 87 });
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'd', code: 'KeyD', windowsVirtualKeyCode: 68 });
    await sleep(1600);
    const driveShot = await cdp.send('Page.captureScreenshot', { format: 'png' });
    const drivePath = path.join(EVIDENCE_DIR, 'profile7-drive-passat-hud.png');
    fs.writeFileSync(drivePath, Buffer.from(driveShot.data, 'base64'));
    const driveState = (await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const cluster = document.getElementById('native-vehicle-cluster');
        return {
          clusterPresent: Boolean(cluster),
          clusterHidden: cluster ? cluster.hidden : null,
          clusterText: cluster ? (cluster.innerText || '').trim().slice(0, 400) : null,
          vehicleState: window.__webizenVehicleState ? JSON.parse(JSON.stringify(window.__webizenVehicleState)) : null,
          characterState: window.__webizenCharacterState ? JSON.parse(JSON.stringify(window.__webizenCharacterState)) : null,
          ewmState: window.__webizenEwmState ? JSON.parse(JSON.stringify(window.__webizenEwmState)) : null,
        };
      })()`,
    }))?.result?.value;
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'd', code: 'KeyD', windowsVirtualKeyCode: 68 });
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'w', code: 'KeyW', windowsVirtualKeyCode: 87 });
    bonusCaptures.push({
      id: 'profile7-drive-passat-hud',
      action: driveClick?.result?.value,
      screenshot: drivePath,
      ...driveState,
    });

    // Bonus 4 (Profile 8): Exit Drive Mode, open #native-stats-toggle, wait for 4 Hz telemetry tick, capture Stats Panel
    await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const obs = Array.from(document.querySelectorAll('button')).find(b => /^observe$/i.test((b.textContent || '').trim()));
        if (obs) obs.click();
      })()`,
    });
    await sleep(400);
    const statsClick = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const btn = document.getElementById('native-stats-toggle') || Array.from(document.querySelectorAll('button')).find(b => /^stats$/i.test((b.textContent || '').trim()));
        if (btn) { btn.click(); return { clicked: true, id: btn.id, label: (btn.textContent || '').trim() }; }
        return { clicked: false };
      })()`,
    });
    await sleep(1000);
    const statsShot = await cdp.send('Page.captureScreenshot', { format: 'png' });
    const statsPath = path.join(EVIDENCE_DIR, 'profile8-stats-panel.png');
    fs.writeFileSync(statsPath, Buffer.from(statsShot.data, 'base64'));
    const statsState = (await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const drawer = document.getElementById('native-stats-drawer');
        return {
          drawerPresent: Boolean(drawer),
          drawerHidden: drawer ? drawer.hidden : null,
          drawerText: drawer ? (drawer.innerText || '').trim().slice(0, 600) : null,
          streamStats: window.__webizenStreamStats ? JSON.parse(JSON.stringify(window.__webizenStreamStats)) : null,
        };
      })()`,
    }))?.result?.value;
    bonusCaptures.push({
      id: 'profile8-stats-panel',
      action: statsClick?.result?.value,
      screenshot: statsPath,
      ...statsState,
    });

    // Bonus 5 (Profile 9): Click #native-debug-toggle, verify Stats auto-collapses, test __webizenDebug API + evictLru()
    const debugClick = await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const btn = document.getElementById('native-debug-toggle') || Array.from(document.querySelectorAll('button')).find(b => /^debug$/i.test((b.textContent || '').trim()));
        if (btn) { btn.click(); return { clicked: true, id: btn.id, label: (btn.textContent || '').trim() }; }
        return { clicked: false };
      })()`,
    });
    await sleep(800);
    const debugApi = (await cdp.send('Runtime.evaluate', {
      returnByValue: true,
      expression: `(() => {
        const statsDrawer = document.getElementById('native-stats-drawer');
        const debugDrawer = document.getElementById('native-debug-drawer');
        const dbg = window.__webizenDebug;
        const beforeStats = window.__webizenStreamStats ? JSON.parse(JSON.stringify(window.__webizenStreamStats)) : null;
        let snapshotKeys = null;
        let logCalled = false;
        let evictRes = null;
        if (dbg) {
          try {
            const snap = dbg.snapshot();
            snapshotKeys = snap ? Object.keys(snap) : null;
          } catch (e) { snapshotKeys = ['err:' + e]; }
          try { dbg.log(); logCalled = true; } catch {}
          try { evictRes = dbg.evictLru(); } catch (e) { evictRes = 'err:' + e; }
        }
        const afterStats = window.__webizenStreamStats ? JSON.parse(JSON.stringify(window.__webizenStreamStats)) : null;
        return {
          statsAutoCollapsed: statsDrawer ? statsDrawer.hidden : null,
          debugDrawerPresent: Boolean(debugDrawer),
          debugDrawerHidden: debugDrawer ? debugDrawer.hidden : null,
          debugDrawerText: debugDrawer ? (debugDrawer.innerText || '').trim().slice(0, 600) : null,
          hasDebugApi: Boolean(dbg),
          snapshotKeys,
          logCalled,
          evictRes,
          beforeEvicted: beforeStats?.total_evicted_out ?? null,
          afterEvicted: afterStats?.total_evicted_out ?? null,
          afterStats,
        };
      })()`,
    }))?.result?.value;
    await sleep(600);
    const debugShot = await cdp.send('Page.captureScreenshot', { format: 'png' });
    const debugPath = path.join(EVIDENCE_DIR, 'profile9-debug-drawer.png');
    fs.writeFileSync(debugPath, Buffer.from(debugShot.data, 'base64'));
    bonusCaptures.push({
      id: 'profile9-debug-drawer',
      action: debugClick?.result?.value,
      screenshot: debugPath,
      ...debugApi,
    });
  }

  cdp.close();
  try {
    await fetch(`http://127.0.0.1:${CDP_PORT}/json/close/${target.id}`);
  } catch {}

  return {
    profile: profile.id,
    name: profile.name,
    url,
    viewport: `${profile.width}x${profile.height}`,
    mobile: Boolean(profile.mobile),
    hasTouch: Boolean(profile.hasTouch),
    elapsedMs: Date.now() - start,
    screenshot: pngPath,
    state,
    bonusCaptures,
    consoleLogs,
    exceptions,
    httpErrors,
    failedRequests,
  };
}

async function main() {
  const httpCheck = await checkHttpEndpoints();
  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'webizen-qa-chrome-'));
  const chromeArgs = [
    ...(HEADLESS ? ['--headless=new'] : []),
    `--remote-debugging-port=${CDP_PORT}`,
    `--user-data-dir=${userDataDir}`,
    '--enable-unsafe-webgpu',
    '--ignore-gpu-blocklist',
    '--use-angle=d3d11',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-background-timer-throttling',
    '--window-size=1280,720',
    'about:blank',
  ];

  const chrome = spawn(CHROME_PATH, chromeArgs, { stdio: 'ignore' });
  let browserVersion = null;
  const profiles = [
    {
      id: 'profile1-desktop-webgpu',
      name: 'Desktop WebGPU (TierA)',
      path: '/?ewm=1',
      width: 1280,
      height: 720,
      deviceScaleFactor: 1,
      mobile: false,
      hasTouch: false,
      runBonusInteractive: true,
    },
    {
      id: 'profile2-desktop-webgl2',
      name: 'Desktop WebGL2 Forced (TierB)',
      path: '/?ewm=1&backend=webgl2',
      width: 1280,
      height: 720,
      deviceScaleFactor: 1,
      mobile: false,
      hasTouch: false,
    },
    {
      id: 'profile3-mobile-webgl2-touch',
      name: 'Mobile WebGL2 Touch Emulation (TierC)',
      path: '/?ewm=1&backend=webgl2',
      width: 390,
      height: 844,
      deviceScaleFactor: 2,
      mobile: true,
      hasTouch: true,
    },
    {
      id: 'profile4-html-reading-fallback',
      name: 'HTML No-GPU Reading Fallback',
      path: '/publication/reading.html',
      width: 1280,
      height: 720,
      deviceScaleFactor: 1,
      mobile: false,
      hasTouch: false,
      isHtmlFallback: true,
    },
  ];

  const results = [];
  try {
    browserVersion = await waitForCdp(CDP_PORT);
    for (const p of profiles) {
      console.log(`Running ${p.id}...`);
      const res = await runProfile(p);
      results.push(res);
      console.log(`Done ${p.id} in ${res.elapsedMs}ms -> ${res.screenshot}`);
      for (const b of res.bonusCaptures || []) {
        console.log(`  Bonus ${b.id} -> ${b.screenshot} (${JSON.stringify(b.action)})`);
      }
    }
  } finally {
    try {
      chrome.kill();
    } catch {}
    await sleep(500);
    try {
      fs.rmSync(userDataDir, { recursive: true, force: true });
    } catch {}
  }

  const report = {
    timestamp: new Date().toISOString(),
    baseUrl: BASE_URL,
    headless: HEADLESS,
    browser: browserVersion,
    httpChecks: httpCheck,
    profiles: results,
  };
  const outJson = path.join(EVIDENCE_DIR, 'telemetry.json');
  fs.writeFileSync(outJson, JSON.stringify(report, null, 2));
  console.log(`Saved telemetry to ${outJson}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
