// 简化版 main.js，专注于修复热键录制和保存功能

console.log('=== 简化版main.js开始加载 ===');

let debugCounter = 0;

// 调试信息更新函数
function updateDebugInfo(message) {
    debugCounter++;
    const timestamp = new Date().toLocaleTimeString();
    const debugMsg = `${timestamp}: ${message}`;
    console.log(debugMsg);
    
    const debugElement = document.getElementById('debug-content');
    if (debugElement) {
        const existingContent = debugElement.textContent || '';
        debugElement.textContent = existingContent + '\n' + debugMsg;
        debugElement.scrollTop = debugElement.scrollHeight;
    }
}

// 清除调试信息
function clearDebugInfo() {
    const debugElement = document.getElementById('debug-content');
    if (debugElement) {
        debugElement.textContent = '';
    }
    debugCounter = 0;
    updateDebugInfo('调试信息已清除');
}

// 复制调试日志到剪贴板
function copyDebugLogs() {
    const debugElement = document.getElementById('debug-content');
    if (debugElement) {
        const logs = debugElement.textContent;
        
        if (!logs || logs.trim() === '') {
            alert('没有日志可以复制');
            return;
        }
        
        // 使用现代的 Clipboard API
        if (navigator.clipboard && window.isSecureContext) {
            navigator.clipboard.writeText(logs).then(() => {
                updateDebugInfo('日志已复制到剪贴板');
                alert('日志已复制到剪贴板！');
            }).catch(err => {
                updateDebugInfo('复制失败: ' + err);
                fallbackCopyTextToClipboard(logs);
            });
        } else {
            // 降级方案
            fallbackCopyTextToClipboard(logs);
        }
    }
}

// 降级复制方案
function fallbackCopyTextToClipboard(text) {
    const textArea = document.createElement("textarea");
    textArea.value = text;
    textArea.style.top = "0";
    textArea.style.left = "0";
    textArea.style.position = "fixed";
    
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();
    
    try {
        const successful = document.execCommand('copy');
        if (successful) {
            updateDebugInfo('日志已复制到剪贴板（降级方案）');
            alert('日志已复制到剪贴板！');
        } else {
            throw new Error('execCommand copy failed');
        }
    } catch (err) {
        updateDebugInfo('复制失败: ' + err);
        alert('复制失败，请手动选择并复制日志内容');
    }
    
    document.body.removeChild(textArea);
}

// 将函数暴露到全局作用域
window.clearDebugInfo = clearDebugInfo;
window.copyDebugLogs = copyDebugLogs;

