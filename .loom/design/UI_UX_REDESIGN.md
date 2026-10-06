# UI/UX 全面革新设计稿

> Loom slug: `ui-ux-redesign` | Kind: design | Phase 1 最重交付物
> 配套原型：`.loom/design/prototypes/`（主窗口 1 个 + 轮盘候选 3 个，均可交互）

## 结论

**UI 库选型：坚持 vanilla HTML/CSS/JS，不引入框架。** 理由是 Tauri 打包体积、构建链复杂度、迁移风险与"稳定性优先"原则——现状问题不是缺框架，而是缺**设计系统**：本方案把 Zinc 工业风扩展成明暗双套 token、定义字阶/间距/动效/i18n 四层规范，用 ES module 拆分替换 `main_simple.js` 单文件即可获得框架化收益。轮盘重设计推荐 **A 案：280px Mini 径向 + 跟随光标 + 打字过滤**（3 个候选原型全部做出供对比）。导航 5→4，明暗/中英均可运行时切换。

---

## 1. UI 库选型决策（先决策）

| 方案 | 打包体积 | 构建复杂度 | 动画能力 | 维护风险 | 判定 |
|---|---|---|---|---|---|
| **Vanilla + 设计系统化** | +0 KB | 无构建链，`frontendDist` 不变 | CSS transition/animation 足够（本应用无复杂状态同步动画） | 低；需自律维持模块化 | ✅ **推荐** |
| Tailwind | ~30KB css | 需 PostCSS/build step 改造 | 无本质增强 | 中（样式与设计脱节） | ❌ |
| React + shadcn | ~200KB+ 运行时+构建 | `beforeBuildCommand` 需加 npm build；`frontendDist` 指向 dist/ | 过剩 | 高（1365 行重写 + 状态模型重建） | ❌ |
| Preact + htm（无构建） | ~10KB | 零构建 | 中 | 低 | 🟡 降级退路（若 vanilla 失控再启用） |

**结论**：GUI_RENOVATION_GUIDE.md 定的 Zinc/slate 工业深色语言保留并扩展；"用这个 UI 库延续"= 延续这套设计语言而非引新库。`tauri.conf.json` 的 `frontendDist`/`beforeBuildCommand` **都不需要改**。

代码组织（不改技术栈只改结构）：`main_simple.js`(1365 行) 拆为 `js/app.js`（引导）、`js/store.js`（状态+invoke 包装）、`js/i18n.js`、`js/toast.js`、`js/views/prompts.js` 等——仍是浏览器原生 ES modules，零构建。

## 2. 设计系统 Token（明暗双套）

### 2.1 色彩

延续 Zinc（`GUI_RENOVATION_GUIDE.md:31-41`），亮色为镜像推导：

| Token | Dark | Light |
|---|---|---|
| `--bg-app` | `#09090b` | `#f4f4f5` |
| `--bg-surface` | `#18181b` | `#ffffff` |
| `--bg-elevated` | `#27272a` | `#ffffff`（+shadow 区分） |
| `--border` | `#27272a` | `#e4e4e7` |
| `--border-strong` | `#3f3f46` | `#d4d4d8` |
| `--text-primary` | `#fafafa` | `#18181b` |
| `--text-secondary` | `#a1a1aa` | `#52525b` |
| `--text-muted` | `#71717a` | `#a1a1aa` |
| `--accent` | `#3b82f6` | `#2563eb`（加深保 4.5:1 对比） |
| `--accent-hover` | `#2563eb` | `#1d4ed8` |
| `--accent-soft` | `rgba(59,130,246,.15)` | `rgba(37,99,235,.10)` |
| `--success` | `#22c55e` | `#16a34a` |
| `--danger` | `#ef4444` | `#dc2626` |
| `--warning` | `#f59e0b` | `#d97706` |
| `--wheel-glass` | `rgba(24,24,27,.72)` | `rgba(255,255,255,.82)` |

**轮盘窗口是透明背景**（`wheel.css` body 透明 + `--w95,64,92` 渐变），明暗两套下都必须保证"任意桌面壁纸可读"：方案=轮盘玻璃层 blur 24px + saturate 140% + 内描边 `border: 1px solid rgba(255,255,255,.12)`（暗）/`rgba(0,0,0,.08)`（亮），文字始终用主题 text token。

### 2.2 字阶 / 间距 / 圆角 / 阴影

