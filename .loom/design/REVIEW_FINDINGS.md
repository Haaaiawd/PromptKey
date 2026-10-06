# Review Findings (74)

Source: Devin Review jobs for PRs #3–#8 (extracted 2026-10-06 via `app.devin.ai/api/pr-review/job-result`).
30 findings were posted as GitHub inline comments; 44 are dashboard-only `analysis` entries (`needs_investigation`).

| # | PR | kind | file:line | 标题 | posted | 判定 | 理由 |
|---|----|------|-----------|------|--------|------|------|
| 1 | #3 | bug | src/main.rs:1162 | Settings restart leaves old engines running | GitHub | | |
| 2 | #3 | bug | src/js/views/settings.js:52 | Injection switches do not change injection | GitHub | | |
| 3 | #3 | bug | src/js/wheel.js:74 | Wheel escapes offset secondary monitors | GitHub | | |
| 4 | #3 | security | src/main.rs:646 | Pack fetch exposes private network responses | GitHub | | |
| 5 | #3 | security | service/src/main.rs:108 | Variable values can expand private clipboard data | GitHub | | |
| 6 | #3 | analysis | src/js/wheel.js:184 | Variable inputs trigger prompt selection | dashboard | | |
| 7 | #3 | analysis | service/src/db.rs:454 | Failed injections become quick-hotkey defaults | dashboard | | |
| 8 | #3 | analysis | src/js/views/settings.js:18 | Saved switches silently turn on again | dashboard | | |
| 9 | #3 | analysis | src/main.rs:529 | Wheel preview reuses an old target | dashboard | | |
| 10 | #3 | analysis | service/src/ipc/inject_server.rs:66 | Long variable values lose substitution | dashboard | | |
| 11 | #3 | analysis | src/js/views/library.js:40 | Built-in packs have no manifest | dashboard | | |
| 12 | #3 | analysis | src/js/views/library.js:15 | Imports leave prompt lists stale | dashboard | | |
| 13 | #3 | analysis | src/js/store.js:138 | Fuzzy search drops the tag filter | dashboard | | |
| 14 | #3 | analysis | src/js/store.js:67 | Time placeholder opens an unnecessary form | dashboard | | |
| 15 | #3 | analysis | service/src/main.rs:83 | Quick injection leaves custom fields literal | dashboard | | |
| 16 | #3 | analysis | src/js/views/prompts.js:215 | Manual sort seeds from old positions | dashboard | | |
| 17 | #4 | bug | service/src/main.rs:210 | Failed injections become the default prompt | GitHub | | |
| 18 | #4 | bug | src/js/wheel.js:219 | Quick-created prompts inject empty text | GitHub | | |
| 19 | #4 | security | src/main.rs:646 | Pack fetch exposes internal HTTP services | GitHub | | |
| 20 | #4 | security | service/src/main.rs:108 | Submitted variable values trigger clipboard expansion | GitHub | | |
| 21 | #4 | analysis | src/js/views/library.js:14 | Library import loses wheel configuration | dashboard | | |
| 22 | #4 | analysis | service/src/config/mod.rs:29 | Saved clipboard preferences have no effect | dashboard | | |
| 23 | #4 | analysis | src/js/views/prompts.js:214 | Manual order resets on mode switch | dashboard | | |
| 24 | #4 | analysis | src/js/store.js:70 | Automatic time asks for manual entry | dashboard | | |
| 25 | #4 | analysis | service/src/main.rs:108 | Spaced placeholders remain unrendered | dashboard | | |
| 26 | #4 | analysis | src/js/views/library.js:23 | Library imports remain hidden until reload | dashboard | | |
| 27 | #5 | bug | src/js/wheel.js:184 | Variable entry triggers unintended injections | GitHub | | |
| 28 | #5 | bug | src/js/views/settings.js:49 | Saved settings leave old hotkeys active | GitHub | | |
| 29 | #5 | bug | src/js/views/library.js:114 | Imported prompts stay absent from the grid | GitHub | | |
| 30 | #5 | security | src/main.rs:646 | Pack fetching exposes local network responses | GitHub | | |
| 31 | #5 | security | src/main.rs:647 | Plaintext imports permit pack tampering | GitHub | | |
| 32 | #5 | security | src/main.rs:675 | Oversized pack files exhaust application memory | GitHub | | |
| 33 | #5 | analysis | service/src/ipc/inject_server.rs:66 | Large variable values lose their substitutions | dashboard | | |
| 34 | #5 | analysis | src/main.rs:543 | Pack backup drops prompt properties | dashboard | | |
| 35 | #5 | analysis | service/src/main.rs:112 | Spaced placeholders survive variable entry | dashboard | | |
| 36 | #5 | analysis | src/js/views/settings.js:84 | Settings import bypasses pack preview | dashboard | | |
| 37 | #5 | analysis | src/js/views/library.js:14 | Library import discards wheel layout | dashboard | | |
| 38 | #5 | analysis | service/src/db.rs:454 | Deleted prompt shadows last-used selection | dashboard | | |
| 39 | #5 | analysis | service/src/main.rs:108 | Filled values are parsed as templates | dashboard | | |
| 40 | #6 | bug | service/src/injector/mod.rs:377 | Failed paste discards clipboard contents | GitHub | | |
| 41 | #6 | bug | service/src/injector/mod.rs:102 | Partial backups destroy clipboard formats | GitHub | | |
| 42 | #6 | bug | service/src/injector/mod.rs:186 | Secure checks accumulate COM references | GitHub | | |
| 43 | #6 | security | src/main.rs:647 | Pack fetch reaches internal services | GitHub | | |
| 44 | #6 | security | service/src/injector/mod.rs:187 | Password inputs bypass the secure-field gate | GitHub | | |
| 45 | #6 | analysis | service/src/ipc/inject_server.rs:66 | Long variable messages lose their values | dashboard | | |
| 46 | #6 | analysis | src/js/views/library.js:14 | Library imports lose wheel assignments | dashboard | | |
| 47 | #6 | analysis | service/src/db.rs:458 | Recent prompt ordering lacks a tie-breaker | dashboard | | |
| 48 | #6 | analysis | service/src/main.rs:574 | Spaced placeholders survive injection | dashboard | | |
| 49 | #6 | analysis | src/js/views/prompts.js:215 | Manual sorting unexpectedly reorders pins | dashboard | | |
| 50 | #7 | bug | src/js/views/settings.js:63 | Settings changes multiply background engines | GitHub | | |
| 51 | #7 | bug | service/src/injector/mod.rs:128 | Clipboard restoration loses image data | GitHub | | |
| 52 | #7 | bug | service/src/ipc/inject_server.rs:66 | Large variable values disappear during injection | GitHub | | |
| 53 | #7 | security | src/main.rs:646 | Pack fetch exposes local network responses | GitHub | | |
| 54 | #7 | security | src/main.rs:647 | Plaintext pack downloads allow tampering | GitHub | | |
| 55 | #7 | analysis | src/js/store.js:70 | Spaced placeholders survive injection | dashboard | | |
| 56 | #7 | analysis | src/js/wheel.js:64 | Secondary-monitor wheel placement | dashboard | | |
| 57 | #7 | analysis | src/js/views/library.js:115 | Library imports stay invisible | dashboard | | |
| 58 | #7 | analysis | src/js/views/library.js:14 | Library round-trip loses wheel pins | dashboard | | |
| 59 | #7 | analysis | src/js/wheel.js:25 | Wheel retains outdated preferences | dashboard | | |
| 60 | #7 | analysis | src/js/wheel.js:49 | Blur during loading leaves wheel open internally | dashboard | | |
| 61 | #7 | analysis | src/js/views/settings.js:71 | Default-prompt updates expose an old selection | dashboard | | |
| 62 | #8 | bug | service/src/injector/windows_impl.rs:378 | Failed paste leaves clipboard overwritten | GitHub | | |
| 63 | #8 | bug | service/src/injector/windows_impl.rs:129 | Image clipboard lost after injection | GitHub | | |
| 64 | #8 | bug | src/main.rs:1090 | Settings changes multiply active engines | GitHub | | |
| 65 | #8 | security | src/main.rs:573 | Pack fetch exposes private network responses | GitHub | | |
| 66 | #8 | security | service/src/main.rs:108 | User variables override automatic clipboard value | GitHub | | |
| 67 | #8 | analysis | service/src/injector/windows_impl.rs:164 | COM initialization grows with injections | dashboard | | |
| 68 | #8 | analysis | service/src/ipc/inject_server.rs:66 | Large variable requests lose values | dashboard | | |
| 69 | #8 | analysis | service/src/main.rs:112 | Spaced template variables remain unresolved | dashboard | | |
| 70 | #8 | analysis | src/js/store.js:71 | Automatic time variable opens a form | dashboard | | |
| 71 | #8 | analysis | src/main.rs:442 | Wheel cannot follow negative screen coordinates | dashboard | | |
| 72 | #8 | analysis | service/src/db.rs:457 | Deleted prompt masks recent surviving prompt | dashboard | | |
| 73 | #8 | analysis | src/js/views/prompts.js:223 | Manual mode starts in old order | dashboard | | |
| 74 | #8 | analysis | src/js/wheel.js:231 | Quick-created petal injects nothing | dashboard | | |