// 等待Tauri API加载 - 适配Tauri v2
async function waitForTauri(maxWait = 5000) {
    const start = Date.now();
    while (Date.now() - start < maxWait) {
        // Tauri v2中API结构发生了变化
        if (window.__TAURI__ && (window.__TAURI__.core || window.__TAURI__.invoke)) {
            updateDebugInfo('Tauri v2 API已加载！');
            return window.__TAURI__;
        }
        // 检查是否是Tauri v2的新API结构
        if (window.__TAURI_INVOKE__) {
            updateDebugInfo('检测到Tauri v2 invoke API');
            return { invoke: window.__TAURI_INVOKE__ };
        }
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    updateDebugInfo('Tauri API加载超时');
    return null;
}

// 安全调用Tauri命令 - 适配Tauri v2
async function safeInvoke(cmd, payload = {}) {
    try {
        updateDebugInfo(`准备调用Tauri命令: ${cmd}`);
        
        // 首先检查是否有直接的invoke函数 (Tauri v2)
        if (window.__TAURI_INVOKE__) {
            updateDebugInfo(`使用Tauri v2 直接invoke调用: ${cmd}`);
            const result = await window.__TAURI_INVOKE__(cmd, payload);
            updateDebugInfo(`Tauri命令 ${cmd} 执行成功: ${result}`);
            return result;
        }
        
        const tauri = await waitForTauri();
        if (!tauri) {
            throw new Error('Tauri API 不可用');
        }
        
        // Tauri v1风格的调用
        if (tauri.core && tauri.core.invoke) {
            updateDebugInfo(`使用Tauri v1 core.invoke调用: ${cmd}`);
            const result = await tauri.core.invoke(cmd, payload);
            updateDebugInfo(`Tauri命令 ${cmd} 执行成功: ${result}`);
            return result;
        }
        
        // Tauri v2风格的调用
        if (tauri.invoke) {
            updateDebugInfo(`使用Tauri v2 invoke调用: ${cmd}`);
            const result = await tauri.invoke(cmd, payload);
            updateDebugInfo(`Tauri命令 ${cmd} 执行成功: ${result}`);
            return result;
        }
        
        throw new Error('未找到可用的invoke方法');
        
    } catch (error) {
        updateDebugInfo(`Tauri命令 ${cmd} 执行失败: ${error}`);
        throw error;
    }
}

// 主初始化函数
async function initializeApp() {
    updateDebugInfo('=== 开始初始化应用 ===');
    
    // 检查Tauri API状态
    updateDebugInfo('检查Tauri API状态...');
    updateDebugInfo('window对象存在: ' + (typeof window !== 'undefined'));
    updateDebugInfo('window.__TAURI__存在: ' + !!(window.__TAURI__));
    updateDebugInfo('window.__TAURI_INVOKE__存在: ' + !!(window.__TAURI_INVOKE__));
    
    if (window.__TAURI__) {
        updateDebugInfo('__TAURI__对象详情: ' + Object.keys(window.__TAURI__).join(', '));
        if (window.__TAURI__.core) {
            updateDebugInfo('core对象存在: true');
            updateDebugInfo('invoke方法存在: ' + !!(window.__TAURI__.core.invoke));
        } else {
            updateDebugInfo('core对象存在: false');
        }
        if (window.__TAURI__.invoke) {
            updateDebugInfo('直接invoke方法存在: true');
        }
    }
    
    // 等待并验证Tauri API
    const tauri = await waitForTauri();
    updateDebugInfo('Tauri API状态: ' + (tauri ? '可用' : '不可用'));
    
    // 查找页面元素
    const elements = {
        saveSettingsBtn: document.getElementById('save-settings-btn'),
        resetSettingsBtn: document.getElementById('reset-settings-btn'),
        hotkeyRecordBtn: document.getElementById('hotkey-record-btn'),
        hotkeyInput: document.getElementById('hotkey-input')
    };
    
    updateDebugInfo(`找到元素: save=${!!elements.saveSettingsBtn}, reset=${!!elements.resetSettingsBtn}, record=${!!elements.hotkeyRecordBtn}, input=${!!elements.hotkeyInput}`);
    
    // 绑定保存设置按钮
    if (elements.saveSettingsBtn) {
        elements.saveSettingsBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            updateDebugInfo('保存设置按钮被点击');
            
            try {
                // 获取热键值
                const hotkeyInput = document.getElementById('hotkey-input');
                let currentHotkey = '';
                
                if (hotkeyInput && hotkeyInput.value && hotkeyInput.value !== '请按下快捷键...') {
                    currentHotkey = hotkeyInput.value;
                    updateDebugInfo(`从输入框获取热键: ${currentHotkey}`);
                } else {
                    currentHotkey = localStorage.getItem('recordedHotkey') || '';
                    updateDebugInfo(`从localStorage获取热键: ${currentHotkey}`);
                }
                
                if (!currentHotkey) {
                    alert('请先录制一个热键！');
                    updateDebugInfo('ERROR: 未设置热键，无法保存');
                    return;
                }
                
                updateDebugInfo(`正在保存热键: ${currentHotkey}`);
                
                // 调用Tauri后端保存设置
                const result = await safeInvoke('apply_settings', {
                    hotkey: currentHotkey,
                    uiaMode: null
                });
                
                updateDebugInfo(`保存设置成功: ${result}`);
                alert(`热键已保存并生效: ${currentHotkey}`);
                
            } catch (error) {
                updateDebugInfo(`保存设置失败: ${error}`);
                alert(`保存失败: ${error}`);
            }
        });
        updateDebugInfo('已绑定保存设置按钮');
    } else {
        updateDebugInfo('ERROR: 未找到save-settings-btn');
    }
    
    // 绑定重置设置按钮
    if (elements.resetSettingsBtn) {
        elements.resetSettingsBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            updateDebugInfo('重置设置按钮被点击');
            
            try {
                // 调用Tauri后端重置设置
                const result = await safeInvoke('reset_settings');
                
                // 清除前端数据
                localStorage.removeItem('recordedHotkey');
                localStorage.removeItem('promptManagerSettings');
                
                const hotkeyInput = document.getElementById('hotkey-input');
                if (hotkeyInput) {
                    hotkeyInput.value = '';
                }
                
                updateDebugInfo(`重置设置成功: ${result}`);
                alert('设置已重置为默认值 (Ctrl+Alt+Space)');
                
            } catch (error) {
                updateDebugInfo(`重置设置失败: ${error}`);
                alert(`重置失败: ${error}`);
            }
        });
        updateDebugInfo('已绑定重置设置按钮');
    } else {
        updateDebugInfo('ERROR: 未找到reset-settings-btn');
    }
    
    // 绑定热键录制按钮
    if (elements.hotkeyRecordBtn) {
        elements.hotkeyRecordBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            updateDebugInfo('热键录制按钮被点击');
            
            const hotkeyInput = document.getElementById('hotkey-input');
            if (!hotkeyInput) {
                updateDebugInfo('ERROR: 未找到hotkey-input');
                return;
            }
            
            const originalValue = hotkeyInput.value;
            hotkeyInput.value = '请按下快捷键...';
            updateDebugInfo('开始录制热键，5秒内按下你想要的快捷键组合');
            
            let isRecording = true;
            const pressedKeys = new Set();
            
            const handleKeyDown = (event) => {
                if (!isRecording) return;
                
                event.preventDefault();
                event.stopPropagation();
                
                pressedKeys.add(event.code);
                updateDebugInfo(`按键按下: ${event.code}`);
                
                const modifiers = [];
                let mainKey = '';
                
                // 收集修饰键
                if (pressedKeys.has('ControlLeft') || pressedKeys.has('ControlRight')) {
                    modifiers.push('Ctrl');
                }
                if (pressedKeys.has('AltLeft') || pressedKeys.has('AltRight')) {
                    modifiers.push('Alt');
                }
                if (pressedKeys.has('ShiftLeft') || pressedKeys.has('ShiftRight')) {
                    modifiers.push('Shift');
                }
                
                // 查找主键
                for (const code of pressedKeys) {
                    if (!code.includes('Control') && !code.includes('Alt') && !code.includes('Shift')) {
                        if (code.startsWith('Key')) {
                            mainKey = code.slice(3); // KeyA -> A
                        } else if (code.startsWith('Digit')) {
                            mainKey = code.slice(5); // Digit1 -> 1
                        } else if (code === 'Space') {
                            mainKey = 'Space';
                        }
                        break;
                    }
                }
                
                updateDebugInfo(`修饰键: ${modifiers.join(',')}, 主键: ${mainKey}`);
                
                // 如果有修饰键和主键，完成录制
                if (modifiers.length > 0 && mainKey) {
                    const recordedHotkey = modifiers.join('+') + '+' + mainKey;
                    
                    if ((modifiers.includes('Ctrl') || modifiers.includes('Alt')) && mainKey) {
                        hotkeyInput.value = recordedHotkey;
                        localStorage.setItem('recordedHotkey', recordedHotkey);
                        updateDebugInfo('录制成功: ' + recordedHotkey + '，已保存到localStorage');
                        stopRecording();
                    }
                } else if (modifiers.length > 0) {
                    hotkeyInput.value = modifiers.join('+') + '+';
                }
            };
            
            const stopRecording = () => {
                isRecording = false;
                document.removeEventListener('keydown', handleKeyDown, true);
                clearTimeout(timeoutId);
                updateDebugInfo('热键录制结束');
            };
            
            const timeoutId = setTimeout(() => {
                if (isRecording) {
                    stopRecording();
                    if (hotkeyInput.value === '请按下快捷键...' || hotkeyInput.value.endsWith('+')) {
                        hotkeyInput.value = originalValue;
                        updateDebugInfo('录制超时，恢复原值');
                    }
                }
            }, 5000);
            
            document.addEventListener('keydown', handleKeyDown, true);
        });
        updateDebugInfo('已绑定热键录制按钮');
    } else {
        updateDebugInfo('ERROR: 未找到hotkey-record-btn');
    }
    
    // 加载当前设置
    try {
        updateDebugInfo('尝试从后端加载当前设置...');
        const settings = await safeInvoke('get_settings');
        updateDebugInfo('后端设置: ' + JSON.stringify(settings));
        
        const hotkeyInput = document.getElementById('hotkey-input');
        if (hotkeyInput && settings.hotkey) {
            hotkeyInput.value = settings.hotkey;
            localStorage.setItem('recordedHotkey', settings.hotkey);
            updateDebugInfo(`已加载热键设置: ${settings.hotkey}`);
        }
    } catch (error) {
        updateDebugInfo('从后端加载设置失败: ' + error);
        
        // 尝试从localStorage恢复
        const savedHotkey = localStorage.getItem('recordedHotkey');
        if (savedHotkey) {
            const hotkeyInput = document.getElementById('hotkey-input');
            if (hotkeyInput) {
                hotkeyInput.value = savedHotkey;
                updateDebugInfo(`从localStorage恢复热键: ${savedHotkey}`);
            }
        }
    }
    
    // 绑定导航按钮
    bindNavigationButtons();
    
    // 绑定其他功能按钮
    bindFunctionButtons();
    
    // 初始加载提示词列表
    loadPrompts();
    
    updateDebugInfo('=== 应用初始化完成 ===');
}

