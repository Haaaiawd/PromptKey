// PromptKey Wheel — 280px radial, cursor-following, type-to-filter (Wheel A)
import { $, $$, esc, storage } from './dom.js';
import { icon } from './icons.js';
import { t, refreshI18n } from './i18n.js';
import { refreshTheme } from './theme.js';
import { toast } from './toast.js';
import { state, loadPrompts, wheelPrompts, customVars } from './store.js';

const PAGE = 6;
let open = false;
let query = '';
let page = 0;
let matches = [];
let fuse = null;
let hiding = false;
// Review F60: invalidate a pending prepare() when a blur/hide fires mid-load.
let prepareGen = 0;

const win = () => window.__TAURI__?.window?.getCurrentWindow?.();

async function invokeRaw(cmd, args) {
  const inv = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke;
  if (!inv) throw new Error('no tauri');
  return inv(cmd, args ?? {});
}

function syncPrefs() {
  // Review F59: re-read storage — the main window may have changed theme/lang
  // since this wheel window was created (module state is cached at load).
  refreshTheme();
  refreshI18n();
  const ws = storage.get('pk-wheel-sort', 'auto');
  state.wheelSort = ws;
}

async function prepare() {
  const gen = ++prepareGen;
  syncPrefs();
  const ct = $('#centerTxt');
  if (ct) ct.textContent = t('wh.esc');
  const cc = $('#wheelCenter');
  if (cc) cc.title = t('wh.centerHint');
  try {
    await loadPrompts();
  } catch (e) {
    state.prompts = [];
    console.error('wheel load failed', e);
  }
  if (gen !== prepareGen) return; // superseded or blurred while loading
  const pins = wheelPrompts();
  fuse = typeof Fuse !== 'undefined'
    ? new Fuse(pins, { keys: [{ name: 'name', weight: 0.7 }, { name: 'tags', weight: 0.2 }, { name: 'content', weight: 0.1 }], threshold: 0.34, ignoreLocation: true })
    : null;
  state._wheelPool = pins;
  query = ''; page = 0;
  await clampToViewport();
  if (gen !== prepareGen) return;
  const w = $('#wheel');
  if (!w) return;
  w.classList.remove('closing');
  w.classList.add('show');
  open = true;
  render();
}

async function clampToViewport() {
  const w = win();
  if (!w) return;
  try {
    const [pos, size, monitor] = await Promise.all([w.outerPosition(), w.outerSize(), w.currentMonitor()]);
    const sf = monitor?.scaleFactor || 1;
    // Review F03/F56: monitor origins can be negative on secondary displays —
    // clamp inside position+size, not 0..size.
    const mx = monitor?.position?.x ?? 0;
    const my = monitor?.position?.y ?? 0;
    const mw = monitor?.size.width ?? 4096;
    const mh = monitor?.size.height ?? 2160;
    const m = 16 * sf;
    const x = Math.min(Math.max(pos.x, mx + m), Math.max(mx + m, mx + mw - size.width - m));
    const y = Math.min(Math.max(pos.y, my + m), Math.max(my + m, my + mh - size.height - m));
    if (x !== pos.x || y !== pos.y) {
      await w.setPosition(new window.__TAURI__.window.PhysicalPosition(Math.round(x), Math.round(y)));
    }
  } catch (e) { console.warn('wheel clamp failed', e); }
}

function pool() { return state._wheelPool || []; }

function currentMatches() {
  const p = pool();
  if (!query) return p;
  if (fuse) return fuse.search(query).map(r => r.item);
  const q = query.toLowerCase();
  return p.filter(x => x.name.toLowerCase().includes(q) || (x.tags || []).join(' ').toLowerCase().includes(q));
}