---

## Appendix — full finding text

### F01 · PR #3 · bug · src/main.rs:1162 · severe · GitHub

**Settings restart leaves old engines running** (`BUG_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/3#discussion_r4194609489

Changing a hotkey through `apply_settings` starts a new engine without stopping the old one. `stop_service` only clears `is_active`, so old hotkeys and pipe listeners remain alive.

**Recommended fix:** The embedded engine runs in a spawned thread that loops forever in [run_service](service/src/main.rs:49-99). [stop_service](src/main.rs:85-89) only sets `is_active` to false; it never signals or joins that thread. Each settings change now calls stop then [start_service](src/main.rs:66-82), spawning another engine. Existing hotkeys stay registered, and the first pipe server can continue receiving requests using the old configuration.

**Example:** Change the wheel hotkey from Ctrl+Alt+Space to Ctrl+Alt+W. The old engine still owns Ctrl+Alt+Space; the new engine registers Ctrl+Alt+W, and both engine loops continue running.

**Recommended fix:** Add a shutdown signal and owned join handles for the engine and its pipe server, unregister hotkeys when the hotkey thread exits, and wait for shutdown before starting the replacement engine.

### F02 · PR #3 · bug · src/js/views/settings.js:52 · severe · GitHub

**Injection switches do not change injection** (`BUG_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/3#discussion_r4194609609

Toggling `swClipboard`, `swRestore`, or `swGate` saves preferences, but `Injector` never reads them. Injection still uses clipboard and restores it regardless of the selected switches.

**Recommended fix:** The Settings view passes these three values through [apply_settings](src/main.rs:1150-1154) and persists them in YAML. The [injector](service/src/injector/mod.rs:91-111) always tries clipboard first, and its [clipboard path](service/src/injector/mod.rs:265-283) always restores backed-up text. Neither `secure_gate` nor these new preferences affect injection in the current engine.

**Example:** Turn off clipboard injection in Settings and inject a prompt. The engine still calls `inject_via_clipboard` and pastes with Ctrl+V rather than using SendInput alone.

**Recommended fix:** Consume the persisted flags in `Injector::inject` and `inject_via_clipboard`, and implement or remove the advertised secure-control gate until it works.

### F03 · PR #3 · bug · src/js/wheel.js:74 · severe · GitHub

**Wheel escapes offset secondary monitors** (`BUG_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/3#discussion_r4194609725

On an offset monitor, `clampToViewport` compares absolute window coordinates with monitor dimensions starting at zero. The wheel jumps away from the cursor or onto another monitor.

**Recommended fix:** A monitor's `size` describes its extent, while window `outerPosition` uses virtual desktop coordinates. [present_wheel](src/main.rs:501-506) also floors both coordinates to zero before showing the window. Neither path accounts for monitors with negative or positive virtual origins, so the clamp can move the wheel off its target monitor.

**Example:** A secondary screen spans x=-1920 to -1. A cursor at x=-900 is initially placed at x=0 by `present_wheel`, then clamped within 0..monitor.width; the wheel opens on the primary screen.

**Recommended fix:** Position and clamp against the current monitor's `position` plus `size`, including negative origins, and remove the zero floor in `present_wheel`.

### F04 · PR #3 · security · src/main.rs:646 · critical · GitHub

**Pack fetch exposes private network responses** (`SEC_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/3#discussion_r4194609893

`fetch_pack_url` accepts loopback and private HTTP URLs and returns their response bodies. A caller with command access can read services reachable only from the user's machine.

**Recommended fix:** Validate destination addresses on each redirect and restrict fetches to trusted public HTTPS endpoints where feasible.

### F05 · PR #3 · security · service/src/main.rs:108 · warning · GitHub

**Variable values can expand private clipboard data** (`SEC_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/3#discussion_r4194610042

A filled custom variable containing `{{clipboard}}` gets expanded during automatic substitution. The injected text can include clipboard contents absent from the selected prompt.

**Recommended fix:** Use a single-pass template renderer that matches placeholders only in the stored prompt and appends replacement values literally.

### F06 · PR #3 · analysis · src/js/wheel.js:184 · n/a · dashboard

**Variable inputs trigger prompt selection** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0001`)

Typing `1` into a variable field also invokes `pick` on the first petal. The document-level shortcut handler does not exclude the focused form.

### F07 · PR #3 · analysis · service/src/db.rs:454 · n/a · dashboard

**Failed injections become quick-hotkey defaults** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0002`)

`resolve_default_prompt` selects successful logs, but [handle_injection_request](service/src/main.rs:210-237) logs success before injection. Failed attempts can become the next default.

### F08 · PR #3 · analysis · src/js/views/settings.js:18 · n/a · dashboard

**Saved switches silently turn on again** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0003`)

`get_settings` returns flat preferences, but the switches read `s.injection`. Saved `false` values display as enabled and are overwritten on the next save.

### F09 · PR #3 · analysis · src/main.rs:529 · n/a · dashboard

**Wheel preview reuses an old target** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0004`)

`show_wheel_window` does not update [last_active_context](service/src/main.rs:46-61). After a hotkey launch, selecting a preview petal can inject into the earlier window.

### F10 · PR #3 · analysis · service/src/ipc/inject_server.rs:66 · n/a · dashboard

**Long variable values lose substitution** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0005`)