// 绑定导航按钮
function bindNavigationButtons() {
    const navButtons = [
        { id: 'prompts-btn', panel: 'prompts-panel' },
        { id: 'settings-btn', panel: 'settings-panel' },
        { id: 'market-btn', panel: 'market-panel' },
        { id: 'logs-btn', panel: 'logs-panel' }
    ];
    
    navButtons.forEach(({ id, panel }) => {
        const button = document.getElementById(id);
        if (button) {
            button.addEventListener('click', (e) => {
                e.preventDefault();
                e.stopPropagation();
                updateDebugInfo(`导航按钮被点击: ${id}`);
                
                // 移除所有导航按钮的active类
                document.querySelectorAll('.nav-btn').forEach(btn => btn.classList.remove('active'));
                // 添加当前按钮的active类
                button.classList.add('active');
                
                // 隐藏所有面板
                document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
                // 显示目标面板
                const targetPanel = document.getElementById(panel);
                if (targetPanel) {
                    targetPanel.classList.add('active');
                    updateDebugInfo(`已切换到面板: ${panel}`);
                } else {
                    updateDebugInfo(`ERROR: 未找到面板: ${panel}`);
                }
            });
            updateDebugInfo(`已绑定导航按钮: ${id}`);
        } else {
            updateDebugInfo(`ERROR: 未找到导航按钮: ${id}`);
        }
    });
}

