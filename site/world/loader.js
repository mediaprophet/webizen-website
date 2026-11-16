/* Loader: "Every mark is a beginning". A short celebration of human creativity that
   plays while the native world loads. Every path is exported by Synth (see
   /production/loader/loader-art.vibe); this file only stages and animates them.
   Progress is read from the native runtime's own status line; nothing here
   controls or delays the world. */
(function () {
  const doc = document, NS = 'http://www.w3.org/2000/svg';
  const status = doc.getElementById('native-world-status');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const el = (tag, attrs = {}, parent) => { const n = tag.startsWith('svg:') ? doc.createElementNS(NS, tag.slice(4)) : doc.createElement(tag); for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v); if (parent) parent.appendChild(n); return n; };

  const root = el('div', { id: 'creation-loader', role: 'presentation' });
  root.innerHTML = `
    <div class="cl-sky"><div class="cl-dust"></div></div>
    <div class="cl-top"><svg class="cl-wordmark" aria-hidden="true"></svg><p class="cl-kicker">Every mark is a beginning</p></div>
    <figure class="cl-stage" aria-hidden="true"><svg class="cl-ring" viewBox="-300 -300 600 600"></svg><svg class="cl-mark" viewBox="-120 -120 240 240"></svg></figure>
    <div class="cl-caption" aria-live="off"><p class="cl-era"></p><h2 class="cl-title"></h2><p class="cl-line"></p></div>
    <div class="cl-thread" aria-hidden="true"><span></span></div>
    <p class="cl-state"></p>
    <button type="button" class="cl-skip">Skip to the world</button>`;
  doc.body.appendChild(root);
  doc.documentElement.classList.add('cl-loading');
  const $ = s => root.querySelector(s);
  const dust = $('.cl-dust');
  for (let i = 0; i < 70; i++) { const s = el('i', {}, dust); s.style.cssText = `left:${Math.random() * 100}%;top:${Math.random() * 100}%;animation-delay:${-Math.random() * 12}s;opacity:${.2 + Math.random() * .6};transform:scale(${.4 + Math.random()})`; }

  let art = null, marks = [], shown = -1, target = 0, ready = false, failed = false, done = false, lastSwitch = 0;
  const DWELL = reduced ? 900 : 2300, FAST = reduced ? 250 : 650;

  function fit(svg, paths) {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const p of paths) { const b = p.getBBox(); x0 = Math.min(x0, b.x); y0 = Math.min(y0, b.y); x1 = Math.max(x1, b.x + b.width); y1 = Math.max(y1, b.y + b.height); }
    const pad = Math.max(x1 - x0, y1 - y0) * .14; svg.setAttribute('viewBox', `${x0 - pad} ${y0 - pad} ${x1 - x0 + pad * 2} ${y1 - y0 + pad * 2}`);
  }
  function draw(svg, id, slow = 1) {
    svg.textContent = '';
    const entry = id === 'world' ? null : art[id]; const g = el('svg:g', { class: 'cl-g cl-' + id }, svg); const paths = [];
    if (id === 'world') {
      // Finale: every earlier mark gathered into one constellation.
      const order = ['hand', 'wedges', 'wave', 'spiral', 'glyph', 'orbit', 'helix', 'network'];
      order.forEach((k, i) => { const a = i / order.length * Math.PI * 2 - Math.PI / 2, sub = el('svg:g', { transform: `translate(${Math.cos(a) * 82} ${Math.sin(a) * 82}) scale(.28)` }, g); for (const d of art[k].d) paths.push(el('svg:path', { d, class: 'cl-p cl-' + k, transform: k === 'glyph' ? 'translate(-39 -75)' : '' }, sub)); });
      const c = el('svg:circle', { r: 24, class: 'cl-p cl-sun' }, g); paths.push(c);
      svg.setAttribute('viewBox', '-130 -130 260 260');
    } else {
      for (const d of entry.d) paths.push(el('svg:path', { d, class: 'cl-p' }, g));
      fit(svg, paths);
    }
    if (id === 'hand') { const halo = g.cloneNode(true); halo.setAttribute('class', 'cl-halo'); g.parentNode.insertBefore(halo, g); }
    paths.forEach((p, i) => {
      let len = 400; try { len = p.getTotalLength(); } catch (e) {}
      p.style.setProperty('--len', len); p.style.animationDelay = (reduced ? 0 : (i / Math.max(1, paths.length)) * 900 * slow) + 'ms';
    });
    requestAnimationFrame(() => svg.classList.add('cl-live'));
  }
  function constellate(i) {
    const ring = $('.cl-ring'); const count = marks.length - 1; if (i >= count) return;
    const a = i / count * Math.PI * 2 - Math.PI / 2, r = 255; const k = marks[i].id;
    const g = el('svg:g', { class: 'cl-star', transform: `translate(${Math.cos(a) * r} ${Math.sin(a) * r})` }, ring);
    const inner = el('svg:g', { transform: k === 'glyph' ? 'scale(.32) translate(-39 -75)' : 'scale(.32)' }, g);
    for (const d of art[k].d) el('svg:path', { d, class: 'cl-' + k }, inner);
    if (i > 0) { const b = (i - 1) / count * Math.PI * 2 - Math.PI / 2; el('svg:path', { class: 'cl-link', d: `M${Math.cos(b) * r} ${Math.sin(b) * r}A${r} ${r} 0 0 1 ${Math.cos(a) * r} ${Math.sin(a) * r}` }, ring); }
  }
  function show(i) {
    if (i === shown) return; if (shown >= 0) constellate(shown);
    shown = i; lastSwitch = performance.now(); const m = marks[i];
    const cap = $('.cl-caption'); cap.classList.remove('cl-in'); void cap.offsetWidth;
    $('.cl-era').textContent = m.era; $('.cl-title').textContent = m.title; $('.cl-line').textContent = m.line; cap.classList.add('cl-in');
    const svg = $('.cl-mark'); svg.classList.remove('cl-live'); draw(svg, m.id, ready ? .5 : 1);
  }
  function progress(text) {
    const m = /(\d+)\s+of\s+(\d+)/.exec(text || '');
    if (m) return .12 + .83 * (+m[1] / +m[2]);
    if (/Explore the world/i.test(text)) return 1;
    if (/Opening/i.test(text)) return .06;
    return null;
  }
  function onStatus() {
    const text = status ? status.textContent : '';
    if (doc.getElementById('native-fallback')) { failed = true; }
    const p = progress(text); if (p !== null) target = Math.max(target, p);
    if (p === 1) ready = true;
    $('.cl-thread span').style.transform = `scaleX(${failed ? 1 : target})`;
    $('.cl-state').textContent = failed ? 'The living world could not start on this device. The reading view is always available.' : (ready ? 'The world is ready.' : (text || 'Gathering the world…'));
    root.classList.toggle('cl-failed', failed);
  }
  function finish() {
    if (done) return; done = true; root.classList.add('cl-out'); doc.documentElement.classList.remove('cl-loading');
    setTimeout(() => root.remove(), reduced ? 50 : 1400);
  }
  function tick(now) {
    if (done) return;
    if (marks.length) {
      const last = marks.length - 1;
      const allowed = failed ? shown : (ready ? last : Math.min(last - 1, Math.floor(target * last)));
      const dwell = ready ? FAST : DWELL;
      if (shown < allowed && now - lastSwitch > dwell) show(shown + 1);
      else if (shown < 0) show(0);
      else if (ready && shown === last && now - lastSwitch > (reduced ? 900 : 3200)) finish();
    } else if (ready) finish();
    requestAnimationFrame(tick);
  }
  $('.cl-skip').addEventListener('click', finish);
  root.addEventListener('keydown', e => { if (e.key === 'Escape') finish(); });
  if (status) new MutationObserver(onStatus).observe(status, { childList: true, characterData: true, subtree: true });
  new MutationObserver(onStatus).observe(doc.body, { childList: true });
  onStatus();

  fetch('/world/loader-art.json').then(r => r.json()).then(data => {
    art = data.art; marks = data.marks;
    const wm = $('.cl-wordmark'); wm.setAttribute('viewBox', art.wordmark.viewBox);
    for (const d of art.wordmark.d) { const p = el('svg:path', { d }, wm); let len = 600; try { len = p.getTotalLength(); } catch (e) {} p.style.setProperty('--len', len); }
    requestAnimationFrame(() => wm.classList.add('cl-live'));
    requestAnimationFrame(tick);
  }).catch(() => { requestAnimationFrame(tick); });
})();