function render() {
  const w = $('#wheel');
  if (!w) return;
  w.querySelectorAll('.petal,.filter-tag,.empty-tip,.var-fill').forEach(x => x.remove());
  const all = currentMatches();
  const pages = Math.max(1, Math.ceil(all.length / PAGE));
  page = Math.min(page, pages - 1);
  matches = all.slice(page * PAGE, page * PAGE + PAGE);
  const R = 96, CX = 160, CY = 160; // wheel centered in 320 window
  const slots = Math.max(matches.length, 6);
  matches.forEach((p, i) => {
    const a = (-90 + i * (360 / slots)) * Math.PI / 180;
    const b = document.createElement('button');
    b.className = 'petal';
    b.style.left = CX + 'px'; b.style.top = CY + 'px';
    b.style.setProperty('--pos', `translate(${R * Math.cos(a)}px,${R * Math.sin(a)}px)`);
    b.style.animationDelay = (i * 28) + 'ms';
    b.innerHTML = `<span class="k">${i + 1}</span><span class="pn">${esc(p.name)}</span>`;
    b.title = p.name;
    b.addEventListener('click', e => { e.stopPropagation(); pick(b, p); });
    w.appendChild(b);
  });
  if (!matches.length) {
    const e = document.createElement('div');
    e.className = 'empty-tip';
    e.textContent = query ? `${t('wh.nomatch')} · ${t('wh.new')}` : t('wh.nomatch');
    w.appendChild(e);
  }
  if (query) {
    const f = document.createElement('div');
    f.className = 'filter-tag';
    f.innerHTML = `${icon('search', 12)} ${esc(query)}`;
    f.style.display = 'inline-flex'; f.style.alignItems = 'center'; f.style.gap = '5px';
    w.appendChild(f);
  }
  const c = $('#centerTxt');
  if (c) c.textContent = query ? `${t('wh.filtering')}: ${query.slice(0, 6)}` : (pages > 1 ? t('wh.page', { a: page + 1, b: pages }) : t('wh.esc'));
}

function hide() {
  prepareGen++; // any in-flight prepare() must not mark a hidden wheel open
  if (!open || hiding) return;
  hiding = true;
  const w = $('#wheel');
  if (!w) { hiding = false; open = false; return; }
  w.classList.remove('show');
  w.classList.add('closing');
  setTimeout(() => { w.classList.remove('closing'); }, 130);
  win()?.hide?.().catch?.(() => {});
  open = false;
  query = ''; page = 0;
  setTimeout(() => { hiding = false; }, 150);
}

async function pick(elm, p) {
  elm.classList.add('picked');
  const needsVars = customVars(p.content).length > 0;
  setTimeout(async () => {
    try {
      if (needsVars) {
        // custom vars are collected in the wheel before injecting
        await fillAndInject(p);
      } else {
        await invokeRaw('trigger_wheel_injection', { promptId: p.id });
        hide();
      }
    } catch (e) {
      toast('err', t('toast.injectedFail', { e: typeof e === 'string' ? e : e?.message || e }));
      setTimeout(hide, 1400);
    }
  }, 200);
}

// D5: variable fill form inside the wheel
function fillAndInject(p) {
  return new Promise((resolve, reject) => {
    const w = $('#wheel');
    const vars = customVars(p.content);
    const panel = document.createElement('div');
    panel.className = 'filter-tag var-fill';
    panel.style.cssText = 'position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);pointer-events:auto;display:flex;flex-direction:column;gap:8px;padding:14px;min-width:220px;z-index:5;background:var(--surface-solid);border:1px solid var(--border-strong);border-radius:12px;box-shadow:0 12px 40px rgba(0,0,0,.4);color:var(--text)';
    panel.innerHTML = `<div style="font-size:11px;font-weight:600">${esc(t('varfill.title', { n: p.name }))}</div>` +
      vars.map(v => `<label style="display:flex;flex-direction:column;gap:3px;font-size:10px;color:var(--text2)">${esc(v)}<input data-v="${esc(v)}" style="background:transparent;border:1px solid var(--border-strong);border-radius:6px;color:var(--text);padding:6px 8px;font-size:11px;outline:none"></label>`).join('') +
      `<div style="display:flex;gap:6px;justify-content:flex-end;margin-top:4px"><button data-act="cancel" style="border:1px solid var(--border-strong);background:none;color:var(--text2);border-radius:8px;padding:5px 12px;font-size:11px;cursor:pointer">${esc(t('common.cancel'))}</button><button data-act="go" style="border:none;background:var(--accent);color:#fff;border-radius:8px;padding:5px 12px;font-size:11px;font-weight:600;cursor:pointer">${esc(t('varfill.inject'))}</button></div>`;
    w.appendChild(panel);
    const first = panel.querySelector('input');
    first?.focus();
    panel.addEventListener('click', async e => {
      const act = e.target.closest('[data-act]')?.dataset.act;
      if (act === 'cancel') { panel.remove(); hide(); resolve(); return; }
      if (act === 'go') {
        const values = {};
        panel.querySelectorAll('input[data-v]').forEach(inp => values[inp.dataset.v] = inp.value);
        try {
          await invokeRaw('trigger_wheel_injection_vars', { promptId: p.id, varsJson: JSON.stringify(values) });
          panel.remove(); hide(); resolve();
        } catch (err) { panel.remove(); reject(err); }
      }
    });
    // keys inside the form must not feed the wheel filter; Enter submits, Esc cancels
    panel.addEventListener('keydown', e => {
      e.stopPropagation();
      if (e.key === 'Enter') { e.preventDefault(); panel.querySelector('[data-act="go"]')?.click(); }
      if (e.key === 'Escape') { e.preventDefault(); panel.querySelector('[data-act="cancel"]')?.click(); }
    });
  });
}

