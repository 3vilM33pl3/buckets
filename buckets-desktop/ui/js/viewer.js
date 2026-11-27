// File viewer: image compare (side by side, swipe, onion skin, difference),
// channel isolation, zoom and pan, text diffs and binary summaries.
import { api } from './api.js';
import { icon } from './icons.js';
import { esc, bytes, ext, kindColor, fileKind, APP_NAMES, statusGlyph, toast } from './util.js';

// Viewer preferences carry over from file to file.
const prefs = { mode: 'side', channel: 'rgb', bg: 'checker', onion: 50 };
try { Object.assign(prefs, JSON.parse(localStorage.getItem('viewer') || '{}')); } catch { /* ignore */ }
const savePrefs = () => { try { localStorage.setItem('viewer', JSON.stringify(prefs)); } catch { /* ignore */ } };

const channelCache = new Map();
function isolate(dataUrl, channel) {
  if (channel === 'rgb' || !dataUrl) return Promise.resolve(dataUrl);
  const key = `${channel}:${dataUrl.length}:${dataUrl.slice(-64)}`;
  if (channelCache.has(key)) return Promise.resolve(channelCache.get(key));
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement('canvas');
      c.width = img.naturalWidth; c.height = img.naturalHeight;
      const g = c.getContext('2d');
      g.drawImage(img, 0, 0);
      const d = g.getImageData(0, 0, c.width, c.height);
      const k = { r: 0, g: 1, b: 2, a: 3 }[channel];
      const px = d.data;
      for (let i = 0; i < px.length; i += 4) {
        const v = px[i + k];
        px[i] = v; px[i + 1] = v; px[i + 2] = v; px[i + 3] = 255;
      }
      g.putImageData(d, 0, 0);
      const url = c.toDataURL('image/png');
      if (channelCache.size > 24) channelCache.clear();
      channelCache.set(key, url);
      resolve(url);
    };
    img.onerror = () => resolve(dataUrl);
    img.src = dataUrl;
  });
}

/**
 * Show one file. `left` / `right` are { source, label, sub } or null when that
 * side does not exist (a new or deleted file).
 */
// Only one image viewer is live at a time; this tears down its window
// listeners and resize observer.
let disposeActive = null;
export function disposeViewer() {
  if (disposeActive) { const d = disposeActive; disposeActive = null; d(); }
}

export async function showFile(el, { bucketId, path, status, left, right, actions = '' }) {
  disposeViewer();
  const token = Symbol('view');
  el._token = token;
  el.innerHTML = `<div class="viewer"><div class="viewer-head">${title(path, status)}</div><div class="loading"><div class="spinner"></div>Loading preview…</div></div>`;
  let data;
  try {
    if (left && right) {
      data = await api.diff(bucketId, path, left.source, right.source);
    } else {
      const only = left || right;
      const p = await api.preview(bucketId, path, only.source);
      data = left ? { left: p, right: null } : { left: null, right: p };
    }
  } catch (e) {
    if (el._token !== token) return;
    el.innerHTML = `<div class="viewer"><div class="viewer-head">${title(path, status)}</div><div class="empty">${icon('alert')}<div class="big">No preview</div><div>${esc(e)}</div></div></div>`;
    return;
  }
  if (el._token !== token) return;
  const ctx = { el, bucketId, path, status, left, right, data, actions };
  const kind = (data.right || data.left).kind;
  if (kind === 'image' && (data.left?.kind ?? 'image') === 'image' && (data.right?.kind ?? 'image') === 'image') {
    renderImage(ctx);
  } else if (data.text || kind === 'text') {
    renderText(ctx);
  } else {
    renderBinary(ctx);
  }
}

function title(path, status) {
  const i = path.lastIndexOf('/');
  const dir = i < 0 ? '' : path.slice(0, i + 1);
  return `<div class="viewer-title">${status ? statusGlyph(status) : ''}<span class="ellipsis"><span class="faint">${esc(dir)}</span><span class="p">${esc(path.slice(i + 1))}</span></span></div>`;
}

const sideTag = (side, fallback) => side ? `<span class="tag">${esc(side.label || fallback)}${side.sub ? ` <span class="h">${esc(side.sub)}</span>` : ''}</span>` : '';

