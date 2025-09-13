use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("正在启动 PromptKey...");
    
    // 启动服务进程
    let service_handle = thread::spawn(|| {
        println!("正在启动后台服务...");
        let output = Command::new("cargo")
            .args(&["run", "-p", "service"])
            .output()
            .expect("无法启动服务进程");
            
        if !output.status.success() {
            eprintln!("服务启动失败: {}", String::from_utf8_lossy(&output.stderr));
        }
    });
    
    // 等待一段时间确保服务启动
    thread::sleep(Duration::from_secs(2));
    
    // 启动GUI进程
    let gui_handle = thread::spawn(|| {
        println!("正在启动GUI界面...");
        let output = Command::new("cargo")
            .args(&["run"])
            .output()
            .expect("无法启动GUI进程");
            
        if !output.status.success() {
            eprintln!("GUI启动失败: {}", String::from_utf8_lossy(&output.stderr));
        }
    });
    
    println!("PromptKey 已启动，后台服务和GUI界面正在运行中...");
    println!("提示：请在文本编辑器中按下 Ctrl+Alt+Space 测试功能");
    
    // 等待线程完成
    let _ = service_handle.join();
    let _ = gui_handle.join();
}