import { $, $$, esc, debounce, storage, fmtTime } from '../dom.js';
import { t } from '../i18n.js';
import { icon } from '../icons.js';
import { toast, confirmModal } from '../toast.js';
import { state, ipc, loadPrompts, rebuildIndex, filterPrompts, wheelPrompts, allTags, relTime, extractVars, customVars, renderVars } from '../store.js';

let editingId = null;
let dirty = false;

export async function renderPrompts() {
  renderGrid();
  renderWheelMirror();
  renderTags();
}

export function filteredList() { return filterPrompts(); }

function firstLine(content) {
  return String(content || '').split('\n').find(l => l.trim()) || '';
}

function renderGrid() {
  const grid = $('#grid');
  if (!grid) return;
  grid.className = 'grid' + (state.viewMode === 'list' ? ' listmode' : '');
  const list = filterPrompts();
  grid.innerHTML = list.length ? list.map(p => `
    <div class="card" data-id="${p.id}">
      <div class="card-top">
        <div class="card-name" title="${esc(p.name)}">${esc(p.name)}</div>
        <button class="star ${p.is_pinned ? 'on' : ''}" data-star="${p.id}" title="${esc(t('f.pinned'))}" aria-label="pin">${icon('star', 16)}</button>
      </div>
      <div class="card-preview">${esc(firstLine(p.content))}</div>
      <div class="card-foot">
        ${(p.tags || []).slice(0, 4).map(x => `<span class="ctag">${esc(x)}</span>`).join('')}
        <span class="cmeta">${p.usage_count > 0 ? t('prompt.used', { n: p.usage_count }) + ' · ' : ''}${esc(relTime(p.last_used_at))}</span>
      </div>
    </div>`).join('')
    : `<div class="empty">${esc(t('empty.prompts'))}</div>`;
}

function renderWheelMirror() {
  const mirror = $('#wheelMirror');
  if (!mirror) return;
  const pins = wheelPrompts();
  mirror.innerHTML = pins.length
    ? pins.map((p, i) => `<div class="wm-item" data-wheel-id="${p.id}">
        <span class="num">${i + 1}</span><span class="nm" title="${esc(p.name)}">${esc(p.name)}</span>
        <span class="drag" data-drag="${p.id}" tabindex="0" role="button" aria-label="${esc(t('wheel.dragHint'))}" title="${esc(t('wheel.dragHint'))}">${icon('grip-vertical', 14)}</span>
      </div>`).join('')
    : `<div class="wm-empty">${esc(t('wheel.empty'))}</div>`;
  const btn = $('#wheelSortBtn');
  if (btn) {
    btn.textContent = state.wheelSort === 'manual' ? t('wheel.sortManual') : t('wheel.sortAuto');
    btn.title = state.wheelSort === 'manual' ? t('wheel.sortSwitchToAuto') : t('wheel.sortSwitchToManual');
  }
  bindMirrorDrag();
}

function renderTags() {
  const cloud = $('#tagsCloud');
  if (!cloud) return;
  cloud.innerHTML = allTags().map(x =>
    `<span class="tagchip ${state.tagFilter === x ? 'on' : ''}" data-tag="${esc(x)}">${esc(x)}</span>`).join('');
}

/* ---- drawer ---- */
export function openDrawer(id, draft) {
  const drawer = $('#drawer');
  if (!drawer) { console.warn('[prompts] #drawer missing'); return; }
  const setV = (sel, v) => { const n = $(sel); if (n) n.value = v; };
  const p = state.prompts.find(x => x.id === id) || null;
  editingId = id;
  const title = $('#drawerTitle');
  if (title) title.textContent = p ? t('drawer.edit') : t('drawer.new');
  setV('#fName', p ? p.name : (draft?.name || ''));
  setV('#fContent', p?.content || '');
  setV('#fTags', (p?.tags || []).join(', '));
  // drafts arriving from the wheel's quick-create gesture intend a petal, so
  // the pin switch starts on (the user can uncheck before saving).
  $('#fPin')?.classList.toggle('on', p ? !!p.is_pinned : !!draft?.pin);
  const meta = $('#fMeta');
  if (meta) meta.innerHTML = p
    ? `<span>${esc(t('f.version', { v: p.version || 1, t: p.updated_at || '—' }))}</span>`
    : (draft ? `<span>${esc(t('drawer.fromWheel'))}</span>` : '');
  updateVarHint();
  dirty = false;
  drawer.classList.add('show');
  $('#drawerMask')?.classList.add('show');
  drawer.setAttribute('aria-hidden', 'false');
  // a seeded draft already carries a name — drop the cursor where content goes
  (draft?.name ? $('#fContent') : $('#fName'))?.focus();
}

