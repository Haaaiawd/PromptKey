<div align="center">

# PromptKey 🎯

**面向 AI 重度用户的系统级提示词管理器**

![PromptKey Logo](PromptKey_aiextract.png)

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/tauri-%2324C8DB.svg?style=for-the-badge&logo=tauri&logoColor=%23FFFFFF)](https://tauri.app/)
[![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)](https://www.microsoft.com/windows/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge)](LICENSE)

[![SQLite](https://img.shields.io/badge/sqlite-%2307405e.svg?style=for-the-badge&logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![HTML5](https://img.shields.io/badge/html5-%23E34F26.svg?style=for-the-badge&logo=html5&logoColor=white)](https://developer.mozilla.org/docs/Web/HTML)
[![CSS3](https://img.shields.io/badge/css3-%231572B6.svg?style=for-the-badge&logo=css3&logoColor=white)](https://developer.mozilla.org/docs/Web/CSS)
[![JavaScript](https://img.shields.io/badge/javascript-%23323330.svg?style=for-the-badge&logo=javascript&logoColor=%23F7DF1E)](https://developer.mozilla.org/docs/Web/JavaScript)

**[下载最新版本](https://github.com/Haaaiawd/PromptKey/releases/latest)** | **[更新日志](CHANGELOG.md)**

</div>

---

PromptKey 是一个专为 AI 重度用户设计的系统级提示词管理器：全局热键呼出**跟随光标的径向轮盘**，在任何软件中一键把高质量 Prompt 注入当前输入框。

## ✨ 功能特点

### 核心功能

- **🎡 光标轮盘** — 热键呼出跟随鼠标的 280px 径向轮盘，无边框透明窗口，多显示器下正确落位，失焦自动隐藏
- **📌 Pin 即轮盘** — 置顶是提示词的内联属性，轮盘就是置顶集合的实时投影，无需单独维护
- **⚡ 默认直注** — 第二组热键跳过轮盘，直接注入「默认提示词」（固定指定 / 最近使用，可选）
- **📚 模板库** — 内置中英精选包，支持 URL / 本地文件导入、导出分享包
- **🧩 模板变量** — `{{变量}}` 注入前表单填写；`{{clipboard}}` `{{date}}` `{{time}}` 自动填充
- **🌓 设计系统** — 明暗双主题 + 中英双语运行时切换

### 注入与安全

- **💉 双策略注入** — 剪贴板粘贴为主，SendInput 逐键模拟兜底（应对禁止粘贴的场景）
- **🔒 密码框门禁** — 焦点在密码/安全输入框时拒绝注入（ES_PASSWORD + UIA IsPassword 双重只读探测）
- **📋 剪贴板保护** — 注入前备份剪贴板**全部格式**，用后恢复；遇到无法备份的格式自动改用 SendInput，不污染用户数据
- **🌐 SSRF 防护** — URL 导入强制 https、逐跳校验重定向、拒绝内网/回环/云元数据地址
- **📊 真实日志** — 每次注入记录真实成败、所用策略与耗时

## 💻 系统要求

- Windows 10 / 11（x64）
- 已安装 **WebView2 Runtime**（Windows 11 与多数 Windows 10 自带；安装包不自动安装）
- 安装包未做代码签名，SmartScreen 提示「未知发布者」属正常，选择「仍要运行」

## 🚀 快速开始

### 安装

1. 从 [Releases](https://github.com/Haaaiawd/PromptKey/releases/latest) 下载 `PromptKey_x.x.x_x64-setup.exe`（NSIS，中英双语安装向导）或 `.msi`
2. 运行安装包，按向导完成安装

### 使用

1. **启动应用** — 开始菜单或桌面快捷方式
2. **添加提示词** — 「提示词」页点击新建；勾选 📌 置顶即进入轮盘
3. **呼出轮盘** — 默认 `Ctrl+Alt+Space`，轮盘出现在光标处
4. **选择注入** — 点击扇区或按数字键；含 `{{变量}}` 的模板会先弹出填写表单
5. **默认直注** — `Ctrl+Alt+A` 直接注入默认提示词（在设置里可改为固定指定某条）

### 默认热键

| 热键 | 功能 |
|------|------|
| `Ctrl+Alt+Space` | 呼出轮盘 |
| `Ctrl+Alt+A` | 直接注入默认提示词 |
| `1-9` | 轮盘内选择 |
| `Esc` | 关闭轮盘 |

## ⚙️ 配置

```
%APPDATA%/PromptKey/config.yaml   # 配置（热键、注入开关、数据库路径）
%APPDATA%/PromptKey/promptmgr.db  # SQLite 数据库
```

主要配置项：`hotkey`、`quick_hotkey`、`injection.allow_clipboard` / `restore_clipboard` / `secure_gate`、`database_path`。设置页可图形化修改，保存后服务平滑重启生效。

## 🛠️ 从源码构建

环境要求：

- **Rust stable ≥ 1.85**（`service/` 使用 edition 2024）
- **Node.js LTS**（仅用于安装 Tauri CLI；前端为纯 HTML/CSS/JS，无构建步骤）
- Windows 10/11

```bash
git clone https://github.com/Haaaiawd/PromptKey.git
cd PromptKey

npm install -g @tauri-apps/cli   # 或 cargo install tauri-cli --version "^2"

cargo run                        # 开发模式运行
tauri build                      # 产出 NSIS + MSI 安装包（target/release/bundle/）
```

CI/CD：PR 与 master push 运行 `cargo check`/`clippy`/`test` + 前端静态检查；推送 `v*` tag 自动构建并发布 GitHub Release（见 `.github/workflows/`）。

### 项目结构

```
PromptKey/
├── src/                      # Tauri GUI（Rust 主进程 + 纯前端）
│   ├── main.rs               # Tauri 命令、托盘、轮盘窗口、pack 导入导出
│   ├── ipc_listener.rs       # 命名管道监听（service → GUI）
│   ├── inject_pipe_client.rs # 注入请求客户端（GUI → service）
│   ├── index.html / js/      # 主界面（提示词/模板库/记录/设置 四页）
│   ├── wheel.html / wheel.css / js/wheel.js   # 轮盘
│   ├── packs/                # 内置模板包 + manifest
│   └── icons/                # lucide + 品牌图标
├── service/                  # 内嵌引擎（workspace 成员，edition 2024）
│   └── src/
│       ├── main.rs           # 引擎主循环、模板渲染、注入调度
│       ├── injector/         # Injector trait + Windows 实现（剪贴板/SendInput）
│       ├── context/          # Context trait + Windows 前台上下文
│       ├── hotkey/           # Hotkey trait + Windows 消息循环
│       ├── ipc/              # 命名管道服务端
│       ├── config/           # YAML 配置
│       └── db.rs             # SQLite
├── tauri.conf.json           # Tauri 配置（版本号权威来源）
└── .github/workflows/        # CI + Release
```

## 📋 更新日志

见 [CHANGELOG.md](CHANGELOG.md)。

---

<div align="center">

### 🙏 感谢使用 PromptKey

如果这个项目对你有帮助，请考虑给个 ⭐ Star！

**让 AI 提示词管理变得更简单** 💪

[报告问题](https://github.com/Haaaiawd/PromptKey/issues) · [功能建议](https://github.com/Haaaiawd/PromptKey/issues)

</div>