function openButtons(ctx) {
  const name = APP_NAMES[fileKind(ctx.path)];
  const exists = ctx.right && ctx.right.source.kind === 'working' && ctx.data.right?.kind !== 'missing';
  if (!exists) return ctx.actions;
  return `${ctx.actions}<button class="btn sm" data-v="open" title="Open with the default app">${icon('external', 'sm')}Open in ${esc(name)}</button>
    <button class="btn sm icon" data-v="reveal" title="Show in folder">${icon('folder', 'sm')}</button>`;
}

function wireOpen(ctx) {
  ctx.el.querySelector('[data-v="open"]')?.addEventListener('click', () => api.open(ctx.bucketId, ctx.path, false).catch((e) => toast(String(e), 'err')));
  ctx.el.querySelector('[data-v="reveal"]')?.addEventListener('click', () => api.open(ctx.bucketId, ctx.path, true).catch((e) => toast(String(e), 'err')));
}

// ---------------- images ----------------

function renderImage(ctx) {
  disposeViewer();
  const { data } = ctx;
  const both = !!(data.left && data.right);
  const modes = both
    ? [['side', 'sidebyside', 'Side by side', '1'], ['swipe', 'swipe', 'Swipe', '2'], ['onion', 'onion', 'Onion skin', '3'], ['diff', 'difference', 'Difference', '4']]
    : [];
  if (!both) prefs.mode = prefs.mode === 'diff' ? 'side' : prefs.mode;
  const mode = both ? prefs.mode : 'single';
  const ref = data.right || data.left;
  const st = { zoom: 1, x: 0, y: 0, swipe: 0.5, fitted: false };

  ctx.el.innerHTML = `<div class="viewer">
    <div class="viewer-head">${title(ctx.path, ctx.status)}<div class="viewer-tools">${openButtons(ctx)}</div></div>
    <div class="viewer-head sub">
      <div class="viewer-tools">
        ${both ? `<div class="seg" data-g="mode">${modes.map(([k, ic, t, key]) => `<button class="${mode === k ? 'on' : ''}" data-mode="${k}" title="${t} (${key})">${icon(ic, 'sm')}<span class="lbl">${t}</span></button>`).join('')}</div>` : ''}
        <div class="seg" data-g="ch">${['rgb', 'r', 'g', 'b', 'a'].map((c) => `<button class="${prefs.channel === c ? 'on' : ''}" data-ch="${c}" title="${c === 'rgb' ? 'Colour' : `${c.toUpperCase()} channel`}">${c.toUpperCase()}</button>`).join('')}</div>
        <div class="seg" data-g="bg">${[['checker', 'Checker'], ['dark', 'Black'], ['grey', 'Grey'], ['light', 'White']].map(([k, t]) => `<button class="${prefs.bg === k ? 'on' : ''}" data-bg="${k}" title="${t} background"><span style="display:inline-block;width:10px;height:10px;border-radius:3px;border:1px solid var(--border-strong)" class="bg-${k}"></span></button>`).join('')}</div>
      </div>
    </div>
    <div class="stage-area bg-${prefs.bg}" tabindex="0"></div>
    <div class="viewer-foot"><div class="foot-stats">${imageFoot(data, mode)}</div>
      ${mode === 'onion' ? `<span class="row">Before<input class="slider" type="range" min="0" max="100" value="${prefs.onion}" data-v="onion">After</span>` : ''}
      <button class="btn ghost sm" data-v="fit" title="Fit (F)">${icon('maximize', 'sm')}Fit</button>
      <button class="btn ghost sm" data-v="actual" title="Actual size (0)">1:1</button>
      <span class="zoom-label" data-v="zoom"></span>
    </div>
  </div>`;

  const area = ctx.el.querySelector('.stage-area');
  const urls = { left: data.left?.dataUrl, right: data.right?.dataUrl, diff: data.image?.dataUrl };

  const build = async () => {
    const ch = prefs.channel;
    const [l, r] = await Promise.all([isolate(urls.left, ch), isolate(urls.right, ch)]);
    const layer = (src, cls = '') => `<div class="layer ${cls}"><img src="${src}" draggable="false"></div>`;
    if (mode === 'single') {
      area.innerHTML = `<div class="cells"><div class="cell">${sideTag(data.left ? ctx.left : ctx.right, '')}${layer(l || r)}</div></div>`;
    } else if (mode === 'side') {
      area.innerHTML = `<div class="cells"><div class="cell">${sideTag(ctx.left, 'Before')}${layer(l)}</div><div class="cell">${sideTag(ctx.right, 'After')}${layer(r)}</div></div>`;
    } else if (mode === 'swipe') {
      area.innerHTML = `<div class="cells"><div class="cell">${layer(r)}<div class="swipe-clip">${layer(l)}</div>
        <span class="tag corner-l">${esc(ctx.left.label)}</span>
        <span class="tag corner-r">${esc(ctx.right.label)}</span>
        <div class="swipe-handle"></div></div></div>`;
    } else if (mode === 'onion') {
      area.innerHTML = `<div class="cells"><div class="cell">${layer(l)}${layer(r, 'top')}<span class="tag corner-l">Onion skin</span></div></div>`;
    } else {
      area.innerHTML = `<div class="cells"><div class="cell">${layer(urls.diff)}<span class="tag corner-l">Difference · brighter means more change</span></div></div>`;
    }
    apply();
  };

  const shownSize = () => {
    const img = area.querySelector('.layer img');
    return img ? [img.naturalWidth || 1, img.naturalHeight || 1] : [1, 1];
  };
  const cellSize = () => {
    const c = area.querySelector('.cell');
    return c ? [c.clientWidth, c.clientHeight] : [area.clientWidth, area.clientHeight];
  };
  const fit = () => {
    const [iw, ih] = shownSize(); const [cw, ch] = cellSize();
    st.zoom = Math.min((cw - 40) / iw, (ch - 40) / ih, 8);
    st.x = 0; st.y = 0; apply();
  };
  const actual = () => {
    const [iw] = shownSize();
    st.zoom = (ref.width || iw) / iw; st.x = 0; st.y = 0; apply();
  };
  const apply = () => {
    const [iw, ih] = shownSize();
    const t = `translate(${st.x - (iw * st.zoom) / 2}px, ${st.y - (ih * st.zoom) / 2}px) scale(${st.zoom})`;
    area.querySelectorAll('.layer').forEach((L) => {
      L.style.transform = t;
      L.classList.toggle('pixelated', st.zoom * (iw / (ref.width || iw)) > 2.5 || st.zoom > 2.5);
    });
    const top = area.querySelector('.layer.top');
    if (top) top.style.opacity = prefs.onion / 100;
    const clip = area.querySelector('.swipe-clip');
    const handle = area.querySelector('.swipe-handle');
    if (clip && handle) {
      const cw = area.querySelector('.cell').clientWidth;
      clip.style.clipPath = `inset(0 ${(1 - st.swipe) * 100}% 0 0)`;
      handle.style.left = `${st.swipe * cw - 1}px`;
    }
    const z = ctx.el.querySelector('[data-v="zoom"]');
    if (z) z.textContent = `${Math.round(st.zoom * (iw / (ref.width || iw)) * 100)}%`;
  };

  // Pan with drag, zoom around the cursor with the wheel.
  let drag = null;
  area.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if (e.target.closest('.swipe-handle')) { drag = { swipe: true }; return; }
    drag = { x: e.clientX, y: e.clientY, ox: st.x, oy: st.y };
    area.classList.add('dragging');
  });
  const move = (e) => {
    if (!drag) return;
    if (drag.swipe) {
      const r = area.querySelector('.cell').getBoundingClientRect();
      st.swipe = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    } else {
      st.x = drag.ox + e.clientX - drag.x; st.y = drag.oy + e.clientY - drag.y;
    }
    apply();
  };
  const up = () => { drag = null; area.classList.remove('dragging'); };
  window.addEventListener('mousemove', move);
  window.addEventListener('mouseup', up);
  area.addEventListener('wheel', (e) => {
    e.preventDefault();
    const cell = e.target.closest('.cell') || area;
    const r = cell.getBoundingClientRect();
    const cx = e.clientX - r.left - r.width / 2; const cy = e.clientY - r.top - r.height / 2;
    const k = Math.exp(-e.deltaY * 0.0015);
    const nz = Math.min(64, Math.max(0.02, st.zoom * k));
    const f = nz / st.zoom;
    st.x = cx - (cx - st.x) * f; st.y = cy - (cy - st.y) * f; st.zoom = nz;
    apply();
  }, { passive: false });
  area.addEventListener('dblclick', fit);

  const ro = new ResizeObserver(() => (st.fitted ? fit() : null));
  ro.observe(area);
  const cleanup = () => {
    window.removeEventListener('mousemove', move); window.removeEventListener('mouseup', up); ro.disconnect(); document.removeEventListener('keydown', keys);
    if (disposeActive === cleanup) disposeActive = null;
  };
  disposeActive = cleanup;
  const keys = (e) => {
    if (!document.body.contains(area)) return cleanup();
    if (e.target.closest('input, textarea, select') || e.ctrlKey || e.metaKey || e.altKey) return;
    const m = { 1: 'side', 2: 'swipe', 3: 'onion', 4: 'diff' }[e.key];
    if (m && both) { prefs.mode = m; savePrefs(); renderImage(ctx); return cleanup(); }
    if (e.key === 'f') fit();
    if (e.key === '0') actual();
    const c = { c: 'rgb', r: 'r', g: 'g', b: 'b', a: 'a' }[e.key];
    if (c) { prefs.channel = c; savePrefs(); ctx.el.querySelectorAll('[data-ch]').forEach((b) => b.classList.toggle('on', b.dataset.ch === c)); build(); }
  };
  document.addEventListener('keydown', keys);

  ctx.el.querySelectorAll('[data-mode]').forEach((b) => b.addEventListener('click', () => { prefs.mode = b.dataset.mode; savePrefs(); cleanup(); renderImage(ctx); }));
  ctx.el.querySelectorAll('[data-ch]').forEach((b) => b.addEventListener('click', () => {
    prefs.channel = b.dataset.ch; savePrefs();
    ctx.el.querySelectorAll('[data-ch]').forEach((x) => x.classList.toggle('on', x === b));
    build();
  }));
  ctx.el.querySelectorAll('[data-bg]').forEach((b) => b.addEventListener('click', () => {
    prefs.bg = b.dataset.bg; savePrefs();
    area.className = `stage-area bg-${prefs.bg}`;
    ctx.el.querySelectorAll('[data-bg]').forEach((x) => x.classList.toggle('on', x === b));
  }));
  ctx.el.querySelector('[data-v="onion"]')?.addEventListener('input', (e) => { prefs.onion = +e.target.value; savePrefs(); apply(); });
  ctx.el.querySelector('[data-v="fit"]').addEventListener('click', fit);
  ctx.el.querySelector('[data-v="actual"]').addEventListener('click', actual);
  wireOpen(ctx);

  build().then(() => {
    const img = area.querySelector('.layer img');
    const go = () => { st.fitted = true; fit(); };
    if (img && !img.complete) img.addEventListener('load', go, { once: true }); else go();
  });
}

