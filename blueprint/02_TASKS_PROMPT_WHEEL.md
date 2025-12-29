# Task Decomposition - PromptWheel Feature

**Project**: PromptKey  
**Blueprint Phase**: Approved  
**RFC Reference**: `blueprint/02_RFC_PROMPT_WHEEL.md`  
**Audit Score**: 6/10 (APPROVED)

---

## 📋 Task List

### Legend
- **ID**: TW001, TW002... (TW = Task-Wheel)
- **[P]**: Parallelizable (can run independently)
- **[CHK]**: Verification checkpoint (story milestone)
- **User Story**: Maps to PRD (US01-US07)
- **→**: Dependency (A → B means B depends on A)

---

### Phase 1: IPC Infrastructure (Backend)

#### TW001 - Create Inject Pipe Server Module
- **User Story**: US04 (Direct Injection)
- **File**: `service/src/ipc/inject_server.rs`
- **Description**: Create Named Pipe server listening on `\\.\pipe\promptkey_inject` for `INJECT_PROMPT:{id}` messages.
- **Dependencies**: None
- **Done When**:
  1. `inject_server::start()` spawns async listener thread
  2. Parses message format `INJECT_PROMPT:123\n`
  3. Returns `Option<i64>` (prompt_id) or None on parse error
  4. `cargo check -p service` passes

#### TW002 - Integrate Inject Listener in Service Main Loop
- **User Story**: US04
- **File**: `service/src/main.rs`
- **Description**: Start inject_listener thread and handle incoming prompt IDs via channel.
- **Dependencies**: TW001
- **Done When**:
  1. `main()` calls `inject_listener::start()`
  2. Main loop receives prompt_id from channel
  3. Manual test: GUI sends message → Service logs "Received inject request: 123"

#### TW003 - Modify handle_injection_request with force_prompt_id
- **User Story**: US04
- **File**: `service/src/main.rs` (existing function)
- **Description**: Add `force_prompt_id: Option<i64>` parameter. If Some, skip selected_prompt_id query and use forced value.
- **Dependencies**: None (pure refactor)
- **Done When**:
  1. Function signature updated
  2. All existing callers pass `None` (backward compatible)
  3. Wheel path passes `Some(id)`
  4. `cargo check -p service` passes

#### TW004 - [P] Create GUI IPC Client for Inject Pipe
- **User Story**: US04
- **File**: `src/inject_pipe_client.rs`
- **Description**: Create client to write `INJECT_PROMPT:{id}\n` messages to inject pipe.
- **Dependencies**: None (parallel to service work)
- **Done When**:
  1. `send_inject_request(id: i32) -> Result<()>` function exists
  2. Opens pipe `\\.\pipe\promptkey_inject` as client
  3. Writes formatted message and flushes
  4. `cargo check` passes

#### TW005 - Implement trigger_wheel_injection Tauri Command
- **User Story**: US04
- **File**: `src/main.rs`
- **Description**: Create Tauri command that calls inject_pipe_client and returns immediately.
- **Dependencies**: TW004
- **Done When**:
  1. `#[tauri::command] fn trigger_wheel_injection(prompt_id: i32) -> Result<(), String>`
  2. Calls `inject_pipe_client::send_inject_request(id)`
  3. Registered in `tauri::Builder` invoke_handler
  4. Frontend can call via `invoke('trigger_wheel_injection', {promptId: 123})`

#### TW005-CHK - [Verification] Verify IPC injection Flow
- **Type**: Checkpoint
- **Dependencies**: TW001-TW005
- **Done When**:
  1. Start Service in one terminal (`cargo run --release -p service`)
  2. Start GUI in another (`cargo run --release`)
  3. GUI console: `invoke('trigger_wheel_injection', {promptId: 1})`
  4. Service logs: "Injecting prompt ID=1"
  5. Text appears in active window (e.g., Notepad)

---

### Phase 2: Backend Data Query

#### TW006 - Implement get_top_prompts_paginated Tauri Command
- **User Story**: US05, US06 (Data + Pagination)
- **File**: `src/main.rs`
- **Description**: SQL query Top N prompts with pagination support, return WheelPromptsPage struct.
- **Dependencies**: None
- **Done When**:
  1. Struct `WheelPromptsPage {prompts, current_page, total_pages, total_count}` defined
  2. Struct `WheelPrompt {id, name, content}` defined
  3. SQL: `SELECT ... ORDER BY COUNT(u.id) DESC ... LIMIT ?1 OFFSET ?2`
  4. Separate query for total_count
  5. Command callable from frontend: `invoke('get_top_prompts_paginated', {page: 0, perPage: 6})`
  6. Manual test: Returns 6 prompts for page=0, correct total_pages

