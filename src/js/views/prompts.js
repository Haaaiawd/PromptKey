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
        <span class="drag" data-drag="${p.id}" title="${esc(t('wheel.dragHint'))}">${icon('grip-vertical', 14)}</span>
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
  let apps = [];
  try { apps = p?.app_scopes_json ? JSON.parse(p.app_scopes_json) : []; } catch { apps = []; }
  setV('#fApps', Array.isArray(apps) ? apps.join(', ') : '');
  setV('#fOrder', p?.inject_order || '');
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
  const apps = ($('#fApps')?.value || '').split(/[,，]/).map(s => s.trim()).filter(Boolean);
  const p = state.prompts.find(x => x.id === editingId);
  const payload = {
    id: editingId,
    name,
    content,
    tags,
    content_type: p?.content_type || 'text',
    variables_json: p?.variables_json || null,
    app_scopes_json: JSON.stringify(apps),
    inject_order: ($('#fOrder')?.value || '').trim() || null,
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

function bindMirrorDrag() {
  let dragId = null;
  $$('#wheelMirror [data-drag]').forEach(handle => {
    const row = handle.closest('.wm-item');
    handle.addEventListener('mousedown', () => { row.draggable = true; });
    handle.addEventListener('mouseup', () => { row.draggable = false; });
    row.addEventListener('dragstart', e => {
      if (state.wheelSort !== 'manual') { e.preventDefault(); toast('info', t('wheel.sortSwitchToManual')); return; }
      dragId = +row.dataset.wheelId;
      row.classList.add('dragging');
      e.dataTransfer.effectAllowed = 'move';
    });
    row.addEventListener('dragend', () => { row.classList.remove('dragging'); row.draggable = false; });
    row.addEventListener('dragover', e => { e.preventDefault(); row.classList.add('dragover'); });
    row.addEventListener('dragleave', () => row.classList.remove('dragover'));
    row.addEventListener('drop', async e => {
      e.preventDefault(); row.classList.remove('dragover'); row.draggable = false;
      const overId = +row.dataset.wheelId;
      if (!dragId || dragId === overId) return;
      const pins = wheelPrompts().map(p => p.id);
      const from = pins.indexOf(dragId), to = pins.indexOf(overId);
      if (from < 0 || to < 0) return;
      pins.splice(to, 0, pins.splice(from, 1)[0]);
      try {
        await ipc('set_pin_order', { ids: pins });
        await loadPrompts(); rebuildIndex(); renderPrompts();
      } catch { /* toasted */ }
    });
  });
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
  ['fName', 'fContent', 'fTags', 'fApps', 'fOrder'].forEach(id =>
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
