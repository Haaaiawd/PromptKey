# TODO — PromptKey（Phase 2 后）

更新时间：2026-10-06

## 当前状态

- 构建：GitHub Actions CI（windows-latest）跑 `cargo check`/`clippy`/`test`；`v*` tag 触发 release 构建出 NSIS+MSI。
- 注入：**UIA 写入路径已移除**。主链路 = 剪贴板粘贴 → SendInput 兜底；UIA 仅用于密码框只读探测（IsPassword）。
- 模型：提示词与轮盘已合并 —— pin 是提示词属性，轮盘是 pinned 投影；默认直注热键走 `app_settings` 策略（fixed / last_used）。
- 模板库：内置包 + URL 导入（SSRF 防护）+ 文件导入 + 导出均已实现（原计划的 market 远端商店未做，本地优先替代）。
- 变量：`{{var}}` 表单收集 + `{{clipboard}}`/`{{date}}`/`{{time}}` 自动变量，单遍渲染。

## 待办

P0 — Windows 实机验证（CI 只能编译，不能验证行为）
- [ ] 安装包安装/卸载/升级（perMachine NSIS，中英向导）
- [ ] 热键呼出轮盘 → 光标跟随 → 点击注入，在记事本/VS Code/浏览器输入框实测
- [ ] 密码框门禁实测（浏览器登录框、Windows 凭据框）
- [ ] 剪贴板备份恢复实测（先复制图片再注入，确认剪贴板图片还在）
- [ ] WebView2 缺失场景的表现（`webviewInstallMode: skip` 下直接装会怎样）

P1 — 工程质量
- [ ] Rust 侧单元测试（模板渲染、配置加载、解析器）—— 当前测试覆盖≈0
- [ ] 注入成功率本地统计页（usage_logs 已有数据，缺可视化）
- [ ] 兼容性清单冒烟：Notepad/VS Code/Edge/Chrome/JetBrains/VS

P2 — 发布与体验
- [ ] 代码签名（Azure Trusted Signing 或 OV 证书；去掉 SmartScreen 拦截）
- [ ] WebView2 引导（webviewInstallMode 改为 embedBootstrapper/downloadBootstrapper，或用引导器）
- [ ] Tauri updater + 差量更新
- [ ] SendInput 高级优化（粘连键防护、长文本分批）

P3 — 跨平台（trait 已抽象，未实装）
- [ ] macOS：AXUIElement/CGEvent 注入 + AX 上下文 + Carbon 热键
- [ ] Linux-X11：XTEST + 剪贴板；Wayland：明确降级或 uinput（需权限）

## 已完成（Phase 2 摘要）

- [x] 轮盘 A 形态：光标跟随 280px 径向轮盘，多显示器正确落位
- [x] 提示词/轮盘合并：pin 内联属性 + frecency 排序 + 手动排序
- [x] 模板库：内置 4 包 + URL/文件导入 + 导出
- [x] 注入加固：密码门禁 + 全格式剪贴板备份恢复 + 真实成败日志
- [x] 前端加固 + 设计系统（双主题/中英切换/新图标）
- [x] Injector/Context/Hotkey trait 抽象（Windows 实现迁入 windows_impl.rs）
- [x] 74 项 review 发现全数处理
- [x] GitHub Actions CI + tag 触发 Release 流水线
