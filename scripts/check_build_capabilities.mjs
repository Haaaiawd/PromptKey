#!/usr/bin/env node
// Post-build capability gate — portable (node) version of the pwsh step in the
// Windows release job, for the Linux/macOS release jobs.
//
// tauri-build resolves capabilities/ at compile time and rewrites
// gen/schemas/capabilities.json with the resolved map. This file is the
// build-time proof that capabilities made it into the binary's context:
// an empty {} here = the 2.0.x bug where every plugin:* IPC call is
// ACL-denied at runtime while app commands still answer.
//
// Run AFTER `tauri build`. Exits non-zero if the resolved map is empty or
// does not cover both webview labels (main, wheel-panel).

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const path = join(process.cwd(), 'gen/schemas/capabilities.json');
const caps = JSON.parse(readFileSync(path, 'utf8'));

const names = Object.keys(caps);
if (names.length === 0) {
  console.error('tauri build resolved ZERO capabilities — plugin:* IPC would be ACL-denied at runtime (the 2.0.x regression)');
  process.exit(1);
}

const labels = [];
for (const n of names) {
  for (const w of caps[n].windows ?? []) {
    labels.push(typeof w === 'string' ? w : w?.identifier);
  }
}

for (const req of ['main', 'wheel-panel']) {
  if (!labels.includes(req) && !labels.includes('*')) {
    console.error(`resolved capabilities do not cover window '${req}' (${labels.join(', ')})`);
    process.exit(1);
  }
}

console.log(`resolved capabilities: ${names.join(', ')} — windows: ${labels.join(', ')}`);
