# Icon sourcing

- **Library**: Lucide (ISC license) — `lucide-static@1.48.0`, downloaded 2026-10-06 from `unpkg.com/lucide-static@1.48.0/icons/<name>.svg`.
- **License**: `src/icons/lucide/LICENSE` (ISC, bundled upstream).
- **Usage**: single source for ALL UI icons. JS wrapper: `src/js/icons.js` (generated — re-run generation script if icons are added; do not hand-edit). SVGs use `stroke="currentColor"`, `stroke-width="2"`, 24×24 viewBox; render at 16/20/24px.
- **Rules**: no hand-drawn SVG geometry, no emoji/character glyphs as icons (◀ ▶ ⭐ ✓ 🔍 ⚙ etc. all banned), no runtime CDN fetches, no mixing icon libraries.

## Brand assets

- Source: `PromptKey.ico` (app icon, in use) and `PromptKey_aiextract.png` (brand logo master).
- `src/icons/brand/` holds derived assets (tray icon, about-page logo, wheel center mark, platform icons). Derivations only resize/crop — the logo's shape, colors, and composition must not be altered.

## Icon inventory (55)

app-window, braces, check, chevron-down, chevron-right, circle-alert, circle-check, circle-dot, circle-x, clipboard, clock, copy, database, download, ellipsis, external-link, eye, file-json, folder-open, grip-vertical, history, info, keyboard, languages, layers, library, link, list, loader-pinwheel, minus, moon, octagon-alert, package, pencil, pin, play, plus, refresh-cw, save, search, send, settings, shield-check, sparkles, star, sun, sun-moon, tags, trash-2, triangle-alert, upload, wand-sparkles, x, zap
