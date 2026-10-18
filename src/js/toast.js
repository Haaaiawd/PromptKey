import { icon } from './icons.js';
import { $, el } from './dom.js';

const ICONS = { ok: 'circle-check', err: 'octagon-alert', warn: 'triangle-alert', info: 'info' };

export function toast(kind, msg, ms = 2600, key = null) {
  const box = $('#toasts');
  if (!box) { console.warn('[toast]', kind, msg); return; }
  // keyed toast: a repeated message (e.g. holding an out-of-range digit, whose
  // keydown auto-repeat fires dozens of times) replaces the live one instead of
  // stacking up — stacked toasts cover the wheel petals and intercept clicks.
  if (key) {
    const live = box.querySelector(`[data-toast-key="${CSS.escape(key)}"]`);
    if (live) {
      live.lastElementChild.textContent = msg;
      clearTimeout(live._toastTimer);
      live._toastTimer = setTimeout(() => { live.style.opacity = '0'; setTimeout(() => live.remove(), 180); }, ms);
      return;
    }
  }
  const node = el('div', `toast ${kind}`);
  if (key) node.dataset.toastKey = key;
  node.innerHTML = `<span class="ico">${icon(ICONS[kind] || 'info', 16)}</span><span></span>`;
  node.lastElementChild.textContent = msg;
  box.appendChild(node);
  while (box.children.length > 5) box.firstElementChild.remove();
  node._toastTimer = setTimeout(() => { node.style.opacity = '0'; setTimeout(() => node.remove(), 180); }, ms);
}

// Promise-based confirm modal (replaces native confirm())
let modalState = null;
export function confirmModal({ title, body, okText, danger = true }) {
  return new Promise(resolve => {
    const mask = $('#modalMask');
    const ok = $('#modalOk'), cancel = $('#modalCancel');
    if (!mask || !ok || !cancel) { console.warn('[confirmModal] modal DOM missing, defaulting to cancel'); resolve(false); return; }
    const titleEl = $('#modalTitle'), bodyEl = $('#modalBody');
    if (titleEl) titleEl.textContent = title || '';
    if (bodyEl) bodyEl.textContent = body || '';
    ok.textContent = okText || 'OK';
    ok.className = danger ? 'btn btn-danger' : 'btn btn-primary';
    mask.classList.add('show');
    const done = v => { mask.classList.remove('show'); modalState = null; resolve(v); };
    modalState = done;
    ok.onclick = () => done(true);
    cancel.onclick = () => done(false);
    mask.onclick = e => { if (e.target === mask) done(false); };
  });
}
export function modalOpen() { return !!modalState; }
export function modalEscape() { if (modalState) modalState(false); }