```
字阶（system-ui 栈 + 中文 PingFang/Microsoft YaHei/Noto Sans CJK）：
  --fs-caption 11px / --fs-small 12px / --fs-body 13px
  --fs-title 15px-600 / --fs-head 20px-650 / --fs-hero 26px-700
间距（4pt）：4 8 12 16 20 24 32 48
圆角：--r-sm 6 / --r-md 10 / --r-lg 14 / --r-pill 999 / --r-round 50%
阴影：
  --shadow-sm   0 1px 2px rgba(0,0,0,.25)
  --shadow-md   0 4px 16px rgba(0,0,0,.35)
  --shadow-lg   0 12px 40px rgba(0,0,0,.45)
  --wheel-glow  0 0 0 1px border + 0 20px 60px rgba(0,0,0,.5)
```

### 2.3 动效 Token（新增）

```
时长：--dur-fast 100ms / --dur-normal 180ms / --dur-slow 280ms
缓动：--ease-out  cubic-bezier(.16,1,.3,1)     ← 主缓动（iOS 式利落）
      --ease-pop  cubic-bezier(.34,1.4,.44,1)  ← 轻回弹（轮盘展开）
      --ease-linear linear
语义：
  panel-switch   opacity + translateY(6px)     180ms ease-out
  card-hover     translateY(-1px)+shadow-sm    100ms
  wheel-open     scale(.82→1)+opacity          220ms ease-pop
  wheel-petal-in stagger 30ms/瓣               每瓣 fade+scale
  wheel-close    scale(1→.9)+opacity           120ms ease-out
  toast-in       translateY(-8px)+opacity      180ms
  select-pulse   accent ring 扩散              300ms
全部动效在 prefers-reduced-motion 下降为 opacity-only/零位移
（wheel.css 已有媒体查询骨架 :180-188，扩展为全局规范）
```

**性能红线**：动画只碰 `transform`/`opacity`（GPU 合成层）；禁止 width/height/top/left 动画；轮盘 `backdrop-filter: blur` 设上限 24px（实测更低的 blur 在集成显卡上掉帧）。

## 3. 轮盘重设计（用户点名，3 候选）

**问题诊断**（现状 `wheel.css:6-7`）：`--wheel-size:500px` 直径占 1080p 屏幕高约 46%；中心 `PromptWheel` 圆盘 140px 是纯装饰；径向优势（肌肉记忆+大命中区）被过大尺寸稀释为"满屏玻璃"；分页靠 PageUp/Down 违反"快"的本质。

三个候选都做成交互原型（`prototypes/wheel-a/b/c.html`）：

### 候选 A —— Mini 径向（推荐）
- **直径 280px**（缩小 44%），6 扇区 × 内半径 46px → 每瓣命中区 ~60×44px，仍满足 Fitts
- **跟随光标弹出**：以光标为圆心展开（径向菜单惯例，移动距离≈0）；屏幕边缘 clamp 16px
- **打开即听键**：敲字 → 扇区实时变为 Fuse 过滤结果（突破 6 格限制，消灭独立 selector 的存在理由）
- 中心缩为 56px 圆：显示过滤状态/页码/`Esc`
- 数字键 1-6 直选保留；`Esc`/点击外部关闭；选中 pulse 300ms 后收起+注入
- 分页键 `←→` 保留兼容但默认排序 frecency 使翻页极少发生

### 候选 B —— 半环 Arc Menu
- 120px 半径扇形从光标向右下展开 120°，最多 5 项 + "更多…"
- 优点：移动距离最短、视觉最轻；缺点：项数受限、非常规控件学习成本、扇形边命中区三角化
- 适合极客向；作为备选不做主推

### 候选 C —— 紧凑 Palette Strip
- 320×自适应（≤6 行）纵向列表，跟随光标或屏幕中心上 1/3 处
- 打字过滤为主交互（类 Spotlight）；每行 32px = icon+名称+快捷键
- 优点：搜索最强、项数无限、实现最简单；缺点：完全失去径向肌肉记忆与品牌形态

### 推荐与论证

**推荐 A，C 作设置里的"列表模式"备选。** 从"热键→注入完成"的代价论证：

