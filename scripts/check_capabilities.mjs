#!/usr/bin/env node
// Tauri 2 capability regression gate — the static half of the "never silently
// lose capabilities again" fix (the runtime half is probeIpcEnvironment).
//
// Why this exists: 2.0.1/2.0.2 shipped with ZERO capability files. Tauri 2
// then ACL-denies every plugin:* IPC call (event.listen, window.set_position,
// window.hide…) while app commands still answer — so prompts loaded, the
// service dot was green, and the wheel was permanently deaf. Every e2e test
// mocked window.__TAURI__, so nothing caught it.
//
// Checks (pure Node, no Rust toolchain):
//   1. capabilities/ exists and holds at least one .json file
//   2. union of `windows` covers the two webview labels: main + wheel-panel
//   3. every referenced permission identifier exists in
//      gen/schemas/acl-manifests.json (catches typos statically)
//   4. resolved allow-* set covers the plugin commands the JS layer calls
//
// Exits non-zero with a precise message on any failure.

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const CAP_DIR = join(ROOT, 'capabilities');
const MANIFEST_PATH = join(ROOT, 'gen/schemas/acl-manifests.json');

const failures = [];
const fail = (msg) => failures.push(msg);

/* ---------- window coverage ---------- */
// labels come from tauri.conf.json (main) and src/main.rs WebviewWindowBuilder
const REQUIRED_WINDOWS = ['main', 'wheel-panel'];

/* ---------- plugin commands the JS layer actually invokes ---------- */
// wheel.js: event.listen x2, win.outerPosition/outerSize/currentMonitor,
//           win.setPosition, win.hide      (window "wheel-panel")
// app.js:   event.listen('wheel-new-prompt') (window "main")
// Everything else the frontend invokes is an app command — ungated while the
// crate ships no permissions/ dir (verified: build.rs calls tauri_build::build
// with default attributes; no app ACL manifest is emitted).
const REQUIRED_ALLOW = {
  'core:event': ['allow-listen'],
  'core:window': [
    'allow-outer-position',
    'allow-outer-size',
    'allow-current-monitor',
    'allow-set-position',
    'allow-hide',
  ],
};

/* ---------- collect capability files ---------- */
if (!existsSync(CAP_DIR)) {
  fail('capabilities/ directory is missing — Tauri 2 will deny every plugin:* IPC call at runtime');
  report();
}
const files = readdirSync(CAP_DIR).filter(f => f.endsWith('.json'));
if (!files.length) fail('capabilities/ exists but contains no .json capability file');

const windows = [];
const permIds = []; // raw identifiers, string or {identifier, ...}
for (const f of files) {
  const p = join(CAP_DIR, f);
  let v;
  try { v = JSON.parse(readFileSync(p, 'utf8')); }
  catch (e) { fail(`${f}: invalid JSON — ${e.message}`); continue; }
  for (const w of Array.isArray(v.windows) ? v.windows : []) {
    windows.push(typeof w === 'string' ? w : w?.identifier);
  }
  for (const perm of Array.isArray(v.permissions) ? v.permissions : []) {
    permIds.push(typeof perm === 'string' ? perm : perm?.identifier);
  }
}

// glob-ish window matching: '*' or an entry equal to the label
const covers = (label) => windows.some(w => w === '*' || w === label);
for (const label of REQUIRED_WINDOWS) {
  if (!covers(label)) fail(`no capability covers window label '${label}'`);
}

/* ---------- permission identifier validation ---------- */
let manifest = null;
if (existsSync(MANIFEST_PATH)) {
  manifest = JSON.parse(readFileSync(MANIFEST_PATH, 'utf8'));
} else {
  console.warn('warning: gen/schemas/acl-manifests.json not found — identifier validation skipped');
}

// split 'a:b:c' → prefix 'a:b', name 'c'  (last ':' separates the permission name)
function splitId(id) {
  const i = id.lastIndexOf(':');
  return i < 0 ? [id, ''] : [id.slice(0, i), id.slice(i + 1)];
}
// resolve an identifier to the flat allow-* list it grants, using the manifest.
// Entries inside a default_permission list can be bare 'allow-x' names —
// those are relative to the containing plugin's prefix (ctxPrefix).
function resolveAllows(id, seen = new Set(), ctxPrefix = '') {
  const key = `${ctxPrefix}|${id}`; // bare names resolve under different prefixes
  if (seen.has(key)) return [];
  seen.add(key);
  let [prefix, name] = splitId(id);
  if (!name) { prefix = ctxPrefix; name = id; }
  if (!manifest) return name.startsWith('allow-') ? [[prefix, name]] : [];
  const m = manifest[prefix];
  if (!m) { fail(`permission '${id}': unknown plugin prefix '${prefix}'`); return []; }
  if (name === 'default') {
    const dp = m.default_permission;
    if (!dp) { fail(`permission '${id}': plugin '${prefix}' has no default`); return []; }
    return (dp.permissions || []).flatMap(p => resolveAllows(p, seen, prefix));
  }
  if (!m.permissions?.[name]) { fail(`permission '${id}': not found in manifest for '${prefix}'`); return []; }
  return name.startsWith('allow-') ? [[prefix, name]] : [];
}

const granted = {}; // prefix -> Set(allow-*)
for (const id of permIds) {
  if (!id) continue;
  for (const [prefix, allow] of resolveAllows(id)) {
    (granted[prefix] ??= new Set()).add(allow);
  }
}

for (const [prefix, allows] of Object.entries(REQUIRED_ALLOW)) {
  for (const allow of allows) {
    if (!granted[prefix]?.has(allow)) {
      fail(`capability set grants no '${prefix}:${allow}' — a real JS call site needs it`);
    }
  }
}

report();

function report() {
  if (failures.length) {
    console.error('capability check FAILED:');
    for (const f of failures) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log(`capabilities OK — ${files.length} file(s), windows: ${[...new Set(windows)].join(', ')}`);
  process.exit(0);
}
