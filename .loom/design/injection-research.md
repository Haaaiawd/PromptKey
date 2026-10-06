# 注入机制研究与三平台策略

- Kind: research
- Status: done (Phase 1 调研完成)

## 正文位置

**`.loom/design/INJECTION_RESEARCH.md`**（按 Phase 1 brief 规定的交付文件名）。

## 结论摘要

保留 Clipboard→SendInput 主链路；以"只读门禁"形式拿回 UIA/AX 能力做密码框拒绝（当前无任何实现，违反 North_Star 边界）；新增 WM_PASTE/EM_REPLACESEL 经典控件快速通道作可选头段；跨平台采用统一 `Injector` trait + 三实现（win32/macos/linux），Linux Wayland 走 uinput/portal 并诚实降级。详见正文。

## Related documents and capabilities

- 正文：`.loom/design/INJECTION_RESEARCH.md`
- Capability：`.loom/capabilities/injection-engine/capability.md`
- 关联：`.loom/design/cross-platform-plan.md`（跨平台落地路线）