| 维度 | A Mini径向 | B 半环 | C 列表 |
|---|---|---|---|
| 常用路径步数 | 热键→点瓣 = 2 | 同左 | 热键→看列表→点行 = 2~3（多了视觉扫描） |
| 深层项路径 | 打字过滤 = 热键+~2键+点 | 需翻页，弱 | 打字过滤，最强 |
| 视觉干扰 | 280px 圆，跟随光标，注视范围最小 | 最小 | 中（列表需逐行读） |
| 肌肉记忆 | 位置固定可记忆（径向核心价值） | 有 | 无（每次位置依赖列表长度） |
| 实现/风险 | 现有 DOM 结构可保留改造 | 全新 | 最简单 |
| 品牌一致性 | 保留"轮盘"形态（用户只说要美化缩小，没说消灭） | 变形 | 消灭 |

A 案同时满足："缩小"（500→280）、"美化"（玻璃+微动效+真正的 petal-in 交错动画）、"样式要改"（中心装饰→功能圆点、分页→过滤）、"UX 角度"（跟随光标=最短 Fitts 距离）。

**高分屏**：轮盘尺寸用 `px` 逻辑像素 + Tauri `scale_factor` 换算，DPI≥2 时 `-webkit-font-smoothing` 修正；跟随光标取 `cursor_position`（`src/main.rs` 已取 cursor_pos 做 wheel_pos）。

## 4. 信息架构与页面布局

导航 4 项（MERGE_ARCHITECTURE.md §2）：`提示词 / 模板库 / 记录 / 设置`。窗口 1000×700 保留，侧边栏 200px。

**提示词页**（合并后核心，布局见 MERGE_ARCHITECTURE.md §3 ASCII 图）：
- 顶部工具栏：搜索框（左 40%）+ 标签筛选下拉 + 排序 + 视图切换 + "新建"
- 左栏 220px：⭐ 轮盘镜像列表（只读+排序入口）+ 标签云
- 右栏主区：卡片网格（默认）/ 紧凑列表切换；卡片 = 名称/首行摘要/标签/⭐/使用次数/更新时间/⋯菜单
- 编辑：右侧抽屉 420px 滑入（`transform` 动画）——名称/内容（等宽 textarea, 行数自适应）/标签 input/变量检测预览/pin/应用范围/保存

**模板库**：来源 tab（内置精选/链接导入/文件导入）+ 卡片网格（包名/作者/条数/预览）+ 导入确认对话框
**记录**：顶部统计条（总次数/成功率/平均耗时——接入修好的真实数据）+ 表格（时间/提示词/应用/策略/耗时/状态）+ 筛选 + "清空"
**设置**：分组列表（通用：语言/主题/跟随系统；热键：4 条录制框；注入：延迟/策略开关/剪贴板恢复；数据：导入导出/数据库路径；关于）
**通用组件**：Toast（右上，成功/错误/info）、确认对话框、空态插画、快捷键提示 kbd 样式

## 5. i18n 方案（中英双版）

- **字典文件**：`src/i18n/zh-CN.json` + `en-US.json`，运行时 `fetch` 加载
- **运行时切换**：设置页选择 → `localStorage` + `settings.json` 双写 → 所有窗口收到 `language-changed` IPC 重渲染（主窗/轮盘各自持一份 `t()`）
- **绑定方式**：HTML 元素 `data-i18n="nav.prompts"` 批量替换 textContent；动态文案 `t('toast.saved', {name})` 插值；`document.documentElement.lang` 同步切换
- **默认**：`navigator.language` 探测，zh→中文其余→英文，用户可覆盖
- **后端串**：service 返回的错误串暂保持英文，前端按错误码映射

**Key 清单**（从 `index.html`/`wheel.html`/`main_simple.js` 全量抽取，~60 键）：

```
nav.prompts / nav.library / nav.log / nav.settings
nav.appTagline                          "Prompt Manager"
search.placeholder                      "搜索提示词..."
filter.tag / filter.sort / filter.view.card / filter.view.list
wheel.sectionTitle                      "⭐ 轮盘"
wheel.preview / wheel.empty             "轮盘为空，给提示词加星即可加入"
prompt.new / prompt.edit / prompt.delete / prompt.copy
prompt.field.name/content/tags/vars/apps/pinned
prompt.usageCount                       "使用 {n} 次"
prompt.updatedAt                        "更新于 {t}"
library.tab.builtin/url/file
library.import / library.export / library.confirmImport
log.stat.total/successRate/avgLatency
log.col.time/prompt/app/strategy/duration/status
log.empty / log.clear
settings.group.general/hotkey/injection/data/about
settings.language / settings.theme / settings.themeAuto
settings.hotkey.direct/context/search/wheel
settings.inject.delay/allowClipboard/restoreClipboard
settings.data.import/export/dbPath
common.save/cancel/confirm/delete/close/preview
toast.saved/deleted/imported/copied/error
wheel.centerLabel / wheel.pageOf / wheel.hintKeys
empty.prompts                           "暂无提示词 → 去模板库导入或新建第一条"
err.loadFailed/saveFailed/injectFailed
```