function imageFoot(data, mode) {
  const L = data.left; const R = data.right;
  const dim = (p) => (p ? `${p.width}×${p.height}` : '');
  const parts = [];
  if (L && R) {
    parts.push(dim(L) === dim(R) ? `<span><b>${dim(R)}</b></span>` : `<span><b>${dim(L)}</b> → <b>${dim(R)}</b></span>`);
    parts.push(`<span>${esc(R.format || '')}</span>`);
    parts.push(`<span><b>${bytes(L.size)}</b> → <b>${bytes(R.size)}</b></span>`);
    if (data.image) {
      parts.push(`<span><b>${data.image.changedPercent.toFixed(1)}%</b> of pixels changed</span>`);
      parts.push(`<span>mean Δ <b>${data.image.meanDelta.toFixed(1)}</b> · max <b>${data.image.maxDelta}</b></span>`);
      if (!data.image.sameSize) parts.push('<span class="pill pending">Resolution changed</span>');
    }
  } else {
    const p = L || R;
    parts.push(`<span><b>${dim(p)}</b></span><span>${esc(p.format || '')}</span><span><b>${bytes(p.size)}</b></span>`);
  }
  const note = (R || L)?.note;
  if (note) parts.push(`<span class="faint">${esc(note)}</span>`);
  void mode;
  return parts.join('');
}