// 绑定功能按钮
function bindFunctionButtons() {
    // 添加提示词按钮
    const addPromptBtn = document.getElementById('add-prompt-btn');
    if (addPromptBtn) {
        addPromptBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            updateDebugInfo('添加提示词按钮被点击');
            
            // 创建模态框
            showAddPromptModal();
        });
        updateDebugInfo('已绑定添加提示词按钮');
    } else {
        updateDebugInfo('ERROR: 未找到add-prompt-btn');
    }
    
    // 刷新市场按钮
    const refreshMarketBtn = document.getElementById('refresh-market-btn');
    if (refreshMarketBtn) {
        refreshMarketBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            updateDebugInfo('刷新市场按钮被点击');
            alert('市场功能暂未实现');
        });
        updateDebugInfo('已绑定刷新市场按钮');
    }
    
    // 清空日志按钮
    const clearLogsBtn = document.getElementById('clear-logs-btn');
    if (clearLogsBtn) {
        clearLogsBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            updateDebugInfo('清空日志按钮被点击');
            if (confirm('确定要清空所有使用日志吗？')) {
                alert('日志清空功能暂未实现');
            }
        });
        updateDebugInfo('已绑定清空日志按钮');
    }
}

// 显示添加提示词模态框
function showAddPromptModal() {
    // 创建模态框HTML
    const modalHtml = `
        <div id="add-prompt-modal" class="modal-overlay">
            <div class="modal-content">
                <div class="modal-header">
                    <h3>添加新提示词</h3>
                    <button class="modal-close" onclick="closeAddPromptModal()">&times;</button>
                </div>
                <div class="modal-body">
                    <div class="form-group">
                        <label for="prompt-name">提示词名称*</label>
                        <input type="text" id="prompt-name" placeholder="请输入提示词名称" maxlength="100">
                    </div>
                    <div class="form-group">
                        <label for="prompt-content">提示词内容*</label>
                        <textarea id="prompt-content" placeholder="请输入提示词内容" rows="8"></textarea>
                    </div>
                    <div class="form-group">
                        <label for="prompt-tags">标签 (可选)</label>
                        <input type="text" id="prompt-tags" placeholder="用逗号分隔多个标签，如：工作,邮件,AI">
                    </div>
                </div>
                <div class="modal-footer">
                    <button class="secondary-btn" onclick="closeAddPromptModal()">取消</button>
                    <button class="primary-btn" onclick="submitPrompt()">保存</button>
                </div>
            </div>
        </div>
    `;
    
    // 添加到页面
    document.body.insertAdjacentHTML('beforeend', modalHtml);
    
    // 聚焦到名称输入框
    setTimeout(() => {
        const nameInput = document.getElementById('prompt-name');
        if (nameInput) nameInput.focus();
    }, 100);
    
    updateDebugInfo('已显示添加提示词模态框');
}

