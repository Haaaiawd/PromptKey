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

**[下载最新版本](https://github.com/Haaaiawd/PromptKey/releases/latest)** | **[查看更新日志](#更新日志)**

</div>

---

PromptKey 是一个专为 AI 重度用户设计的系统级提示词管理器，支持全局快捷键和 **提示词轮盘** 快速调用，让用户在任何软件中一键注入高质量 Prompt。

## ✨ 功能特点

### 核心功能
- **🎡 提示词轮盘** - 热键呼出 6 槽位快速选择轮盘，iOS 风格玻璃拟态设计
- **📌 轮盘配置面板** - 可视化管理哪些提示词显示在轮盘上
- **⌨️ 全局热键** - 可自定义热键，随时随地快速调用
- **📋 一键复制** - 卡片上直接复制提示词内容

### 界面特性
- **🎨 现代 UI** - shadcn/ui 风格的黑白简约设计
- **📊 卡片/列表视图** - iOS 风格分段控制器切换
- **🏷️ 标签管理** - 按标签分类组织提示词
- **📝 使用日志** - 追踪每次注入的详细记录

### 技术亮点
- **💾 本地存储** - SQLite 安全存储，数据完全掌控
- **🔧 智能注入** - 剪贴板 + 模拟输入多重策略
- **🪟 单实例运行** - 避免重复启动，智能窗口管理

## 🚀 快速开始

### 安装

1. 从 [Releases](https://github.com/Haaaiawd/PromptKey/releases/latest) 下载最新安装包
2. 运行 `PromptKey_x.x.x_x64-setup.exe`
3. 按照安装向导完成安装

### 使用

1. **启动应用** - 从开始菜单或桌面快捷方式启动
2. **添加提示词** - 在"提示词"页面点击"添加提示词"
3. **配置轮盘** - 在"轮盘"页面勾选要置顶的提示词
4. **使用热键** - 按 `Ctrl+Alt+Space` (默认) 呼出轮盘
5. **快速注入** - 点击轮盘扇区或按数字键 1-6

### 默认热键

| 热键 | 功能 |
|------|------|
| `Ctrl+Alt+Space` | 呼出提示词轮盘 |
| `1-6` | 选择轮盘对应位置 |
| `Esc` | 关闭轮盘 |
| `PageUp/Down` | 轮盘翻页 |

## 🎡 轮盘排序规则

轮盘显示的提示词按以下优先级排序：

1. **📌 置顶优先** - 在轮盘配置面板中勾选的提示词
2. **🕐 最近使用** - 最近使用过的提示词靠前
3. **📈 使用频率** - 使用次数多的提示词靠前

## ⚙️ 配置

### 配置文件位置
```
%APPDATA%/PromptKey/config.yaml
```

### 主要配置项

| 参数 | 默认值 | 描述 |
|------|--------|------|
| `hotkey` | `Ctrl+Alt+Space` | 轮盘呼出热键 |
| `database_path` | `%APPDATA%/PromptKey/promptmgr.db` | 数据库路径 |

## 🛠️ 开发

### 环境要求

- Rust 1.70+
- Node.js 18+ (仅用于 Tauri CLI)
- Windows 10/11

### 构建

```bash
# 克隆项目
git clone https://github.com/Haaaiawd/PromptKey.git
cd PromptKey

# 开发模式运行
cargo run --release

# 构建安装包
cargo tauri build
```

### 项目结构

```
PromptKey/
├── src/                      # GUI 应用源码
│   ├── main.rs               # Tauri 主进程
│   ├── index.html            # 主界面
│   ├── wheel.html            # 轮盘界面
│   ├── wheel.css             # 轮盘样式 (Jelly Glass)
│   ├── wheel.js              # 轮盘逻辑
│   ├── styles.css            # 主界面样式
│   └── main_simple.js        # 主界面逻辑
├── service/                  # 内嵌服务模块
│   └── src/
│       ├── hotkey/           # 热键监听
│       ├── injector/         # 文本注入
│       └── ipc/              # 进程通信
└── blueprint/                # 设计文档
```

## 📋 更新日志

### v1.2.0 (2025-01-07)
- ✨ 新增轮盘配置面板
- ✨ 提示词置顶功能
- 🎨 iOS 风格分段控制器
- 🎨 卡片一键复制按钮
- 🔧 移除旧版热键逻辑
- 🐛 修复编译警告
- 🐛 隐藏控制台窗口

### v1.1.0
- ✨ 提示词轮盘 (Prompt Wheel)
- ✨ iOS Jelly Glass 玻璃拟态设计
- ✨ 键盘快捷键支持 (1-6, PageUp/Down)

### v1.0.0
- 🎉 首次发布
- ✨ 基础提示词管理
- ✨ 全局热键注入

---

<div align="center">

### 🙏 感谢使用 PromptKey

如果这个项目对你有帮助，请考虑给个 ⭐ Star！

**让 AI 提示词管理变得更简单** 💪

[报告问题](https://github.com/Haaaiawd/PromptKey/issues) · [功能建议](https://github.com/Haaaiawd/PromptKey/issues)

</div>