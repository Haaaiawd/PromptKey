# Devin Brief: 提取 Review Flags → 落盘 → 修复 → Squash 合并 → 测试

## 项目
`/home/haa/sites/promptkey`，当前在 `feat/phase2-cross-platform-traits`。

---

# 背景

Phase 2 交付了 6 个 stacked 分支 / 6 个 PR（#3-#8），全部 open 未合。每个 PR 都有 Devin Review：
- 30 条 inline comment 已贴到 GitHub（`gh api repos/Haaaiawd/promptkey/pulls/<N>/comments` 可拿）
- 另有 44 条只存在于 Devin Review dashboard

**合计 74 条 finding。** 前一轮 Devin 已经跑通了两件事（但你不知道，重新做一遍，成本很低）：

1. dashboard 的完整数据可以拿到，方法：
   - Devin Review dashboard 是 JS SPA，直接 fetch 拿不到
   - 从 SPA 的懒加载 chunk 里发现内部 API：`https://app.devin.ai/api/pr-review/jobs` 和 `/api/pr-review/jobs-result/{jobId}/{versionId}`
   - 用 CLI 凭证做 `Bearer` 认证（Devin CLI 已登录 Windsurf OAuth，凭证在 `~/.local/share/devin/credentials.toml` 附近）
   - **`pr_path` 参数必须带 `github.com/` 前缀**，例如 `github.com/Haaaiawd/promptkey`
   - 响应是 gzip，要解压
2. 各分支的代码差异已摸清：`src/main.rs` 各分支只差 TASK-006 的 cfg-gate，被 flag 的函数在各分支是同一份；`HotkeyService.stop()` 在所有分支都已存在

**但前一轮没把 74 条落盘就退出了，你重新提取一遍并落盘。**

---

# 用户已批准的两个决定（硬要求）

## 1. 修复范围（用户原话）

> 「修完一起合并呗，让devin把其认为是问题的给修了，不是问题的就算了」

**由你判断哪些是真问题并修复，不是问题的跳过并说明理由。** 不必逐条问。每条 finding 在交付里给一行判定：`FIXED` / `SKIPPED (not a bug)` + 理由 / `SKIPPED (out of scope)` + 理由。

## 2. 合并（用户原话）

> 「对的哦，也可以做squash」

**修复后 squash 合并到 master**，方案你定（逐分支 #3→#8 squash-merge，或整体合成一个 PR），交付里说明理由。master 历史要干净可读。

## 3. 合并后必须测试（用户原话：「然后我们测试测试」）

- 前端是纯 HTML/JS，本机无 cargo 编译不了 Rust，**但前端可以真跑**
- 起静态服务器（`python3 -m http.server`），主窗口 / 轮盘 / 模板库 / 设置 / 记录全部过一遍
- 明暗 × 中英四种组合
- 轮盘 280px 跟随光标 + 打字过滤 + 数字键 1-6
- 变量填充表单、快速创建、导入预览弹窗
- 每修一条 finding，尽量用可执行方式验证（例：SSRF 防护写好脚本打本地 127.0.0.1:任意端口确认被拒）
- 静态检查：`node --check` 全 JS、i18n key 对齐、DOM id 审计
- **预览分发到 `https://box.haaaiawd.live/dl/promptkey-preview/`**（目录需自建，做法见下）

---

# 执行步骤（严格按序，每步落盘再做下一步）

## Step 0：立即落盘保护

**先做这个，再干别的。** 把 74 条 finding 写到 `.loom/design/REVIEW_FINDINGS.md`，格式：

```markdown
# Review Findings (74)

| # | PR | kind | file:line | 标题 | 判定 | 理由 |
|---|----|------|-----------|------|------|------|
| 1 | #3 | bug | src/main.rs:1162 | Settings restart leaves old engines running | | |
...
```

正文附录每条 finding 的完整原文（含 recommended fix）。**这个文件必须先 commit push**，再开始修 —— 前一轮就是丢在这里。

## Step 1：逐条读代码判定

对每条 finding：读对应的文件:行号，确认是真问题还是误报。已知几条（前一轮已确认的，你仍要自己再看一眼）：

| PR | 位置 | 问题 |
|---|---|---|
| #3 | src/main.rs:1162 | 改设置起新引擎不停旧的 → 老热键+pipe 双份存活 |
| #3 | src/js/views/settings.js:56 | 注入开关是假的，`Injector` 不读这三个值 |
| #3 | src/js/wheel.js:77 | `clampToViewport` 副屏偏移场景判断错 |
| #3 | — | 另有 11 条 dashboard-only |
| #4 | service/src/main.rs:215 | 失败注入变默认提示词（log_usage 在 inject 前写） |
| #4 | src/js/wheel.js:224 | 快速创建空内容提示词 → 注入空文本 |
| #4 | src/main.rs:653 | **SSRF**：`fetch_pack_url` 接受任意 URL，可读 loopback/内网 |
| #4 | — | 另有 6 条 dashboard-only |
| #5-#8 | 多 | 其余 44 条 |

