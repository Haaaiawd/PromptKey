// 现代化GUI交互逻辑 - Tauri应用程序前端
// 动态等待 Tauri 注入，提供浏览器回退，避免点击后"毫无反应"

console.log('=== main.js 开始加载 ===');
console.log('当前时间:', new Date().toLocaleString());
console.log('window对象状态:', typeof window);
console.log('document对象状态:', typeof document);
console.log('window.__TAURI__ 状态:', typeof window !== 'undefined' ? typeof window.__TAURI__ : 'window未定义');

// 更新调试信息显示
function updateDebugInfo(message) {
    console.log('DEBUG:', message);
    const debugContent = document.getElementById('debug-content');
    if (debugContent) {
        debugContent.innerHTML += '<br>' + new Date().toLocaleTimeString() + ': ' + message;
    }
}

// 添加全局测试函数
window.testFunction = function() {
    console.log('测试函数被调用了！');
    updateDebugInfo('测试函数被调用！');
    alert('JavaScript正在工作！');
};

// 立即更新调试信息
setTimeout(() => {
    updateDebugInfo('main.js已加载');
    updateDebugInfo('document.readyState: ' + document.readyState);
    updateDebugInfo('__TAURI__: ' + (window.__TAURI__ ? '已加载' : '未加载'));
}, 100);

// 立即测试基本功能
try {
    console.log('=== 立即测试基本功能 ===');
    console.log('document.body:', document.body);
    console.log('document.readyState:', document.readyState);
    
    updateDebugInfo('基本功能测试开始');
    
    // 添加一个简单的点击测试
    if (document.body) {
        document.body.addEventListener('click', function(e) {
            console.log('=== 检测到点击事件 ===');
            console.log('点击目标:', e.target);
            console.log('点击位置:', e.clientX, e.clientY);
            updateDebugInfo('点击了: ' + (e.target.id || e.target.tagName));
        });
        console.log('已添加全局点击监听器');
        updateDebugInfo('已添加全局点击监听器');
    }
    
} catch (error) {
    console.error('=== 基本功能测试失败 ===', error);
    updateDebugInfo('基本功能测试失败: ' + error.message);
}

function __getTauriUnsafe() {
    return (typeof window !== 'undefined' && window.__TAURI__) ? window.__TAURI__ : null;
}

async function __ensureTauri(timeoutMs = 4000) {
    const existing = __getTauriUnsafe();
    console.log('检查Tauri状态:', existing);
    if (existing) return existing;
    
    const start = Date.now();
    console.log('开始等待Tauri API...');
    
    return await new Promise((resolve, reject) => {
        const timer = setInterval(() => {
            const t = __getTauriUnsafe();
            if (t) {
                console.log('Tauri API 已准备好');
                clearInterval(timer);
                resolve(t);
            } else if (Date.now() - start > timeoutMs) {
                console.error('Tauri API 等待超时');
                clearInterval(timer);
                reject(new Error('Tauri 全局 API 未就绪'));
            }
        }, 50);
    });
}

async function safeInvoke(cmd, payload) {
    try {
        console.log('执行 safeInvoke:', cmd, payload);
        const t = await __ensureTauri();
        if (!t?.core?.invoke) throw new Error('Tauri core API 不可用');
        const result = await t.core.invoke(cmd, payload);
        console.log('safeInvoke 结果:', result);
        return result;
    } catch (e) {
        console.error('safeInvoke 调用失败:', e);
        throw e;
    }
}

async function safeMessage(text, opts) {
    console.log('显示消息:', text, opts);
    const t = __getTauriUnsafe();
    if (t?.dialog?.message) {
        return t.dialog.message(text, opts);
    }
    // 浏览器回退，确保用户能看到反馈
    alert(typeof text === 'string' ? text : JSON.stringify(text));
}

async function safeConfirm(text, opts) {
    console.log('显示确认对话框:', text, opts);
    const t = await (async () => {
        try { return await __ensureTauri(); } catch { return null; }
    })();
    if (t?.dialog?.confirm) {
        return t.dialog.confirm(text, opts);
    }
    // 浏览器回退
    return window.confirm(typeof text === 'string' ? text : JSON.stringify(text));
}