// ---------------- text ----------------

function renderText(ctx) {
  const { data } = ctx;
  let lines = data.text;
  if (!lines) {
    const t = (data.right || data.left).text || '';
    const tag = data.right ? 'insert' : 'delete';
    lines = t.split('\n').map((s, i) => ({ tag, oldLine: data.right ? null : i + 1, newLine: data.right ? i + 1 : null, text: s }));
  }
  const ins = lines.filter((l) => l.tag === 'insert').length;
  const del = lines.filter((l) => l.tag === 'delete').length;
  const CONTEXT = 3;
  const keep = lines.map(() => false);
  lines.forEach((l, i) => { if (l.tag !== 'equal') for (let k = Math.max(0, i - CONTEXT); k <= Math.min(lines.length - 1, i + CONTEXT); k++) keep[k] = true; });
  const anyChange = ins + del > 0;
  let html = ''; let skipped = 0;
  const flush = () => { if (skipped) { html += `<div class="skip">⋯ ${skipped} unchanged line${skipped === 1 ? '' : 's'}</div>`; skipped = 0; } };
  lines.forEach((l, i) => {
    if (anyChange && !keep[i]) { skipped++; return; }
    flush();
    html += `<div class="ln ${l.tag}"><span class="num">${l.oldLine ?? ''}</span><span class="num">${l.newLine ?? ''}</span><span class="sign">${l.tag === 'insert' ? '+' : l.tag === 'delete' ? '−' : ''}</span><span>${esc(l.text)}</span></div>`;
  });
  flush();
  ctx.el.innerHTML = `<div class="viewer">
    <div class="viewer-head">${title(ctx.path, ctx.status)}<div class="viewer-tools">
      <span class="delta"><span class="a">+${ins}</span><span class="d">−${del}</span></span>${openButtons(ctx)}</div></div>
    <div class="pane-body"><div class="textdiff">${html || '<div class="empty">Empty file</div>'}</div></div>
    <div class="viewer-foot">${sideInfo(data)}</div>
  </div>`;
  wireOpen(ctx);
}