#### TW007 - Update Usage Logging for Wheel Action
- **User Story**: US04 (Logging)
- **File**: `service/src/main.rs` (handle_injection_request)
- **Description**: When `force_prompt_id` is used, log usage with `action='wheel_select'`.
- **Dependencies**: TW003
- **Done When**:
  1. After successful injection, calls `db.log_usage()` with `action='wheel select'`
  2. Manual test injection → Check `usage_logs` table has new row with action='wheel_select'

---

### Phase 3: Frontend UI

#### TW008 - Create wheel.html Structure
- **User Story**: US01 (Radial Display)
- **File**: `src/wheel.html`
- **Description**: HTML structure with 6 petal divs, center overlay, and pagination controls.
- **Dependencies**: None (can start in parallel)
- **Done When**:
  1. `<div class="wheel-container">` with 6 `.petal` children
  2. `<div class="center-overlay">` for prompt preview
  3. `<div class="pagination">` with ◀ text ▶ buttons
  4. Loads `wheel.css` and `wheel.js`
  5. File viewable in browser (static, no functionality yet)

#### TW009 - Implement wheel.css with Glassmorphism  
- **User Story**: US01, US07 (Visual Style)
- **File**: `src/wheel.css`
- **Description**: CSS for circular layout, 6 clip-path petals, backdrop-filter glassmorphism.
- **Dependencies**: TW008
- **Done When**:
  1. `.wheel-container` has `border-radius: 50%`, `backdrop-filter: blur(10px)`
  2. `.petal:nth-child(N)` rotates by 60deg increments
  3. `.petal` uses `clip-path: polygon(...)` to create pie slice
  4. `:hover` effect on petals (brightness increase or border glow)
  5. Static HTML displays 6 colored petals in circle

#### TW010 - Implement wheel.js Core Logic
- **User Story**: US02, US04 (Selection + Injection)
- **File**: `src/wheel.js`
- **Description**: Load prompts, render to petals, handle click to trigger injection.
- **Dependencies**: TW006 (backend query), TW005 (injection command)
- **Done When**:
  1. `init()` calls `invoke('get_top_prompts_paginated', {page: 0, perPage: 6})`
  2. `renderWheel(prompts)` updates `.petal` text content (truncate to 10 chars)
  3. `selectPrompt(id)` calls `invoke('trigger_wheel_injection', {promptId: id})`
  4. Mouse click on petal triggers selectPrompt
  5. Manual test: Click petal → injection happens

#### TW011 - Implement Keyboard Navigation (1-6, PageUp/Down, ESC)
- **User Story**: US03, US06 (Keyboard Support + Pagination)
- **File**: `src/wheel.js` (extend event listeners)
- **Description**: Number keys 1-6 select petals, PageUp/Down change pages, ESC closes.
- **Dependencies**: TW010
- **Done When**:
  1. `keydown` event listener registered
  2. Keys '1'-'6' call `selectPromptByIndex(keyNumber - 1)`
  3. `PageDown` increments current_page, calls `loadPage(page+1)`
  4. `PageUp` decrements page (if page > 0)
  5. `Escape` calls `window.hide()`
  6. Manual test: Press '1' → first petal selected + injection

#### TW012 - Petal Hover Preview in Center
- **User Story**: US02 (Content Display)
- **File**: `src/wheel.js` + `wheel.css`
- **Description**: Mouse hover on petal shows full name in center overlay.
- **Dependencies**: TW010
- **Done When**:
  1. `mouseover` event on `.petal` updates `.center-overlay` text
  2. CSS ensures center overlay is visible, centered, and readable
  3. Manual test: Hover over petal with long name → center shows untruncated text

---

### Phase 4: Window Configuration

#### TW013 - Configure Selector Window for Wheel UI
- **User Story**: US01, US07 (Window Properties)
- **File**: `src/main.rs` (window pre-creation)
- **Description**: Modify selector-panel window to load `wheel.html` and set transparent: true.
- **Dependencies**: TW008 (wheel.html exists)
- **Done When**:
  1. `WindowBuilder::new().title("selector-panel").url("/wheel.html")`
  2. `transparent(true), decorations(false)` set
  3. Window size 400x400 (or configurable)
  4. `cargo run` shows transparent wheel window when triggered

