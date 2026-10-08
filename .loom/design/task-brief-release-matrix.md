# Devin Brief: Release 流水线扩到 Linux + macOS 三平台

## 项目
`/home/haa/sites/promptkey`，当前 `master` HEAD `5c4efc3`（含 2.0.5 bump + CHANGELOG）。新建分支 `feat/release-matrix-linux-macos`。

## 背景

用户要求（原话）：「可能要发这三版本哦」—— **Windows / Linux / macOS 三个平台的安装包都要从 Release 出**。

### 现状（我已核实）

- `.github/workflows/release.yml` **只有 `build-windows` 一个 job**（`runs-on: windows-latest`），产出 `tauri build` 的 NSIS `.exe` + `.msi`
- `.github/workflows/ci.yml` 的 `rust-unix` job 已用 `matrix.os: [ubuntu-latest, macos-latest]` 跑 `cargo check --all-targets`，**说明两个平台的代码能编译**（这是 2.0.5 跨平台 trait 重构后的既有资产）
- `tauri.conf.json:18` → `"targets": "all"`（全平台目标）
- 图标资产齐全：`icons/icon.icns`(345KB)、`icon.ico`、`128x128.png`、`128x128@2x.png`、`32x32.png`、`icon.png`
- 依赖已按平台隔离：`tauri-plugin-autostart`（三平台 autostart）、macOS 专属 `macos-private-api` + `image-png` feature
- **当前 `master` 版本号是 2.0.5，HEAD 已打 tag `v2.0.5`（Windows Release 已发布成功）**

### 硬约束：Windows 绝不能再炸

2.0.5 的 Windows Release 是刚发布的（MSI 4988928B + exe 3678036B，均成功）。**你在改 workflow 时，Windows job 的现有步骤要保留原样**（含 capabilities 校验、bundle 校验、NSIS/MSI 上传），只做**新增**，不要重构 Windows job。

---

# 任务

## Task 1: release.yml 增加 Linux + macOS job

新增两个 job，与 `build-windows` **并列**（不是串联，任一平台失败不影响其他平台发布）。

### Linux job

- `runs-on: ubuntu-latest`
- **Tauri 2 Linux 系统依赖必须装**（否则 `tauri build` 必炸）：
  ```bash
  sudo apt-get update
  sudo apt-get install -y libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf
  ```
  **具体包名你要自己核实**（这是最容易踩的坑：Tauri 2 需要 webkit2gtk **4.1**，不是 4.0），并且加一步 `pkg-config --exists webkit2gtk-4.1` 之类的**前置校验**，依赖缺失时立刻失败并给出清晰报错，而不是等到 bundle 阶段才炸
- Rust `stable`（照抄 Windows job 的工具链安装方式）
- 装 Node LTS + `@tauri-apps/cli@2.12.0`（与 Windows job 同版本）
- 编译门 `cargo check --workspace --all-targets`
- `tauri build --verbose`
- 产物：`target/release/bundle/deb/*.deb` + `target/release/bundle/appimage/*.AppImage`
- **校验**：两个产物至少一个存在，否则 throw
- 上传到 Release（`softprops/action-gh-release@v2`，**与 Windows 共用同一个 Release**，追加 asset 而非新建）
- **注意**：Linux 构建时 `custom-protocol` feature 的处理 —— `tauri build` release 会自动加 `--features custom-protocol`，确认这是对的

### macOS job

- `runs-on: macos-latest`
- Rust stable + Node LTS + tauri CLI 2.12.0
- **目标架构要论证**：`aarch64-apple-darwin` / `x86_64-apple-darwin` / universal？
  - Apple Silicon 是主流，但 Intel Mac 仍在用
  - **你给出推荐 + 理由**，并在交付里说明。如果做一个，说明另一个为何不做（或后续做）
- 编译门 `cargo check --workspace --all-targets`
- **macOS 打包前置**：
  - `.icns` 已在（`icons/icon.icns`）
  - 产物：`target/release/bundle/macos/*.app` → 打成 `.dmg`（`tauri build` 的 `dmg` target，或用 `create-dmg`）
  - **未签名未公证是预期**：workflow 不能尝试签名（无证书），但要在日志里明确输出「unsigned / not notarized」提示，并确认 `tauri.conf.json` 没有配 `macOS.signingIdentity`（有就报错打断，别让构建假装在签）
- 产物：`*.dmg`（如果同时产出 `.app.tar.gz` 也一起传）
- 上传到同一个 Release

## Task 2: Release body 要三平台通用

