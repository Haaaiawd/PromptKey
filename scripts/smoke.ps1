param(
    [string]$Hotkey = "Ctrl+Alt+Space",
    [int]$RunSeconds = 3
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
Set-Location $root

# 1) Build
Write-Host "[smoke] Building..." -ForegroundColor Cyan
cargo build | Out-Host

# 2) Start service
$env:RUST_LOG = "info"
$logs = Join-Path $root "logs"
if (!(Test-Path $logs)) { New-Item -ItemType Directory -Path $logs | Out-Null }
$svcOut = Join-Path $logs "service.smoke.stdout.log"
$svcErr = Join-Path $logs "service.smoke.stderr.log"
if (Test-Path $svcOut) { Remove-Item $svcOut -Force }
if (Test-Path $svcErr) { Remove-Item $svcErr -Force }

Write-Host "[smoke] Starting service..." -ForegroundColor Cyan
$svc = Start-Process -FilePath (Join-Path $root "target/debug/service.exe") -RedirectStandardOutput $svcOut -RedirectStandardError $svcErr -PassThru
Start-Sleep -Seconds 1.5

# 3) Launch Notepad
Write-Host "[smoke] Launching notepad..." -ForegroundColor Cyan
$np = Start-Process -FilePath "notepad.exe" -PassThru
Start-Sleep -Seconds 1

# 4) Bring Notepad to foreground & send hotkey
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Native {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
}
"@

# Try bring to front
try {
  [Native]::ShowWindow((Get-Process -Id $np.Id).MainWindowHandle, 5) | Out-Null # SW_SHOW
  Start-Sleep -Milliseconds 200
  [Native]::SetForegroundWindow((Get-Process -Id $np.Id).MainWindowHandle) | Out-Null
  Start-Sleep -Milliseconds 200
} catch {}

# Send with WScript.Shell to avoid low-level keybd_event
$shell = New-Object -ComObject WScript.Shell
$shell.AppActivate($np.Id) | Out-Null
Start-Sleep -Milliseconds 200

Write-Host "[smoke] Sending hotkey: $Hotkey" -ForegroundColor Cyan
# Build SendKeys string
function Convert-ToSendKeys($hk) {
    $map = @{ 'Ctrl'='^'; 'Alt'='%'; 'Shift'='+' }
    $parts = $hk -split '\+' | ForEach-Object { $_.Trim() }
    $mods = ($parts | Where-Object { $map.ContainsKey($_) } | ForEach-Object { $map[$_] }) -join ''
  $key  = @($parts | Where-Object { -not $map.ContainsKey($_) })[0]
    switch ($key.ToLower()) {
        'space' { return "${mods} " }
        default { return "${mods}$key" }
    }
}
$send = Convert-ToSendKeys $Hotkey
$shell.SendKeys($send)

Start-Sleep -Seconds $RunSeconds

# 5) Dump logs
Write-Host "[smoke] ---- service stderr ----" -ForegroundColor Yellow
if (Test-Path $svcErr) { Get-Content $svcErr }

# 6) Cleanup
Write-Host "[smoke] Cleaning up..." -ForegroundColor Cyan
if (Get-Process -Id $svc.Id -ErrorAction SilentlyContinue) { Stop-Process -Id $svc.Id -Force }
if (Get-Process -Id $np.Id -ErrorAction SilentlyContinue) { Stop-Process -Id $np.Id -Force }

Write-Host "[smoke] Done." -ForegroundColor Green