/* ---- keyboard ---- */
document.addEventListener('keydown', e => {
  if (!open) return;
  // while the variable form (or any input) is focused, don't steal keys
  if ($('.var-fill') || e.target.closest?.('input,textarea,[contenteditable]')) return;
  if (e.key === 'Escape') { e.preventDefault(); hide(); return; }
  if (e.key === 'Backspace') { query = query.slice(0, -1); page = 0; render(); return; }
  if (e.key === 'Enter') { const el = $$('.petal')[0]; if (el && matches[0]) pick(el, matches[0]); return; }
  if (e.key === 'ArrowRight' || e.key === 'PageDown') { page++; render(); return; }
  if (e.key === 'ArrowLeft' || e.key === 'PageUp') { page = Math.max(0, page - 1); render(); return; }
  const n = parseInt(e.key, 10);
  if (n >= 1 && n <= PAGE && !e.ctrlKey && !e.metaKey && !e.altKey) {
    const els = $$('.petal');
    if (matches[n - 1] && els[n - 1]) pick(els[n - 1], matches[n - 1]);
    return;
  }
  // typing filter: single printable chars (digits 1-6 are selection keys per design)
  if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
    query += e.key; page = 0; render();
  }
});

/* ---- click outside petals closes ---- */
document.addEventListener('mousedown', e => {
  if (!open) return;
  if (!e.target.closest('.petal') && !e.target.closest('.center') && !e.target.closest('.var-fill') && !e.target.closest('.toast')) hide();
});
let suppressCenterClick = false;
$('#wheelCenter')?.addEventListener('click', e => {
  e.stopPropagation();
  if (suppressCenterClick) { suppressCenterClick = false; return; }
  hide();
});

// D4: quick-create — right-click or long-press(550ms) the center dot hands the
// current filter text to the main window's new-prompt drawer and hides the
// wheel. No record is created here: a prompt exists only after the user writes
// real content and saves through the drawer's normal validation, so the wheel
// can never gain a petal that injects a placeholder (review F18 follow-up: the
// previous fix set content=name, which made petals inject their own name).
async function quickCreate() {
  const name = (query || '').trim();
  try {
    await invokeRaw('present_main_window_new_prompt', { name });
    hide();
  } catch (e) {
    // keep the wheel open so the error toast is visible
    toast('err', t('err.generic', { e: typeof e === 'string' ? e : e?.message || e }));
  }
}
let pressTimer = 0;
const center = $('#wheelCenter');
center?.addEventListener('contextmenu', e => { e.preventDefault(); e.stopPropagation(); suppressCenterClick = true; quickCreate(); });
center?.addEventListener('mousedown', e => { if (e.button === 0) pressTimer = setTimeout(() => { suppressCenterClick = true; quickCreate(); }, 550); });
center?.addEventListener('mouseup', () => clearTimeout(pressTimer));
center?.addEventListener('mouseleave', () => clearTimeout(pressTimer));

/* ---- lifecycle ---- */
window.addEventListener('blur', hide);

function onShow() { prepare(); }

// IPC failure must be VISIBLE in the wheel: when the listener never registers,
// Rust still shows this transparent overlay on every hotkey press — a silent
// catch is how 2.0.x shipped an invisible dead window over the user's work.
function envError(msg) {
  const el = $('#envErr');
  if (el) { el.textContent = msg; el.classList.add('show'); }
}
async function init() {
  syncPrefs();
  // first paint state
  const listen = window.__TAURI__?.event?.listen;
  if (typeof listen !== 'function') {
    envError(t('wh.envNoBridge'));
    return;
  }
  try {
    await listen('wheel-show', onShow);
    await listen('wheel-hide', () => hide());
  } catch (e) {
    console.warn('wheel event listen failed', e);
    envError(t('wh.envAcl'));
  }
}
init();