// ---------------- binary ----------------

function renderBinary(ctx) {
  const { data } = ctx;
  const e = ext(ctx.path).toUpperCase() || 'FILE';
  const row = (k, a, b) => `<tr><td>${k}</td><td class="mono">${a ?? '—'}</td>${data.left && data.right ? `<td class="mono">${b ?? '—'}</td>` : ''}</tr>`;
  const L = data.left; const R = data.right;
  const head = L && R ? `<tr><td></td><td class="faint">${esc(ctx.left.label)}</td><td class="faint">${esc(ctx.right.label)}</td></tr>` : '';
  const one = L || R;
  const sameContent = L && R && L.hash === R.hash;
  ctx.el.innerHTML = `<div class="viewer">
    <div class="viewer-head">${title(ctx.path, ctx.status)}<div class="viewer-tools">${openButtons(ctx)}</div></div>
    <div class="pane-body" style="display:flex">
      <div class="binary-card">
        <div class="glyph" style="--c:${kindColor(ctx.path)}">${esc(e.slice(0, 6))}</div>
        <div><div style="font-size:15px;font-weight:600">${esc(ctx.path.split('/').pop())}</div>
        <div class="muted" style="margin-top:4px">${L && R ? (sameContent ? 'Contents are identical' : 'Binary file changed. Open it in its app to inspect.') : 'No built-in preview for this file type.'}</div></div>
        <table class="compare-table">${head}
          ${row('Size', bytes(L ? L.size : one.size), R && bytes(R.size))}
          ${row('Hash', esc((L ? L.hash : one.hash || '').slice(0, 16)), R && esc((R.hash || '').slice(0, 16)))}
          ${one.note ? row('Note', esc(one.note), '') : ''}
        </table>
      </div>
    </div>
  </div>`;
  wireOpen(ctx);
}

function sideInfo(data) {
  const L = data.left; const R = data.right;
  if (L && R) return `<span><b>${bytes(L.size)}</b> → <b>${bytes(R.size)}</b></span>`;
  const p = L || R;
  return `<span><b>${bytes(p.size)}</b></span>`;
}
