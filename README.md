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

</div>

---

PromptKey 是一个专为 AI 重度用户设计的系统级提示词管理器，支持全局快捷键和专业模板管理，让用户在任何软件中一键调用高质量 Prompt。

## 功能特点

- **全局热键唤起** - 随时随地快速调用提示词
- **智能注入策略** - UIA → 剪贴板 → SendInput 多重保障
- **模板管理** - 专业的提示词模板管理系统
- **上下文感知** - 按应用自动路由合适的提示词
- **本地存储** - 基于 SQLite 的安全本地数据存储
- **单实例运行** - 避免重复启动，智能窗口管理

## 快速开始

### 安装与构建

#### 构建步骤

```bash
# 克隆项目
git clone <repository-url>
cd PromptKey

# 清理之前的构建缓存（可选）
cargo clean

# 构建 GUI 应用
cargo build --release --bin promptkey

# 构建后台服务
cargo build --release -p service

# 验证构建结果
ls target/release/
# 应该看到: promptkey.exe, service.exe, launcher.exe
```

### 项目架构

PromptKey 采用现代化的双进程架构设计：

| 组件 | 功能描述 | 技术栈 |
|------|----------|--------|
| **promptkey.exe** | 主 GUI 应用，负责用户界面和系统托盘 | Tauri v2 + HTML/CSS/JS |
| **service.exe** | 后台服务，负责全局热键监听和文本注入 | Rust + Windows API |
| **launcher.exe** | 启动器，同时启动 GUI 和服务进程 | Rust |

## 使用指南

### 启动应用

```bash
# 方式1: 直接运行 GUI（推荐）
./target/release/promptkey.exe

# 方式2: 使用启动脚本
./start-prompt-manager.ps1

# 方式3: 开发模式
cargo run --bin promptkey
```

应用启动后会在系统托盘中显示图标，支持以下操作：

| 操作 | 功能 |
|------|------|
| **双击托盘图标** | 显示/隐藏主窗口 |
| **右键 → 显示/隐藏** | 切换主窗口可见性 |
| **右键 → 退出** | 完全退出应用 |

###  快捷键使用

1. 启动服务后，在任何文本编辑器中将光标定位到目标输入区域
2. 按下默认热键 `Ctrl+Alt+Space`
3. 程序将使用UIA注入策略插入测试文本

## 配置说明

### 配置文件位置
```
%APPDATA%/PromptKey/config.yaml
```

### 默认配置
```yaml
hotkey: "Ctrl+Alt+Space"
database_path: "C:\\Users\\<you>\\AppData\\Roaming\\PromptKey\\promptkey.db"
injection:
  order: ["uia", "clipboard", "sendinput"]
  allow_clipboard: true
  uia_value_pattern_mode: "overwrite"
```

### 配置参数说明

| 参数 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `hotkey` | String | `"Ctrl+Alt+Space"` | 全局热键组合，支持 Ctrl、Alt、Shift 修饰键 |
| `database_path` | String | `%APPDATA%/PromptKey/promptkey.db` | SQLite 数据库文件路径 |
| `injection.order` | Array | `["uia", "clipboard", "sendinput"]` | 文本注入策略优先级 |
| `injection.allow_clipboard` | Boolean | `true` | 是否允许使用剪贴板注入 |
| `injection.uia_value_pattern_mode` | String | `"overwrite"` | UIA 注入模式（overwrite/append） |

## 开发指南

### 技术栈

本项目基于现代 Rust 生态系统构建：

| 组件 | 技术 | 版本 | 用途 |
|------|------|------|------|
| **桌面框架** | Tauri | v2.x | 跨平台桌面应用框架 |
| **系统 API** | Windows crate | v0.52 | Windows API 绑定 |
| **异步运行时** | Tokio | v1.x | 异步处理和并发 |
| **数据库** | rusqlite | v0.32 | SQLite 数据库操作 |
| **序列化** | serde | v1.x | 数据序列化/反序列化 |
| **前端** | HTML/CSS/JS | - | 用户界面 |

### 开发环境运行

```bash
# 运行 GUI 应用（开发模式）
cargo run --bin promptkey

# 单独运行后台服务（调试）
cargo run -p service

# 运行启动器
cargo run --bin launcher
```

### 构建命令

```bash
# 构建开发版本
cargo build

# 构建发布版本（推荐）
cargo build --release

# 仅构建 GUI
cargo build --release --bin promptkey

# 仅构建服务
cargo build --release -p service

# 清理构建缓存
cargo clean
```

### 项目结构

```
PromptKey/
├── src/                     # GUI 应用源码
│   ├── main.rs                 # 主 GUI 应用 (Tauri)
│   ├── launcher.rs             # 启动器
│   ├── index.html              # 前端界面
│   ├── styles.css              # 界面样式
│   ├── main_simple.js          # 前端逻辑
│   └── icons/                  # 应用图标
├── service/                 # 后台服务源码
│   └── src/
│       ├── main.rs             # 服务主程序
│       ├── db.rs               # 数据库操作
│       ├── win.rs              # Windows特定功能
│       ├── config/             # 配置管理
│       ├── hotkey/             # 热键处理
│       ├── injector/           # 文本注入 (UIA/Clipboard/SendInput)
│       └── context/            # 应用上下文感知
├── target/                  # 构建输出
│   ├── debug/                  # 开发版本
│   └── release/                # 发布版本
├── sidecar/                 # Tauri sidecar 二进制
└── start-prompt-manager.*   # 启动脚本
```

---

<div align="center">

### 🙏 感谢使用 PromptKey

如果这个项目对你有帮助，请考虑给个 ⭐ Star！

**让 AI 提示词管理变得更简单** 💪

</div>