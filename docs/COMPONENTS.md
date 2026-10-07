# 前端组件与模块说明

PromptKey 前端是**纯 HTML/CSS/JS（ES modules）**，无构建步骤、无框架。
主界面 `src/index.html` + `src/styles.css` + `src/js/`；轮盘 `src/wheel.html` + `src/wheel.css` + `src/js/wheel.js`。
两个 webview 各自加载一份前端代码，通过 `__TAURI__` 桥接与 Rust 后端通信。

---

## 1. 模块划分（`src/js/`）

| 文件 | 职责 |
|------|------|
| `app.js` | 入口与外壳：视图路由、导航、`error`/`unhandledrejection` 全局兜底 toast、环境探测横幅 |
| `store.js` | 共享状态（`state.prompts` 等）+ **`ipc()` 守卫**（超时/无声开关/失败自动 toast）+ 数据派生（`wheelPrompts`、frecency 排序、`extractVars`/`renderVars` 模板变量、Fuse 搜索索引） |
| `dom.js` | DOM 小工具：`$`/`$$` 选择器、`esc()` HTML 转义、`storage` localStorage 封装 |
| `toast.js` | 通知队列：`toast('ok'|'err'|'info', msg)`，容器 `#toasts` |
| `theme.js` | 主题：`auto/light/dark`，`data-theme` attribute + `prefers-color-scheme` 媒体查询；`refreshTheme()` 供轮盘窗口重读 |
| `i18n.js` | 运行时中英切换（见 §4） |
| `icons.js` | 生成文件——Lucide v1.48 SVG path 表 + `icon(name,size)` 渲染（见 §5） |
| `hotkey_recorder.js` | 热键录入器（见 §3）；`comboFromEvent` 等纯函数可单测 |
| `wheel.js` | 轮盘窗口全部逻辑：监听 `wheel-show`/`wheel-hide`、`prepare()` 拉数据、`render()` 排花瓣、键盘/指针选择、变量表单、`envErr` IPC 失败可见化 |
| `views/prompts.js` | 提示词页：卡片/列表视图、搜索、标签筛选、编辑抽屉、⭐置顶、**指针拖拽排序** |
| `views/library.js` | 模板库页：内置包导入、URL/文件导入（SSRF 防护在后端）、导出 |
| `views/log.js` | 记录页：注入日志列表、空态 |
| `views/settings.js` | 设置页：热键录入+状态行、保存/回滚、链路诊断、**平台状态/AX 权限/开机启动** |

i18n 字典：`src/js/i18n/zh-CN.js`、`en-US.js`。

## 2. 设计系统（`src/styles.css` `:root`）

全部从 `styles.css`/`wheel.css` 实际提取。明暗双主题通过 `body[data-theme]` 覆盖变量。

### 色彩 tokens

| token | dark | light | 用途 |
|-------|------|-------|------|
| `--bg-app` / `--bg-surface` / `--bg-elevated` | `#09090b`/`#18181b`/`#27272a` | `#f4f4f5`/`#fff`/`#fff` | 应用底/面板/抬起面 |
| `--border` / `--border-strong` | `#27272a`/`#3f3f46` | `#e4e4e7`/`#d4d4d8` | 分隔/强调边 |
| `--text-primary` / `--text-secondary` / `--text-muted` | `#fafafa`/`#a1a1aa`/`#71717a` | `#18181b`/`#52525b`/`#a1a1aa` | 三级文字 |
| `--accent` / `--accent-hover` / `--accent-soft` | `#3b82f6`/`#2563eb`/15% 透明 | `#2563eb`/`#1d4ed8`/10% 透明 | 主色 |
| `--success` / `--danger` / `--warning` | `#22c55e`/`#ef4444`/`#f59e0b` | `#16a34a`/`#dc2626`/`#d97706` | 状态色 |
| `--focus-ring` | accent 45% | accent 35% | `:focus-visible` 外描边 |

### 间距 / 圆角 / 字号 / 动效

