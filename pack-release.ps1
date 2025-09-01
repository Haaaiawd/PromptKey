# 构建并打包为可分发的 EXE 文件夹（GUI + Service）
param(
    [switch]$Zip
)

$ErrorActionPreference = 'Stop'

Write-Host "[1/5] 构建 Release 二进制..." -ForegroundColor Yellow
cargo build --release -p service
cargo build --release --bin prompt-manager

$dist = Join-Path -Path (Get-Location) -ChildPath 'dist/PromptManager'
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Write-Host "[2/5] 复制可执行文件..." -ForegroundColor Yellow
Copy-Item -Force "target/release/prompt-manager.exe" "$dist/prompt-manager.exe"
Copy-Item -Force "target/release/service.exe" "$dist/service.exe"

Write-Host "[3/5] 复制资源(可选)..." -ForegroundColor Yellow
if (Test-Path 'PM.ico') { Copy-Item -Force 'PM.ico' "$dist/PM.ico" }

Write-Host "[4/5] 验证启动..." -ForegroundColor Yellow
# 以独立进程试运行一次，确保托盘常驻并自动拉起 service
Start-Process -FilePath "$dist/prompt-manager.exe"
Start-Sleep -Seconds 2

Write-Host "已在 dist/PromptManager 生成可分发目录。" -ForegroundColor Green
Write-Host "关闭窗口后会最小化到托盘，右键托盘的“退出”将停止后台服务。" -ForegroundColor Green

if ($Zip) {
  Write-Host "[5/5] 生成 ZIP..." -ForegroundColor Yellow
  $zipPath = Join-Path (Get-Location) 'dist/PromptManager.zip'
  if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
  Compress-Archive -Path "$dist/*" -DestinationPath $zipPath
  Write-Host "ZIP 包已生成: $zipPath" -ForegroundColor Green
}