// 关闭模态框
function closeAddPromptModal() {
    const modal = document.getElementById('add-prompt-modal');
    if (modal) {
        modal.remove();
        updateDebugInfo('已关闭添加提示词模态框');
    }
}

// 提交提示词
async function submitPrompt() {
    const name = document.getElementById('prompt-name')?.value?.trim();
    const content = document.getElementById('prompt-content')?.value?.trim();
    const tagsInput = document.getElementById('prompt-tags')?.value?.trim();
    
    if (!name) {
        alert('请输入提示词名称');
        document.getElementById('prompt-name')?.focus();
        return;
    }
    
    if (!content) {
        alert('请输入提示词内容');
        document.getElementById('prompt-content')?.focus();
        return;
    }
    
    // 处理标签
    let tags = null;
    if (tagsInput) {
        tags = tagsInput.split(',').map(tag => tag.trim()).filter(tag => tag.length > 0);
        if (tags.length === 0) tags = null;
    }
    
    try {
        updateDebugInfo(`正在创建提示词: ${name}`);
        const result = await safeInvoke('create_prompt', {
            prompt: {
                name: name,
                content: content,
                tags: tags,
                content_type: null,
                variables_json: null,
                app_scopes_json: null,
                inject_order: null,
                version: 1
            }
        });
        
        updateDebugInfo(`提示词创建成功，ID: ${result}`);
        closeAddPromptModal();
        
        // 显示成功提示
        showNotification(`提示词 "${name}" 创建成功！`, 'success');
        
        // 刷新提示词列表
        loadPrompts();
        
    } catch (error) {
        updateDebugInfo(`创建提示词失败: ${error}`);
        alert(`创建失败: ${error}`);
    }
}

