import { $, $$, esc } from '../dom.js';
import { t } from '../i18n.js';
import { icon } from '../icons.js';
import { toast } from '../toast.js';
import { ipc, loadPrompts, rebuildIndex } from '../store.js';

let packs = [];

export function validatePack(obj) {
  if (!obj || typeof obj !== 'object') return null;
  if (obj.format !== 'promptkey-pack' || !Array.isArray(obj.prompts)) return null;
  const prompts = obj.prompts
    .filter(p => p && typeof p.name === 'string' && typeof p.content === 'string' && p.name.trim() && p.content.trim())
    .map(p => ({
      name: p.name.trim(),
      content: p.content,
      tags: Array.isArray(p.tags) ? p.tags.filter(x => typeof x === 'string') : [],
      app_scopes: Array.isArray(p.app_scopes) ? p.app_scopes.filter(x => typeof x === 'string') : [],
    }));
  return { meta: obj.pack || {}, prompts };
}

export async function importPackPrompts(list) {
  const existing = new Set((await loadPrompts()).map(p => p.name));
  let added = 0, skipped = 0;
  for (const p of list) {
    if (existing.has(p.name)) { skipped++; continue; }
    await ipc('create_prompt', {
      prompt: {
        id: null, name: p.name, content: p.content, tags: p.tags,
        content_type: 'text', variables_json: null,
        app_scopes_json: JSON.stringify(p.app_scopes || []),
        inject_order: null, version: 1, updated_at: null,
      },
    });
    added++;
    existing.add(p.name);
  }
  return { added, skipped };
}

async function loadBuiltinPacks() {
  try {
    const res = await fetch('packs/manifest.json');
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const files = await res.json();
    const out = [];
    for (const f of files) {
      try {
        const data = await fetch(`packs/${f}`).then(r => r.json());
        const v = validatePack(data);
        if (v) out.push({ file: f, meta: v.meta, prompts: v.prompts });
      } catch (e) { console.warn('bad pack', f, e); }
    }
    packs = out;
  } catch (e) {
    console.warn('no builtin packs', e);
    packs = [];
  }
}

export async function renderLibrary() {
  const grid = $('#packGrid');
  if (!grid) return;
  if (!packs.length) await loadBuiltinPacks();
  grid.innerHTML = packs.length ? packs.map((pk, i) => {
    const meta = pk.meta || {};
    const names = pk.prompts.slice(0, 4).map(p => p.name);
    const more = pk.prompts.length - names.length;
    return `<div class="pack">
      <h4>${esc(meta.name || pk.file)}</h4>
      <div class="meta">${esc(meta.author || 'PromptKey')} · ${esc(t('lib.promptCount', { n: pk.prompts.length }))}${meta.lang ? ' · ' + esc(meta.lang) : ''}</div>
      <div class="desc">${esc(meta.desc || '')}</div>
      <div class="pack-preview">${names.map(esc).join(' · ')}${more > 0 ? ` ${esc('+' + String(more))}` : ''}</div>
      <button class="btn btn-primary" data-pack="${i}" style="width:100%">${icon('download', 14)} ${esc(t('lib.import'))}</button>
    </div>`;
  }).join('') : `<div class="empty">${esc(t('lib.empty'))}</div>`;
}

function openPackPreview(meta, prompts) {
  const mask = $('#packMask');
  if (!mask) return;
  $('#packTitle').textContent = meta.name || t('lib.import');
  $('#packBody').innerHTML = `<div class="import-note"><span class="ico ico-sm">${icon('shield-check', 14)}</span><span>${esc(t('lib.urlWarn'))}</span></div>` +
    `<div style="margin-top:10px;display:flex;flex-direction:column;gap:6px">` +
    prompts.map((p, i) => `
      <label class="pack-item"><input type="checkbox" data-pi="${i}" checked>
      <span><span class="pi-name">${esc(p.name)}</span>
      ${p.tags?.length ? ` <span class="ctag">${p.tags.map(esc).join('</span> <span class="ctag">')}</span>` : ''}
      <div class="pi-content">${esc(p.content.slice(0, 160))}</div></span></label>`).join('') + '</div>';
  const ok = $('#packImport');
  const updateOk = () => {
    const n = $$('#packBody [data-pi]:checked').length;
    ok.textContent = n === prompts.length ? t('lib.importAll', { n }) : t('lib.importSelected', { n });
    ok.disabled = n === 0;
  };
  updateOk();
  $('#packBody').onchange = updateOk;
  $('#packSelAll').onclick = () => {
    const boxes = $$('#packBody [data-pi]');
    const all = boxes.every(b => b.checked);
    boxes.forEach(b => b.checked = !all);
    updateOk();
    $('#packSelAll').textContent = all ? t('lib.selAll') : t('lib.selNone');
  };
  mask.classList.add('show');
  $('#packCancel').onclick = () => mask.classList.remove('show');
  mask.onclick = e => { if (e.target === mask) mask.classList.remove('show'); };
  ok.onclick = async () => {
    const idx = $$('#packBody [data-pi]:checked').map(b => +b.dataset.pi);
    const chosen = idx.map(i => prompts[i]);
    mask.classList.remove('show');
    try {
      const { added, skipped } = await importPackPrompts(chosen);
      rebuildIndex();
      toast('ok', skipped ? t('lib.importedSkip', { n: added, s: skipped }) : t('lib.imported', { n: added }));
    } catch { /* toasted */ }
  };
}

export function previewPackJson(text) {
  let obj;
  try { obj = JSON.parse(text); } catch (e) { toast('err', t('lib.parseFail', { e: e.message })); return; }
  const v = validatePack(obj);
  if (!v || !v.prompts.length) { toast('err', t('lib.badPack')); return; }
  openPackPreview(v.meta, v.prompts);
}

export function wireLibrary() {
  $('#libTabs')?.addEventListener('click', e => {
    const tab = e.target.closest('[data-lib]');
    if (!tab) return;
    $$('#libTabs .lib-tab').forEach(x => x.classList.toggle('on', x === tab));
    ['builtin', 'url', 'file'].forEach(k => $('#lib-' + k)?.classList.toggle('hidden', k !== tab.dataset.lib));
    if (tab.dataset.lib === 'builtin') renderLibrary();
  });

  $('#packGrid')?.addEventListener('click', e => {
    const b = e.target.closest('[data-pack]');
    if (!b) return;
    const pk = packs[+b.dataset.pack];
    if (pk) openPackPreview(pk.meta, pk.prompts);
  });

  // URL import — needs backend fetch command (Task 5 branch completes the fetch path)
  $('#urlImportBtn')?.addEventListener('click', async () => {
    const url = $('#urlInput').value.trim();
    if (!url) return;
    try {
      const text = await ipc('fetch_pack_url', { url });
      previewPackJson(text);
    } catch { /* toasted */ }
  });

  $('#filePickBtn')?.addEventListener('click', async () => {
    try {
      const r = await ipc('pick_pack_file');
      if (r) { $('#packText').value = r.text; $('#fileName').textContent = r.name || ''; }
    } catch { /* toasted */ }
  });
  $('#textImportBtn')?.addEventListener('click', () => {
    const txt = $('#packText').value.trim();
    if (!txt) { toast('warn', t('lib.badPack')); return; }
    previewPackJson(txt);
  });
}