export function drawerOpen() { return $('#drawer')?.classList.contains('show'); }

export async function closeDrawer() {
  if (dirty) {
    const ok = await confirmModal({ title: t('common.cancel'), body: t('del.dirtyAsk'), okText: t('common.ok'), danger: true });
    if (!ok) return;
  }
  dirty = false;
  $('#drawer')?.classList.remove('show');
  $('#drawerMask')?.classList.remove('show');
  $('#drawer')?.setAttribute('aria-hidden', 'true');
}
export function forceCloseDrawer() {
  dirty = false;
  $('#drawer')?.classList.remove('show');
  $('#drawerMask')?.classList.remove('show');
  $('#drawer')?.setAttribute('aria-hidden', 'true');
}

function updateVarHint() {
  const hint = $('#varHint');
  const contentEl = $('#fContent');
  if (!hint || !contentEl) return;
  const vars = extractVars(contentEl.value);
  const custom = customVars(contentEl.value);
  hint.innerHTML = vars.length
    ? `<span>${esc(t('f.vars'))}:</span> ${vars.map(v => `<code>{{${esc(v)}}}</code>`).join(' ')}`
      + (custom.length ? ` <span class="dim">·</span> <span>${esc(t('f.autoVars'))}</span>` : ` <span>${esc(t('f.autoVars'))}</span>`)
    : '';
}

async function saveDrawer() {
  const nameEl = $('#fName'), contentEl = $('#fContent');
  if (!nameEl || !contentEl) return;
  const name = nameEl.value.trim();
  const content = contentEl.value;
  if (!name || !content.trim()) { toast('warn', t('err.generic', { e: t('f.name') + ' / ' + t('f.content') })); return; }
  const tags = ($('#fTags')?.value || '').split(/[,，]/).map(s => s.trim()).filter(Boolean);
  const p = state.prompts.find(x => x.id === editingId);
  const payload = {
    id: editingId,
    name,
    content,
    tags,
    content_type: p?.content_type || 'text',
    variables_json: p?.variables_json || null,
    // internal fields — not user-editable; preserve stored values so a save
    // never wipes scopes or a manual pin order written by set_pin_order
    app_scopes_json: p?.app_scopes_json ?? null,
    inject_order: p?.inject_order ?? null,
    version: (p?.version || 0) + (editingId ? 1 : 0) || 1,
    updated_at: null,
  };
  try {
    if (editingId) await ipc('update_prompt', { prompt: payload });
    else editingId = await ipc('create_prompt', { prompt: payload });
    const pinOn = !!$('#fPin')?.classList.contains('on');
    if (!!p?.is_pinned !== pinOn) await ipc('toggle_prompt_pin', { id: editingId });
    forceCloseDrawer();
    await loadPrompts(); rebuildIndex(); renderPrompts();
    toast('ok', t('toast.saved'));
  } catch { /* ipc already toasted */ }
}

/* ---- wheel mirror manual reorder ----
   Pointer-event implementation, not HTML5 drag-and-drop: toggling `draggable`
   inside mousedown races WebView2's drag detector (dragstart may never fire),
   and the old path silently preventDefault'd whenever wheelSort was 'auto'
   (the default) — users saw a grip icon but nothing moved.

   Model: pointerdown arms a gesture (mouse: anywhere on the row — the row has
   no click action; touch/pen: only on the grip handle, so a vertical press on
   a row still scrolls .leftcol via touch-action: pan-y). Once the pointer
   passes DRAG_THRESHOLD the row lifts out of flow (position:fixed, follows
   the pointer) and a .wm-insert placeholder marks the landing slot — sibling
   rows reflow around it through normal flex layout, no FLIP math. Drop walks
   the container children to build the id sequence, then commits via
   set_pin_order. A reorder is itself an explicit ordering intent, so a commit
   from auto mode flips wheelSort to 'manual' and persists it — the dropped
   order must still be there after reload. Keyboard: focus the grip (or row)
   and use ArrowUp/ArrowDown/Home/End for the same commit path. */
const DRAG_THRESHOLD = 5;
let mirrorDragBound = false;
let mirrorDrag = null;
let suppressClickUntil = 0;

function bindMirrorDrag() {
  const mirror = $('#wheelMirror');
  if (!mirror || mirrorDragBound) return;
  mirrorDragBound = true;
  mirror.addEventListener('pointerdown', onMirrorPointerDown);
  mirror.addEventListener('pointermove', onMirrorPointerMove);
  mirror.addEventListener('pointerup', onMirrorPointerUp);
  mirror.addEventListener('pointercancel', onMirrorPointerCancel);
  mirror.addEventListener('lostpointercapture', onMirrorPointerCancel);
  mirror.addEventListener('keydown', onMirrorKeyDown);
  // a real drag must not end in a click landing on the row underneath
  mirror.addEventListener('click', e => {
    if (Date.now() < suppressClickUntil) { e.preventDefault(); e.stopPropagation(); }
  }, true);
}