`listen_once` reads one 8192-byte chunk. Longer variable JSON reaches [render_template](service/src/main.rs:108-115) incomplete, so custom placeholders remain in the injected prompt.

### F11 · PR #3 · analysis · src/js/views/library.js:40 · n/a · dashboard

**Built-in packs have no manifest** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0006`)

`loadBuiltinPacks` requests `packs/manifest.json`, which is absent from the packaged frontend. The built-in library remains empty.

### F12 · PR #3 · analysis · src/js/views/library.js:15 · n/a · dashboard

**Imports leave prompt lists stale** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0007`)

`importPackPrompts` loads existing prompts before insertion, then never reloads. Successful imports stay absent from the grid and wheel mirror until reload.

### F13 · PR #3 · analysis · src/js/store.js:138 · n/a · dashboard

**Fuzzy search drops the tag filter** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0008`)

With a tag selected, `fuse.search` replaces the filtered list with results from all prompts. Unrelated tags reappear during text searches.

### F14 · PR #3 · analysis · src/js/store.js:67 · n/a · dashboard

**Time placeholder opens an unnecessary form** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0009`)

`AUTO_VARS` excludes `time`, though [render_template](service/src/main.rs:122-126) replaces it automatically. Time-only prompts still open the variable form.

### F15 · PR #3 · analysis · service/src/main.rs:83 · n/a · dashboard

**Quick injection leaves custom fields literal** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0010`)

Hotkey ID 5 passes no variables to `render_template`. A default prompt containing `{{name}}` injects that placeholder verbatim.

### F16 · PR #3 · analysis · src/js/views/prompts.js:215 · n/a · dashboard

**Manual sort seeds from old positions** (`ANALYSIS_pr-review-job-7f9d455aaaa446a6ba447f2ad5d07025_0011`)

`wheelSort` becomes manual before `wheelPrompts` computes the seed order. Old `inject_order` values override the automatic positions the user just saw.

### F17 · PR #4 · bug · service/src/main.rs:210 · severe · GitHub

**Failed injections become the default prompt** (`BUG_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/4#discussion_r4194604312

When `inject` fails, `log_usage` already records success. `resolve_default_prompt` selects that failed attempt as the most recent success.

**Recommended fix:** The usage log is the source of truth for both the recent-success default prompt and the frecency ranking. The service currently records a successful injection before the injector runs. A failed attempt therefore ranks as a successful use and can replace the last genuinely injected default.

