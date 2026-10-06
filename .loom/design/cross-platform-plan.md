# 跨平台工程化计划

- Kind: research
- Status: done (Phase 1 调研完成)

## 正文位置

**`.loom/design/CROSS_PLATFORM_PLAN.md`**

## 结论摘要

四阶段：P1 抽三 trait（Injector/Context/Hotkey）+ Windows 加固 → P2 Linux-X11 → P3 macOS（AX 权限引导+公证 DMG）→ P4 Wayland 尽力降级。硬限制已如实列出：Wayland 无免权限注入/全局热键、macOS 必须手动授权 AX、需 Apple 开发者账号公证。service 维持内嵌线程架构，不做独立守护进程。

## Related documents and capabilities

- 正文：`.loom/design/CROSS_PLATFORM_PLAN.md`
- Capability：`.loom/capabilities/injection-engine/capability.md`
- 关联：INJECTION_RESEARCH.md §5（三平台注入细节）
