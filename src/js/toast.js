import { icon } from './icons.js';
import { $, el } from './dom.js';

const ICONS = { ok: 'circle-check', err: 'octagon-alert', warn: 'triangle-alert', info: 'info' };

export function toast(kind, msg, ms = 2600) {
  const box = $('#toasts');
  if (!box) { console.warn('[toast]', kind, msg); return; }
  const node = el('div', `toast ${kind}`);
  node.innerHTML = `<span class="ico">${icon(ICONS[kind] || 'info', 16)}</span><span></span>`;
  node.lastElementChild.textContent = msg;
  box.appendChild(node);
  while (box.children.length > 5) box.firstElementChild.remove();
  setTimeout(() => { node.style.opacity = '0'; setTimeout(() => node.remove(), 180); }, ms);
}

// Promise-based confirm modal (replaces native confirm())
let modalState = null;
export function confirmModal({ title, body, okText, danger = true }) {
  return new Promise(resolve => {
    const mask = $('#modalMask');
    if (!mask) { console.warn('[confirmModal] no #modalMask, defaulting to cancel'); resolve(false); return; }
    $('#modalTitle').textContent = title || '';
    $('#modalBody').textContent = body || '';
    const ok = $('#modalOk');
    ok.textContent = okText || 'OK';
    ok.className = danger ? 'btn btn-danger' : 'btn btn-primary';
    mask.classList.add('show');
    const done = v => { mask.classList.remove('show'); modalState = null; resolve(v); };
    modalState = done;
    $('#modalOk').onclick = () => done(true);
    $('#modalCancel').onclick = () => done(false);
    mask.onclick = e => { if (e.target === mask) done(false); };
  });
}
export function modalOpen() { return !!modalState; }
export function modalEscape() { if (modalState) modalState(false); }