多屏 clamp 那条特别注意：**用 `window.screen` / 还是 Tauri 的 monitor API？** 副屏偏移要用 monitor 的 work area 而非 `screen.width/height`。

## Step 2：修

- 每 PR 在**自己的分支**上修，push 到同一分支（PR 自动更新）
- 一个分支一个 fix commit：`fix: <what> (review #<PR> <short-id>)`
- **同模式全扫**：如果一个 flag 是普遍模式（一类 XSS、一类未防护 DOM、一类未校验输入），把同模式全修掉并说明扫了哪些地方
- 重点自查（这项目的已知薄弱面）：
  1. **SSRF 修复的正确性**：`ureq` 2.x 可用 `AgentBuilder::redirects(0)` 拿到 3xx 手动校验跳转。要挡：loopback (127/8, ::1)、私有段 (10/8, 172.16/12, 192.168/16)、link-local (169.254/16, fe80::/10)、多云 metadata (169.254.169.254 最重要)、非 http(s) 协议、DNS rebinding（至少解析后校验 IP，最好 pin IP）
  2. **XSS 面**：pack 导入 JSON / 模板内容 / URL 导入内容的渲染路径，全部走 textContent 或等价转义。项目有 XSS 前科（4月 PR #1 标题就是"XSS安全修复"）
  3. **路径遍历**：导入时文件名/ID 拼路径的地方
  4. **i18n 完整性**：新 UI 有没有漏翻 key
  5. **`{{var}}` 注入面**：用户控制的模板 + 用户控制的变量值
  6. **Rust 未编译**：11 个文件从没编译过，逐个人工核对 `windows` 0.52 / `rusqlite` 0.32 / `tauri` 2.8.5 / `ureq` 2.x / `serde_yaml` 0.9 的 API 签名，可疑处列出

## Step 3：Squash 合并到 master

1. 方案自定，交付里说明
2. 合并前逐分支确认 diff 符合职责
3. **合并信息必须明确写"Rust 未编译验证"**，别让 squash 掩盖这个事实
4. 合后 `git log --oneline master` + `git show master:<关键文件>` 抽查
5. **不要 auto-merge 后不报告**

## Step 4：测试

按上面 Step 3 的清单全跑，实测结果写进交付。**没做的就是没做，别把不能编译说成构建通过。**

## Step 5：分发预览

```bash
mkdir -p /home/haa/.local/caddy/data/dl/promptkey-preview
cp -a <预览文件> /home/haa/.local/caddy/data/dl/promptkey-preview/
```
（注意：box 域名的 `/dl/*` 走 Caddy 静态分发，root 是容器内 `/data`，对应宿主 `/home/haa/.local/caddy/data`。**改完 Caddyfile 必须 `docker restart caddy` 才生效**，`caddy reload` 在容器内 exec 不生效 —— 已验证过的坑。）

---

# 硬约束

- 不引新的 npm 依赖
- 不动 `00_READ_ME_FIRST.md` 到 `07_DECISIONS_AND_OPEN_QUESTIONS.md`
- 不删 LOOM 的验收证据（deliverables.json 保留在 master）
- 不做额外重构
- 拿不到的数据就说拿不到，**禁止编造 finding 内容或测试结果**

---

## 最终交付格式

```
## Step 0 — Findings 落盘
（REVIEW_FINDINGS.md 路径 + commit）

## 判定表
（74 条一行：ID / PR / FIXED 或 SKIPPED / 理由）

## 修复内容
（按分支列 fix commit + 扫了哪些 hotspot）

## Squash 合并
（方案 / 理由 / master 最终历史 / Rust 未编译的明确声明）

## 测试结果
（实际跑了什么、输出、哪些没法验）

## 预览地址
（https://box.haaaiawd.live/dl/promptkey-preview/...）

## 遗留项
（Windows 上必做：cargo check 清单、真机测试清单）
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-review-fixes-squash
deliverables:
- 74 findings extracted and persisted to REVIEW_FINDINGS.md
- adjudicated FIXED / SKIPPED with reasons
- fixes pushed per branch
- squashed and merged to master (clean history)
- frontend preview verified light/dark x zh/en
- static checks passed
- preview URL distributed
- Windows followups listed honestly
status: success
errors: none
```
