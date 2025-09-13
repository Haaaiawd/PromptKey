@echo off
echo 正在启动 PromptKey...

echo 启动后台服务...
start "PromptKey Service" /min cmd /c "cargo run -p service || pause"

timeout /t 3 /nobreak >nul

echo 启动GUI界面...
start "PromptKey GUI" cmd /c "cargo run --bin promptkey || pause"

echo.
echo PromptKey 已启动！
echo.
echo 使用方法：
echo 1. 在任何文本编辑器中定位光标
echo 2. 按下 Ctrl+Alt+Space 触发文本注入
echo.
echo 按任意键关闭此窗口...
pause >nul