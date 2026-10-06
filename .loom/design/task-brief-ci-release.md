# Devin Brief: GitHub Actions 云端构建 + Release 流程 + 版本 bump

## 项目
`/home/haa/sites/promptkey`，分支从当前 `master` 新建。

## 现状（已核实，别重复查）

| 项 | 值 |
|---|---|
| Tauri | 2.8.5（`Cargo.lock`），Rust |
| 前端 | **纯 HTML/CSS/JS，零构建** —— **仓库里没有 package.json**，`tauri.conf.json` 的 `beforeBuildCommand`/`beforeDevCommand` 都是空字符串，`frontendDist: "./src"` |
| edition | 根 `Cargo.toml` = 2021，**`service/Cargo.toml` = 2024**（需要较新 Rust stable，CI 里别用太旧的 toolchain） |
| bundle targets | `"all"`，icon 只有 `PromptKey.ico`，nsis `perMachine` + 语言选择器（SimpChinese/English） |
| 当前版本 | `tauri.conf.json` version = **1.2.1**（**Phase 2 大改后从没 bump 过**） |
| 最新 release | v1.2.1（2026-01-07），资产 `PromptKey_1.2.1_x64-setup.exe` (3.0MB) + `.msi` (4.2MB) |
| CI | **完全没有 `.github/` 目录** |
| 未编译验证 | **11+ 个 Rust 文件从未编译过**（Phase 2 全部 Rust 改动 + PR #9 的 `src/main.rs` 新命令）—— 这次 CI 的真正价值就是把它们编译出来 |

## 目标（用户原话）

> 「A. GitHub Actions 云端构建吧...去做一做试试看」
> 「整理好，我们做合并，发release啥的」
> 「把这个windows版本安装包发我」

**即：让 push tag 就能自动产出 Windows 安装包并发 Release，最终给我一个能装的 exe。**

---

# 任务

## Task 1：GitHub Actions workflow（windows-latest）

写 `.github/workflows/release.yml`，要求：

### 触发
- `push: tags: ['v*']` 为主（发版用）
- `workflow_dispatch` 手动触发（调试用，可指定 ref）
- **PR 上不要跑完整打包**（太慢），但要有 `.github/workflows/ci.yml` 跑 `cargo check` + `cargo test` + 前端静态检查，让 PR 有红绿信号

### release.yml 步骤
1. checkout（`fetch-depth: 0` 或至少够 `git describe`）
2. 装 Rust stable（用 `dtolnay/rust-toolchain@stable`）
3. 装 Node LTS（Tauri CLI 需要，即使没前端构建）
4. 装 Tauri CLI（`tauri-action` 或手动 `npm i -g @tauri-apps/cli` 后 `tauri build`）
5. **`cargo check` / `cargo clippy` 先跑** —— 这是重点，**11 个文件从没编译过，我要看到真实编译错误**
6. `tauri build` 出 NSIS + MSI
7. 收 artifacts，`tauri-action` 或 `softprops/action-gh-release` 发 release
8. 失败要有清晰报错（不要把编译错误吞掉）

### 必须处理的坑（我预判的，你逐个验证）
1. **`service` crate 的 edition 2024** —— CI 的 stable toolchain 版本要够新，workflow 里别 pin 旧版
2. **workspace 结构**：根 `Cargo.toml` 是 workspace，members 含 `service`。`tauri build` 在根目录跑，确认能解析
3. **`tauri-plugin-*` 依赖**：`tauri-plugin-shell`/`dialog`/`fs`/`single-instance` 都在 `Cargo.toml` 里，CI 装依赖会拉很多 crate，构建时间可能 10-20 分钟，设置合理 `timeout-minutes`
4. **NSIS 中文语言包**：`displayLanguageSelector: true` + `SimpChinese` —— Tauri 中文 NSIS 需要额外语言文件吗？确认是否需要 `nsis.language` 配置
5. **`winres = 0.1`** 在 `[build-dependencies]`（Phase 2 已收进 cfg 块？确认）—— Windows 资源嵌入
6. `beforeBuildCommand` 空 → 确认 Tauri 会不会报错（空命令有时会）
7. **没签名**：明确在 release notes 里写「未代码签名，SmartScreen 会提示」，并研究加签名的成本（Azure Trusted Signing / OV 证书）—— **本次不实现签名**，只要写清楚
8. `webviewInstallMode: skip` —— 意味着用户必须已装 WebView2。release notes 要提醒