#### TW014 - Center Window on Screen
- **User Story**: US01 (Position)
- **File**: `src/main.rs` (show_selector_window command or ipc_listener)
- **Description**: Calculate screen center and position window before showing.
- **Dependencies**: TW013
- **Done When**:
  1. Uses Tauri `current_monitor()` to get screen dimensions
  2. Centers window: `set_position(LogicalPosition::new(x, y))`
  3. Manual test on dual monitor setup → window appears on active screen center

#### TW015 - Configure Tauri Windows Transparency Settings
- **User Story**: US07 (Glassmorphism)
- **File**: `tauri.conf.json`
- **Description**: Ensure webview settings support backdrop-filter on Windows.
- **Dependencies**: None
- **Done When**:
  1. `"webviewInstallMode": "embedBootstrapper"` or equivalent (if needed for Windows transparency)
  2. Test on Windows: window has no black background artifact
  3. `backdrop-filter` CSS actually blurs content behind window

---

### Phase 5: Integration & Polish

#### TW016 - E2E Manual Test Full Flow
- **Type**: Verification
- **Dependencies**: All TW001-TW015
- **Done When**:
  1. Press `Ctrl+Shift+H` → Wheel window appears at screen center
  2. Window shows 6 prompts (or fewer if DB has <6)
  3. Hover petal → center shows full name
  4. Click petal → text injected to active app (test in Notepad, VS Code)
  5. Press number key '3' → injects 3rd prompt
  6. Press PageDown → shows next 6 prompts (if total > 6)
  7. Press PageUp → returns to first page
  8. Press ESC → window closes
  9. Check `usage_logs` table → new rows with action='wheel_select'

#### TW017 - Performance Test (<500ms Criteria)
- **Type**: Non-functional test  
- **Dependencies**: TW016
- **Done When**:
  1. Hotkey press to window display < 500ms (measure with stopwatch or logging)
  2. Click to injection to text appearance < 500ms
  3. Page change <100ms
  4. No perceivable UI lag during interactions

#### TW018 - UI Polish (Truncation, Colors, Focus States)
- **Type**: Refinement
- **File**: `wheel.css`, `wheel.js`
- **Dependencies**: TW016
- **Done When**:
  1. Petal text truncation works correctly (CSS `text-overflow: ellipsis`)
  2. Focused/hovered petal has clear visual distinction
  3. Empty petals (last page if totalPrompts % 6 != 0) are hidden or  grayed out
  4. Pagination buttons disabled at boundaries (◀ on page 0, ▶ on last page)
  5. User feedback: "UI looks polished and intuitive"

---

## 🔗 Dependency Graph

```
Phase 1 (IPC):
TW001 (inject_server) → TW002 (service integration)
TW004 (GUI client) [P] → TW005 (tauri command)
TW003 (force_prompt_id) [P]
  → TW002, TW005 → TW005-CHK

Phase 2 (Backend):
TW006 (paginated query) [P]
TW007 (logging) (depends on TW003)

Phase 3 (Frontend):
TW008 (HTML) → TW009 (CSS) → TW010 (JS core)
TW010 → TW011 (keyboard) [parallel with TW012 (hover)]

Phase 4 (Window):
TW013 (config) → TW014 (position)
TW015 (transparency) [P]

Phase 5 (Integration):
All above → TW016 (E2E) → TW017 (perf) → TW018 (polish)
```

---

## 📊 Summary

| Phase | Total Tasks | Verification Tasks |
|-------|-------------|--------------------|
| 1 (IPC) | 5 + 1 CHK  | 1                  |
| 2 (Backend) | 2      | 0                  |
| 3 (Frontend) | 5     | 0                  |
| 4 (Window) | 3        | 0                  |
| 5 (Integration) | 3  | 3                  |
| **Total** | **18 + 1 CHK** | **4**          |

---

## ✅ Acceptance Criteria

Before proceeding to `/build`:
- [ ] All 18 tasks have unique IDs
- [ ] Dependencies explicit (→ notation or [P])
- [ ] Each task has "Done When" criterion
- [ ] No task contains actual code snippets
- [ ] User approved this decomposition

---

**Next Step**: Proceed to `/build` workflow for sequential implementation.