现在 release.yml 的 body 只写了 Windows 安装说明。改成三平台结构：

```markdown
## PromptKey vX.Y.Z

### 下载
| 平台 | 安装包 | 说明 |
|---|---|---|
| Windows | `PromptKey_*_x64-setup.exe` | NSIS，中英双语向导（推荐） |
| Windows | `PromptKey_*_x64_en-US.msi` | MSI 备选 |
| Linux | `PromptKey_*_amd64.deb` | Debian/Ubuntu |
| Linux | `PromptKey_*_amd64.AppImage` | 通用，无需安装 |
| macOS | `PromptKey_*_aarch64.dmg` | Apple Silicon（拖入 Applications） |

### 安装说明
（每平台 2-3 行**

### 系统要求
- Windows 10/11 x64 + WebView2 Runtime
- Linux: glibc ≥ 2.31（Debian 12+ / Ubuntu 22.04+）+ WebKit2GTK 4.1
- macOS 11+

### 注意
- 三个平台均**未做代码签名**（SmartScreen / Gatekeeper 提示属正常）
- macOS 首次运行需在「系统设置 → 隐私与安全性」中点「仍要打开」
- macOS 自动注入需在「系统设置 → 隐私与安全性 → 辅助功能」中授权
- Wayland 会话下自动注入受平台限制，应用内会明确提示降级
```

（表格内容以你实际产出的文件名为准，别照抄我的假设文件名）

## Task 3: 三平台产物的本地校验脚本

在 `scripts/` 下加一个校验脚本（或在 workflow 内 step 实现），对每个平台的产物做基本健全性检查：

- `.exe` → PE32 可执行
- `.deb` → `ar t` 能列出 `debian-binary`/`control.tar.*`/`data.tar.*`
- `.AppImage` → 有 `AI\x02` 魔数（type-2 AppImage）且是可执行文件
- `.dmg` → 有 `koly` trailer（Apple Disk Image）
- 每种格式的**最小体积阈值**（防空文件/截断上传），阈值你定并说明依据

## Task 4: 文档

- `docs/PLATFORMS.md`：补「打包产物」一栏（每平台的产物文件名 + 安装方式），并把 CI 三平台构建的状态写进去
- README：下载区说明三个平台各自的安装包怎么选

## Task 5（重要）: 你不能自己跑 Release，所以必须给「待执行清单」

本机无 cargo、无法跑 workflow。你的交付必须包含一个**明确的执行清单**，让我（或用户）在合并后照做：

```
合并 → 打 tag → workflow 跑 → 逐平台检查 asset 是否齐全 → 下载每个产物 → box 分发 → 告知用户
```

并列出**每平台可能失败的点**和对应的排查命令。

---

# 硬约束

- 无 cargo，本机不编译；**CI 必须全绿**（含新增的两个 job）
- **不得破坏 2.0.5 已发布的 Windows Release**；Windows job 只增不改
- 三平台 job 并行，互不阻塞
- 产物都要上传到**同一个** Release（软props action 的 `files` 支持多 job 追加）
- 一个 PR，多个 commit 按 job 分（linux / macos / docs / verify），push，`--base master`，**不合并**
- 遵守 LOOM 流程

---

## 最终交付

```
## Linux job
（系统依赖包清单 + 你怎么核实的 + 前置校验；产物与校验）

## macOS job
（**目标架构的推荐 + 论证**；未签名处理；产物与校验）

## Release body
（三平台表格 + 注意项）

## 校验脚本
（每种格式怎么验 + 体积阈值依据）

## 文档
（PLATFORMS.md / README 改了什么）

## 待执行清单（合并后）
（从合并到分发的逐步操作 + 每平台失败点排查）

## 风险
（哪些是你无法在本机验证的；Linux 构建依赖在不同 runner 镜像上可能的差异；macOS 架构选择的风险）
```

```json
COMPLETION_NOTIFY
source: assistant
task: promptkey-release-matrix-3platform
deliverables:
- release.yml extended with linux + macos jobs, parallel to windows, windows job untouched
- linux system deps correct & verified with early pkg-config gate
- macos target arch recommended with reasoning; unsigned/not-notarized explicit; no fake signing
- three-platform release body with download matrix + per-platform notes
- artifact sanity verification script (PE/deb/AppImage magic/dmg koly + size floors)
- docs/PLATFORMS.md + README updated for 3-platform artifacts
- CI green including both new jobs
- explicit post-merge execution checklist (tag → verify → distribute)
- branch feat/release-matrix-linux-macos + PR against master (not merged)
status: success
errors: none
```