- 间距：`--sp-1…--sp-12` = 4/8/12/16/20/24/32/48px
- 圆角：`--r-sm:6` `--r-md:10` `--r-lg:14` `--r-pill:999`
- 字号：`--fs-caption:11` `--fs-small:12` `--fs-body:13` `--fs-title:15` `--fs-head:20` `--fs-hero:26`
- 动效：`--dur-fast:100ms` `--dur-normal:180ms` `--dur-slow:280ms`；缓动 `--ease-out` / `--ease-pop`（轮盘开合用 pop）。
- 全局过渡：背景/边框/文字色统一 `var(--dur-normal)`；遵循系统 `prefers-reduced-motion`。
- 工具类：`.hidden`（`display:none!important`，**隐藏元素一律用它，`hidden` 属性会被 `.set-row{display:flex}` 等覆盖**）、`.flex-spacer`、`.ico`（图标容器，`ico-sm` 14px）。

## 3. 组件清单

### 按钮体系（styles.css:116-126）
- `.btn` 基础按钮；`.btn-primary` 主操作；`.btn-danger` 危险操作；`.btn-ghost` 无边弱按钮；`.btn-icon` 方形图标钮。
- `.mini-link` 虚线下划线的微型文字链（次要操作）。
- `:disabled` 统一 `opacity:.45`。

### 开关 `.switch`
iOS 风格拨杆：`<span class="switch" role="switch">`，JS 里 `classList.toggle('on')`。
用于设置页「开机启动」等布尔项（见 `settings.js` `#swAutostart`）。

### 分段控件 `.seg`（styles.css:68-72）
`<div class="seg"><button class="on">…`，`button.on` 为选中项；`.seg-icon` 为图标版窄宽度。
用于主题三选、语言二选、排序模式。

### 抽屉 `.drawer` + `.drawer-mask`（styles.css:198-200+）
右侧滑出编辑面板：`.show` 控制开合，mask 负责点击关闭与背景压暗。
编辑提示词、新建提示词均走抽屉。

### Toast `#toasts`（toast.js）
`toast(kind,msg)` → `.toast.ok|err|info` 自动进出与过期。
全局 `error`/`unhandledrejection` 也会走这里（app.js:88-92）——**新异步路径要处理异常，否则会变成神秘 toast**。

### 卡片 `.card`（styles.css:175-191）
提示词卡片：`.card-name` + `.star`（置顶，`.on` 金色）+ `.card-preview`（首行）+ `.ctag` 标签 + `.cmeta` 元信息。
`.wm-item` 是轮盘管理区里的行式项：`.num` 序号、`.nm` 名称、`.drag` 拖柄（hover 才显形，`touch-action:none` 允许指针拖拽）。

### 热键状态行 `.hk-state`
设置页热键输入框下方小字行：`ok`（绿，已生效）/`err`（红，冲突/失败）/`warn`/`muted`（检查中）。
平台状态行复用同一视觉（`#platformState`）。

### 热键录入器 `hotkey_recorder.js`
点击输入框 → `.recording` 脉冲边框 → 捕获下一次非修饰键 keydown：
- `mainKeyFromCode`：`event.code` → 规范名（`KeyA`→A、`Digit7`→7、`F1–F24`、OEM 标点、小键盘）。
- `modsFromEvent`：Ctrl/Alt/Shift/Win 固定顺序（与服务端 `parse_hotkey` 一致）。
- `comboFromEvent`：组合校验——无修饰键→`need_modifier`、无映射码→`unsupported`。
- Esc/失焦=取消还原；录入期间 `preventDefault` 吞掉浏览器快捷键。
- 提交回调 `onCommit(combo)` 由 settings.js 接成 保存→状态→失败回滚 流程。

