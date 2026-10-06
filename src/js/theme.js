const KEY = 'pk-theme';
const mq = matchMedia('(prefers-color-scheme: dark)');
const listeners = new Set();

function readMode() {
  try { const v = localStorage.getItem(KEY); return ['light', 'dark', 'auto'].includes(v) ? v : 'auto'; }
  catch { return 'auto'; }
}

let mode = readMode();

export function getThemeMode() { return mode; }
export function resolvedTheme() { return mode === 'auto' ? (mq.matches ? 'dark' : 'light') : mode; }

export function applyTheme() {
  document.body.dataset.theme = resolvedTheme();
}

export function setThemeMode(m) {
  if (!['light', 'dark', 'auto'].includes(m)) return;
  mode = m;
  try { localStorage.setItem(KEY, m); } catch { /* storage may be blocked */ }
  applyTheme();
  listeners.forEach(fn => fn(resolvedTheme()));
}

export function onThemeChange(fn) { listeners.add(fn); return () => listeners.delete(fn); }

// Re-read the stored mode — used by the wheel window, whose module state is
// populated at load and would otherwise go stale (review F59).
export function refreshTheme() {
  mode = readMode();
  applyTheme();
}

mq.addEventListener('change', () => { if (mode === 'auto') { applyTheme(); listeners.forEach(fn => fn(resolvedTheme())); } });