function onMirrorPointerDown(e) {
  if (mirrorDrag) return;
  const row = e.target.closest('.wm-item');
  if (!row || !$('#wheelMirror')?.contains(row)) return;
  if (e.pointerType === 'mouse' && e.button !== 0) return;
  const onHandle = !!e.target.closest('.drag');
  if (e.pointerType !== 'mouse' && !onHandle) return;
  if (e.pointerType !== 'mouse') e.preventDefault();
  mirrorDrag = { pointerId: e.pointerId, row, id: +row.dataset.wheelId, startY: e.clientY, active: false, marker: null };
  try { row.setPointerCapture(e.pointerId); } catch { /* pointer already gone */ }
}

function onMirrorPointerMove(e) {
  const d = mirrorDrag;
  if (!d || e.pointerId !== d.pointerId) return;
  const dy = e.clientY - d.startY;
  if (!d.active) {
    if (Math.abs(dy) < DRAG_THRESHOLD) return;
    startMirrorDrag(d);
  }
  d.row.style.transform = `translate3d(0, ${dy}px, 0)`;
  placeMirrorMarker(d, e.clientY);
  autoScrollMirror(e.clientY);
}

function startMirrorDrag(d) {
  const mirror = $('#wheelMirror');
  d.active = true;
  const rect = d.row.getBoundingClientRect();
  const marker = document.createElement('div');
  marker.className = 'wm-insert';
  marker.style.height = `${rect.height}px`;
  marker.setAttribute('aria-hidden', 'true');
  d.marker = marker;
  mirror.insertBefore(marker, d.row);
  d.row.classList.add('dragging');
  Object.assign(d.row.style, {
    left: `${rect.left}px`, top: `${rect.top}px`, width: `${rect.width}px`,
  });
  mirror.classList.add('drag-live');
}

function placeMirrorMarker(d, y) {
  const mirror = $('#wheelMirror');
  let before = null;
  for (const r of $$('.wm-item', mirror)) {
    if (r === d.row) continue;
    const rc = r.getBoundingClientRect();
    if (y < rc.top + rc.height / 2) { before = r; break; }
  }
  if (before) mirror.insertBefore(d.marker, before);
  else mirror.appendChild(d.marker);
}

// edge-scroll the .leftcol while the pointer hovers near its top/bottom
function autoScrollMirror(y) {
  const col = $('#wheelMirror')?.closest('.leftcol');
  if (!col) return;
  const rc = col.getBoundingClientRect();
  if (y < rc.top + 28) col.scrollTop -= 8;
  else if (y > rc.bottom - 28) col.scrollTop += 8;
}

async function onMirrorPointerUp(e) {
  const d = mirrorDrag;
  if (!d || e.pointerId !== d.pointerId) return;
  mirrorDrag = null;
  if (!d.active) return;
  suppressClickUntil = Date.now() + 350;
  const ids = [];
  for (const child of $('#wheelMirror').children) {
    if (child === d.marker) ids.push(d.id);
    else if (child !== d.row && child.classList?.contains('wm-item')) ids.push(+child.dataset.wheelId);
  }
  cleanupMirrorDrag(d);
  if (ids.join(',') === wheelPrompts().map(p => p.id).join(',')) return;
  await commitPinOrder(ids);
}

function onMirrorPointerCancel(e) {
  const d = mirrorDrag;
  if (!d || e.pointerId !== d.pointerId) return;
  mirrorDrag = null;
  cleanupMirrorDrag(d);
}

function cleanupMirrorDrag(d) {
  d.marker?.remove();
  d.row.classList.remove('dragging');
  d.row.style.transform = '';
  d.row.style.left = d.row.style.top = d.row.style.width = '';
  $('#wheelMirror')?.classList.remove('drag-live');
  try { d.row.releasePointerCapture(d.pointerId); } catch { /* released */ }
}

async function commitPinOrder(ids) {
  const wasAuto = state.wheelSort !== 'manual';
  try {
    await ipc('set_pin_order', { ids });
    // the drop IS an ordering intent — only flip once it actually persisted
    if (wasAuto) { state.wheelSort = 'manual'; storage.set('pk-wheel-sort', 'manual'); }
    await loadPrompts(); rebuildIndex(); renderPrompts();
    if (wasAuto) toast('info', t('wheel.sortAutoSwitch'));
  } catch { /* toasted */ }
}

