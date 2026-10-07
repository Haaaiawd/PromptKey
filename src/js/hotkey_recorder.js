// Hotkey recorder: click into the field → the next real chord is captured,
// normalized to the Rust parse_hotkey canonical form, and committed.
// (Replaces the old free-text input — users typed "Ctrl+Alt+Space" by hand.)
import { t } from './i18n.js';

// Modifier key *codes* — these update the held-modifier preview, never commit.
const MODIFIER_CODES = new Set([
  'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight',
  'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight',
]);

// event.code → canonical key name understood by service::hotkey::parse_hotkey.
// `code` (physical position) is used, not `key` (layout-dependent char), so a
// QWERTZ user pressing the Z-position key records the key the VK means.
const CODE_TO_KEY = {
  Space: 'Space', Enter: 'Enter', NumpadEnter: 'Enter', Tab: 'Tab',
  Backspace: 'Backspace', Delete: 'Delete', Insert: 'Insert',
  Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
  ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right',
  CapsLock: 'CapsLock', NumLock: 'NumLock', ScrollLock: 'ScrollLock',
  PrintScreen: 'PrintScreen', Pause: 'Pause',
  Semicolon: ';', Comma: ',', Period: '.', Slash: '/', Backquote: '`',
  BracketLeft: '[', BracketRight: ']', Backslash: '\\', Quote: "'",
  Minus: '-', Equal: '=',
  NumpadMultiply: 'Num*', NumpadAdd: 'NumAdd', NumpadSubtract: 'Num-',
  NumpadDecimal: 'Num.', NumpadDivide: 'Num/',
};

/// Physical event.code → canonical main-key name, or null if the Rust parser
/// cannot express it (IntlBackslash, IME keys, media keys, …).
export function mainKeyFromCode(code) {
  if (CODE_TO_KEY[code]) return CODE_TO_KEY[code];
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);          // KeyA → A
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);        // Digit7 → 7
  if (/^Numpad[0-9]$/.test(code)) return 'Num' + code.slice(6); // Numpad5 → Num5
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;      // F1..F24
  return null;
}

/// Canonical modifier list in the order parse_hotkey emits: Ctrl+Alt+Shift+Win.
export function modsFromEvent(e) {
  const m = [];
  if (e.ctrlKey) m.push('Ctrl');
  if (e.altKey) m.push('Alt');
  if (e.shiftKey) m.push('Shift');
  if (e.metaKey) m.push('Win');
  return m;
}

/// Classify a non-modifier keydown into a commit candidate.
///   { ok: true, combo }                       — e.g. "Ctrl+Alt+Space"
///   { ok: false, reason: 'need_modifier' }    — bare key, e.g. Space alone
///   { ok: false, reason: 'unsupported', key } — code has no VK mapping
export function comboFromEvent(e) {
  const mods = modsFromEvent(e);
  const key = mainKeyFromCode(e.code);
  if (key === null) return { ok: false, reason: 'unsupported', key: e.code };
  if (mods.length === 0) return { ok: false, reason: 'need_modifier', key };
  return { ok: true, combo: [...mods, key].join('+') };
}

/**
 * Wire a settings input into record-on-focus capture.
 *   input    — the (readonly) text input showing the combo
 *   opts.stateEl   — the .hk-state line under it (recording/error hints)
 *   opts.onCommit(combo) — called once a valid chord is captured (save flow)
 *   opts.onRefresh()     — re-pull the real hotkey state (cancel/error exits)
 *
 * Esc always cancels and restores the previous value — it can never be the
 * main key, matching user muscle memory for "get me out of here".
 */
export function attachRecorder(input, { stateEl = null, onCommit = null, onRefresh = null } = {}) {
  if (!input) return;
  let recording = false;
  let original = '';

  const setState = (key, vars, cls = 'muted') => {
    if (!stateEl) return;
    stateEl.className = `sub hk-state ${cls}`;
    stateEl.textContent = vars ? t(key, vars) : t(key);
  };

  const start = () => {
    if (recording) return;
    recording = true;
    original = input.value;
    input.value = '';
    input.classList.add('recording');
    input.placeholder = t('hk.recording');
    setState('hk.recording');
  };

  const stop = (restore) => {
    recording = false;
    input.classList.remove('recording');
    input.placeholder = t('hk.hint');
    if (restore) input.value = original;
  };

  input.addEventListener('focus', start);

  input.addEventListener('keydown', (e) => {
    if (!recording) { e.preventDefault(); return; }
    // Never let a chord type characters, navigate focus, or trigger browser
    // shortcuts (Ctrl+W etc.) while capturing.
    e.preventDefault();
    e.stopPropagation();

    if (e.code === 'Escape') {
      stop(true);
      setState('hk.recCancel');
      input.blur();
      onRefresh?.();
      return;
    }
    if (MODIFIER_CODES.has(e.code)) {
      // Live preview of held modifiers: "Ctrl+Alt+…"
      const mods = modsFromEvent(e);
      input.value = mods.length ? mods.join('+') + '+' : '';
      return;
    }
    const r = comboFromEvent(e);
    if (!r.ok) {
      setState(r.reason === 'need_modifier' ? 'hk.recNeedMod' : 'hk.recUnsup',
        { k: r.key || e.code }, 'err');
      input.value = original;
      return; // stay in recording — the next chord can still commit
    }
    input.value = r.combo;
    stop(false);
    input.blur();
    onCommit?.(r.combo, original);
  });

  // Clicking away / Tab-out is also a cancel: restore the saved combo.
  input.addEventListener('blur', () => {
    if (recording) { stop(true); onRefresh?.(); }
  });

  // Defense in depth: readonly + keydown preventDefault already block typing,
  // but if anything still lands (autofill, IME quirk) it must not linger.
  // Programmatic .value writes (the modifier preview) do not fire 'input'.
  input.addEventListener('input', () => {
    if (recording) input.value = '';
  });
}
