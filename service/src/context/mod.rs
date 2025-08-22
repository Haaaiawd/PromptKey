use std::result::Result as StdResult;
use windows::{
    Win32::UI::WindowsAndMessaging::*,
    Win32::Foundation::*,
    Win32::System::Threading::*,
    Win32::System::ProcessStatus::*,
};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub process_name: String,
    pub window_title: String,
    #[allow(dead_code)]
    pub window_handle: HWND,
}

pub struct ContextManager;

impl ContextManager {
    pub fn new() -> Self {
        log::debug!("ContextManager created");
        ContextManager
    }
    
    pub fn get_foreground_context(&self) -> StdResult<AppContext, Box<dyn std::error::Error>> {
        log::debug!("Getting foreground context");
        unsafe {
            // 获取前台窗口句柄
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                log::warn!("No foreground window found");
                return Err("No foreground window found".into());
            }
            log::debug!("Foreground window handle: {:?}", hwnd.0);
            
            // 获取窗口标题
            let window_title = Self::get_window_title(hwnd)?;
            log::debug!("Window title: {}", window_title);
            
            // 获取进程ID
            let mut process_id = 0;
            let thread_id = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            log::debug!("Process ID: {}, Thread ID: {}", process_id, thread_id);
            
            if process_id == 0 {
                log::warn!("Invalid process ID");
                return Err("Invalid process ID".into());
            }
            
            // 获取进程名
            let process_name = Self::get_process_name(process_id)?;
            log::debug!("Process name: {}", process_name);
            
            Ok(AppContext {
                process_name,
                window_title,
                window_handle: hwnd,
            })
        }
    }
    
    fn get_window_title(hwnd: HWND) -> StdResult<String, Box<dyn std::error::Error>> {
    // 使用更大的缓冲区以避免因长度不足导致乱码或截断
    let mut buffer = [0u16; 1024];
        let len = unsafe {
            GetWindowTextW(hwnd, &mut buffer)
        };
        
        log::debug!("Window title length: {}", len);
        
        if len == 0 {
            let error = unsafe { GetLastError() };
            if error.0 != 0 {
                log::warn!("GetWindowTextW failed with error: {:?}", error);
            }
            return Ok(String::new());
        }
        
        let title = OsString::from_wide(&buffer[..len as usize])
            .to_string_lossy()
            .into_owned();
        
        log::debug!("Retrieved window title: {}", title);
        Ok(title)
    }
    
    fn get_process_name(process_id: u32) -> StdResult<String, Box<dyn std::error::Error>> {
        log::debug!("Getting process name for ID: {}", process_id);
        if process_id == 0 {
            log::warn!("Process ID is 0");
            return Ok(String::new());
        }
        
        unsafe {
            // 打开进程
            let process_handle = match OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                process_id,
            ) {
                Ok(handle) => {
                    log::debug!("Process handle opened successfully");
                    handle
                }
                Err(e) => {
                    log::error!("Failed to open process: {}", e);
                    return Err(e.into());
                }
            };
            
            // 获取进程映像文件名
            let mut buffer = [0u16; 260];
            
            let result = K32GetProcessImageFileNameW(
                process_handle,
                &mut buffer,
            );
            
            // 关闭进程句柄
            let _ = CloseHandle(process_handle);
            
            if result == 0 {
                let error = GetLastError();
                log::warn!("K32GetProcessImageFileNameW failed with error: {:?}", error);
                return Ok(String::new());
            }
            
            let path = OsString::from_wide(&buffer[..result as usize])
                .to_string_lossy()
                .into_owned();
            
            log::debug!("Process image file path: {}", path);
            
            // 提取文件名
            let process_name = std::path::Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    log::debug!("Using full path as process name");
                    path.clone()
                });
            
            log::debug!("Extracted process name: {}", process_name);
            Ok(process_name)
        }
    }
}