## Task 2：版本管理与 changelog

1. **bump 版本**：1.2.1 → 下一个合理版本。Phase 2 是大改（合并轮盘、删除本地降级、新 UI、64 项修复），建议 **2.0.0**（breaking），你定并在交付里论证
2. **同步版本号**：`tauri.conf.json` 的 `version`（这是 Tauri 唯一权威源，检查是否还有别处硬编码版本）
3. 写 `CHANGELOG.md`：把 Phase 1 研究结论、Phase 2 全部 8 个 Task、64 条 review 修复、PR #9 的 quickCreate 修复，浓缩成**用户能看懂的**条目（中文），不要堆 commit hash
4. 如果 CI 里 version 有单一来源问题（比如 Windows 资源版本号），处理好

## Task 3：建 tag + 触发构建 + 拿安装包

1. 在 master 上 bump 版本 + changelog，PR 合掉（**`--base master`，你自己合并或我来合，交付里说清**）
2. 打 tag `v<version>`，push
3. **盯 workflow 跑到出 artifact**（`gh run watch` 或轮询）
4. **把编译错误全数抓出来** —— 这是本次最重要的交付物。11 个从未编译的文件必然有错，逐个列出文件:行号 + 错误信息
5. 修到 workflow 绿（或明确报告"卡在某个必须 Windows 侧确认的问题"）
6. release 发出来后，**把 exe 下载到本机**并校验（大小、能否解出、`file` 判断是 PE）
7. 分发到 `https://box.haaaiawd.live/dl/promptkey/`（做法：宿主目录 `/home/haa/.local/caddy/data/dl/`，容器内 `/data`，对应 URL `/dl/promptkey/<file>`。**改 Caddyfile 后必须 `docker restart caddy` 才生效**，`caddy reload` 在容器内 exec 不生效 —— 已验证的坑）

## Task 4：README / 文档

1. README 更新到 Phase 2 后的真实功能（轮盘 A、合并、模板库、注入策略、快捷键）
2. 加「系统要求」：Windows 10/11 + WebView2（因为 `skip` 模式）
3. 加「从源码构建」章节（Rust 版本要求、命令）
4. 更新 `North_Star.md` / `TODO.md` 里已过时的状态（TODO 里还写着 UIA 主路径，实际已移除；market 未实现，实际已有模板库）

---

# 硬约束

- **CI 要真跑，不要只写 yaml 就交**。workflow 的真假以 `gh run list` 有真实运行为准
- **编译错误必须如实报告**，不要因为"应该能过"就跳过
- 修编译错误时：如果是 Phase 2 引入的真 bug，修；如果是 API 版本差异，对齐
- 一个功能一个 commit，中文或规范英文 commit message
- 版本 bump 单独 commit
- **不要 auto-merge 除版本 bump 外的 PR 而不报告**
- 本机不能编译，所以**本地改的 Rust 仍需 CI 兜底验证** —— 别声称"已修复编译问题"除非 CI 真的绿了

---

## 最终交付

```
## Workflow 文件
（release.yml + ci.yml 路径 + 关键设计说明）

## CI 首次运行结果
（run URL、状态、时长）

## 编译错误清单（最重要）
（文件:行号 + 错误 + 修复方式；未修的说明为什么）

## Release
（版本号、tag、资产列表+大小、release URL）

## 分发地址
（https://box.haaaiawd.live/dl/promptkey/... 的实际 200 验证）

## 已知限制
（未签名、WebView2 要求、macOS/Linux 未出包的原因）
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-ci-release
deliverables:
- .github/workflows/release.yml + ci.yml created and REAL runs executed
- full compile error list from first run, with fixes
- version bumped + CHANGELOG.md
- tag pushed, Release published with exe+msi artifacts
- installer downloaded, verified, distributed to box URL
status: success
errors: none
```