（zh 值为上表右列；en 值在原型 `app.js` 的 `I18N` 字典里已实装两套供参考。）

## 6. 明暗模式

- 双层 CSS 变量：`body.theme-light`/`body.theme-dark` 覆盖 token；JS 单函数 `applyTheme(mode)`，`mode ∈ {light, dark, auto}`
- `auto` = `prefers-color-scheme` + `matchMedia` 监听系统切换
- 持久化 `localStorage.pk-theme` + settings.json；三个窗口独立读取同一键，轮盘打开时读最新值
- 切换动画：仅 `--bg-app`/`--text-primary` 加 `transition: background-color 180ms`（全元素过渡会闪）
- 轮盘透明窗：明暗都可读的关键是玻璃层自带底色 + 描边（§2.1），不依赖桌面亮度

## 7. 搜索策略评估（brief 原话：现有策略不错但可有更优）

现状：Fuse.js 前端模糊搜索（`fuse.min.js` 本地版，`main_simple.js:206-208`），权重 name .6/tags .25/content .15，实时过滤。
评估：方案本身**继续保留**（轻、零后端、即时），加两个零成本增强：
1. **前缀快进语法**：`#tag` 按标签、`@name` 只搜名称、`c:xxx` 强制全文——重度用户效率刚需，~20 行解析
2. **fallback**：若未来库 >5000 条 Fuse 变慢，降级方案=后端 SQL `LIKE` + 前端只渲染（WAL 模式读很快）——先不做，记录在案

## 8. 前端稳定性加固清单

| 现状问题 | 证据 | 措施 |
|---|---|---|
| `alert()` 当兜底 | `main_simple.js:564,655` 等 6 处 | 全部换 Toast 组件；函数统一 `notify(type,msg)` |
| `updateDebugInfo` 当日志 | `index.html:209` debug-info DOM + `:1284` 函数 | 删除 debug 条；错误走 Toast + console |
| invoke 裸调无保护 | 全文 30+ 处直接 `invoke()` | `store.js` 统一包装：超时 5s + 异常 toast + 错误码 |
| DOM 查询不判空 | 全文 `getElementById` 直接使用 | 视图模块入口集中 `qs()` 断言，缺失即报错不静默 |
| IPC 断线无提示 | `ipc_listener.rs` 静默退出 | 前端心跳检测，断线 toast "后台服务已断开" |
| 编辑误关丢数据 | modal 直接 `display:none` | 关闭前 dirty 检查 + 确认 |
| localStorage 裸调 | 多处 | 包一层 try/catch（隐私模式会抛） |
| 轮盘窗口 resize 抖动 | `main.rs` resize 逻辑 + `--wheel-size` 硬编码 | 尺寸改 280px 后无需 resize；移除该路径 |
| `window.onerror` 无 | 全文无全局兜底 | 加 error/unhandledrejection → Toast + 写 log 通道 |

## 9. 无障碍

- 对比度：text/bg ≥ 4.5:1（token 表已按此校准）
- 键盘：全页面可 Tab 到达；轮盘数字键/Esc/方向键；提示词页 `↑↓` 选卡 `Enter` 打开
- `prefers-reduced-motion`：见 §2.3
- 焦点环：`outline: 2px solid var(--accent)` offset 2px（不用 `outline:none`）

## 10. 与现有代码的差异清单（Phase 2 施工索引）

1. `styles.css`：重构为 `tokens.css`（变量层）+ `base.css` + `components/` + `views/`（物理拆分或单文件分区均可）
2. `index.html`：nav 5→4；删 wheel-panel DOM；body 默认 `theme-auto`；所有硬编码文案换 `data-i18n`
3. `wheel.html/css/js`：280px、跟随光标、过滤输入、中心圆点重构、删分页按钮视觉（保留键位）
4. `main_simple.js`：拆 ES modules；selector.* 三个文件废弃（能力并入轮盘过滤）；删除 market 空壳改模板库
5. 新增：`i18n/*.json`、`js/toast.js`、`js/store.js`
