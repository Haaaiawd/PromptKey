import zhCN from './i18n/zh-CN.js';
import enUS from './i18n/en-US.js';

const DICTS = { 'zh-CN': zhCN, 'en-US': enUS };
const KEY = 'pk-lang';

function readPref() {
  try { return localStorage.getItem(KEY) || 'auto'; } catch { return 'auto'; }
}
function systemLang() {
  return (navigator.language || '').toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US';
}
export function resolveLang(pref) {
  return pref === 'auto' ? systemLang() : pref;
}

let pref = readPref();
let lang = resolveLang(pref);
const listeners = new Set();

export function getLang() { return lang; }
export function getLangPref() { return pref; }

export function t(key, vars) {
  let s = DICTS[lang]?.[key] ?? DICTS['zh-CN'][key] ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
}

export function applyI18n(root = document) {
  document.documentElement.lang = lang;
  root.querySelectorAll('[data-i18n]').forEach(el => {
    const v = t(el.dataset.i18n);
    if (el.dataset.i18nHtml === '') el.innerHTML = v; else el.textContent = v;
  });
  root.querySelectorAll('[data-i18n-ph]').forEach(el => { el.placeholder = t(el.dataset.i18nPh); });
  root.querySelectorAll('[data-i18n-title]').forEach(el => { el.title = t(el.dataset.i18nTitle); });
}

export function setLangPref(p) {
  pref = p === 'auto' ? 'auto' : p;
  try { localStorage.setItem(KEY, pref); } catch { /* storage may be blocked */ }
  lang = resolveLang(pref);
  applyI18n();
  listeners.forEach(fn => fn(lang));
}

export function onLangChange(fn) { listeners.add(fn); return () => listeners.delete(fn); }

export function initI18n() { applyI18n(); }

// Re-read the stored pref — used by the wheel window, whose module state is
// populated at load and would otherwise go stale (review F59).
export function refreshI18n() {
  pref = readPref();
  lang = resolveLang(pref);
  applyI18n();
}