// 显示通知
function showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.textContent = message;
    
    document.body.appendChild(notification);
    
    // 自动移除通知
    setTimeout(() => {
        notification.remove();
    }, 3000);
}

// 将函数暴露到全局作用域
window.closeAddPromptModal = closeAddPromptModal;
window.submitPrompt = submitPrompt;

// 加载提示词列表
async function loadPrompts() {
    try {
        updateDebugInfo('正在加载提示词列表...');
        const prompts = await safeInvoke('get_all_prompts');
        updateDebugInfo(`加载到 ${prompts.length} 个提示词`);
        
        const promptList = document.querySelector('.prompt-list');
        if (!promptList) {
            updateDebugInfo('ERROR: 未找到.prompt-list元素');
            return;
        }
        
        if (prompts.length === 0) {
            promptList.innerHTML = `
                <div class="empty-state">
                    <p>暂无提示词</p>
                    <p class="hint">点击"添加提示词"按钮创建第一个提示词</p>
                </div>
            `;
        } else {
            const promptsHtml = prompts.map(prompt => `
                <div class="prompt-item" data-id="${prompt.id}">
                    <div class="prompt-header">
                        <h3>${prompt.name}</h3>
                        <div class="prompt-actions">
                            <button class="edit-btn" onclick="editPrompt(${prompt.id})">编辑</button>
                            <button class="delete-btn" onclick="deletePrompt(${prompt.id})">删除</button>
                        </div>
                    </div>
                    <div class="prompt-content">
                        <p>${prompt.content.substring(0, 100)}${prompt.content.length > 100 ? '...' : ''}</p>
                    </div>
                </div>
            `).join('');
            
            promptList.innerHTML = promptsHtml;
        }
        
    } catch (error) {
        updateDebugInfo(`加载提示词列表失败: ${error}`);
    }
}

// 全局函数用于提示词操作
window.editPrompt = async (id) => {
    updateDebugInfo(`编辑提示词: ${id}`);
    alert('编辑功能暂未实现');
};

window.deletePrompt = async (id) => {
    updateDebugInfo(`删除提示词: ${id}`);
    if (confirm('确定要删除这个提示词吗？')) {
        try {
            await safeInvoke('delete_prompt', { id: id });
            updateDebugInfo(`提示词 ${id} 删除成功`);
            alert('删除成功！');
            loadPrompts(); // 刷新列表
        } catch (error) {
            updateDebugInfo(`删除提示词失败: ${error}`);
            alert(`删除失败: ${error}`);
        }
    }
};

// DOM加载完成后执行初始化
if (document.readyState === 'loading') {
    updateDebugInfo('等待DOM加载完成');
    document.addEventListener('DOMContentLoaded', initializeApp);
} else {
    updateDebugInfo('DOM已经加载完成，立即初始化');
    initializeApp();
}

// 导出测试函数
window.testFunction = () => {
    updateDebugInfo('测试函数被调用');
    alert('JavaScript正常运行！');
};

window.testButtonBinding = () => {
    const saveBtn = document.getElementById('save-settings-btn');
    const recordBtn = document.getElementById('hotkey-record-btn');
    updateDebugInfo(`按钮测试 - 保存按钮: ${!!saveBtn}, 录制按钮: ${!!recordBtn}`);
    if (saveBtn) {
        updateDebugInfo('保存按钮存在，事件监听器已绑定');
    }
    if (recordBtn) {
        updateDebugInfo('录制按钮存在，事件监听器已绑定');
    }
};
