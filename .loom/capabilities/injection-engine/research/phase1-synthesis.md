# Phase 1 注入引擎研究汇总

综合来源：`.loom/design/INJECTION_RESEARCH.md`（缺口清单与修复方案）、`North_Star.md` 边界条款（不注入密码框/不越权高完整性）、`service/src/injector/mod.rs` 现状审计（Clipboard→SendInput 两级、只备份 CF_UNICODETEXT）、Phase0 实测（2930 字符 SendInput 264ms）、被删 UIA 代码考古（commit cc41dd0 含 IsPassword 判定）、平台机制调研（macOS CGEventKeyboardSetUnicodeString / Linux enigo x11rb / ydotool / Wayland virtual-keyboard-v1 在 Mutter 未实现的证据）。

专家立场：
- 安全字段门禁 fail-open：只在确认是密码/安全控件时拒绝注入；UIA 只读探测恢复为门禁而非注入路径。
- 剪贴板备份覆盖全部格式（EnumClipboardFormats），污染窗口最小化。
- 日志必须在注入完成后按真实结果写入；usage_count 只对成功计数。
- 平台差异收进 trait 边界，Windows 先行。
