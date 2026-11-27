import { icon } from './icons.js';

export const esc = (v) => String(v ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

export function ago(iso) {
  if (!iso) return '';
  const s = (Date.now() - new Date(iso).getTime()) / 1000;
  if (s < 45) return 'just now';
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  if (s < 86400 * 7) return `${Math.round(s / 86400)} d ago`;
  return new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
}

export const clock = (iso) => new Date(iso).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });

export function dayLabel(iso) {
  const d = new Date(iso);
  const today = new Date();
  const y = new Date(); y.setDate(today.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return 'Today';
  if (d.toDateString() === y.toDateString()) return 'Yesterday';
  return d.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' });
}

export function bytes(n) {
  if (n == null) return '';
  const u = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0; let v = n;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v < 10 && i ? v.toFixed(1) : Math.round(v)} ${u[i]}`;
}

const STAGE_HUES = [32, 200, 145, 280, 350, 55, 175, 230, 15, 310];
export const stageColor = (i) => `hsl(${STAGE_HUES[i % STAGE_HUES.length]} 72% 60%)`;

export function personColor(name) {
  let h = 0;
  for (const c of String(name)) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return `hsl(${h % 360} 45% 46%)`;
}
export function initials(owner) {
  const user = String(owner).split('@')[0];
  const parts = user.split(/[._\-\s]+/).filter(Boolean);
  return ((parts[0]?.[0] || '?') + (parts[1]?.[0] || '')).toUpperCase();
}
export const avatar = (owner) => `<span class="avatar" style="--c:${personColor(owner)}" title="${esc(owner)}">${esc(initials(owner))}</span>`;

export const splitPath = (p) => {
  const i = p.lastIndexOf('/');
  return i < 0 ? ['', p] : [p.slice(0, i), p.slice(i + 1)];
};
export const ext = (p) => (p.split('.').pop() || '').toLowerCase();

const KIND = {
  image: ['png', 'jpg', 'jpeg', 'tga', 'tif', 'tiff', 'bmp', 'webp', 'exr', 'hdr', 'gif', 'psd'],
  blender: ['blend'], houdini: ['hip', 'hiplc', 'hipnc', 'hda', 'hdalc', 'otl'], unreal: ['uasset', 'umap'],
  substance: ['sbs', 'sbsar', 'spp'], mesh: ['fbx', 'obj', 'gltf', 'glb', 'usd', 'usda', 'usdc', 'abc'],
  text: ['txt', 'md', 'json', 'ini', 'toml', 'yaml', 'yml', 'py', 'xml', 'csv', 'lua'],
};
export function fileKind(p) {
  const e = ext(p);
  for (const [k, list] of Object.entries(KIND)) if (list.includes(e)) return k;
  return 'other';
}
const KIND_COLOR = { image: '#5aa9ff', blender: '#f0a43a', houdini: '#ff7a45', unreal: '#c9ced6', substance: '#e3e05a', mesh: '#4fc387', text: '#9aa1ab', other: '#9aa1ab' };
export const kindColor = (p) => KIND_COLOR[fileKind(p)];
export const kindIcon = (p) => ({ image: 'image', text: 'text', mesh: 'box' }[fileKind(p)] || 'file');
export const APP_NAMES = { blender: 'Blender', houdini: 'Houdini', unreal: 'Unreal Editor', substance: 'Substance', image: 'image viewer', mesh: 'default app', text: 'editor', other: 'default app' };

export const STATUS_LETTER = { new: 'A', modified: 'M', deleted: 'D', committed: '·' };
export const STATUS_WORD = { new: 'New', modified: 'Modified', deleted: 'Deleted', committed: 'Committed' };
export const statusGlyph = (s) => `<span class="status-glyph ${s}" title="${STATUS_WORD[s] || s}">${STATUS_LETTER[s] || '?'}</span>`;

// ---------- toasts ----------
export function toast(message, kind = 'ok', ms = 4200) {
  const host = document.getElementById('toasts');
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.innerHTML = `${icon(kind === 'ok' ? 'checkCircle' : 'alert')}<div class="grow">${esc(message)}</div>`;
  host.appendChild(el);
  setTimeout(() => el.remove(), kind === 'ok' ? ms : Math.max(ms, 9000));
  el.addEventListener('click', () => el.remove());
}

// ---------- modal ----------
export function modal({ title, subtitle = '', body = '', actions = [], onMount, wide = false }) {
  return new Promise((resolve) => {
    const host = document.getElementById('overlay');
    const scrim = document.createElement('div');
    scrim.className = 'scrim';
    scrim.innerHTML = `<div class="modal" role="dialog" aria-modal="true" style="${wide ? 'width:min(640px,calc(100vw - 40px))' : ''}">
      <header><h2>${esc(title)}</h2>${subtitle ? `<p>${subtitle}</p>` : ''}</header>
      <div class="body">${body}</div>
      <footer>${actions.map((a, i) => `<button class="btn ${a.kind || ''}" data-i="${i}">${a.label}</button>`).join('')}</footer>
    </div>`;
    const close = (v) => { scrim.remove(); document.removeEventListener('keydown', onKey, true); resolve(v); };
    const onKey = (e) => {
      if (e.key === 'Escape') { e.stopPropagation(); close(null); }
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        const primary = actions.findIndex((a) => a.kind?.includes('primary'));
        if (primary >= 0) { e.preventDefault(); trigger(primary); }
      }
    };
    const trigger = async (i) => {
      const a = actions[i];
      const v = a.value === undefined ? i : (typeof a.value === 'function' ? await a.value(scrim) : a.value);
      if (v === false) return; // validation failed, keep open
      close(v);
    };
    scrim.addEventListener('mousedown', (e) => { if (e.target === scrim) close(null); });
    scrim.querySelectorAll('footer .btn').forEach((b) => b.addEventListener('click', () => trigger(+b.dataset.i)));
    document.addEventListener('keydown', onKey, true);
    host.appendChild(scrim);
    onMount?.(scrim);
    (scrim.querySelector('[autofocus]') || scrim.querySelector('.btn.primary'))?.focus();
  });
}

export function confirmBox(title, text, okLabel = 'Continue', danger = false) {
  return modal({
    title, subtitle: text,
    actions: [{ label: 'Cancel', kind: 'ghost', value: 'cancel' }, { label: okLabel, kind: danger ? 'primary danger-primary' : 'primary', value: true }],
  }).then((v) => v === true);
}

// ---------- context menu ----------
export function menu(x, y, items) {
  closeMenu();
  const el = document.createElement('div');
  el.className = 'menu';
  el.id = 'menu';
  el.innerHTML = items.map((it, i) => {
    if (it === '-') return '<hr>';
    if (it.label && !it.run) return `<div class="label">${esc(it.label)}</div>`;
    return `<button data-i="${i}">${it.icon ? icon(it.icon) : ''}<span class="grow">${esc(it.text)}</span>${it.hint ? `<span class="faint mono">${esc(it.hint)}</span>` : ''}</button>`;
  }).join('');
  document.body.appendChild(el);
  const r = el.getBoundingClientRect();
  el.style.left = `${Math.min(x, innerWidth - r.width - 8)}px`;
  el.style.top = `${Math.min(y, innerHeight - r.height - 8)}px`;
  el.querySelectorAll('button').forEach((b) => b.addEventListener('click', () => { closeMenu(); items[+b.dataset.i].run(); }));
  setTimeout(() => document.addEventListener('mousedown', outside, true));
}
function outside(e) { if (!e.target.closest('#menu')) closeMenu(); }
export function closeMenu() {
  document.getElementById('menu')?.remove();
  document.removeEventListener('mousedown', outside, true);
}
