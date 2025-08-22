# Prompt Manager 启动脚本

Write-Host "正在启动 Prompt Manager..." -ForegroundColor Green

# 启动后台服务
Write-Host "预构建可执行文件..." -ForegroundColor Yellow
cargo build ; if ($LASTEXITCODE -ne 0) { Write-Host "构建失败" -ForegroundColor Red; exit 1 }

# 启动GUI界面（由 GUI 内部启动 service.exe）
Write-Host "启动GUI界面..." -ForegroundColor Yellow
Start-Process -NoNewWindow -FilePath "cargo" -ArgumentList "run", "--bin", "prompt-manager"

Write-Host "Prompt Manager 已启动！" -ForegroundColor Green
Write-Host ""
Write-Host "使用方法：" -ForegroundColor Cyan
Write-Host "1. 在任何文本编辑器中定位光标" -ForegroundColor Cyan
Write-Host "2. 按下 Ctrl+Alt+Space 触发文本注入" -ForegroundColor Cyan
Write-Host ""
Write-Host "按任意键关闭此窗口..." -ForegroundColor Gray
$host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")