### 轮盘 `wheel.css`
- `#wheel`：320×320 fixed 居中容器，`display:none`→`.show` 时 `open 220ms ease-pop` 弹出。
- `.glass`：毛玻璃圆盘底（donut 形态）。
- `.center`：中心圆（logo + `.center-txt` 状态字：过滤词/页码/Esc 提示）。
- `.petal`：花瓣按钮，`--pos` CSS 变量定位（wheel.js 算极坐标），`.k` 数字角标，`.kb` 键盘高亮，`.picked` 选中动画。
- `.filter-tag`：搜索词标签；`.empty-tip`：无匹配态；`.var-fill`：变量填写表单。
- `.env-err`：**IPC 失败时的可见错误条**——轮盘是透明无边窗，没有它断链就是「按了没反应」（2.0.x 教训，wheel.js:267 注释）。

### 其他
- `.tagchip`：标签筛选 chip（`.on` 高亮）。
- `.field` / `.set-group` / `.set-row`：设置页分组与行（`.set-row` = 左标签右控件 flex 行；`.sub` 次级说明）。
- `.pane-narrow`：窄内容栏。
- `.env-err`（index.html 版）：无桥接环境的顶部横幅（`no-bridge`/`acl-denied` 两种，见 `probeIpcEnvironment`）。

## 4. i18n 机制（`i18n.js`）

- 字典即 JS module：`DICTS={'zh-CN':zhCN,'en-US':enUS}`，纯 key→string 平铺。
- `t(key, vars)`：当前语言查表 → 回落 zh-CN → 回落 key 本身；`{var}` 占位替换。
- 偏好存 `localStorage['pk-lang']`：`auto`（跟 `navigator.language`，zh 前缀→中文）/ `zh-CN` / `en-US`。
- 运行时切换：`setLangPref()` → `applyI18n()` 扫 `[data-i18n]`（textContent）、`[data-i18n-html]`（innerHTML）、`[data-i18n-ph]`（placeholder）、`[data-i18n-title]`（title）→ 通知 `onLangChange` 监听者重渲染动态内容。
- **新增语言**：`src/js/i18n/<locale>.js` 复制一份字典 → `i18n.js` 顶部 import + `DICTS` 登记 → 设置页语言 `.seg` 加按钮。新 key 必须两本字典都加。
- 轮盘窗口独立进程内存：每次 `wheel-show` 前 `refreshI18n()`/`refreshTheme()` 重读 localStorage（否则窗口语言会停留在创建时）。

## 5. 图标（`icons.js`）

- **生成文件，勿手改**：`const P={name:svgPath}` 是从 Lucide v1.48（ISC）提取的 path 表；`icon(name,size)` 返回完整 `<svg>` 字符串（24 viewBox、stroke=currentColor、aria-hidden）。
- `ICON_NAMES` 导出全部可用名（55 个，含 `shield-check`、`loader-pinwheel` 等）。
- 用法：模板里 `data-icon="name"` 由视图渲染时统一 `icon()` 注入，或直接 `` `${icon('star',16)}` ``。
- **加新图标**：从 Lucide 源 SVG 取 `<path>` 等 inner 内容加进 `P`；保持 ISC 许可。
- 品牌图标是 PNG（`src/icons/brand/`：`logo-56.png`、`tray-template.png` 等，`scripts/make_icons.py` 从 master 派生）。

## 6. 测试资产（`tests/e2e/`）

四个 Playwright 套件，均本地起 `src/` 静态服务器 + `add_init_script` 桩掉 `__TAURI__`（真实前端、假后端）：

| 套件 | 覆盖 |
|------|------|
| `hotkey_recorder_e2e.py` | 录入器：修饰预览、组合提交、Esc/失焦取消、无修饰/不支持键拒绝（22 断言） |
| `hotkey_status_e2e.py` | 状态行：ok/conflict/engine-failed 渲染 + 保存冲突时 err toast 与回滚（10 断言） |
| `wheel_sort_drag_e2e.py` | 指针拖拽排序：抬起/悬停插入位/松手提交/键盘可达性（17 断言） |
| `no_tauri_e2e.py` | 无 `__TAURI__` 环境降级：横幅提示、不白屏（13 断言） |

运行：`python3 tests/e2e/<suite>.py`（需 `pip install playwright && playwright install chromium`）。
截图脚本 `scripts/screenshots.py` 复用同一桩机制产出 `docs/screenshots/`。
