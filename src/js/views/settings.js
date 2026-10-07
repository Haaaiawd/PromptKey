import { $, $$ } from '../dom.js';
import { t, setLangPref, getLangPref } from '../i18n.js';
import { setThemeMode, getThemeMode } from '../theme.js';
import { toast, confirmModal } from '../toast.js';
import { ipc, state } from '../store.js';
import { previewPackJson } from './library.js';
import { attachRecorder } from '../hotkey_recorder.js';

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

  refreshHotkeyStates();
}
function esc0(s) { return String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c])); }

// Self-check surface: shows what the engine ACTUALLY registered per hotkey —
// ok / conflict / unsupported / failed / not_registered. A dead engine or a
// rejected RegisterHotKey is visible here instead of silent.
async function refreshHotkeyStates() {
  const wheelEl = $('#hotkeyWheelState');
  const quickEl = $('#hotkeyQuickState');
  if (!wheelEl && !quickEl) return null;
  const mark = (el, cls, txt) => { if (el) { el.className = `sub hk-state ${cls}`; el.textContent = txt; } };
  mark(wheelEl, 'muted', t('hk.stateChecking'));
  mark(quickEl, 'muted', t('hk.stateChecking'));
  let rep;
  try {
    rep = await ipc('check_hotkeys', {}, { silent: true });
  } catch (e) {
    mark(wheelEl, 'err', t('hk.stateCheckFail'));
    mark(quickEl, 'err', t('hk.stateCheckFail'));
    return null;
  }
  const engineFailed = rep && rep.engine === 'failed';
  for (const h of rep?.hotkeys || []) {
    const el = h.id === 4 ? wheelEl : h.id === 5 ? quickEl : null;
    if (!el) continue;
    if (engineFailed) {
      mark(el, 'err', t('hk.engineDead', { e: rep.engine_error || '' }));
      continue;
    }
    const combo = h.canonical || h.combo || '';
    switch (h.status) {
      case 'ok': mark(el, 'ok', `${t('hk.stateOk')} · ${combo}`); break;
      case 'disabled': mark(el, 'muted', t('hk.stateDisabled')); break;
      case 'conflict': mark(el, 'err', `${t('hk.stateConflict')} · ${combo}`); break;
      case 'unsupported': mark(el, 'err', t('hk.stateUnsupported', { e: h.detail || '' })); break;
      case 'failed': mark(el, 'err', t('hk.stateFailed', { e: h.detail || '' })); break;
      default: mark(el, 'warn', t('hk.stateOff')); break;
    }
  }
  return rep;
}

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
      // apply_settings returned Ok — but registration could still have failed;
      // the self-check statuses say the truth before we celebrate.
      const rep = await refreshHotkeyStates();
      const bad = (rep?.hotkeys || []).filter(h => !['ok', 'disabled'].includes(h.status));
      if (rep?.engine === 'failed') {
        toast('err', t('hk.engineDead', { e: rep.engine_error || '' }));
      } else if (bad.length) {
        toast('err', t('hk.savedInactive', { e: bad.map(b => b.canonical || b.combo).join(', ') }));
      } else {
        toast('ok', t('hk.saved'));
      }
      return rep;
    } catch { return null; /* toasted */ }
  };

  // Hotkey fields are recorders, not text boxes: focus arms capture, the next
  // chord is normalized to the Rust parser's canonical form and committed.
  // Esc / clicking away cancels and restores the previous combo.
  const HOTKEY_ID = { wheel: 4, quick: 5 };
  const commitHotkey = async (field, combo, restore) => {
    const rep = await saveHotkeys();
    const rec = (rep?.hotkeys || []).find(h => h.id === HOTKEY_ID[field]);
    const failed = !rep || rep.engine === 'failed'
      || (rec && !['ok', 'disabled'].includes(rec.status));
    if (!failed) return;
    // The combo was rejected (or the engine is dead) — but apply_settings may
    // already have persisted it. Roll the field AND the config back so an
    // unusable hotkey never silently sticks.
    const input = $(field === 'wheel' ? '#hotkeyInput' : '#quickHotkeyInput');
    if (input) {
      input.value = restore;
      input.classList.add('invalid');
      setTimeout(() => input.classList.remove('invalid'), 1600);
    }
    await saveHotkeys();
    toast('info', t('hk.recReverted'));
  };
  attachRecorder($('#hotkeyInput'), {
    stateEl: $('#hotkeyWheelState'),
    onCommit: (combo, restore) => commitHotkey('wheel', combo, restore),
  });
  attachRecorder($('#quickHotkeyInput'), {
    stateEl: $('#hotkeyQuickState'),
    onCommit: (combo, restore) => commitHotkey('quick', combo, restore),
  });

  // diagnose_hotkey_pipeline: per-link snapshot of the hotkey→wheel chain.
  // The status line shows localized hints; the full JSON is on hover so a bug
  // report can quote it verbatim.
  $('#btnHotkeyDiag')?.addEventListener('click', async () => {
    const el = $('#hotkeyDiagState');
    if (!el) return;
    el.className = 'sub hk-state muted';
    el.textContent = t('diag.running');
    try {
      const d = await ipc('diagnose_hotkey_pipeline', {}, { silent: true });
      const hints = Array.isArray(d?.hints) ? d.hints : [];
      const ok = hints.length === 1 && hints[0] === 'ok';
      el.className = `sub hk-state ${ok ? 'ok' : 'err'}`;
      el.textContent = hints.map(h => t(`diag.${h}`)).join('；') || t('diag.ok');
      el.title = JSON.stringify(d);
    } catch {
      el.className = 'sub hk-state err';
      el.textContent = t('diag.failed');
    }
  });
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
