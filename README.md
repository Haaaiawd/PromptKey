# Prompt Manager

Prompt Manager 是一个面向 AI 重度用户的系统级提示词管理器，支持全局快捷键、按应用上下文感知和专业模板管理，让用户在任何软件中一键调用高质量 Prompt。

## 功能特点

- 全局热键唤起与注入
- 智能注入策略（UIA → 剪贴板 → SendInput）
- 模板/提示词管理
- 按应用上下文路由
- 本地 SQLite 存储

## 安装与运行

```bash
# 克隆项目
git clone <repository-url>
cd PromptManager

# 构建项目
cargo build
```

## 使用方法

### 启动应用

```bash
# 启动系统托盘应用
cargo run
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

配置文件位于: `%APPDATA%/PromptManager/config.yaml`

默认配置:
```yaml
hotkey: "Ctrl+Alt+Space"
database_path: "C:\\Users\\<you>\\AppData\\Roaming\\PromptManager\\promptmgr.db"
injection:
  order: ["uia", "clipboard", "sendinput"]
  allow_clipboard: true
```

## 开发

本项目使用 Rust 编写，主要依赖:
- windows: Windows API 绑定
- tokio: 异步运行时
- rusqlite: SQLite 数据库
- serde: 序列化/反序列化

运行开发版本:
```bash
cargo run
```

构建发布版本:
```bash
cargo build --release
```