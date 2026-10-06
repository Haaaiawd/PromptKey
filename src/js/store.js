import { t } from './i18n.js';
import { toast } from './toast.js';
import { storage } from './dom.js';

export function hasTauri() {
  return !!(window.__TAURI__ && (window.__TAURI__.core?.invoke || window.__TAURI__.invoke));
}

function rawInvoke() {
  return window.__TAURI__.core?.invoke || window.__TAURI__.invoke;
}

// Guarded IPC: rejects on timeout, toasts unless silent
export async function ipc(cmd, args, { timeout = 12000, silent = false } = {}) {
  if (!hasTauri()) {
    const msg = t('ipc.noTauri');
    if (!silent) toast('err', msg);
    throw new Error(msg);
  }
  let timer;
  const timeoutP = new Promise((_, rej) => {
    timer = setTimeout(() => rej(new Error(t('ipc.timeout'))), timeout);
  });
  try {
    return await Promise.race([rawInvoke()(cmd, args ?? {}), timeoutP]);
  } catch (e) {
    if (!silent) toast('err', t('ipc.fail', { e: typeof e === 'string' ? e : (e?.message || e) }));
    throw e;
  } finally {
    clearTimeout(timer);
  }
}

export const state = {
  prompts: [],
  loaded: false,
  q: '',
  tagFilter: null,
  sortMode: storage.get('pk-sort', 'recent'),
  viewMode: storage.get('pk-view', 'card'),
  wheelSort: storage.get('pk-wheel-sort', 'auto'),
};

export async function loadPrompts() {
  state.prompts = await ipc('get_prompts_view');
  state.loaded = true;
  return state.prompts;
}

export function allTags() {
  const s = new Set();
  for (const p of state.prompts) (p.tags || []).forEach(x => s.add(x));
  return [...s].sort((a, b) => a.localeCompare(b));
}

export function wheelPrompts() {
  const pins = state.prompts.filter(p => p.is_pinned);
  if (state.wheelSort === 'manual') {
    pins.sort((a, b) => (pinOrder(a) - pinOrder(b)) || (b.frecency - a.frecency) || a.name.localeCompare(b.name));
  } else {
    pins.sort((a, b) => (b.frecency - a.frecency) || ((b.last_used_at || 0) - (a.last_used_at || 0)) || a.name.localeCompare(b.name));
  }
  return pins;
}
function pinOrder(p) {
  const n = parseInt(p.inject_order, 10);
  return Number.isFinite(n) && n > 0 ? n : 9999;
}

const VAR_RE = /\{\{\s*([a-zA-Z_][\w.-]*)\s*\}\}/g;
// Review F14/F24/F70: the service renders {{time}} automatically, so it must
// not open a fill form.
export const AUTO_VARS = new Set(['clipboard', 'date', 'time']);

export function extractVars(content) {
  const out = [];
  for (const m of String(content || '').matchAll(VAR_RE)) {
    if (!out.includes(m[1])) out.push(m[1]);
  }
  return out;
}
export function customVars(content) {
  return extractVars(content).filter(v => !AUTO_VARS.has(v));
}
export function renderVars(content, values = {}) {
  const today = new Date();
  const pad = n => String(n).padStart(2, '0');
  const auto = {
    date: `${today.getFullYear()}-${pad(today.getMonth() + 1)}-${pad(today.getDate())}`,
    time: `${pad(today.getHours())}:${pad(today.getMinutes())}`,
    clipboard: values.clipboard ?? '',
  };
  return String(content || '').replace(VAR_RE, (_, k) => {
    if (k in values) return values[k];
    if (k in auto) return auto[k];
    return `{{${k}}}`;
  });
}

export function relTime(ms) {
  if (!ms) return t('prompt.never');
  const diff = Date.now() - ms;
  if (diff < 60_000) return t('rel.justnow');
  if (diff < 3600_000) return t('rel.minutes', { n: Math.floor(diff / 60_000) });
  if (diff < 86400_000) return t('rel.hours', { n: Math.floor(diff / 3600_000) });
  return t('rel.days', { n: Math.floor(diff / 86400_000) });
}

// Fuse.js search with prefix fast-paths (#tag @name c:content)
let fuse = null;
export function rebuildIndex() {
  if (typeof Fuse === 'undefined') { fuse = null; return; }
  fuse = new Fuse(state.prompts, {
    keys: [
      { name: 'name', weight: 0.6 },
      { name: 'tags', weight: 0.25 },
      { name: 'content', weight: 0.15 },
    ],
    threshold: 0.34,
    ignoreLocation: true,
    includeScore: true,
  });
}

export function filterPrompts() {
  let list = state.prompts.slice();
  if (state.tagFilter) list = list.filter(p => (p.tags || []).includes(state.tagFilter));
  const q = state.q.trim();
  if (q) {
    if (q.startsWith('#')) {
      const tag = q.slice(1).toLowerCase();
      list = list.filter(p => (p.tags || []).some(x => x.toLowerCase().includes(tag)));
    } else if (q.startsWith('@')) {
      const nm = q.slice(1).toLowerCase();
      list = list.filter(p => p.name.toLowerCase().includes(nm));
    } else if (q.startsWith('c:')) {
      const body = q.slice(2).toLowerCase();
      list = list.filter(p => p.content.toLowerCase().includes(body));
    } else if (fuse) {
      // Review F13: fuse indexes ALL prompts — keep the tag filter applied to
      // its results instead of replacing the filtered list wholesale.
      list = fuse.search(q).map(r => r.item)
        .filter(p => !state.tagFilter || (p.tags || []).includes(state.tagFilter));
    } else {
      const s = q.toLowerCase();
      list = list.filter(p => p.name.toLowerCase().includes(s)
        || (p.tags || []).join(' ').toLowerCase().includes(s)
        || p.content.toLowerCase().includes(s));
    }
  }
  const by = state.sortMode;
  if (by === 'freq') list.sort((a, b) => b.usage_count - a.usage_count);
  else if (by === 'name') list.sort((a, b) => a.name.localeCompare(b.name));
  else list.sort((a, b) => ((b.last_used_at || 0) - (a.last_used_at || 0)) || (b.frecency - a.frecency));
  return list;
}
