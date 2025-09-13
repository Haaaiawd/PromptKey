# PromptKey

PromptKey 是一个面向 AI 重度用户的系统级提示词管理器，支持全局快捷键、按应用上下文感知和专业模板管理，让用户在任何软件中一键调用高质量 Prompt。

## 功能特点

- 全局热键唤起与注入
- 智能注入策略（UIA → 剪贴板 → SendInput）
- 模板/提示词管理
- 按应用上下文路由
- 本地 SQLite 存储

## 安装与构建

### 构建步骤

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

PromptKey 采用双进程架构：
- `promptkey.exe`: 主 GUI 应用，负责用户界面和系统托盘
- `service.exe`: 后台服务，负责全局热键监听和文本注入
- `launcher.exe`: 启动器，同时启动 GUI 和服务进程

## 使用方法

### 启动应用

```bash
# 方式1: 直接运行 GUI（推荐）
./target/release/promptkey.exe

# 方式2: 使用启动脚本
./start-prompt-manager.ps1

# 方式3: 开发模式
cargo run --bin promptkey
```

应用启动后会在系统托盘中显示图标，可以通过以下方式控制：

1. 双击托盘图标：显示/隐藏主窗口
2. 右键托盘图标：
   - 显示/隐藏：显示/隐藏主窗口
   - 退出：完全退出应用

## 快捷键使用

1. 启动服务后，在任何文本编辑器中将光标定位到目标输入区域
2. 按下默认热键 `Ctrl+Alt+Space`
3. 程序将使用UIA注入策略插入测试文本

## 配置

配置文件位于: `%APPDATA%/PromptKey/config.yaml`

默认配置:
```yaml
hotkey: "Ctrl+Alt+Space"
database_path: "C:\\Users\\<you>\\AppData\\Roaming\\PromptKey\\promptkey.db"
injection:
  order: ["uia", "clipboard", "sendinput"]
  allow_clipboard: true
  uia_value_pattern_mode: "overwrite"
```

### 配置说明

- `hotkey`: 全局热键组合，支持 Ctrl、Alt、Shift 修饰键
- `database_path`: SQLite 数据库文件路径
- `injection.order`: 文本注入策略优先级
- `injection.allow_clipboard`: 是否允许使用剪贴板注入
- `injection.uia_value_pattern_mode`: UIA 注入模式（overwrite/append）

## 开发

本项目使用 Rust 编写，基于 Tauri v2 框架，主要依赖:
- tauri: 跨平台桌面应用框架
- windows: Windows API 绑定
- tokio: 异步运行时
- rusqlite: SQLite 数据库
- serde: 序列化/反序列化

### 开发环境运行

```bash
# 运行 GUI 应用（开发模式）
cargo run --bin promptkey

# 单独运行后台服务（调试）
cargo run -p service

# 运行启动器
cargo run --bin launcher
```

### 构建说明

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
├── src/                    # GUI 应用源码
│   ├── main.rs            # 主 GUI 应用
│   ├── launcher.rs        # 启动器
│   ├── index.html         # 前端界面
│   ├── styles.css         # 界面样式
│   └── main_simple.js     # 前端逻辑
├── service/               # 后台服务源码
│   └── src/
│       ├── main.rs        # 服务主程序
│       ├── config/        # 配置管理
│       ├── hotkey/        # 热键处理
│       ├── injector/      # 文本注入
│       └── context/       # 应用上下文
├── target/                # 构建输出
│   ├── debug/            # 开发版本
│   └── release/          # 发布版本
└── start-prompt-manager.* # 启动脚本
```