async function onMirrorKeyDown(e) {
  if (e.defaultPrevented || mirrorDrag) return;
  const row = e.target.closest?.('.wm-item');
  if (!row) return;
  let j;
  if (e.key === 'ArrowUp') j = -1;
  else if (e.key === 'ArrowDown') j = 1;
  else if (e.key === 'Home') j = -Infinity;
  else if (e.key === 'End') j = Infinity;
  else return;
  const ids = wheelPrompts().map(p => p.id);
  const i = ids.indexOf(+row.dataset.wheelId);
  if (i < 0) return;
  e.preventDefault();
  const to = Math.max(0, Math.min(ids.length - 1, i + j));
  if (to === i) return;
  ids.splice(to, 0, ids.splice(i, 1)[0]);
  const id = +row.dataset.wheelId;
  await commitPinOrder(ids);
  // renderPrompts rebuilt the list — hand focus back to the moved row's grip
  $(`#wheelMirror .wm-item[data-wheel-id="${id}"] .drag`)?.focus();
}

/* ---- events (wired once by app.js) ---- */
export function wirePrompts({ openPreview }) {
  const search = $('#searchInput');
  if (search) search.addEventListener('input', debounce(e => { state.q = e.target.value; renderGrid(); }, 120));

  $('#sortSel')?.addEventListener('change', e => {
    state.sortMode = e.target.value; storage.set('pk-sort', state.sortMode); renderGrid();
  });
  $('#viewCard')?.addEventListener('click', () => setView('card'));
  $('#viewList')?.addEventListener('click', () => setView('list'));

  $('#grid')?.addEventListener('click', e => {
    const star = e.target.closest('[data-star]');
    if (star) {
      e.stopPropagation();
      const id = +star.dataset.star;
      ipc('toggle_prompt_pin', { id }).then(async () => {
        await loadPrompts(); rebuildIndex(); renderPrompts();
        const p = state.prompts.find(x => x.id === id);
        toast('info', p?.is_pinned ? t('prompt.pinnedToast') : t('prompt.unpinnedToast'));
      }).catch(() => {});
      return;
    }
    const card = e.target.closest('.card');
    if (card) openDrawer(+card.dataset.id);
  });

  $('#tagsCloud')?.addEventListener('click', e => {
    const c = e.target.closest('[data-tag]');
    if (!c) return;
    state.tagFilter = state.tagFilter === c.dataset.tag ? null : c.dataset.tag;
    renderTags(); renderGrid();
  });

  $('#wheelSortBtn')?.addEventListener('click', async () => {
    // Review F16/F23/F49/F73: capture the *displayed* order BEFORE flipping to
    // manual — after the toggle wheelPrompts() re-sorts by stale inject_order
    // values and the seed would encode positions the user never saw.
    const seedIds = state.wheelSort === 'manual' ? null : wheelPrompts().map(p => p.id);
    state.wheelSort = state.wheelSort === 'manual' ? 'auto' : 'manual';
    storage.set('pk-wheel-sort', state.wheelSort);
    if (seedIds) {
      try { await ipc('set_pin_order', { ids: seedIds }); await loadPrompts(); rebuildIndex(); } catch { /* toasted */ }
    }
    renderPrompts();
  });

  $('#newBtn')?.addEventListener('click', () => openDrawer(null));
  $('#drawerClose')?.addEventListener('click', closeDrawer);
  $('#cancelBtn')?.addEventListener('click', closeDrawer);
  $('#drawerMask')?.addEventListener('click', closeDrawer);
  $('#fPin')?.addEventListener('click', e => e.currentTarget.classList.toggle('on'));
  ['fName', 'fContent', 'fTags'].forEach(id =>
    $('#' + id)?.addEventListener('input', () => { dirty = true; if (id === 'fContent') updateVarHint(); }));
  $('#saveBtn')?.addEventListener('click', saveDrawer);
  $('#delBtn')?.addEventListener('click', async () => {
    if (!editingId) return;
    const p = state.prompts.find(x => x.id === editingId);
    if (!await confirmModal({ title: t('common.delete'), body: t('del.ask', { n: p?.name || '' }), okText: t('common.delete') })) return;
    try {
      await ipc('delete_prompt', { id: editingId });
      forceCloseDrawer();
      await loadPrompts(); rebuildIndex(); renderPrompts();
      toast('ok', t('toast.deleted'));
    } catch { /* toasted */ }
  });
  $('#previewBtn')?.addEventListener('click', () => openPreview?.({
    name: $('#fName').value || t('drawer.new'),
    content: $('#fContent').value,
  }));
}

function setView(mode) {
  state.viewMode = mode; storage.set('pk-view', mode);
  $('#viewCard')?.classList.toggle('on', mode === 'card');
  $('#viewList')?.classList.toggle('on', mode === 'list');
  renderGrid();
}
