use std::result::Result as StdResult;
use windows::{
    core::*,
    Win32::UI::Accessibility::*,
    Win32::System::Com::*,
};

fn main() -> StdResult<(), Box<dyn std::error::Error>> {
    // 初始化COM
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }

    // 获取UI自动化对象
    let automation: IUIAutomation = unsafe {
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?
    };

    // 获取焦点元素
    let focused_element = unsafe {
        automation.GetFocusedElement()?
    };

    // 获取元素名称
    let name = unsafe {
        focused_element.CurrentName()?
    };

    // 检查是否为密码框
    let is_password = unsafe {
        focused_element.CurrentIsPassword()?
    };

    println!("Focused element name: {}", name);
    println!("Is password field: {}", is_password.as_bool());

    // 尝试获取ValuePattern
    let value_pattern_result = unsafe {
        focused_element.GetCurrentPattern(UIA_ValuePatternId)
    };

    match value_pattern_result {
        Ok(pattern) => {
            println!("ValuePattern found");
            let value_pattern: IUIAutomationValuePattern = pattern.cast()?;
            
            // 获取当前值
            let current_value = unsafe {
                value_pattern.CurrentValue()?
            };
            println!("Current value: {}", current_value);
        }
        Err(_) => {
            println!("ValuePattern not available");
        }
    }

    // 尝试获取TextPattern
    let text_pattern_result = unsafe {
        focused_element.GetCurrentPattern(UIA_TextPatternId)
    };

    match text_pattern_result {
        Ok(pattern) => {
            println!("TextPattern found");
            let text_pattern: IUIAutomationTextPattern = pattern.cast()?;
            
            // 获取选择范围
            let selection = unsafe {
                text_pattern.GetSelection()?
            };
            
            let length = unsafe {
                selection.Length()?
            };
            
            println!("Selection length: {}", length);
        }
        Err(_) => {
            println!("TextPattern not available");
        }
    }

    Ok(())
}