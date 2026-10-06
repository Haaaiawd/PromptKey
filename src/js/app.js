import { $, $$, esc, storage } from './dom.js';
import { icon } from './icons.js';
import { t, initI18n, setLangPref, getLangPref, onLangChange } from './i18n.js';
import { applyTheme, setThemeMode, getThemeMode, resolvedTheme } from './theme.js';
import { toast, modalOpen, modalEscape } from './toast.js';
import { state, ipc, loadPrompts, rebuildIndex, renderVars, customVars, wheelPrompts } from './store.js';
import { wirePrompts, renderPrompts, openDrawer, closeDrawer, drawerOpen } from './views/prompts.js';
import { wireLibrary, renderLibrary, previewPackJson } from './views/library.js';
import { wireLog, renderLog } from './views/log.js';
import { wireSettings, renderSettings } from './views/settings.js';

/* ---- icons: fill every [data-icon] slot ---- */
function hydrateIcons(root = document) {
  $$('[data-icon]', root).forEach(el => { el.innerHTML = icon(el.dataset.icon, el.dataset.iconSize ? +el.dataset.iconSize : 16); });
}

/* ---- navigation ---- */
const VIEW_RENDER = { prompts: renderPrompts, library: renderLibrary, log: renderLog, settings: renderSettings };
let currentView = 'prompts';

function switchView(name) {
  currentView = name;
  $$('.nav-item').forEach(b => b.classList.toggle('active', b.dataset.view === name));
  $$('.view').forEach(v => v.classList.toggle('active', v.id === 'view-' + name));
  const nav = $(`.nav-item[data-view="${name}"] [data-i18n]`);
  $('#pageTitle').textContent = nav ? nav.textContent : name;
  VIEW_RENDER[name]?.();
}

function syncShell() {
  // sidebar segmented controls
  $$('#themeSeg button').forEach(b => b.classList.toggle('on', b.dataset.theme === getThemeMode()));
  $$('#langSeg button').forEach(b => b.classList.toggle('on', b.dataset.lang === getLangPref() || (getLangPref() === 'auto' && b.dataset.lang === (navigator.language?.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US'))));
  $('#topHint').textContent = '';
  $('#pageTitle').textContent = $(`.nav-item[data-view="${currentView}"] [data-i18n]`)?.textContent || '';
}

/* ---- rendered preview (D3) ---- */
async function openPreview({ name, content }) {
  let clip = '';
  try { clip = await navigator.clipboard.readText(); } catch { /* clipboard unavailable */ }
  const rendered = renderVars(content, { clipboard: clip });
  showInfoModal(t('preview.title', { n: name }), rendered);
}

function showInfoModal(title, text) {
  const mask = $('#packMask');
  $('#packTitle').textContent = title;
  $('#packBody').innerHTML = `<div class="preview-render">${esc(text)}</div>`;
  $('#packImport').style.display = 'none';
  $('#packSelAll').style.display = 'none';
  $('#packCancel').textContent = t('common.close');
  mask.classList.add('show');
  $('#packCancel').onclick = () => { mask.classList.remove('show'); restorePackModal(); };
  mask.onclick = e => { if (e.target === mask) { mask.classList.remove('show'); restorePackModal(); } };
}
function restorePackModal() {
  $('#packImport').style.display = '';
  $('#packSelAll').style.display = '';
  $('#packCancel').textContent = t('common.cancel');
}

/* ---- service status dot ---- */
async function checkService() {
  const dot = $('#serviceDot'), txt = $('#serviceTxt');
  try {
    const ok = await ipc('check_service_status', {}, { silent: true, timeout: 5000 });
    dot.classList.toggle('ok', !!ok);
    dot.classList.toggle('err', !ok);
    txt.textContent = ok ? t('svc.online') : t('svc.offline');
  } catch {
    dot.classList.add('err'); dot.classList.remove('ok');
    txt.textContent = t('svc.offline');
  }
}

/* ---- global error surface (never silent) ---- */
window.addEventListener('error', e => {
  console.error(e.error || e.message);
  toast('err', t('err.generic', { e: e.message || 'unknown' }));
});
window.addEventListener('unhandledrejection', e => {
  console.error(e.reason);
  toast('err', t('err.generic', { e: e.reason?.message || e.reason || 'unknown' }));
});

/* ---- boot ---- */
async function boot() {
  applyTheme();
  initI18n();
  hydrateIcons();
  syncShell();

  // nav
  $$('.nav-item').forEach(b => b.addEventListener('click', () => switchView(b.dataset.view)));
  $$('#themeSeg button').forEach(b => b.addEventListener('click', () => { setThemeMode(b.dataset.theme); syncShell(); }));
  $$('#langSeg button').forEach(b => b.addEventListener('click', () => { setLangPref(b.dataset.lang); }));

  // wheel preview → real wheel window
  $('#previewWheel')?.addEventListener('click', () => {
    ipc('show_wheel_window').catch(() => {});
  });

  // keyboard
  document.addEventListener('keydown', e => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      $('#searchInput')?.focus();
      return;
    }
    if (e.key === 'Escape') {
      if (modalOpen()) { modalEscape(); return; }
      if (drawerOpen()) { closeDrawer(); return; }
    }
  });

  wirePrompts({ openPreview });
  wireLibrary();
  wireLog();
  wireSettings({ syncShell });
  onLangChange(() => { syncShell(); VIEW_RENDER[currentView]?.(); });

  // data
  try {
    await loadPrompts();
    rebuildIndex();
    renderPrompts();
    renderLibrary();
  } catch { /* toasted */ }
  checkService();

  // restore sort/view controls
  const sortSel = $('#sortSel');
  if (sortSel) sortSel.value = state.sortMode;
  $('#viewCard')?.classList.toggle('on', state.viewMode === 'card');
  $('#viewList')?.classList.toggle('on', state.viewMode === 'list');
}

if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
else boot();
