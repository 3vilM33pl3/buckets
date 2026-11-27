import { api, working, object } from './api.js';
import { icon } from './icons.js';
import { showFile, disposeViewer } from './viewer.js';
import {
  esc, ago, clock, dayLabel, bytes, stageColor, avatar, splitPath, kindColor, kindIcon,
  statusGlyph, toast, modal, confirmBox, menu, fileKind,
} from './util.js';

const root = document.getElementById('root');

// Keep the last script errors around for diagnostics (and end-to-end tests).
window.__errors = [];
window.addEventListener('error', (e) => window.__errors.push(String(e.message)));
window.addEventListener('unhandledrejection', (e) => window.__errors.push(String(e.reason)));

const S = {
  status: null,
  board: null,
  order: [],          // bucket ids in pipeline order
  colors: {},         // bucket id -> colour
  view: 'pipeline',   // pipeline | activity | locks | bucket
  bucketId: null,
  tab: 'changes',     // changes | files | history
  files: null,        // BucketFiles for the open bucket
  history: null,
  commitId: null,
  commitChanges: null,
  selected: null,     // selected file path
  checked: new Set(), // files ticked in the Files tab
  pebbles: new Set(), // pebbles ticked on the board
  showDone: new Set(),
  filter: '',
  fileFilter: 'all',
  lockFilter: 'all',
  activity: null,
  draft: {},          // commit message drafts per bucket
  busy: false,
};

const prefs = (() => { try { return JSON.parse(localStorage.getItem('ui') || '{}'); } catch { return {}; } })();
const savePrefs = () => { try { localStorage.setItem('ui', JSON.stringify(prefs)); } catch { /* ignore */ } };

// ---------------- theme ----------------

