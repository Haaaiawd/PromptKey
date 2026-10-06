import { $, $$ } from '../dom.js';
import { t, setLangPref, getLangPref } from '../i18n.js';
import { setThemeMode, getThemeMode } from '../theme.js';
import { toast, confirmModal } from '../toast.js';
import { ipc, state } from '../store.js';
import { previewPackJson } from './library.js';

export async function renderSettings() {
  // sync selects with current prefs
  const langSel = $('#langSel');
  if (langSel) langSel.value = getLangPref();
  const themeSel = $('#themeSel');
  if (themeSel) themeSel.value = getThemeMode();

  try {
    const s = await ipc('get_settings', {}, { silent: true });
    if ($('#hotkeyInput')) $('#hotkeyInput').value = s.hotkey || '';
    if ($('#quickHotkeyInput')) $('#quickHotkeyInput').value = s.quick_hotkey || '';
    if ($('#swClipboard')) $('#swClipboard').classList.toggle('on', s.allow_clipboard !== false);
    if ($('#swRestore')) $('#swRestore').classList.toggle('on', s.restore_clipboard !== false);
    if ($('#swGate')) $('#swGate').classList.toggle('on', s.secure_gate !== false);
  } catch { /* leave defaults */ }

  // default prompt selector
  const sel = $('#defaultPromptSel');
  if (sel) {
    let mode = 'last_used', fid = 0;
    try {
      const as = await ipc('get_app_settings', {}, { silent: true });
      mode = as?.default_prompt_mode || 'last_used';
      fid = +(as?.default_prompt_id || 0);
    } catch { /* defaults */ }
    const opts = [`<option value="last_used">${esc0(t('hk.lastUsed'))}</option>`]
      .concat(state.prompts.map(p =>
        `<option value="${p.id}" ${mode === 'fixed' && fid === p.id ? 'selected' : ''}>${esc0(t('hk.fixed'))} · ${esc0(p.name)}</option>`));
    sel.innerHTML = opts.join('');
    sel.value = mode === 'fixed' && fid ? String(fid) : 'last_used';
  }
}
function esc0(s) { return String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c])); }

export function wireSettings({ syncShell }) {
  $('#langSel')?.addEventListener('change', e => { setLangPref(e.target.value); syncShell?.(); });
  $('#themeSel')?.addEventListener('change', e => { setThemeMode(e.target.value); syncShell?.(); });

  const saveHotkeys = async () => {
    const hk = $('#hotkeyInput')?.value.trim();
    const qk = $('#quickHotkeyInput')?.value.trim();
    try {
      await ipc('apply_settings', {
        hotkey: hk || null,
        quickHotkey: qk || null,
        injection: {
          allowClipboard: $('#swClipboard')?.classList.contains('on'),
          restoreClipboard: $('#swRestore')?.classList.contains('on'),
          secureGate: $('#swGate')?.classList.contains('on'),
        },
      });
      toast('ok', t('hk.saved'));
    } catch { /* toasted */ }
  };
  $('#hotkeyInput')?.addEventListener('change', saveHotkeys);
  $('#quickHotkeyInput')?.addEventListener('change', saveHotkeys);
  ['#swClipboard', '#swRestore', '#swGate'].forEach(id =>
    $(id)?.addEventListener('click', e => { e.currentTarget.classList.toggle('on'); saveHotkeys(); }));

  $('#defaultPromptSel')?.addEventListener('change', async e => {
    const v = e.target.value;
    try {
      // Review F61: write the id BEFORE flipping mode to fixed — a quick-hotkey
      // press between the two calls would otherwise read a stale/empty id.
      if (v === 'last_used') await ipc('set_app_setting', { key: 'default_prompt_mode', value: 'last_used' });
      else {
        await ipc('set_app_setting', { key: 'default_prompt_id', value: v });
        await ipc('set_app_setting', { key: 'default_prompt_mode', value: 'fixed' });
      }
      toast('ok', t('toast.saved'));
    } catch { /* toasted */ }
  });

  $('#exportBtn')?.addEventListener('click', async () => {
    try {
      const r = await ipc('export_prompts_pack');
      if (r) toast('ok', t('toast.exported'));
    } catch { /* toasted */ }
  });
  $('#importBtn')?.addEventListener('click', async () => {
    try {
      // Review F36: route through the same pick→preview→import flow the
      // library uses instead of inserting unseen prompts directly.
      const r = await ipc('pick_pack_file');
      if (r) previewPackJson(r.text);
    } catch { /* toasted */ }
  });
  $('#libExport')?.addEventListener('click', () => $('#exportBtn')?.click());
  $('#resetBtn')?.addEventListener('click', async () => {
    if (!await confirmModal({ title: t('data.reset'), body: t('data.resetAsk'), okText: t('data.resetBtn') })) return;
    try { await ipc('reset_settings'); toast('ok', t('data.resetDone')); renderSettings(); }
    catch { /* toasted */ }
  });
  $('#restartSvc')?.addEventListener('click', async () => {
    const b = $('#restartSvc');
    if (b) { b.disabled = true; }
    toast('info', t('about.restarting'));
    try { await ipc('restart_service'); toast('ok', t('about.restarted')); }
    catch { /* toasted */ }
    finally { if (b) b.disabled = false; }
  });
}