// 主初始化函数
function initializeApp() {
    console.log('DOM加载完成，开始初始化GUI');
    console.log('开始绑定事件监听器...');

    // 测试一个简单的点击事件
    document.body.addEventListener('click', (e) => {
        console.log('检测到点击事件:', e.target);
    });

    // 全局错误可视化，避免"点了没反应"的错觉
    window.addEventListener('error', (e) => {
        const msg = e && e.message ? e.message : String(e);
        console.error('全局错误:', msg);
        safeMessage(`脚本错误: ${msg}`, { title: 'Prompt Manager', type: 'error' });
    });
    window.addEventListener('unhandledrejection', (e) => {
        const reason = e && e.reason ? (e.reason.message || String(e.reason)) : '未知原因';
        console.error('未处理的Promise拒绝:', reason);
        safeMessage(`操作失败: ${reason}`, { title: 'Prompt Manager', type: 'error' });
    });
    
    // 恢复保存的主题设置
    try {
        const savedTheme = localStorage.getItem('promptManagerTheme');
        if (savedTheme) {
            const themeSelect = document.getElementById('theme-select');
            if (themeSelect) {
                themeSelect.value = savedTheme;
            }
            applyTheme(savedTheme);
        }
    } catch (e) {
        console.error('恢复主题设置失败:', e);
    }
    
    // 导航切换
    const navButtons = document.querySelectorAll('.nav-btn');
    const panels = document.querySelectorAll('.panel');
    
    console.log('找到导航按钮数量:', navButtons.length);
    console.log('找到面板数量:', panels.length);
    
    navButtons.forEach((button, index) => {
        console.log(`绑定导航按钮 ${index}:`, button.id);
        button.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('导航按钮被点击:', button.id);
            
            try {
                const targetPanel = button.id.replace('-btn', '') + '-panel';
                console.log('目标面板:', targetPanel);
                
                // 更新导航按钮状态
                navButtons.forEach(btn => {
                    if (btn) btn.classList.remove('active');
                });
                button.classList.add('active');
                
                // 显示目标面板
                let targetPanelFound = false;
                panels.forEach(panel => {
                    if (panel) {
                        panel.classList.remove('active');
                        if (panel.id === targetPanel) {
                            panel.classList.add('active');
                            targetPanelFound = true;
                            console.log('成功切换到面板:', targetPanel);
                        }
                    }
                });
                
                if (!targetPanelFound) {
                    console.warn('未找到目标面板:', targetPanel);
                }
            } catch (error) {
                console.error('面板切换时出错:', error);
            }
        });
    });
    
    // 初始化设置UI
    initSettingsUI().then(() => {
        console.log('初始化设置完成');
    });
    
    // 快捷键录制功能
    setupHotkeyRecording();
    
    // 添加提示词按钮
    const addPromptBtn = document.getElementById('add-prompt-btn');
    console.log('找到添加提示词按钮:', addPromptBtn);
    if (addPromptBtn) {
        addPromptBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('添加提示词按钮被点击');
            openPromptModal();
        });
    } else {
        console.error('无法找到add-prompt-btn元素');
    }
    
    // 编辑和删除按钮事件处理
    document.addEventListener('click', async (e) => {
        console.log('文档点击事件:', e.target);
        
        if (e.target.closest('.edit-btn')) {
            const promptItem = e.target.closest('.prompt-item');
            const promptId = promptItem.dataset.promptId;
            
            if (promptId) {
                try {
                    const prompts = await safeInvoke('get_all_prompts');
                    const prompt = prompts.find(p => p.id == promptId);
                    if (prompt) {
                        openPromptModal(prompt);
                    }
                } catch (error) {
                    console.error('获取提示词详情失败:', error);
                    await safeMessage('获取提示词详情失败', { title: 'Prompt Manager', type: 'error' });
                }
            }
        }
        
        if (e.target.closest('.delete-btn')) {
            const promptItem = e.target.closest('.prompt-item');
            const promptId = promptItem.dataset.promptId;
            
            if (promptId) {
                const confirmed = await safeConfirm('确定要删除这个提示词吗？', {
                    title: 'Prompt Manager',
                    type: 'warning'
                });
                
                if (confirmed) {
                    try {
                        await safeInvoke('delete_prompt', { id: parseInt(promptId) });
                        await safeMessage('提示词已删除', { title: 'Prompt Manager', type: 'info' });
                        loadPrompts();
                    } catch (error) {
                        console.error('删除提示词失败:', error);
                        await safeMessage(`删除失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
                    }
                }
            }
        }
    });
    
    // 保存设置按钮
    const saveSettingsBtn = document.getElementById('save-settings-btn');
    console.log('找到保存设置按钮:', saveSettingsBtn);
    if (saveSettingsBtn) {
        saveSettingsBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('保存设置按钮被点击');
            
            try {
                saveSettingsBtn.disabled = true;
                const hotkeyInput = document.getElementById('hotkey-input');
                const injectionMode = document.getElementById('injection-mode');
                const hotkey = hotkeyInput ? hotkeyInput.value : 'Ctrl+Alt+Space';
                const uiaMode = injectionMode ? injectionMode.value : 'append';
                
                const result = await safeInvoke('apply_settings', { hotkey: hotkey, uia_mode: uiaMode });
                await safeMessage(result, { title: 'Prompt Manager', type: 'info' });
            } catch (error) {
                console.error('保存设置失败:', error);
                await safeMessage(`保存失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            } finally {
                saveSettingsBtn.disabled = false;
            }
        });
    } else {
        console.error('无法找到save-settings-btn元素');
    }
    
    // 重置设置按钮
    const resetSettingsBtn = document.getElementById('reset-settings-btn');
    console.log('找到重置设置按钮:', resetSettingsBtn);
    if (resetSettingsBtn) {
        resetSettingsBtn.addEventListener('click', async (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('重置设置按钮被点击');
            
            try {
                const confirmed = await safeConfirm('确定要重置所有设置吗？', {
                    title: 'Prompt Manager',
                    type: 'warning'
                });
                
                if (confirmed) {
                    resetSettingsBtn.disabled = true;
                    
                    const hotkeyInput = document.getElementById('hotkey-input');
                    if (hotkeyInput) {
                        hotkeyInput.value = 'Ctrl+Alt+Space';
                    }
                    const injectionMode = document.getElementById('injection-mode');
                    if (injectionMode) {
                        injectionMode.value = 'append';
                    }
                    const themeSelect = document.getElementById('theme-select');
                    if (themeSelect) {
                        themeSelect.value = 'light';
                    }
                    
                    try {
                        await safeInvoke('reset_settings');
                        await safeMessage('设置已重置', { title: 'Prompt Manager', type: 'info' });
                    } catch (resetError) {
                        console.error('重置设置失败:', resetError);
                        await safeMessage(`重置设置失败: ${resetError}`, { title: 'Prompt Manager', type: 'error' });
                    } finally {
                        resetSettingsBtn.disabled = false;
                    }
                }
            } catch (error) {
                console.error('重置设置过程中出错:', error);
                await safeMessage(`操作失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    } else {
        console.error('无法找到reset-settings-btn元素');
    }
    
    // 主题切换功能
    const themeSelect = document.getElementById('theme-select');
    console.log('找到主题选择器:', themeSelect);
    if (themeSelect) {
        applyTheme(themeSelect.value);
        themeSelect.addEventListener('change', () => {
            console.log('主题改变:', themeSelect.value);
            applyTheme(themeSelect.value);
        });
    } else {
        console.error('无法找到theme-select元素');
    }

    // 市场面板功能
    setupMarketPanel();

    // 日志面板功能
    setupLogsPanel();

    // 初始化提示词列表
    loadPrompts();
    
    console.log('所有事件监听器绑定完成');
}

// 设置快捷键录制功能
function setupHotkeyRecording() {
    const hotkeyInput = document.getElementById('hotkey-input');
    const hotkeyRecordBtn = document.getElementById('hotkey-record-btn');

    console.log('设置快捷键录制功能:', {hotkeyInput, hotkeyRecordBtn});

    // 设置默认快捷键
    if (hotkeyInput && !hotkeyInput.value) {
        hotkeyInput.value = 'Ctrl+Alt+Space';
    }

    if (hotkeyRecordBtn && hotkeyInput) {
        hotkeyRecordBtn.addEventListener('click', () => {
            console.log('快捷键录制按钮被点击');
            const oldValue = hotkeyInput.value;
            hotkeyInput.value = '请按键...';
            hotkeyInput.focus();
            
            safeMessage('请在5秒内按下快捷键组合，需要包含Ctrl和Alt键', { 
                title: 'Prompt Manager', 
                type: 'info' 
            });
            
            let timeoutId;
            
            const handleKeyDown = (e) => {
                e.preventDefault();
                
                let hotkey = '';
                if (e.ctrlKey) hotkey += 'Ctrl+';
                if (e.shiftKey) hotkey += 'Shift+';
                if (e.altKey) hotkey += 'Alt+';
                
                switch (e.key.toLowerCase()) {
                    case ' ':
                    case 'space':
                        hotkey += 'Space';
                        break;
                    case 'escape':
                        hotkey += 'Esc';
                        break;
                    case 'control':
                    case 'shift':
                    case 'alt':
                        break;
                    default:
                        if (e.key.match(/^F[1-9]|F1[0-2]$/)) {
                            hotkey += e.key;
                        } else {
                            hotkey += e.key.length === 1 ? e.key.toUpperCase() : e.key;
                        }
                }
                
                if (hotkey && !['Control', 'Shift', 'Alt'].includes(e.key) && 
                    hotkey.includes('Ctrl') && hotkey.includes('Alt')) {
                    hotkeyInput.value = hotkey;
                    document.removeEventListener('keydown', handleKeyDown);
                    clearTimeout(timeoutId);
                    
                    safeMessage('快捷键设置成功', { 
                        title: 'Prompt Manager', 
                        type: 'info' 
                    });
                } else if (hotkey && !['Control', 'Shift', 'Alt'].includes(e.key)) {
                    safeMessage('快捷键必须包含Ctrl和Alt键，请重新设置', { 
                        title: 'Prompt Manager', 
                        type: 'warning' 
                    });
                }
            };
            
            document.addEventListener('keydown', handleKeyDown);
            
            timeoutId = setTimeout(() => {
                document.removeEventListener('keydown', handleKeyDown);
                if (hotkeyInput.value === '请按键...') {
                    hotkeyInput.value = oldValue;
                    safeMessage('快捷键设置超时，已恢复之前的设置', { 
                        title: 'Prompt Manager', 
                        type: 'warning' 
                    });
                }
            }, 5000);
        });
    } else {
        console.error('无法找到快捷键相关元素');
    }
}

// 应用主题
function applyTheme(theme) {
    console.log('应用主题:', theme);
    const body = document.body;
    
    body.classList.remove('theme-light', 'theme-dark');
    
    switch (theme) {
        case 'dark':
            body.classList.add('theme-dark');
            break;
        case 'light':
        default:
            body.classList.add('theme-light');
            break;
    }
    
    try {
        localStorage.setItem('promptManagerTheme', theme);
    } catch (e) {
        console.error('保存主题设置失败:', e);
    }
}

// 设置市场面板
function setupMarketPanel() {
    const refreshMarketBtn = document.getElementById('refresh-market-btn');
    console.log('设置市场面板，刷新按钮:', refreshMarketBtn);
    
    if (refreshMarketBtn) {
        refreshMarketBtn.addEventListener('click', async () => {
            try {
                console.log('刷新市场提示词模板');
                const marketList = document.getElementById('market-list');
                if (marketList) {
                    marketList.innerHTML = '<div class="loading">加载中...</div>';
                }
                
                await new Promise(resolve => setTimeout(resolve, 1000));
                renderMarketTemplates([]);
            } catch (error) {
                console.error('刷新市场提示词模板失败:', error);
                await safeMessage(`刷新失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
                
                const marketList = document.getElementById('market-list');
                if (marketList) {
                    marketList.innerHTML = '<div class="error-state">加载失败，请稍后重试</div>';
                }
            }
        });
    }

    const marketSearchInput = document.getElementById('market-search');
    const marketSearchBtn = document.getElementById('market-search-btn');
    if (marketSearchInput && marketSearchBtn) {
        marketSearchBtn.addEventListener('click', () => {
            const keyword = marketSearchInput.value.trim();
            console.log('搜索市场提示词模板:', keyword);
        });
        
        marketSearchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                marketSearchBtn.click();
            }
        });
    }
}