function applyTheme() {
  const pref = S.status?.settings?.theme || 'dark';
  const dark = pref === 'dark' || (pref === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
}
matchMedia('(prefers-color-scheme: dark)').addEventListener('change', applyTheme);

// ---------------- data ----------------

const bucketById = (id) => S.board?.buckets.find((b) => b.id === id);
const nameOf = (id) => bucketById(id)?.name ?? 'unknown';

function pipelineOrder(board) {
  // Producers before consumers: an expectation "from A in B" draws A -> B.
  const ids = board.buckets.map((b) => b.id);
  const indeg = Object.fromEntries(ids.map((id) => [id, 0]));
  const out = Object.fromEntries(ids.map((id) => [id, new Set()]));
  for (const e of board.expectations) {
    if (e.fromBucketId && e.fromBucketId !== e.bucketId && out[e.fromBucketId] && !out[e.fromBucketId].has(e.bucketId)) {
      out[e.fromBucketId].add(e.bucketId);
      indeg[e.bucketId]++;
    }
  }
  const order = []; const queue = ids.filter((id) => indeg[id] === 0);
  while (queue.length) {
    const id = queue.shift(); order.push(id);
    for (const n of out[id]) if (--indeg[n] === 0) queue.push(n);
  }
  for (const id of ids) if (!order.includes(id)) order.push(id); // cycles: keep creation order
  return order;
}

async function loadBoard() {
  const board = await api.board();
  S.board = board;
  S.order = pipelineOrder(board);
  S.order.forEach((id, i) => { S.colors[id] = stageColor(i); });
  for (const id of [...S.pebbles]) if (!board.pebbles.some((p) => p.id === id && p.status === 'active')) S.pebbles.delete(id);
}

async function loadBucket(keepSelection = false) {
  const id = S.bucketId;
  const [files, history] = await Promise.all([api.files(id), api.history(id)]);
  if (S.bucketId !== id) return;
  S.files = files; S.history = history;
  if (!keepSelection) { S.selected = null; S.commitId = null; S.commitChanges = null; }
  if (S.commitId && !history.some((c) => c.id === S.commitId)) { S.commitId = null; S.commitChanges = null; }
}

async function refresh({ quiet = false } = {}) {
  try {
    await loadBoard();
    if (S.view === 'bucket' && S.bucketId) {
      if (!bucketById(S.bucketId)) { S.view = 'pipeline'; S.bucketId = null; } else await loadBucket(true);
    }
    if (S.view === 'activity') S.activity = await api.activity();
    render({ keepViewer: true });
  } catch (e) {
    if (!quiet) toast(String(e), 'err');
  }
}

async function act(action, okText) {
  if (S.busy) return null;
  S.busy = true; document.body.style.cursor = 'progress';
  try {
    const { result, warnings } = await api.action(action);
    if (okText) toast(typeof okText === 'function' ? okText(result) : okText);
    for (const w of warnings || []) toast(w, 'warn', 12000);
    await refresh({ quiet: true });
    return result ?? true;
  } catch (e) {
    toast(String(e), 'err');
    return null;
  } finally {
    S.busy = false; document.body.style.cursor = '';
  }
}

// ---------------- shell ----------------

function render(opts = {}) {
  if (!S.status?.repo) { renderWelcome(); return; }
  const viewerHost = document.getElementById('viewer');
  const keep = opts.keepViewer && viewerHost && viewerHost.dataset.key === viewerKey();
  const saved = keep ? viewerHost : null;
  if (!saved) disposeViewer();
  const scroll = [...document.querySelectorAll('[data-scroll]')].map((el) => [el.dataset.scroll, el.scrollTop, el.scrollLeft]);

  root.innerHTML = `<div class="app">${topbar()}${sidebar()}<main class="main">${mainView()}</main></div>`;

  for (const [k, top, left] of scroll) {
    const el = document.querySelector(`[data-scroll="${k}"]`);
    if (el) { el.scrollTop = top; el.scrollLeft = left; }
  }
  const host = document.getElementById('viewer');
  if (host) {
    if (saved) host.replaceWith(saved); else mountViewer(host);
  }
  const msg = document.getElementById('commit-msg');
  if (msg && opts.focusCommit) msg.focus();
}

function topbar() {
  const { repo, me } = S.status;
  const remote = repo.remote;
  return `<header class="topbar">
    <div class="brand"><img src="assets/logo.png" alt="">Buckets</div>
    <button class="repo-switch" data-act="repo-menu" title="Switch repository">
      ${icon('folder')}<span class="name">${esc(repo.name)}</span><span class="path ellipsis">${esc(repo.root)}</span>${icon('chevronDown', 'sm')}
    </button>
    <div class="spacer"></div>
    <div class="sync" title="${remote ? `Remote store: ${esc(remote)}` : 'No remote store set'}">${icon('cloud', 'sm')}${remote ? esc(shortPath(remote)) : 'No remote'}</div>
    <button class="btn sm" data-act="push" title="Copy committed objects to the remote store" ${remote ? '' : 'disabled'}>${icon('upload', 'sm')}Push</button>
    <button class="btn icon sm ghost" data-act="refresh" title="Refresh (Ctrl+R)">${icon('refresh', 'sm')}</button>
    <button class="btn icon sm ghost" data-act="settings" title="Settings">${icon('settings', 'sm')}</button>
    <div class="me" title="Locks and commits are made as ${esc(me)}">${avatar(me)}${esc(me)}</div>
  </header>`;
}

const shortPath = (p) => (p.length > 34 ? `…${p.slice(-32)}` : p);

function sidebar() {
  const b = S.board;
  const active = b ? b.pebbles.filter((p) => p.status === 'active').length : 0;
  const myLocks = b ? b.locks.filter((l) => l.mine).length : 0;
  const nav = (view, ic, label, count = '') => `<button class="nav-item ${S.view === view ? 'active' : ''}" data-act="nav" data-view="${view}">${icon(ic)}${label}${count !== '' ? `<span class="count">${count}</span>` : ''}</button>`;
  const buckets = (b ? S.order.map(bucketById) : []).map((bk) => {
    const n = bk.changes.added + bk.changes.modified + bk.changes.deleted;
    const todo = b.pebbles.filter((p) => p.currentBucketId === bk.id && p.status === 'active').length;
    return `<button class="nav-item ${S.view === 'bucket' && S.bucketId === bk.id ? 'active' : ''}" data-act="open-bucket" data-id="${bk.id}">
      <span class="dot" style="--c:${S.colors[bk.id]}"></span><span class="ellipsis">${esc(bk.name)}</span>
      <span class="badges">${n ? `<span class="pill modified" title="${n} uncommitted">${n}</span>` : ''}${todo ? `<span class="pill pending" title="${todo} to deliver">${todo}</span>` : ''}</span>
    </button>`;
  }).join('');
  return `<aside class="sidebar">
    ${nav('pipeline', 'board', 'Pipeline', active || '')}
    ${nav('activity', 'activity', 'Activity')}
    ${nav('locks', 'lock', 'Locks', b ? `${myLocks ? `${myLocks} mine · ` : ''}${b.locks.length}` : '')}
    <div class="nav-label">Buckets</div>
    ${buckets || '<div class="muted" style="padding:6px 10px">No buckets yet</div>'}
  </aside>`;
}

function mainView() {
  if (!S.board) return `<div class="loading"><div class="spinner"></div>Reading repository…</div>`;
  if (S.view === 'activity') return activityView();
  if (S.view === 'locks') return locksView();
  if (S.view === 'bucket') return bucketView();
  return pipelineView();
}

// ---------------- pipeline ----------------

function pipelineView() {
  const b = S.board;
  const active = b.pebbles.filter((p) => p.status === 'active');
  const delivered = b.expectations.filter((e) => e.delivered).length;
  const uncommitted = b.buckets.reduce((n, x) => n + x.changes.added + x.changes.modified + x.changes.deleted, 0);
  const stats = `<div class="stats">
    <div class="stat"><div class="k">${icon('pebble', 'sm')}Open requests</div><div class="v">${active.length}</div></div>
    <div class="stat"><div class="k">${icon('checkCircle', 'sm')}Expectations delivered</div><div class="v">${delivered}<small>of ${b.expectations.length}</small></div></div>
    <div class="stat"><div class="k">${icon('changes', 'sm')}Uncommitted files</div><div class="v">${uncommitted}</div></div>
    <div class="stat"><div class="k">${icon('lock', 'sm')}Locked files</div><div class="v">${b.locks.length}<small>${b.locks.filter((l) => l.mine).length} yours</small></div></div>
  </div>`;
  const cols = S.order.map((id, i) => stageColumn(bucketById(id), i)).join(`<div class="flow-arrow">${icon('chevronRight')}</div>`);
  return `<div class="page-head"><div class="grow"><h1>Pipeline</h1><div class="sub">Each bucket is a stage. Requests flow downstream to the stage that has to deliver them.</div></div>
      <button class="btn primary" data-act="new-expectation">${icon('plus', 'sm')}New expectation</button></div>
    <div class="page-body" data-scroll="pipeline" style="display:flex;flex-direction:column">${stats}
      ${b.buckets.length ? `<div class="board" data-scroll="board">${cols}</div>` : `<div class="empty">${icon('bucket')}<div class="big">No buckets in this repository</div><div>Create one with <span class="mono">buckets create &lt;name&gt;</span>.</div></div>`}
    </div>`;
}

function stageColumn(bk, i) {
  const b = S.board;
  const mine = b.pebbles.filter((p) => p.currentBucketId === bk.id);
  const todo = mine.filter((p) => p.status === 'active');
  const done = mine.filter((p) => p.status !== 'active');
  const waits = b.expectations.filter((e) => e.bucketId === bk.id);
  const locks = b.locks.filter((l) => l.bucketId === bk.id);
  const n = bk.changes.added + bk.changes.modified + bk.changes.deleted;
  const met = waits.filter((e) => e.delivered).length;
  const picked = todo.filter((p) => S.pebbles.has(p.id)).length;
  const showDone = S.showDone.has(bk.id);
  const ticket = (p) => {
    const target = p.originBucketId !== p.currentBucketId ? `<span class="chip"><span class="dot" style="--c:${S.colors[p.originBucketId]}"></span>for ${esc(nameOf(p.originBucketId))}</span>` : '<span class="chip">own goal</span>';
    const isDone = p.status !== 'active';
    return `<div class="ticket ${S.pebbles.has(p.id) ? 'selected' : ''} ${isDone ? 'done' : ''}">
      ${isDone ? `<span class="faint" style="margin-top:1px">${icon('checkCircle')}</span>` : `<button class="check ${S.pebbles.has(p.id) ? 'on' : ''}" data-act="pick-pebble" data-id="${p.id}" title="Select">${icon('check')}</button>`}
      <div class="grow"><div class="desc">${esc(p.description)}</div><div class="meta">${target}<span>${ago(p.createdAt)}</span></div></div>
    </div>`;
  };
  return `<section class="stage" style="--c:${S.colors[bk.id]}">
    <div class="stage-head" data-act="open-bucket" data-id="${bk.id}">
      <div class="row"><span class="stage-index">STAGE ${String(i + 1).padStart(2, '0')}</span><span class="grow"></span>
        ${waits.length ? `<div class="ring" style="--p:${Math.round((met / waits.length) * 100)}" title="${met} of ${waits.length} expectations delivered"><span>${met}/${waits.length}</span></div>` : ''}</div>
      <div class="stage-name">${esc(bk.name)}</div>
      <div class="stage-meta">
        ${n ? `<span class="pill modified">${n} uncommitted</span>` : '<span class="pill plain">Clean</span>'}
        ${locks.length ? `<span class="pill lock">${icon('lock', 'sm')}${locks.length}</span>` : ''}
        <span class="pill plain">${bk.fileCount} files</span>
      </div>
      <div class="stage-last">${bk.lastCommit ? `${icon('history', 'sm')}<span class="msg ellipsis grow">${esc(bk.lastCommit.message)}</span><span>${ago(bk.lastCommit.createdAt)}</span>` : '<span>No commits yet</span>'}</div>
    </div>
    <div class="stage-body">
      <div class="stage-section">
        <h4>To deliver <span class="n">${todo.length}</span>
          ${todo.length ? `<button class="btn sm ${picked ? 'primary' : ''}" data-act="finalize" data-id="${bk.id}" title="Mark the selected requests as delivered (finalize)">${icon('flag', 'sm')}${picked ? `Finalize ${picked}` : 'Finalize all'}</button>` : ''}</h4>
        ${todo.map(ticket).join('') || '<div class="faint" style="padding:2px 4px 4px">Nothing requested</div>'}
        ${done.length ? `<button class="toggle-done" data-act="toggle-done" data-id="${bk.id}">${icon(showDone ? 'chevronDown' : 'chevronRight', 'sm')}${done.length} delivered</button>${showDone ? done.map(ticket).join('') : ''}` : ''}
      </div>
      <div class="stage-section">
        <h4>Waiting on <span class="n">${waits.length}</span><button class="btn sm ghost" data-act="new-expectation" data-id="${bk.id}" title="Add an expectation to ${esc(bk.name)}">${icon('plus', 'sm')}</button></h4>
        ${waits.map((e) => `<div class="wait">
            <span class="state" style="color:${e.delivered ? 'var(--new)' : 'var(--accent)'}">${icon(e.delivered ? 'checkCircle' : 'clock')}</span>
            <div class="grow"><div class="${e.delivered ? 'muted' : ''}">${esc(e.description)}</div>
              <div class="meta row" style="margin-top:5px;font-size:12px;color:var(--muted)">${e.fromBucketId ? `<span class="chip"><span class="dot" style="--c:${S.colors[e.fromBucketId]}"></span>from ${esc(nameOf(e.fromBucketId))}</span>` : '<span class="chip">own goal</span>'}
              <span class="pill ${e.delivered ? 'done' : 'pending'}">${e.delivered ? (e.status === 'met' ? 'Met' : 'Delivered') : 'Pending'}</span></div></div>
          </div>`).join('') || '<div class="faint" style="padding:2px 4px 4px">Not waiting on anything</div>'}
      </div>
    </div>
    <div class="stage-foot">
      <button class="btn sm grow" data-act="open-bucket" data-id="${bk.id}" data-tab="changes">${icon('changes', 'sm')}Changes${n ? ` (${n})` : ''}</button>
      <button class="btn sm" data-act="open-bucket" data-id="${bk.id}" data-tab="history">${icon('history', 'sm')}History</button>
      <button class="btn sm icon" data-act="open-folder" data-id="${bk.id}" title="Open folder">${icon('folder', 'sm')}</button>
    </div>
  </section>`;
}

async function newExpectation(holderId) {
  const buckets = S.order.map(bucketById);
  if (!buckets.length) return;
  const holder = holderId || buckets[buckets.length - 1].id;
  const opts = (sel, extra = '') => extra + buckets.map((b) => `<option value="${b.id}" ${b.id === sel ? 'selected' : ''}>${esc(b.name)}</option>`).join('');
  const upstream = S.board.expectations.find((e) => e.bucketId === holder && e.fromBucketId)?.fromBucketId
    || buckets[Math.max(0, buckets.findIndex((b) => b.id === holder) - 1)]?.id;
  const result = await modal({
    title: 'New expectation',
    subtitle: 'Say what one stage needs from another. It shows up as a request in the stage that has to deliver it.',
    body: `<div class="flow-pick">
        <div class="field"><label>Delivered by</label><select class="input" name="from">${opts(upstream === holder ? '' : upstream, '<option value="">The same stage (own goal)</option>')}</select></div>
        <div class="arrow">${icon('arrowRight')}</div>
        <div class="field"><label>Needed by</label><select class="input" name="holder">${opts(holder)}</select></div>
      </div>
      <div class="field"><label>What is expected</label>
        <textarea class="input" name="desc" rows="3" placeholder="e.g. Brick set tiling at 2 m, with a dark variant" autofocus></textarea>
        <div class="hint">Be specific about the finish line. Ctrl+Enter to add.</div></div>`,
    actions: [
      { label: 'Cancel', kind: 'ghost', value: 'cancel' },
      {
        label: 'Add expectation', kind: 'primary',
        value: (m) => {
          const desc = m.querySelector('[name=desc]').value.trim();
          if (!desc) { m.querySelector('[name=desc]').focus(); return false; }
          return { desc, holder: m.querySelector('[name=holder]').value, from: m.querySelector('[name=from]').value || null };
        },
      },
    ],
  });
  if (!result || result === 'cancel') return;
  await act({ type: 'expect', bucketId: result.holder, fromBucketId: result.from === result.holder ? null : result.from, description: result.desc },
    `Expectation added to ${nameOf(result.holder)}`);
}

async function finalize(bucketId) {
  const todo = S.board.pebbles.filter((p) => p.currentBucketId === bucketId && p.status === 'active');
  const picked = todo.filter((p) => S.pebbles.has(p.id));
  const list = picked.length ? picked : todo;
  const ok = await confirmBox(
    picked.length ? `Finalize ${picked.length} request${picked.length === 1 ? '' : 's'}?` : `Finalize all ${todo.length} requests in ${nameOf(bucketId)}?`,
    `<div style="margin-top:8px;display:grid;gap:4px">${list.map((p) => `<div class="row">${icon('flag', 'sm')}<span>${esc(p.description)}</span></div>`).join('')}</div>
     <div style="margin-top:10px">Make sure the work is committed first. Finalizing marks these requests as delivered.</div>`,
    'Finalize',
  );
  if (!ok) return;
  const res = await act({ type: 'finalize', bucketId, pebbleIds: picked.map((p) => p.id) },
    (r) => `Delivered ${r?.resolved?.length ?? list.length} request${(r?.resolved?.length ?? list.length) === 1 ? '' : 's'}`);
  if (res) picked.forEach((p) => S.pebbles.delete(p.id));
}

// ---------------- bucket ----------------

function bucketView() {
  const bk = bucketById(S.bucketId);
  if (!bk) return '';
  const files = S.files?.files ?? [];
  const changed = files.filter((f) => f.status !== 'committed');
  const tab = (id, ic, label, n) => `<button class="tab ${S.tab === id ? 'on' : ''}" data-act="tab" data-tab="${id}">${icon(ic, 'sm')}${label}${n != null ? `<span class="n">${n}</span>` : ''}</button>`;
  let body = `<div class="loading"><div class="spinner"></div>Scanning files…</div>`;
  if (S.files) body = S.tab === 'files' ? filesTab(files) : S.tab === 'history' ? historyTab() : changesTab(changed);
  return `<div class="page-head" style="padding-bottom:12px">
      <span class="dot" style="--c:${S.colors[bk.id]};width:12px;height:12px;margin-bottom:8px"></span>
      <div class="grow"><h1>${esc(bk.name)}</h1><div class="sub mono">${esc(S.status.repo.root)}/${esc(bk.path)}</div></div>
      <button class="btn" data-act="open-folder" data-id="${bk.id}">${icon('folder', 'sm')}Open folder</button>
      <button class="btn" data-act="pull" title="Update working files to the latest commit, fetching objects from the remote store">${icon('download', 'sm')}Pull</button>
      <button class="btn icon" data-act="bucket-menu" title="More">${icon('more')}</button>
    </div>
    <nav class="tabs">${tab('changes', 'changes', 'Changes', changed.length)}${tab('files', 'folder', 'Files', files.filter((f) => f.status !== 'deleted').length)}${tab('history', 'history', 'History', S.history?.length ?? null)}</nav>
    ${body}`;
}

function fileRow(f, { check = false, actions = true, sizeCol = true } = {}) {
  const [dir, name] = splitPath(f.path);
  const lock = f.lock ? `<span class="pill lock" title="Locked by ${esc(f.lock.owner)} ${ago(f.lock.createdAt)}">${icon('lock', 'sm')}${f.lock.mine ? 'You' : esc(f.lock.owner.split('@')[0])}</span>` : '';
  const lockBtn = !actions ? '' : f.lock
    ? `<button class="btn sm icon ghost" data-act="unlock" data-path="${esc(f.path)}" title="${f.lock.mine ? 'Unlock' : `Break ${esc(f.lock.owner)}'s lock`}">${icon('unlock', 'sm')}</button>`
    : (f.status !== 'deleted' ? `<button class="btn sm icon ghost" data-act="lock" data-path="${esc(f.path)}" title="Lock so nobody else can commit it">${icon('lock', 'sm')}</button>` : '');
  return `<div class="list-item ${S.selected === f.path ? 'on' : ''}" data-act="select-file" data-path="${esc(f.path)}">
    ${check ? `<button class="check ${S.checked.has(f.path) ? 'on' : ''}" data-act="check-file" data-path="${esc(f.path)}">${icon('check')}</button>` : ''}
    ${f.status !== 'committed' ? statusGlyph(f.status) : `<span style="color:${kindColor(f.path)}">${icon(kindIcon(f.path), 'sm')}</span>`}
    <div class="name"><div class="file ellipsis" title="${esc(f.path)}">${esc(name)}</div>${dir ? `<div class="dir ellipsis">${esc(dir)}</div>` : ''}</div>
    ${lock}${sizeCol && f.size != null ? `<span class="size">${bytes(f.size)}</span>` : ''}
    <span class="actions">${lockBtn}</span>
  </div>`;
}

function matches(path) {
  return !S.filter || path.toLowerCase().includes(S.filter.toLowerCase());
}

function changesTab(changed) {
  const list = changed.filter((f) => matches(f.path));
  const blocked = changed.filter((f) => f.lock && !f.lock.mine);
  const groups = [['modified', 'Modified'], ['new', 'New'], ['deleted', 'Deleted']]
    .map(([s, label]) => {
      const g = list.filter((f) => f.status === s);
      return g.length ? `<div class="group-label">${label} <span class="muted">${g.length}</span></div>${g.map((f) => fileRow(f)).join('')}` : '';
    }).join('');
  const draft = S.draft[S.bucketId] || '';
  if (!S.selected && list.length) S.selected = list.find((f) => f.status === 'modified')?.path || list[0].path;
  return `<div class="split">
    <div class="pane">
      <div class="pane-head"><div class="search grow">${icon('search', 'sm')}<input class="input" data-act="filter" placeholder="Filter changes" value="${esc(S.filter)}"></div></div>
      <div class="pane-body" data-scroll="changes">${groups || `<div class="empty">${icon('checkCircle')}<div class="big">Working copy is clean</div><div>Everything matches the latest commit.</div></div>`}</div>
      ${changed.length ? `<div class="composer">
        ${blocked.length ? `<div class="warn">${icon('lock', 'sm')}<div>${blocked.length} changed file${blocked.length === 1 ? ' is' : 's are'} locked by someone else, so the commit will be refused: ${blocked.map((f) => esc(f.path.split('/').pop())).join(', ')}.</div></div>` : ''}
        <textarea class="input" id="commit-msg" data-act="draft" rows="2" placeholder="Describe what changed">${esc(draft)}</textarea>
        <div class="row"><span class="muted grow" style="font-size:12px">Commits all ${changed.length} change${changed.length === 1 ? '' : 's'} in this bucket</span>
          <button class="btn primary" data-act="commit" ${draft.trim() ? '' : 'disabled'}>${icon('check', 'sm')}Commit<span class="kbd">Ctrl ↵</span></button></div>
      </div>` : ''}
    </div>
    <div class="pane" id="viewer" data-key="${viewerKey()}">${S.selected ? '' : `<div class="empty" style="height:100%">${icon('eye')}<div class="big">Select a file to compare</div></div>`}</div>
  </div>`;
}

function filesTab(files) {
  const live = files.filter((f) => f.status !== 'deleted');
  const filters = { all: () => true, locked: (f) => f.lock, mine: (f) => f.lock?.mine, images: (f) => fileKind(f.path) === 'image', scenes: (f) => ['blender', 'houdini', 'unreal', 'substance'].includes(fileKind(f.path)) };
  const list = live.filter((f) => matches(f.path) && filters[S.fileFilter](f));
  let lastDir = null; let rows = '';
  for (const f of list) {
    const [dir] = splitPath(f.path);
    if (dir !== lastDir) { rows += `<div class="folder-label">${icon('folder', 'sm')}${esc(dir || '/')}</div>`; lastDir = dir; }
    rows += fileRow(f, { check: true });
  }
  const checked = [...S.checked].filter((p) => live.some((f) => f.path === p));
  const checkedFiles = live.filter((f) => S.checked.has(f.path));
  const canLock = checkedFiles.some((f) => !f.lock);
  const canUnlock = checkedFiles.some((f) => f.lock);
  return `<div class="split">
    <div class="pane">
      <div class="pane-head" style="flex-wrap:wrap">
        <div class="search grow">${icon('search', 'sm')}<input class="input" data-act="filter" placeholder="Filter files" value="${esc(S.filter)}"></div>
        <div class="seg">${[['all', 'All'], ['scenes', 'Scenes'], ['images', 'Images'], ['locked', 'Locked'], ['mine', 'Mine']].map(([k, t]) => `<button class="${S.fileFilter === k ? 'on' : ''}" data-act="file-filter" data-f="${k}">${t}</button>`).join('')}</div>
      </div>
      ${checked.length ? `<div class="pane-head" style="background:var(--surface-2);min-height:44px">
        <span class="grow">${checked.length} selected</span>
        <button class="btn sm" data-act="lock-checked" ${canLock ? '' : 'disabled'}>${icon('lock', 'sm')}Lock</button>
        <button class="btn sm" data-act="unlock-checked" ${canUnlock ? '' : 'disabled'}>${icon('unlock', 'sm')}Unlock</button>
        <button class="btn sm ghost" data-act="clear-checked">Clear</button></div>` : ''}
      <div class="pane-body" data-scroll="files">${rows || `<div class="empty">${icon('search')}<div class="big">No files match</div></div>`}</div>
    </div>
    <div class="pane" id="viewer" data-key="${viewerKey()}">${S.selected ? '' : `<div class="empty" style="height:100%">${icon('image')}<div class="big">Select a file to preview</div><div>Tick files to lock or unlock several at once.</div></div>`}</div>
  </div>`;
}

function historyTab() {
  const h = S.history || [];
  if (!h.length) return `<div class="empty" style="flex:1">${icon('history')}<div class="big">No commits yet</div></div>`;
  if (!S.commitId) S.commitId = h[0].id;
  const commits = h.map((c, i) => `<div class="commit-item ${S.commitId === c.id ? 'on' : ''}" data-act="select-commit" data-id="${c.id}">
      <div class="msg">${esc(c.message)}</div>
      <div class="meta"><span>${ago(c.createdAt)}</span><span class="delta">${c.added ? `<span class="a">+${c.added}</span>` : ''}${c.modified ? `<span class="m">~${c.modified}</span>` : ''}${c.deleted ? `<span class="d">−${c.deleted}</span>` : ''}</span>
      <span class="grow"></span>${i === 0 ? '<span class="pill plain">Latest</span>' : ''}<span class="mono faint">${c.id.slice(0, 8)}</span></div>
    </div>`).join('');
  const changes = S.commitChanges;
  const commit = h.find((c) => c.id === S.commitId);
  const list = changes == null ? `<div class="loading"><div class="spinner"></div></div>`
    : changes.length ? changes.map((c) => fileRow({ path: c.path, status: c.status, size: null }, { actions: false, sizeCol: false })).join('')
      : `<div class="empty">No file changes in this commit</div>`;
  if (changes?.length && (!S.selected || !changes.some((c) => c.path === S.selected))) S.selected = changes[0].path;
  return `<div class="split three">
    <div class="pane"><div class="pane-body" data-scroll="commits">${commits}</div></div>
    <div class="pane">
      <div class="pane-head" style="display:block">
        <div style="font-weight:600">${esc(commit?.message || '')}</div>
        <div class="muted" style="font-size:12px;margin-top:3px">${commit ? `${new Date(commit.createdAt).toLocaleString()} · ${commit.files} files in snapshot` : ''}</div>
      </div>
      <div class="pane-body" data-scroll="commit-files">${list}</div>
    </div>
    <div class="pane" id="viewer" data-key="${viewerKey()}"></div>
  </div>`;
}

function viewerKey() {
  if (S.view !== 'bucket' || !S.selected) return '';
  if (S.tab === 'history') {
    const c = S.commitChanges?.find((x) => x.path === S.selected);
    return c ? `h:${S.commitId}:${c.path}` : '';
  }
  const f = S.files?.files.find((x) => x.path === S.selected);
  return f ? `${S.tab}:${f.path}:${f.hash}:${f.committedHash}:${f.status}:${f.lock?.owner ?? ''}` : '';
}

function mountViewer(host) {
  if (!S.selected) return;
  const bucketId = S.bucketId;
  const short = (h) => (h ? h.slice(0, 8) : '');
  if (S.tab === 'history') {
    const c = S.commitChanges?.find((x) => x.path === S.selected);
    if (!c) return;
    const idx = S.history.findIndex((x) => x.id === S.commitId);
    const prev = S.history[idx + 1];
    const restore = c.hash ? `<button class="btn sm" data-v="restore" title="Put this version back in the working folder">${icon('undo', 'sm')}Restore this version</button>` : '';
    showFile(host, {
      bucketId, path: c.path, status: c.status,
      left: c.previousHash ? { source: object(c.previousHash), label: 'Before', sub: prev ? prev.id.slice(0, 8) : short(c.previousHash) } : null,
      right: c.hash ? { source: object(c.hash), label: 'This commit', sub: S.commitId.slice(0, 8) } : null,
      actions: restore,
    }).then(() => {
      host.querySelector('[data-v="restore"]')?.addEventListener('click', async () => {
        const ok = await confirmBox('Restore this version?', `<span class="mono">${esc(c.path)}</span> in your working folder will be replaced by the version from “${esc(S.history[idx].message)}”. Uncommitted edits to it are lost.`, 'Restore', true);
        if (ok) await act({ type: 'revert', bucketId, path: c.path, commitId: S.commitId }, `Restored ${c.path.split('/').pop()}`);
      });
    });
    return;
  }
  const f = S.files?.files.find((x) => x.path === S.selected);
  if (!f) return;
  const latest = S.history?.[0];
  const committed = f.committedHash ? { source: object(f.committedHash), label: 'Committed', sub: latest ? latest.id.slice(0, 8) : '' } : null;
  const work = f.status !== 'deleted' ? { source: working, label: 'Working copy' } : null;
  const lockBtn = f.lock
    ? `<button class="btn sm" data-v="unlock">${icon('unlock', 'sm')}${f.lock.mine ? 'Unlock' : 'Break lock'}</button>`
    : (f.status !== 'deleted' ? `<button class="btn sm" data-v="lock">${icon('lock', 'sm')}Lock</button>` : '');
  const discard = f.status === 'modified' || f.status === 'deleted' ? `<button class="btn sm ghost" data-v="discard" title="Throw away the local change and go back to the committed version">${icon('undo', 'sm')}Discard</button>` : '';
  const lockNote = f.lock ? `<span class="pill lock" title="since ${new Date(f.lock.createdAt).toLocaleString()}">${icon('lock', 'sm')}${f.lock.mine ? 'Locked by you' : `Locked by ${esc(f.lock.owner)}`}</span>` : '';
  const isChange = S.tab === 'changes';
  showFile(host, {
    bucketId, path: f.path, status: isChange ? f.status : null,
    left: isChange ? committed : null,
    right: isChange ? work : (work || committed),
    actions: `${lockNote}${isChange ? discard : ''}${lockBtn}`,
  }).then(() => {
    host.querySelector('[data-v="lock"]')?.addEventListener('click', () => lockFiles([f.path]));
    host.querySelector('[data-v="unlock"]')?.addEventListener('click', () => unlockFiles([f.path]));
    host.querySelector('[data-v="discard"]')?.addEventListener('click', async () => {
      const ok = await confirmBox('Discard local changes?', `<span class="mono">${esc(f.path)}</span> goes back to the committed version. This cannot be undone.`, 'Discard', true);
      if (ok) await act({ type: 'revert', bucketId, path: f.path }, `Discarded changes to ${f.path.split('/').pop()}`);
    });
  });
}

async function lockFiles(paths) {
  const res = await act({ type: 'lock', bucketId: S.bucketId, paths }, (r) => lockSummary(r, 'Locked'));
  if (res) S.checked.clear();
}

async function unlockFiles(paths) {
  const files = S.files.files.filter((f) => paths.includes(f.path) && f.lock);
  const others = files.filter((f) => !f.lock.mine);
  if (others.length) {
    const ok = await confirmBox(
      `Break ${others.length === 1 ? 'a lock' : `${others.length} locks`} held by someone else?`,
      `${others.map((f) => `<div class="row" style="margin-top:6px">${avatar(f.lock.owner)}<span class="mono">${esc(f.path)}</span></div>`).join('')}<div style="margin-top:10px">They may be in the middle of editing. Their next commit of these files could overwrite yours.</div>`,
      'Break lock', true,
    );
    if (!ok) return;
  }
  const res = await act({ type: 'unlock', bucketId: S.bucketId, paths: files.map((f) => f.path), force: others.length > 0 }, (r) => lockSummary(r, 'Unlocked'));
  if (res) S.checked.clear();
}

function lockSummary(r, verb) {
  if (!Array.isArray(r)) return `${verb}`;
  const held = r.filter((x) => /held/i.test(x.status));
  const ok = r.length - held.length;
  return held.length ? `${verb} ${ok}. Already locked by others: ${held.map((h) => `${h.file} (${h.owner})`).join(', ')}` : `${verb} ${ok} file${ok === 1 ? '' : 's'}`;
}

async function commit() {
  // The user can switch bucket while the CLI runs, so keep the one committed.
  const bucketId = S.bucketId;
  const message = (S.draft[bucketId] || '').trim();
  if (!message) return;
  const res = await act({ type: 'commit', bucketId, message },
    (r) => (r && r.files != null ? `Committed ${r.new.length + r.modified.length + r.deleted.length} change(s) to ${nameOf(bucketId)}` : 'Committed'));
  if (res) {
    S.draft[bucketId] = '';
    if (S.bucketId === bucketId) S.selected = null;
    render();
  }
}

// ---------------- activity ----------------

function activityView() {
  const items = S.activity;
  let body = `<div class="loading"><div class="spinner"></div></div>`;
  if (items) {
    let last = ''; body = '';
    for (const it of items) {
      const day = dayLabel(it.commit.createdAt);
      if (day !== last) { body += `<div class="day">${esc(day)}</div>`; last = day; }
      body += `<div class="timeline-item" data-act="goto-commit" data-bucket="${it.commit.bucketId}" data-id="${it.commit.id}">
        <span class="time">${clock(it.commit.createdAt)}</span>
        <span class="chip"><span class="dot" style="--c:${S.colors[it.commit.bucketId]}"></span>${esc(it.bucketName)}</span>
        <span class="msg grow ellipsis">${esc(it.commit.message)}</span>
        <span class="muted">${it.commit.files} files</span><span class="mono faint">${it.commit.id.slice(0, 8)}</span>
      </div>`;
    }
    if (!items.length) body = `<div class="empty">${icon('activity')}<div class="big">No commits yet</div></div>`;
  }
  return `<div class="page-head"><div class="grow"><h1>Activity</h1><div class="sub">Every commit across the pipeline, newest first.</div></div></div>
    <div class="page-body" data-scroll="activity" style="max-width:1000px">${body}</div>`;
}

// ---------------- locks ----------------

function locksView() {
  const all = S.board.locks;
  const list = all.filter((l) => S.lockFilter === 'all' || (S.lockFilter === 'mine' ? l.mine : !l.mine));
  const rows = list.map((l) => {
    const [dir, name] = splitPath(l.path);
    return `<tr>
      <td><div class="row"><span style="color:${kindColor(l.path)}">${icon(kindIcon(l.path), 'sm')}</span><div><div style="font-weight:500">${esc(name)}</div><div class="faint" style="font-size:11px">${esc(dir)}</div></div></div></td>
      <td><span class="chip"><span class="dot" style="--c:${S.colors[l.bucketId]}"></span>${esc(nameOf(l.bucketId))}</span></td>
      <td><div class="row">${avatar(l.owner)}<span>${l.mine ? '<b>You</b>' : esc(l.owner)}</span></div></td>
      <td class="muted" title="${new Date(l.createdAt).toLocaleString()}">${ago(l.createdAt)}</td>
      <td class="actions">
        <button class="btn sm ghost" data-act="goto-file" data-bucket="${l.bucketId}" data-path="${esc(l.path)}">${icon('eye', 'sm')}View</button>
        <button class="btn sm ${l.mine ? '' : 'danger'}" data-act="unlock-row" data-bucket="${l.bucketId}" data-path="${esc(l.path)}" data-owner="${esc(l.owner)}" data-mine="${l.mine}">${icon('unlock', 'sm')}${l.mine ? 'Unlock' : 'Break lock'}</button>
      </td></tr>`;
  }).join('');
  return `<div class="page-head"><div class="grow"><h1>Locks</h1><div class="sub">Locked files can only be committed by the person holding the lock. Lock scenes and maps before you open them.</div></div>
      <div class="seg">${[['all', `All ${all.length}`], ['mine', `Mine ${all.filter((l) => l.mine).length}`], ['others', 'Others']].map(([k, t]) => `<button class="${S.lockFilter === k ? 'on' : ''}" data-act="lock-filter" data-f="${k}">${t}</button>`).join('')}</div></div>
    <div class="page-body" data-scroll="locks">${list.length ? `<table class="table"><thead><tr><th>File</th><th>Bucket</th><th>Held by</th><th>Since</th><th></th></tr></thead><tbody>${rows}</tbody></table>`
      : `<div class="empty">${icon('unlock')}<div class="big">No locks</div><div>Lock a file from a bucket's Files tab.</div></div>`}</div>`;
}

// ---------------- welcome & settings ----------------

function renderWelcome() {
  const st = S.status;
  root.innerHTML = `<div class="welcome"><div class="welcome-card">
    <div class="logo"><img src="assets/logo.png" alt=""><div><h1>Buckets</h1><p>Pipeline board, visual diffs and file locks for your game assets.</p></div></div>
    ${st?.error ? `<div class="alert">${icon('alert')}<div>${esc(st.error)}</div></div>` : ''}
    ${st && !st.cli.path ? `<div class="alert info">${icon('info')}<div>The <span class="mono">buckets</span> CLI was not found. Browsing works without it, but committing, locking and finalizing need it. Set its path in Settings.</div></div>` : ''}
    <div class="row"><button class="btn primary" data-act="choose-repo">${icon('folder', 'sm')}Open repository…</button>
      <button class="btn ghost" data-act="settings">${icon('settings', 'sm')}Settings</button></div>
    ${st?.recent?.length ? `<div><div class="nav-label" style="padding-left:2px">Recent</div><div class="recent">${st.recent.map((p) => `<button data-act="open-recent" data-path="${esc(p)}">${icon('folder')}<div class="grow"><div style="font-weight:600">${esc(p.split(/[\\/]/).pop())}</div><div class="muted mono ellipsis">${esc(p)}</div></div>${icon('chevronRight', 'sm')}</button>`).join('')}</div></div>` : ''}
    <p class="faint" style="font-size:12px">Pick the folder that holds <span class="mono">.buckets</span>, or any folder inside it. New repositories are made with <span class="mono">buckets init</span>.</p>
  </div></div>`;
}

async function settingsDialog() {
  const st = S.status;
  const s = st.settings;
  const result = await modal({
    title: 'Settings',
    wide: true,
    body: `<div class="field"><label>buckets CLI</label>
        <input class="input mono" name="cli" placeholder="${esc(st.cli.path || 'Found automatically on PATH')}" value="${esc(s.cliPath || '')}">
        <div class="hint">${st.cli.path ? `Using ${esc(st.cli.path)}${st.cli.version ? ` · ${esc(st.cli.version)}` : ''}` : 'Not found. Install buckets or enter the full path.'}</div></div>
      <div class="field"><label>Appearance</label>
        <div class="seg" data-g="theme">${[['system', 'System'], ['dark', 'Dark'], ['light', 'Light']].map(([k, t]) => `<button type="button" class="${s.theme === k ? 'on' : ''}" data-theme="${k}">${t}</button>`).join('')}</div></div>
      <div class="field"><div class="row"><button type="button" class="switch ${s.similarityCheck ? 'on' : ''}" data-v="sim"></button><label style="margin:0">Warn about duplicate expectations</label></div>
        <div class="hint">Uses semantic search on the server. The first use downloads a 90 MB model, so adding an expectation can take a while.</div></div>
      ${st.repo ? `<div class="field"><label>Remote store for ${esc(st.repo.name)}</label>
        <input class="input mono" name="remote" placeholder="/mnt/nas/buckets-store" value="${esc(st.repo.remote || '')}">
        <div class="hint">A shared folder (e.g. a NAS mount) where Push copies committed objects and Pull fetches them.</div></div>` : ''}
      <div class="field"><label>Identity</label><div class="row">${avatar(st.me)}<span class="mono">${esc(st.me)}</span></div>
        <div class="hint">Locks are held under this name. Set <span class="mono">BUCKETS_USER</span> before starting the app to change it.</div></div>`,
    onMount: (m) => {
      m.querySelectorAll('[data-theme]').forEach((b) => b.addEventListener('click', () => {
        m.querySelectorAll('[data-theme]').forEach((x) => x.classList.toggle('on', x === b));
        S.status.settings.theme = b.dataset.theme; applyTheme();
      }));
      m.querySelector('[data-v="sim"]').addEventListener('click', (e) => e.currentTarget.classList.toggle('on'));
    },
    actions: [
      { label: 'Cancel', kind: 'ghost', value: 'cancel' },
      {
        label: 'Save', kind: 'primary',
        value: (m) => ({
          cliPath: m.querySelector('[name=cli]').value,
          theme: m.querySelector('[data-theme].on')?.dataset.theme || 'system',
          similarityCheck: m.querySelector('[data-v="sim"]').classList.contains('on'),
          remote: m.querySelector('[name=remote]')?.value.trim(),
        }),
      },
    ],
  });
  if (!result || result === 'cancel') { S.status.settings.theme = s.theme; applyTheme(); return; }
  try {
    const { remote, ...patch } = result;
    await api.settings(patch);
    if (st.repo && remote && remote !== (st.repo.remote || '')) await api.action({ type: 'remoteSet', path: remote });
    S.status = await api.status();
    applyTheme();
    toast('Settings saved');
    render({ keepViewer: true });
  } catch (e) { toast(String(e), 'err'); }
}

// ---------------- events ----------------

async function openBucket(id, tab, commitId = null) {
  const switching = S.bucketId !== id;
  S.view = 'bucket'; S.bucketId = id;
  if (tab) S.tab = tab;
  if (switching) { S.files = null; S.history = null; S.selected = null; S.filter = ''; S.checked.clear(); S.commitId = null; S.commitChanges = null; }
  if (commitId) { S.commitId = commitId; S.commitChanges = null; S.selected = null; }
  render();
  try {
    await loadBucket(!switching || !!commitId);
    if (S.tab === 'history') await loadCommit();
    render();
  } catch (e) { toast(String(e), 'err'); }
}

async function loadCommit() {
  if (!S.history?.length) return;
  if (!S.commitId) S.commitId = S.history[0].id;
  const id = S.commitId;
  S.commitChanges = null;
  const changes = await api.commitChanges(S.bucketId, id);
  if (S.commitId === id) S.commitChanges = changes;
}

root.addEventListener('click', async (e) => {
  const el = e.target.closest('[data-act]');
  if (!el) return;
  const a = el.dataset.act;
  const id = el.dataset.id;
  switch (a) {
    case 'nav':
      S.view = el.dataset.view; S.filter = '';
      if (S.view === 'activity') { render(); S.activity = await api.activity().catch((err) => { toast(String(err), 'err'); return []; }); }
      render(); break;
    case 'open-bucket': e.stopPropagation(); await openBucket(id, el.dataset.tab); break;
    case 'open-folder': e.stopPropagation(); api.open(id).catch((err) => toast(String(err), 'err')); break;
    case 'tab':
      S.tab = el.dataset.tab; S.selected = null; S.filter = '';
      render();
      if (S.tab === 'history') { await loadCommit().catch((err) => toast(String(err), 'err')); render(); }
      break;
    case 'select-file':
      if (e.target.closest('.actions, .check')) return;
      S.selected = el.dataset.path; render({ keepViewer: true }); break;
    case 'check-file': {
      const p = el.dataset.path;
      if (S.checked.has(p)) S.checked.delete(p); else S.checked.add(p);
      render({ keepViewer: true }); break;
    }
    case 'clear-checked': S.checked.clear(); render({ keepViewer: true }); break;
    case 'lock-checked': lockFiles([...S.checked].filter((p) => !S.files.files.find((f) => f.path === p)?.lock)); break;
    case 'unlock-checked': unlockFiles([...S.checked]); break;
    case 'lock': lockFiles([el.dataset.path]); break;
    case 'unlock': unlockFiles([el.dataset.path]); break;
    case 'file-filter': S.fileFilter = el.dataset.f; render({ keepViewer: true }); break;
    case 'lock-filter': S.lockFilter = el.dataset.f; render(); break;
    case 'select-commit':
      S.commitId = id; S.selected = null; S.commitChanges = null; render();
      await loadCommit().catch((err) => toast(String(err), 'err')); render(); break;
    case 'goto-commit':
      await openBucket(el.dataset.bucket, 'history', id); break;
    case 'goto-file':
      await openBucket(el.dataset.bucket, 'files'); S.selected = el.dataset.path; render(); break;
    case 'unlock-row': {
      const mine = el.dataset.mine === 'true';
      if (!mine) {
        const ok = await confirmBox(`Break ${el.dataset.owner}'s lock?`, `<span class="mono">${esc(el.dataset.path)}</span><div style="margin-top:8px">They may be in the middle of editing this file.</div>`, 'Break lock', true);
        if (!ok) return;
      }
      act({ type: 'unlock', bucketId: el.dataset.bucket, paths: [el.dataset.path], force: !mine }, 'Lock released'); break;
    }
    case 'pick-pebble':
      if (S.pebbles.has(id)) S.pebbles.delete(id); else S.pebbles.add(id);
      render(); break;
    case 'toggle-done':
      if (S.showDone.has(id)) S.showDone.delete(id); else S.showDone.add(id);
      render(); break;
    case 'finalize': finalize(id); break;
    case 'new-expectation': e.stopPropagation(); newExpectation(id); break;
    case 'commit': commit(); break;
    case 'push': act({ type: 'push' }, (r) => (r?.objects_copied != null ? `Pushed ${r.objects_copied} object(s), ${r.objects_already_there} already there` : 'Pushed')); break;
    case 'pull': pull(false); break;
    case 'refresh': refresh(); break;
    case 'settings': settingsDialog(); break;
    case 'choose-repo': api.chooseRepo().catch((err) => toast(String(err), 'err')); break;
    case 'open-recent': api.openRepo(el.dataset.path).catch((err) => toast(String(err), 'err')); break;
    case 'repo-menu': {
      const r = el.getBoundingClientRect();
      const recent = (S.status.recent || []).filter((p) => p !== S.status.repo.root);
      menu(r.left, r.bottom + 6, [
        ...(recent.length ? [{ label: 'Recent' }, ...recent.map((p) => ({ icon: 'folder', text: p.split(/[\\/]/).pop(), hint: shortPath(p), run: () => api.openRepo(p).catch((err) => toast(String(err), 'err')) })), '-'] : []),
        { icon: 'folder', text: 'Open repository…', run: () => api.chooseRepo().catch((err) => toast(String(err), 'err')) },
        { icon: 'external', text: 'Show repository folder', run: () => bucketsRootOpen() },
      ]);
      break;
    }
    case 'bucket-menu': {
      const r = el.getBoundingClientRect();
      menu(r.right - 240, r.bottom + 6, [
        { icon: 'stash', text: 'Stash changes', run: () => stash(false) },
        { icon: 'undo', text: 'Restore last stash', run: () => stash(true) },
        '-',
        { icon: 'download', text: 'Pull, overwriting local edits', run: () => pull(true) },
        { icon: 'plus', text: 'New expectation here', run: () => newExpectation(S.bucketId) },
      ]);
      break;
    }
    default: break;
  }
});

function bucketsRootOpen() {
  const first = S.board?.buckets[0];
  if (first) api.open(first.id, null, false).catch((err) => toast(String(err), 'err'));
}

async function pull(force) {
  if (force) {
    const ok = await confirmBox('Pull and overwrite local edits?', 'Files you changed but did not commit are replaced by the latest committed versions.', 'Pull and overwrite', true);
    if (!ok) return;
  }
  act({ type: 'pull', bucketId: S.bucketId, force }, (r) => {
    if (!r || typeof r !== 'object') return 'Pulled';
    const parts = [`${r.updated?.length ?? 0} updated`, `${r.removed?.length ?? 0} removed`];
    if (r.conflicts?.length) parts.push(`${r.conflicts.length} skipped because of local edits`);
    return `Pulled ${nameOf(S.bucketId)}: ${parts.join(', ')}`;
  });
}

async function stash(pop) {
  if (!pop) {
    const msg = await modal({
      title: 'Stash changes', subtitle: 'Set your uncommitted changes aside and go back to the last commit. Restore them later from the same menu.',
      body: '<div class="field"><label>Note (optional)</label><input class="input" name="m" placeholder="e.g. trying a darker mortar" autofocus></div>',
      actions: [{ label: 'Cancel', kind: 'ghost', value: 'cancel' }, { label: 'Stash', kind: 'primary', value: (m) => ({ m: m.querySelector('[name=m]').value }) }],
    });
    if (!msg || msg === 'cancel') return;
    act({ type: 'stash', bucketId: S.bucketId, pop: false, message: msg.m || null }, 'Changes stashed');
  } else {
    act({ type: 'stash', bucketId: S.bucketId, pop: true, message: null }, 'Stash restored');
  }
}

root.addEventListener('input', (e) => {
  const el = e.target;
  if (el.dataset.act === 'filter') {
    S.filter = el.value;
    const pos = el.selectionStart;
    render({ keepViewer: true });
    const again = root.querySelector('[data-act="filter"]');
    again?.focus(); again?.setSelectionRange(pos, pos);
  }
  if (el.dataset.act === 'draft') {
    S.draft[S.bucketId] = el.value;
    const btn = root.querySelector('[data-act="commit"]');
    if (btn) btn.disabled = !el.value.trim();
  }
});

document.addEventListener('keydown', (e) => {
  if (document.querySelector('.scrim')) return;
  const typing = e.target.closest?.('input, textarea, select');
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && e.target.id === 'commit-msg') { e.preventDefault(); commit(); return; }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r') { e.preventDefault(); refresh(); return; }
  if (typing) { if (e.key === 'Escape') e.target.blur(); return; }
  if (e.key === '/' && root.querySelector('[data-act="filter"]')) { e.preventDefault(); root.querySelector('[data-act="filter"]').focus(); return; }
  if (S.view === 'bucket' && (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'j' || e.key === 'k')) {
    const items = [...root.querySelectorAll('[data-act="select-file"]')];
    if (!items.length) return;
    const i = items.findIndex((x) => x.dataset.path === S.selected);
    const n = Math.min(items.length - 1, Math.max(0, i + (e.key === 'ArrowDown' || e.key === 'j' ? 1 : -1)));
    S.selected = items[n].dataset.path; e.preventDefault();
    render({ keepViewer: true });
    root.querySelector('.list-item.on')?.scrollIntoView({ block: 'nearest' });
  }
});

// Refresh when the window comes back into focus (artists save from other apps),
// and every 20 s while visible.
window.addEventListener('focus', () => { if (S.status?.repo) refresh({ quiet: true }); });
setInterval(() => {
  if (document.visibilityState === 'visible' && S.status?.repo && !document.querySelector('.scrim') && !S.busy) refresh({ quiet: true });
}, 20000);

// ---------------- start ----------------

(async function start() {
  try {
    S.status = await api.status();
  } catch (e) {
    S.status = { repo: null, error: String(e), recent: [], cli: {}, settings: { theme: 'system' }, me: '' };
  }
  applyTheme();
  if (!S.status.repo) { renderWelcome(); return; }
  render();
  try { await loadBoard(); } catch (e) { toast(String(e), 'err'); }
  if (prefs.view && prefs.view !== 'bucket') S.view = prefs.view;
  render();
})();

window.addEventListener('beforeunload', () => { prefs.view = S.view; savePrefs(); });