**Example:** Prompt A succeeded yesterday. Prompt B fails today. The quick hotkey chooses B because its pre-injection log has `success = 1`, even though B never reached the target.

**Recommended fix:** Call `injector.inject` before `db.log_usage`, then log the returned success, strategy, duration, and error. Keep unsuccessful attempts out of the success-only ranking.

### F18 · PR #4 · bug · src/js/wheel.js:219 · non-severe · GitHub

**Quick-created prompts inject empty text** (`BUG_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/4#discussion_r4194604415

`quickCreate` pins a prompt with empty `content`. Selecting its new wheel petal injects nothing instead of a usable prompt.

**Recommended fix:** Quick-create inserts a record with blank content and immediately adds it to the wheel. Wheel selection sends that record to the injection service, which receives an empty string. The main drawer blocks saving blank prompts, so quick-create bypasses its required-content check.

**Example:** Right-click the wheel center with filter `Greeting`; `Greeting` appears as a petal, but clicking it inserts no text.

**Recommended fix:** Collect content before pinning, or open an editing flow for the newly created draft and omit drafts from the injectable wheel until populated.

### F19 · PR #4 · security · src/main.rs:646 · critical · GitHub

**Pack fetch exposes internal HTTP services** (`SEC_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/4#discussion_r4194604561

`fetch_pack_url` accepts arbitrary HTTP URLs and returns their response bodies. A caller can read loopback or intranet services through the app.

**Recommended fix:** Restrict destinations before requests and on redirects; enforce HTTPS and trusted hosts if a closed catalog can satisfy the feature.

### F20 · PR #4 · security · service/src/main.rs:108 · warning · GitHub

**Submitted variable values trigger clipboard expansion** (`SEC_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/4#discussion_r4194604705

When a filled variable contains `{{clipboard}}`, `render_template` expands it after substitution. A template can inject clipboard contents beyond its original placeholders.

**Recommended fix:** Render placeholders in a single pass over the original template, treating replacement text as literal data.

### F21 · PR #4 · analysis · src/js/views/library.js:14 · n/a · dashboard

**Library import loses wheel configuration** (`ANALYSIS_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0001`)

`validatePack` discards exported `pinned` and `inject_order`. Importing that pack through the library restores prompts without their wheel membership or positions.

### F22 · PR #4 · analysis · service/src/config/mod.rs:29 · n/a · dashboard

**Saved clipboard preferences have no effect** (`ANALYSIS_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0002`)

`allow_clipboard` and `restore_clipboard` are saved, but `inject` still tries the clipboard first and never restores it.

### F23 · PR #4 · analysis · src/js/views/prompts.js:214 · n/a · dashboard

**Manual order resets on mode switch** (`ANALYSIS_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0003`)

Switching `wheelSort` back to manual rewrites stored positions. Previously dragged wheel positions are lost.

### F24 · PR #4 · analysis · src/js/store.js:70 · n/a · dashboard

**Automatic time asks for manual entry** (`ANALYSIS_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0004`)

`AUTO_VARS` omits `time`, so `{{time}}` opens a fill form. A supplied value replaces the clock value.

### F25 · PR #4 · analysis · service/src/main.rs:108 · n/a · dashboard

**Spaced placeholders remain unrendered** (`ANALYSIS_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0005`)

`customVars` recognizes `{{ name }}`, but `render_template` replaces only `{{name}}`. The injected text retains the spaced placeholder.

### F26 · PR #4 · analysis · src/js/views/library.js:23 · n/a · dashboard

**Library imports remain hidden until reload** (`ANALYSIS_pr-review-job-9dc8a2f2d4934a038a2b0be4b5290ca0_0006`)

`importPackPrompts` loads state before inserting prompts. The post-import index uses that stale list, leaving new prompts absent from the grid.

### F27 · PR #5 · bug · src/js/wheel.js:184 · severe · GitHub

**Variable entry triggers unintended injections** (`BUG_pr-review-job-820d582cd9934630b5f94604a2063796_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/5#discussion_r4194613146

When a variable input has focus, `keydown` still treats digits and Enter as wheel selections. Typing a value can inject another prompt before the form is submitted.

**Recommended fix:** The wheel collects custom placeholder values through inputs created by [fillAndInject](src/js/wheel.js:154-180). The document listener also receives keyboard events from those inputs. Digits 1–6 and Enter select petals, while Backspace changes the wheel filter; the handler never checks the focused element. This can inject a different prompt while the user is completing the form.

**Example:** A user opens `{{criteria}}`, focuses its input, and types `3`. The wheel selects petal 3 and starts a second injection instead of merely recording `3`.

**Recommended fix:** Ignore wheel navigation keys when the event target is inside `.var-fill`, and handle form submission explicitly within `fillAndInject`.

### F28 · PR #5 · bug · src/js/views/settings.js:49 · severe · GitHub

**Saved settings leave old hotkeys active** (`BUG_pr-review-job-820d582cd9934630b5f94604a2063796_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/5#discussion_r4194613358

Changing a hotkey or injection toggle calls `apply_settings`, but `stop_service` never stops the original engine thread. Old hotkeys keep using old settings while every save starts another engine.

**Recommended fix:** The settings controls call [apply_settings](src/main.rs:1135-1169) after every change. That command attempts a restart through [stop_service](src/main.rs:85-89) and [start_service](src/main.rs:66-83). Stop only clears `is_active`; the spawned [run_service](service/src/main.rs:49-99) loop has no termination condition. Each save therefore creates another service thread while the original owns its hotkeys and injection pipe.

**Example:** Change the wheel hotkey from Ctrl+Alt+Space to Ctrl+Alt+B. The first engine keeps Ctrl+Alt+Space registered and a second engine tries to start; the saved hotkey does not reliably replace the old one.

**Recommended fix:** Add a shutdown channel or cancellation flag to `run_service`, stop and join its thread before starting a replacement, and make hotkey and pipe listeners terminate with it.

### F29 · PR #5 · bug · src/js/views/library.js:114 · severe · GitHub

**Imported prompts stay absent from the grid** (`BUG_pr-review-job-820d582cd9934630b5f94604a2063796_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/5#discussion_r4194613497

After importing a pack, `rebuildIndex` indexes the old `state.prompts` without reloading it. New prompts remain invisible in the grid and search until another refresh.

**Recommended fix:** [importPackPrompts](src/js/views/library.js:23-40) loads the prompt list before creating new database rows. Its caller rebuilds the search index without fetching the rows afterward. The grid and wheel mirror render from the old list and do not update when the import completes.

**Example:** Import a built-in pack containing `Decision Memo` into an empty library. The success toast says one prompt was added, but the prompt grid still shows empty.

**Recommended fix:** Call `loadPrompts()` after the import and before `rebuildIndex()`, then render the prompt grid and wheel mirror. Refresh likewise after settings' direct file import.

### F30 · PR #5 · security · src/main.rs:646 · critical · GitHub

**Pack fetching exposes local network responses** (`SEC_pr-review-job-820d582cd9934630b5f94604a2063796_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/5#discussion_r4194613651

`fetch_pack_url` accepts loopback and private-network URLs and returns their response bodies. A caller with access to the Tauri command can read services reachable only from the user's machine.

**Recommended fix:** Validate the resolved destination of the initial URL and each redirect against the allowed network policy before making requests; refuse loopback, link-local, and private-network addresses if these are outside the feature's scope.

### F31 · PR #5 · security · src/main.rs:647 · warning · GitHub

**Plaintext imports permit pack tampering** (`SEC_pr-review-job-820d582cd9934630b5f94604a2063796_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/5#discussion_r4194613779

`fetch_pack_url` accepts `http://` URLs. A network intermediary can replace a pack before the user previews and imports its prompts.

### F32 · PR #5 · security · src/main.rs:675 · warning · GitHub

**Oversized pack files exhaust application memory** (`SEC_pr-review-job-820d582cd9934630b5f94604a2063796_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/5#discussion_r4194613889

Selecting a large pack makes `pick_pack_file` read the entire file without a size limit. The import flow can freeze the application or exhaust memory before preview.

**Recommended fix:** Enforce a pack size limit for both file import commands before `read_to_string`, and cap pasted JSON before parsing or preview rendering.

### F33 · PR #5 · analysis · service/src/ipc/inject_server.rs:66 · n/a · dashboard

**Large variable values lose their substitutions** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0001`)

A variable value exceeding the pipe's single 8192-byte read leaves incomplete JSON. `render_template` ignores the parse failure, so the wheel injects unresolved placeholders.

### F34 · PR #5 · analysis · src/main.rs:543 · n/a · dashboard

**Pack backup drops prompt properties** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0002`)

`pack_json_from_db` omits `variables_json` and `content_type`. Import restores neither, so a pack round trip loses these prompt properties.

### F35 · PR #5 · analysis · service/src/main.rs:112 · n/a · dashboard

**Spaced placeholders survive variable entry** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0003`)

`customVars` recognizes `{{ name }}`, but `render_template` replaces only `{{name}}`. The wheel accepts a value, then injects the unresolved placeholder.

### F36 · PR #5 · analysis · src/js/views/settings.js:84 · n/a · dashboard

**Settings import bypasses pack preview** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0004`)

`import_prompts_pack` inserts all valid prompts when the file dialog closes. Unlike library import, Settings offers no preview or per-prompt selection.

### F37 · PR #5 · analysis · src/js/views/library.js:14 · n/a · dashboard

**Library import discards wheel layout** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0005`)

`validatePack` removes exported `pinned` and `inject_order` values. Importing an exported pack through the library leaves its prompts off the wheel.

### F38 · PR #5 · analysis · service/src/db.rs:454 · n/a · dashboard

**Deleted prompt shadows last-used selection** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0006`)

If the newest usage log references a deleted prompt, `resolve_default_prompt` skips surviving logged prompts. The quick hotkey falls back to an unrelated legacy selection.

### F39 · PR #5 · analysis · service/src/main.rs:108 · n/a · dashboard

**Filled values are parsed as templates** (`ANALYSIS_pr-review-job-820d582cd9934630b5f94604a2063796_0007`)

`render_template` expands automatic placeholders after inserting form values. Literal `{{date}}` inside a user's value becomes today's date.

### F40 · PR #6 · bug · service/src/injector/mod.rs:377 · severe · GitHub

**Failed paste discards clipboard contents** (`BUG_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/6#discussion_r4194613602

When paste `SendInput` fails, `inject_via_clipboard` returns before restoring the clipboard. The fallback can still type the prompt, but the user's clipboard stays overwritten.

**Recommended fix:** The clipboard strategy saves the original formats, then replaces them with the prompt before sending Ctrl+V. A zero return from `SendInput` exits before [restore_clipboard_all](service/src/injector/mod.rs:123-146) runs. The caller then falls back to typing, so a successful final injection can still destroy the original clipboard.

**Example:** The user copies a spreadsheet cell, invokes a prompt, and Windows rejects the simulated paste. The prompt is typed by the fallback, but the clipboard still contains the prompt instead of the cell.

**Recommended fix:** Ensure every path after the clipboard replacement restores the backup when restoration is enabled, using a cleanup guard or explicit cleanup before propagating the error.

### F41 · PR #6 · bug · service/src/injector/mod.rs:102 · severe · GitHub

**Partial backups destroy clipboard formats** (`BUG_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/6#discussion_r4194613797

When `backup_clipboard_all` skips a format, `restore_clipboard_all` clears it alongside saved formats. Images or large clipboard items disappear after an otherwise successful paste.

**Recommended fix:** The backup skips formats exceeding 16 MB, formats exceeding the total cap, and formats not stored in global memory. After paste, [restore_clipboard_all](service/src/injector/mod.rs:123-145) empties the entire clipboard and restores only saved formats. If nothing was saved, [the restore guard](service/src/injector/mod.rs:385-388) skips restoration altogether.

**Example:** A clipboard holds a 20 MB bitmap plus a small text representation. The bitmap is skipped, the text is saved, and restore replaces the clipboard with text only.

**Recommended fix:** Treat a partial snapshot as non-restorable: avoid overwriting the clipboard or explicitly communicate that preservation is unavailable. Handle an originally empty clipboard separately by restoring its empty state.

### F42 · PR #6 · bug · service/src/injector/mod.rs:186 · severe · GitHub

**Secure checks accumulate COM references** (`BUG_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/6#discussion_r4194613925

Each injection calls `CoInitialize` on the persistent service thread without a matching `CoUninitialize`. Successful checks accumulate COM initialization references for the service lifetime.

**Recommended fix:** COM initialization is reference-counted per thread. [run_service](service/src/main.rs:48-98) reuses one thread for injection requests, so each successful initialization increments that thread's count. This function returns without a matching `CoUninitialize`, including on early returns from a positive password probe.

**Example:** After 10,000 quick injections, the service thread has 10,000 unmatched COM initializations instead of one reusable initialization.

**Recommended fix:** Initialize COM once for the service thread, or pair each successful `CoInitialize` with `CoUninitialize` through a guard that also covers early returns.

### F43 · PR #6 · security · src/main.rs:647 · critical · GitHub

**Pack fetch reaches internal services** (`SEC_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/6#discussion_r4194614045

If an untrusted caller can invoke `fetch_pack_url`, an HTTP URL targeting localhost or a private address returns its response body. No destination checks restrict redirects or resolved addresses.

**Recommended fix:** Validate the parsed URL and every resolved destination, including redirects, before issuing a request. Limit the command to intended public pack sources and apply an explicit redirect policy.

### F44 · PR #6 · security · service/src/injector/mod.rs:187 · warning · GitHub

**Password inputs bypass the secure-field gate** (`SEC_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/6#discussion_r4194614153

For browser password inputs without their own HWND, `ElementFromHandle(focus)` inspects the parent window instead of the focused input. `CurrentIsPassword` can return false, allowing injection into that password field.

**Recommended fix:** Read the focused UIA element with GetFocusedElement rather than mapping the HWND to its root/host element; keep the classic EDIT check as a separate fast path.

### F45 · PR #6 · analysis · service/src/ipc/inject_server.rs:66 · n/a · dashboard

**Long variable messages lose their values** (`ANALYSIS_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0003`)

A request exceeding 8192 bytes is parsed from its truncated first read. The renderer ignores incomplete JSON, leaving variable placeholders in the injected text.

### F46 · PR #6 · analysis · src/js/views/library.js:14 · n/a · dashboard

**Library imports lose wheel assignments** (`ANALYSIS_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0004`)

The exported pack includes pin and order fields, but `validatePack` drops both. Importing through the library preview recreates prompts without their wheel assignments.

### F47 · PR #6 · analysis · service/src/db.rs:458 · n/a · dashboard

**Recent prompt ordering lacks a tie-breaker** (`ANALYSIS_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0005`)

Two successes in one second share a timestamp. Without an ID tie-breaker, the quick hotkey can select the older prompt.

### F48 · PR #6 · analysis · service/src/main.rs:574 · n/a · dashboard

**Spaced placeholders survive injection** (`ANALYSIS_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0006`)

The wheel collects a value for `{{ name }}`, but the service substitutes only `{{name}}`. The collected value never reaches the injected text.

### F49 · PR #6 · analysis · src/js/views/prompts.js:215 · n/a · dashboard

**Manual sorting unexpectedly reorders pins** (`ANALYSIS_pr-review-job-adbe5d78078b4b679273c3678f4e568f_0007`)

The toggle sets manual mode before capturing displayed IDs. Previously saved positions replace the visible automatic order and get rewritten.

### F50 · PR #7 · bug · src/js/views/settings.js:63 · severe · GitHub

**Settings changes multiply background engines** (`BUG_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/7#discussion_r4194617082

Changing an injection switch calls `apply_settings`, which starts a new engine without stopping the old one. `ServiceState::stop_service` only clears its flag, while [run_service](service/src/main.rs:49) runs forever. Repeated changes leave competing engines running.

**Recommended fix:** A settings change calls `apply_settings`, which stops and starts the embedded service. The service state stores only an active flag, not a join handle or shutdown signal. The original [run_service](service/src/main.rs:49-98) loop has no exit condition, so each save creates another live engine and hotkey worker. Multiple workers compete for the same hotkeys and named pipe.

**Example:** Toggle clipboard off, then on. Both saves report a restart, but the original engine and two new engines remain alive.

**Recommended fix:** Give `ServiceState` a real shutdown signal and joinable worker, and make the service loop and hotkey/pipe workers stop before starting the replacement. Verify that repeated `apply_settings` calls leave exactly one engine.

### F51 · PR #7 · bug · service/src/injector/mod.rs:128 · severe · GitHub

**Clipboard restoration loses image data** (`BUG_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/7#discussion_r4194617257

Injecting with an image plus text on the clipboard makes `restore_clipboard_all` erase the image. [backup_clipboard_all](service/src/injector/mod.rs:83-119) skips non-GMEM formats, but restoration clears every format before replaying its partial backup.

**Recommended fix:** Clipboard formats such as CF_BITMAP are handles, not global memory blocks. [backup_clipboard_all](service/src/injector/mod.rs:83-119) skips them but returns the supported formats. On injection, `restore_clipboard_all` calls EmptyClipboard and restores only that partial list. An image copied together with text therefore disappears even when clipboard restoration is enabled.

**Example:** Copy an image from an editor that exposes bitmap and text formats. Inject a prompt. Text returns, but the copied image cannot be pasted again.

**Recommended fix:** Do not claim a full restore from a partial snapshot. Either preserve unsupported formats through format-specific duplication or avoid the clipboard strategy when the existing clipboard contains formats that cannot be restored reliably.

### F52 · PR #7 · bug · service/src/ipc/inject_server.rs:66 · severe · GitHub

**Large variable values disappear during injection** (`BUG_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/7#discussion_r4194617425

Submitting more than 8192 bytes through `trigger_wheel_injection_vars` makes the pipe parse an incomplete JSON value. `render_template` ignores that parse failure and injects the unfilled prompt.

**Recommended fix:** The wheel sends the entire JSON map in one named-pipe message using [send_inject_request_vars](src/inject_pipe_client.rs:25-32). A pipe read can return fewer bytes than a write, and this server reads at most 8192 bytes once. The truncated suffix still passes `parse_message` as a string. [render_template](service/src/main.rs:108-115) silently skips malformed JSON, so the service types literal placeholders instead of the user's input.

**Example:** Paste a 10 KB document into `{{notes}}` and press Inject. Only the first 8192 bytes reach the parser; `{{notes}}` remains in the injected text.

**Recommended fix:** Frame requests by length or terminator, read until the complete frame arrives with a bounded maximum, and reject malformed JSON before queueing injection.

### F53 · PR #7 · security · src/main.rs:646 · critical · GitHub

**Pack fetch exposes local network responses** (`SEC_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/7#discussion_r4194617558

A caller can give `fetch_pack_url` a loopback or private-network URL and receive its response text. The fetch runs from the desktop host without restricting destinations or redirects.

**Recommended fix:** Validate the parsed URL and resolved address before connection, repeat validation for redirects, and enforce a destination policy suitable for pack imports.

### F54 · PR #7 · security · src/main.rs:647 · warning · GitHub

**Plaintext pack downloads allow tampering** (`SEC_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/7#discussion_r4194617701

`fetch_pack_url` accepts `http://` pack URLs. A network intermediary can replace imported prompt content in transit.

**Recommended fix:** Reject plaintext HTTP URLs for network imports or confine a documented exception to loopback development endpoints.

### F55 · PR #7 · analysis · src/js/store.js:70 · n/a · dashboard

**Spaced placeholders survive injection** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0001`)

`customVars` accepts `{{ name }}`, but [render_template](service/src/main.rs:112) replaces only `{{name}}`. The wheel collects a value that never appears in the injected text.

### F56 · PR #7 · analysis · src/js/wheel.js:64 · n/a · dashboard

**Secondary-monitor wheel placement** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0002`)

`clampToViewport` compares global window positions against monitor dimensions without adding the monitor origin. Opening the wheel on a secondary display moves it away from the cursor.

### F57 · PR #7 · analysis · src/js/views/library.js:115 · n/a · dashboard

**Library imports stay invisible** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0003`)

`importPackPrompts` loads state before inserting new prompts. The success path rebuilds the index without reloading or rerendering the grid, leaving imported prompts invisible.

### F58 · PR #7 · analysis · src/js/views/library.js:14 · n/a · dashboard

**Library round-trip loses wheel pins** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0004`)

`validatePack` discards exported `pinned` and `inject_order` values. Importing the export through a library tab creates unpinned, unordered prompts.

### F59 · PR #7 · analysis · src/js/wheel.js:25 · n/a · dashboard

**Wheel retains outdated preferences** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0005`)

`syncPrefs` reapplies cached theme and language values without rereading storage. Changing either preference in the main window leaves the already-created wheel unchanged.

### F60 · PR #7 · analysis · src/js/wheel.js:49 · n/a · dashboard

**Blur during loading leaves wheel open internally** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0006`)

`prepare` sets `open` after asynchronous loading. If blur hides the window first, `hide` exits early and the pending load marks the hidden wheel open.

### F61 · PR #7 · analysis · src/js/views/settings.js:71 · n/a · dashboard

**Default-prompt updates expose an old selection** (`ANALYSIS_pr-review-job-b378ff677e6a489b967a434ee1ba0379_0007`)

`set_app_setting` writes fixed mode before the selected ID. A quick-hotkey press between calls reads the previous ID or finds no prompt.

### F62 · PR #8 · bug · service/src/injector/windows_impl.rs:378 · severe · GitHub

**Failed paste leaves clipboard overwritten** (`BUG_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/8#discussion_r4194614635

When `SendInput` fails during a paste, `inject_via_clipboard` exits before restoring the user's clipboard. The failed injection leaves the prompt text on the clipboard.

**Recommended fix:** The clipboard strategy writes the prompt as CF_UNICODETEXT before sending Ctrl+V. If SendInput reports failure, the early return bypasses the later restore block, and the fallback strategy types the text while the clipboard remains modified. This happens even with restore_clipboard enabled.

**Example:** A user copies an image, then injects a prompt while SendInput rejects Ctrl+V. The prompt is typed by the fallback, but the image clipboard is replaced with the prompt text.

**Recommended fix:** Restore the snapshot on every path after EmptyClipboard, including SendInput errors; use a scope guard or a single cleanup path. Respect restore_clipboard and avoid clearing unsnapshotted data.

### F63 · PR #8 · bug · service/src/injector/windows_impl.rs:129 · severe · GitHub

**Image clipboard lost after injection** (`BUG_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/8#discussion_r4194614864

When the clipboard contains a skipped format alongside backed-up data, `restore_clipboard_all` clears it permanently. `backup_clipboard_all` skips bitmap and oversized formats, so a successful paste can destroy copied content.

**Recommended fix:** Clipboard backup only records formats it can read as bounded GMEM blocks. Restoration empties the whole clipboard, including formats never recorded. A bitmap or another skipped format therefore disappears even when the text injection succeeds.

**Example:** Copy an image with both bitmap and text representations, then inject a prompt with clipboard restoration enabled. Only the backed-up text representation survives; the bitmap disappears.

**Recommended fix:** Do not clear the original clipboard unless all formats can be preserved, or use an OS-supported clipboard preservation method that retains non-GMEM formats. Treat an incomplete backup as a reason to avoid the clipboard strategy.

### F64 · PR #8 · bug · src/main.rs:1090 · severe · GitHub

**Settings changes multiply active engines** (`BUG_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0003`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/8#discussion_r4194615112

Changing an injection switch restarts the service, but `stop_service` never stops the old engine. Each change spawns another listener, leaving old hotkeys active and new registrations competing.

**Recommended fix:** The UI writes injection preferences using apply_settings, which invokes stop_service and then start_service. [stop_service](src/main.rs:87-92) only changes a Boolean; [run_service](service/src/main.rs:49-98) never reads that Boolean or exits. Repeated changes therefore leave multiple engine and hotkey threads running, with each thread retaining its startup config.

**Example:** Turn clipboard injection off, then on. Three engines continue running. The initial engine still owns the registered hotkeys, so injections can keep using its original setting.

**Recommended fix:** Give the engine a stop signal and join its thread before spawning a replacement; stop its hotkey and named-pipe worker as part of shutdown. Only mark the service active after a successful start.

### F65 · PR #8 · security · src/main.rs:573 · critical · GitHub

**Pack fetch exposes private network responses** (`SEC_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0001`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/8#discussion_r4194615374

When `fetch_pack_url` receives a localhost or private-network URL, it returns the response body to the caller. The HTTP client also follows redirects without checking the final destination.

**Recommended fix:** Validate the resolved destination of the initial URL and every redirect before issuing a request. Prefer a small allowlist of trusted pack hosts; reject private, loopback, and link-local addresses and disable redirects until they can be validated.

### F66 · PR #8 · security · service/src/main.rs:108 · warning · GitHub

**User variables override automatic clipboard value** (`SEC_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0002`)

GitHub: https://github.com/Haaaiawd/PromptKey/pull/8#discussion_r4194615615

A supplied `clipboard` variable replaces `{{clipboard}}` before the service reads the real clipboard. A caller can substitute arbitrary text for an automatic variable.

**Recommended fix:** Reserve clipboard, date, and time before interpolating supplied variable values. Reject these keys or render automatic placeholders first with a single-pass parser so replacement values cannot masquerade as templates.

### F67 · PR #8 · analysis · service/src/injector/windows_impl.rs:164 · n/a · dashboard

**COM initialization grows with injections** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0001`)

Each successful `CoInitialize` in `is_secure_input` needs a matching `CoUninitialize`. The engine runs on one persistent thread, so repeated injections accumulate unmatched initializations.

### F68 · PR #8 · analysis · service/src/ipc/inject_server.rs:66 · n/a · dashboard

**Large variable requests lose values** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0002`)

The GUI sends unrestricted variable JSON, but `listen_once` reads only 8192 bytes. Truncated JSON fails parsing, leaving placeholders in injected text.

### F69 · PR #8 · analysis · service/src/main.rs:112 · n/a · dashboard

**Spaced template variables remain unresolved** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0003`)

The editor accepts `{{ customer }}`, but `render_template` replaces only `{{customer}}`. Entered values do not appear in the injected text.

### F70 · PR #8 · analysis · src/js/store.js:71 · n/a · dashboard

**Automatic time variable opens a form** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0004`)

`AUTO_VARS` omits `time`, although the service computes `{{time}}` automatically. The wheel requests a manual value and substitutes it first.

### F71 · PR #8 · analysis · src/main.rs:442 · n/a · dashboard

**Wheel cannot follow negative screen coordinates** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0005`)

`present_wheel` clamps negative cursor coordinates to zero. On displays left or above the primary display, the wheel opens away from the cursor.

### F72 · PR #8 · analysis · service/src/db.rs:457 · n/a · dashboard

**Deleted prompt masks recent surviving prompt** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0006`)

`resolve_default_prompt` reads only one usage row. If its prompt was deleted, it ignores earlier successful injections of surviving prompts.

### F73 · PR #8 · analysis · src/js/views/prompts.js:223 · n/a · dashboard

**Manual mode starts in old order** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0007`)

`state.wheelSort` switches before `wheelPrompts` captures the displayed order. Existing manual positions replace the automatic order when users switch modes.

### F74 · PR #8 · analysis · src/js/wheel.js:231 · n/a · dashboard

**Quick-created petal injects nothing** (`ANALYSIS_pr-review-job-2d34a506384e47d3baa0e5091f2d4a0b_0008`)

`quickCreate` pins an empty prompt and exposes it as a selectable petal. Selecting it injects no text until the prompt is edited elsewhere.