// 渲染市场提示词模板
function renderMarketTemplates(templates) {
    const marketList = document.getElementById('market-list');
    if (!marketList) return;
    
    if (!templates || templates.length === 0) {
        marketList.innerHTML = `
            <div class="empty-state">
                <p>暂无提示词模板</p>
                <p class="hint">请检查网络连接或稍后重试</p>
            </div>
        `;
        return;
    }
    
    marketList.innerHTML = '<div class="empty-state"><p>功能开发中...</p></div>';
}

// 设置日志面板
function setupLogsPanel() {
    const clearLogsBtn = document.getElementById('clear-logs-btn');
    console.log('设置日志面板，清空按钮:', clearLogsBtn);
    
    if (clearLogsBtn) {
        clearLogsBtn.addEventListener('click', async () => {
            try {
                console.log('清空使用日志');
                const confirmed = await safeConfirm('确定要清空所有使用日志吗？', {
                    title: 'Prompt Manager',
                    type: 'warning'
                });
                
                if (confirmed) {
                    await new Promise(resolve => setTimeout(resolve, 500));
                    loadUsageLogs();
                    await safeMessage('日志已清空', { title: 'Prompt Manager', type: 'info' });
                }
            } catch (error) {
                console.error('清空日志失败:', error);
                await safeMessage(`清空失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    }

    const logsSearchInput = document.getElementById('logs-search');
    const logsSearchBtn = document.getElementById('logs-search-btn');
    if (logsSearchInput && logsSearchBtn) {
        logsSearchBtn.addEventListener('click', () => {
            const keyword = logsSearchInput.value.trim();
            console.log('搜索使用日志:', keyword);
        });
        
        logsSearchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                logsSearchBtn.click();
            }
        });
    }

    loadUsageLogs();
}

// 加载使用日志
async function loadUsageLogs() {
    try {
        console.log('加载使用日志');
        const logsList = document.getElementById('logs-list');
        if (logsList) {
            logsList.innerHTML = '<div class="loading">加载中...</div>';
        }
        
        await new Promise(resolve => setTimeout(resolve, 500));
        renderUsageLogs([]);
    } catch (error) {
        console.error('加载使用日志失败:', error);
        await safeMessage(`加载失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
        
        const logsList = document.getElementById('logs-list');
        if (logsList) {
            logsList.innerHTML = '<div class="error-state">加载失败，请稍后重试</div>';
        }
    }
}

// 渲染使用日志
function renderUsageLogs(logs) {
    const logsList = document.getElementById('logs-list');
    if (!logsList) return;
    
    if (!logs || logs.length === 0) {
        logsList.innerHTML = `
            <div class="empty-state">
                <p>暂无使用日志</p>
                <p class="hint">使用提示词后会在这里显示记录</p>
            </div>
        `;
        return;
    }
    
    logsList.innerHTML = '<div class="empty-state"><p>功能开发中...</p></div>';
}

// 加载提示词列表
async function loadPrompts() {
    try {
        console.log('开始加载提示词列表');
        const promptList = document.querySelector('.prompt-list');
        if (promptList) {
            promptList.innerHTML = '<div class="loading">加载中...</div>';
        }
        
        const prompts = await safeInvoke('get_all_prompts');
        console.log('成功获取提示词列表，数量:', prompts.length);
        renderPrompts(prompts);
    } catch (error) {
        console.error('加载提示词列表失败:', error);
        await safeMessage(`加载提示词失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
        
        const promptList = document.querySelector('.prompt-list');
        if (promptList) {
            promptList.innerHTML = '<div class="error-state">加载失败，请稍后重试</div>';
        }
    }
}

// 渲染提示词列表
function renderPrompts(prompts) {
    const promptList = document.querySelector('.prompt-list');
    if (!promptList) return;
    
    promptList.innerHTML = '';
    
    if (!prompts || prompts.length === 0) {
        promptList.innerHTML = `
            <div class="empty-state">
                <p>暂无提示词</p>
                <p class="hint">点击"添加提示词"按钮创建第一个提示词</p>
            </div>
        `;
        return;
    }
    
    prompts.forEach(prompt => {
        const promptItem = document.createElement('div');
        promptItem.className = 'prompt-item';
        promptItem.dataset.promptId = prompt.id;
        
        let tagsHtml = '';
        if (prompt.tags && prompt.tags.length > 0) {
            tagsHtml = prompt.tags.map(tag => 
                `<span class="tag">${tag}</span>`
            ).join('');
        }
        
        let updateTime = '未知时间';
        if (prompt.updated_at) {
            const d = new Date(prompt.updated_at);
            updateTime = isNaN(d.getTime()) ? String(prompt.updated_at) : d.toLocaleString('zh-CN');
        }
        
        promptItem.innerHTML = `
            <div class="prompt-info">
                <h3>${prompt.name}</h3>
                <p class="prompt-content">${prompt.content}</p>
                <div class="prompt-meta">
                    ${tagsHtml}
                    <span class="last-used">更新: ${updateTime}</span>
                </div>
            </div>
            <div class="prompt-actions">
                <button class="btn-icon edit-btn" title="编辑">
                    <i class="icon-edit"></i>
                </button>
                <button class="btn-icon delete-btn" title="删除">
                    <i class="icon-delete"></i>
                </button>
            </div>
        `;
        
        promptList.appendChild(promptItem);
    });
}

// 打开提示词模态框
function openPromptModal(prompt = null) {
    console.log('打开提示词模态框:', prompt);
    
    const modal = document.createElement('div');
    modal.className = 'modal';
    modal.innerHTML = `
        <div class="modal-content">
            <div class="modal-header">
                <h2>${prompt ? '编辑提示词' : '添加提示词'}</h2>
                <span class="close-btn">&times;</span>
            </div>
            <div class="modal-body">
                <div class="form-group">
                    <label for="prompt-name">名称</label>
                    <input type="text" id="prompt-name" class="form-input" value="${prompt ? prompt.name : ''}">
                </div>
                <div class="form-group">
                    <label for="prompt-tags">标签（逗号分隔）</label>
                    <input type="text" id="prompt-tags" class="form-input" value="${prompt && prompt.tags ? prompt.tags.join(', ') : ''}">
                </div>
                <div class="form-group">
                    <label for="prompt-content">内容</label>
                    <textarea id="prompt-content" class="form-textarea">${prompt ? prompt.content : ''}</textarea>
                </div>
            </div>
            <div class="modal-footer">
                <button class="secondary-btn" id="cancel-btn" type="button">取消</button>
                <button class="primary-btn" id="save-btn" type="button">${prompt ? '更新' : '保存'}</button>
            </div>
        </div>
    `;
    
    document.body.appendChild(modal);
    
    const closeBtn = modal.querySelector('.close-btn');
    const cancelBtn = modal.querySelector('#cancel-btn');
    const saveBtn = modal.querySelector('#save-btn');
    
    function closeModal() {
        document.body.removeChild(modal);
    }
    
    closeBtn.addEventListener('click', closeModal);
    cancelBtn.addEventListener('click', closeModal);

    const nameInput = modal.querySelector('#prompt-name');
    if (nameInput) nameInput.focus();
    
    saveBtn.addEventListener('click', async () => {
        const name = document.getElementById('prompt-name').value;
        const tags = document.getElementById('prompt-tags').value;
        const content = document.getElementById('prompt-content').value;
        
        if (!name || !content) {
            await safeMessage('请填写名称和内容', { title: 'Prompt Manager', type: 'warning' });
            return;
        }
        
        let tagArray = null;
        if (tags) {
            tagArray = tags.split(',').map(tag => tag.trim()).filter(tag => tag.length > 0);
            if (tagArray.length === 0) tagArray = null;
        }
        
        try {
            saveBtn.disabled = true;
            if (prompt) {
                const updatedPrompt = {
                    id: prompt.id,
                    name: name,
                    tags: tagArray,
                    content: content,
                    content_type: prompt.content_type || null,
                    variables_json: prompt.variables_json || null,
                    app_scopes_json: prompt.app_scopes_json || null,
                    inject_order: prompt.inject_order || null,
                    version: (prompt.version || 0) + 1,
                    updated_at: new Date().toISOString()
                };
                
                await safeInvoke('update_prompt', { prompt: updatedPrompt });
                await safeMessage('提示词已更新', { title: 'Prompt Manager', type: 'info' });
            } else {
                const newPrompt = {
                    id: null,
                    name: name,
                    tags: tagArray,
                    content: content,
                    content_type: null,
                    variables_json: null,
                    app_scopes_json: null,
                    inject_order: null,
                    version: 1,
                    updated_at: new Date().toISOString()
                };
                
                await safeInvoke('create_prompt', { prompt: newPrompt });
                await safeMessage('提示词已创建', { title: 'Prompt Manager', type: 'info' });
            }
            
            closeModal();
            loadPrompts();
        } catch (error) {
            console.error('保存提示词失败:', error);
            await safeMessage(`保存失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
        } finally {
            saveBtn.disabled = false;
        }
    });
}

// 初始化设置UI
async function initSettingsUI() {
    try {
        console.log('初始化设置UI');
        const settings = await safeInvoke('get_settings');
        console.log('获取到的设置:', settings);
        
        const hkInput = document.getElementById('hotkey-input');
        if (hkInput && settings.hotkey) {
            hkInput.value = settings.hotkey;
        }
        const modeSel = document.getElementById('injection-mode');
        if (modeSel && settings.uia_mode) {
            const frontValue = settings.uia_mode === 'overwrite' ? 'replace' : settings.uia_mode;
            if ([...modeSel.options].some(o => o.value === frontValue)) {
                modeSel.value = frontValue;
            }
        }
    } catch (e) {
        console.error('初始化设置UI失败:', e);
    }
}

// 智能初始化：检查DOM状态并相应执行
console.log('检查DOM状态:', document.readyState);
updateDebugInfo('检查DOM状态: ' + document.readyState);

// 等待Tauri API加载的函数
async function waitForTauri(maxWait = 5000) {
    const start = Date.now();
    while (Date.now() - start < maxWait) {
        if (window.__TAURI__) {
            updateDebugInfo('Tauri API已加载！');
            return window.__TAURI__;
        }
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    updateDebugInfo('Tauri API加载超时');
    return null;
}

// 修改后的初始化函数
async function initializeApp() {
    console.log('=== 开始初始化应用 ===');
    updateDebugInfo('开始初始化应用');
    
    // 等待Tauri API
    const tauri = await waitForTauri();
    if (tauri) {
        updateDebugInfo('Tauri API可用，继续初始化');
    } else {
        updateDebugInfo('Tauri API不可用，使用降级模式');
    }
    
    // 检查关键元素
    const addPromptBtn = document.getElementById('add-prompt-btn');
    const saveSettingsBtn = document.getElementById('save-settings-btn');
    const resetSettingsBtn = document.getElementById('reset-settings-btn');
    const navButtons = document.querySelectorAll('.nav-btn');
    
    updateDebugInfo('找到元素: add-prompt-btn=' + !!addPromptBtn + ', save-settings-btn=' + !!saveSettingsBtn + ', reset-settings-btn=' + !!resetSettingsBtn + ', nav-buttons=' + navButtons.length);
    
    // 绑定导航按钮
    navButtons.forEach((button, index) => {
        console.log('绑定导航按钮:', button.id);
        updateDebugInfo('绑定导航按钮: ' + button.id);
        
        button.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('导航按钮被点击:', button.id);
            updateDebugInfo('导航按钮被点击: ' + button.id);
            
            // 简单的面板切换逻辑
            const panels = document.querySelectorAll('.panel');
            const navBtns = document.querySelectorAll('.nav-btn');
            
            // 移除所有active类
            panels.forEach(panel => panel.classList.remove('active'));
            navBtns.forEach(btn => btn.classList.remove('active'));
            
            // 添加active类到当前按钮
            button.classList.add('active');
            
            // 显示对应面板
            let targetPanelId = '';
            if (button.id === 'prompts-btn') targetPanelId = 'prompts-panel';
            else if (button.id === 'settings-btn') targetPanelId = 'settings-panel';
            else if (button.id === 'market-btn') targetPanelId = 'market-panel';
            else if (button.id === 'logs-btn') targetPanelId = 'logs-panel';
            
            if (targetPanelId) {
                const targetPanel = document.getElementById(targetPanelId);
                if (targetPanel) {
                    targetPanel.classList.add('active');
                    updateDebugInfo('切换到面板: ' + targetPanelId);
                } else {
                    updateDebugInfo('未找到面板: ' + targetPanelId);
                }
            }
        });
    });
    
    // 绑定添加提示词按钮
    if (addPromptBtn) {
        addPromptBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('添加提示词按钮被点击');
            updateDebugInfo('添加提示词按钮被点击');
            alert('添加提示词功能（需要Tauri API）');
        });
        updateDebugInfo('已绑定添加提示词按钮');
    } else {
        updateDebugInfo('ERROR: 未找到添加提示词按钮');
    }
    
    // 绑定保存设置按钮
    if (saveSettingsBtn) {
        saveSettingsBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('保存设置按钮被点击');
            updateDebugInfo('保存设置按钮被点击');
            alert('保存设置功能（需要Tauri API）');
        });
        updateDebugInfo('已绑定保存设置按钮');
    } else {
        updateDebugInfo('ERROR: 未找到保存设置按钮');
    }
    
    // 绑定重置设置按钮
    if (resetSettingsBtn) {
        resetSettingsBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('重置设置按钮被点击');
            updateDebugInfo('重置设置按钮被点击');
            alert('重置设置功能（需要Tauri API）');
        });
        updateDebugInfo('已绑定重置设置按钮');
    } else {
        updateDebugInfo('ERROR: 未找到重置设置按钮');
    }
    
    // 绑定快捷键录制按钮
    const hotkeyRecordBtn = document.getElementById('hotkey-record-btn');
    if (hotkeyRecordBtn) {
        hotkeyRecordBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            console.log('快捷键录制按钮被点击');
            updateDebugInfo('快捷键录制按钮被点击');
            alert('快捷键录制功能');
        });
        updateDebugInfo('已绑定快捷键录制按钮');
    } else {
        updateDebugInfo('ERROR: 未找到快捷键录制按钮');
    }
    
    updateDebugInfo('应用初始化完成');
    console.log('=== 应用初始化完成 ===');
}

if (document.readyState === 'loading') {
    console.log('DOM仍在加载中，等待DOMContentLoaded事件');
    updateDebugInfo('等待DOM加载完成');
    document.addEventListener('DOMContentLoaded', initializeApp);
} else {
    console.log('DOM已经加载完成，立即执行初始化');
    updateDebugInfo('DOM已加载，立即初始化');
    setTimeout(initializeApp, 100);
}
