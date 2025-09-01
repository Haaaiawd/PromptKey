// 现代化GUI交互逻辑（使用 withGlobalTauri 全局 API）
const { core, dialog } = window.__TAURI__ || {};
const invoke = core?.invoke || (async () => { throw new Error('Tauri core API 不可用'); });
const confirm = dialog?.confirm || (async () => { throw new Error('Tauri dialog.confirm 不可用'); });
const message = dialog?.message || (async () => { throw new Error('Tauri dialog.message 不可用'); });

document.addEventListener('DOMContentLoaded', () => {
    console.log('DOM加载完成，开始初始化GUI');
    
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
    
    if (navButtons.length === 0) {
        console.error('未找到导航按钮元素');
    }
    
    if (panels.length === 0) {
        console.error('未找到面板元素');
    }
    
    navButtons.forEach(button => {
        button.addEventListener('click', () => {
            try {
                console.log('切换面板:', button.id);
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
                        }
                    }
                });
                
                if (!targetPanelFound) {
                    console.warn('未找到目标面板:', targetPanel);
                }
                
                console.log('面板切换完成');
            } catch (error) {
                console.error('面板切换时出错:', error);
                // 可以添加用户友好的错误提示
            }
        });
    });
    
    // 初始化设置（从后端读取当前配置）
    initSettingsUI().then(() => {
        console.log('初始化设置完成');
    });
    
    // 快捷键录制功能
    const hotkeyInput = document.getElementById('hotkey-input');
    const hotkeyRecordBtn = document.getElementById('hotkey-record-btn');

    // 设置默认快捷键（如果还没有设置）
    if (hotkeyInput && !hotkeyInput.value) {
        hotkeyInput.value = 'Ctrl+Alt+Space';
    }

    if (hotkeyRecordBtn && hotkeyInput) {
        hotkeyRecordBtn.addEventListener('click', () => {
            // 保存当前值以便恢复
            const oldValue = hotkeyInput.value;
            hotkeyInput.value = '请按键...';
            hotkeyInput.focus();
            
            // 显示提示信息
            message('请在5秒内按下快捷键组合，需要包含Ctrl和Alt键', { 
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
                
                // 处理特殊键
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
                        // 仅修饰键不处理
                        break;
                    default:
                        // 处理功能键 F1-F12
                        if (e.key.match(/^F[1-9]|F1[0-2]$/)) {
                            hotkey += e.key;
                        } else {
                            // 处理普通键
                            hotkey += e.key.length === 1 ? e.key.toUpperCase() : e.key;
                        }
                }
                
                // 只有当有实际按键且包含必要修饰键时才更新
                if (hotkey && !['Control', 'Shift', 'Alt'].includes(e.key) && 
                    hotkey.includes('Ctrl') && hotkey.includes('Alt')) {
                    hotkeyInput.value = hotkey;
                    document.removeEventListener('keydown', handleKeyDown);
                    clearTimeout(timeoutId);
                    
                    // 提示设置成功
                    message('快捷键设置成功', { 
                        title: 'Prompt Manager', 
                        type: 'info' 
                    });
                } else if (hotkey && !['Control', 'Shift', 'Alt'].includes(e.key)) {
                    // 如果没有必要的修饰键，提示用户
                    message('快捷键必须包含Ctrl和Alt键，请重新设置', { 
                        title: 'Prompt Manager', 
                        type: 'warning' 
                    });
                }
            };
            
            document.addEventListener('keydown', handleKeyDown);
            
            // 5秒后自动移除监听器
            timeoutId = setTimeout(() => {
                document.removeEventListener('keydown', handleKeyDown);
                if (hotkeyInput.value === '请按键...') {
                    // 恢复默认值或之前的值
                    hotkeyInput.value = oldValue;
                    message('快捷键设置超时，已恢复之前的设置', { 
                        title: 'Prompt Manager', 
                        type: 'warning' 
                    });
                }
            }, 5000);
        });
    } else {
        console.error('无法找到快捷键相关元素');
    }
    
    // 添加提示词按钮
    const addPromptBtn = document.getElementById('add-prompt-btn');
    if (addPromptBtn) {
        addPromptBtn.addEventListener('click', async () => {
            // 打开添加提示词对话框
            openPromptModal();
        });
    } else {
        console.error('无法找到add-prompt-btn元素');
    }
    
    // 编辑和删除按钮事件处理
    document.addEventListener('click', async (e) => {
        if (e.target.closest('.edit-btn')) {
            // 获取提示词ID
            const promptItem = e.target.closest('.prompt-item');
            const promptId = promptItem.dataset.promptId;
            
            if (promptId) {
                // 获取提示词详情
                try {
                    const prompts = await invoke('get_all_prompts');
                    const prompt = prompts.find(p => p.id == promptId);
                    if (prompt) {
                        openPromptModal(prompt);
                    }
                } catch (error) {
                    console.error('获取提示词详情失败:', error);
                    await message('获取提示词详情失败', { title: 'Prompt Manager', type: 'error' });
                }
            }
        }
        
        if (e.target.closest('.delete-btn')) {
            // 获取提示词ID
            const promptItem = e.target.closest('.prompt-item');
            const promptId = promptItem.dataset.promptId;
            
            if (promptId) {
                // 确认删除
                const confirmed = await confirm('确定要删除这个提示词吗？', {
                    title: 'Prompt Manager',
                    type: 'warning'
                });
                
                if (confirmed) {
                    try {
                        await invoke('delete_prompt', { id: parseInt(promptId) });
                        await message('提示词已删除', { title: 'Prompt Manager', type: 'info' });
                        // 重新加载提示词列表
                        loadPrompts();
                    } catch (error) {
                        console.error('删除提示词失败:', error);
                        await message(`删除失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
                    }
                }
            }
        }
    });
    
    // 保存设置按钮
    const saveSettingsBtn = document.getElementById('save-settings-btn');
    if (saveSettingsBtn) {
        saveSettingsBtn.addEventListener('click', async () => {
            try {
                const hotkeyInput = document.getElementById('hotkey-input');
                const injectionMode = document.getElementById('injection-mode');
                const hotkey = hotkeyInput ? hotkeyInput.value : 'Ctrl+Alt+Space';
                const uiaMode = injectionMode ? injectionMode.value : 'append';
                // 调用后端保存并重启服务
                await invoke('apply_settings', { hotkey, uia_mode: uiaMode });
                await message('设置已保存', { title: 'Prompt Manager', type: 'info' });
            } catch (error) {
                console.error('保存设置失败:', error);
                await message(`保存失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    } else {
        console.error('无法找到save-settings-btn元素');
    }
    
    // 重置设置按钮
    const resetSettingsBtn = document.getElementById('reset-settings-btn');
    if (resetSettingsBtn) {
        resetSettingsBtn.addEventListener('click', async () => {
            try {
                console.log('开始重置设置');
                const confirmed = await confirm('确定要重置所有设置吗？', {
                    title: 'Prompt Manager',
                    type: 'warning'
                });
                
                if (confirmed) {
                    console.log('用户确认重置设置');
                    // 实现重置设置功能
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
                    
                    // 调用后端重置设置功能
                    try {
                        await invoke('reset_settings');
                        await message('设置已重置', { title: 'Prompt Manager', type: 'info' });
                        console.log('设置重置完成');
                    } catch (resetError) {
                        console.error('重置设置失败:', resetError);
                        await message(`重置设置失败: ${resetError}`, { title: 'Prompt Manager', type: 'error' });
                    }
                } else {
                    console.log('用户取消重置设置');
                }
            } catch (error) {
                console.error('重置设置过程中出错:', error);
                await message(`操作失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    } else {
        console.error('无法找到reset-settings-btn元素');
    }
    
    // 主题切换功能
    const themeSelect = document.getElementById('theme-select');
    if (themeSelect) {
        // 页面加载时应用当前主题
        applyTheme(themeSelect.value);
        
        themeSelect.addEventListener('change', () => {
            applyTheme(themeSelect.value);
        });
    } else {
        console.error('无法找到theme-select元素');
    }

    function applyTheme(theme) {
        console.log('应用主题:', theme);
        const body = document.body;
        
        // 移除所有主题类
        body.classList.remove('theme-light', 'theme-dark');
        
        // 应用选定的主题
        switch (theme) {
            case 'dark':
                body.classList.add('theme-dark');
                break;
            case 'light':
            default:
                body.classList.add('theme-light');
                break;
        }
        
        // 保存主题设置到localStorage
        try {
            localStorage.setItem('promptManagerTheme', theme);
        } catch (e) {
            console.error('保存主题设置失败:', e);
        }
    }

    // 市场面板功能
    const refreshMarketBtn = document.getElementById('refresh-market-btn');
    if (refreshMarketBtn) {
        refreshMarketBtn.addEventListener('click', async () => {
            try {
                console.log('刷新市场提示词模板');
                // 显示加载状态
                const marketList = document.getElementById('market-list');
                if (marketList) {
                    marketList.innerHTML = '<div class="loading">加载中...</div>';
                }
                
                // TODO: 调用后端API获取市场提示词模板
                // 这里暂时使用模拟数据
                await new Promise(resolve => setTimeout(resolve, 1000)); // 模拟网络延迟
                
                // 显示市场提示词模板
                renderMarketTemplates([]);
            } catch (error) {
                console.error('刷新市场提示词模板失败:', error);
                await message(`刷新失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
                
                // 显示错误状态
                const marketList = document.getElementById('market-list');
                if (marketList) {
                    marketList.innerHTML = '<div class="error-state">加载失败，请稍后重试</div>';
                }
            }
        });
    } else {
        console.error('无法找到refresh-market-btn元素');
    }

    // 搜索市场提示词模板
    const marketSearchInput = document.getElementById('market-search');
    const marketSearchBtn = document.getElementById('market-search-btn');
    if (marketSearchInput && marketSearchBtn) {
        marketSearchBtn.addEventListener('click', () => {
            const keyword = marketSearchInput.value.trim();
            console.log('搜索市场提示词模板:', keyword);
            // TODO: 实现搜索功能
        });
        
        marketSearchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                marketSearchBtn.click();
            }
        });
    } else {
        console.error('无法找到市场搜索相关元素');
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
        
        // TODO: 实现模板渲染逻辑
        marketList.innerHTML = '<div class="empty-state"><p>功能开发中...</p></div>';
    }

    // 日志面板功能
    const clearLogsBtn = document.getElementById('clear-logs-btn');
    if (clearLogsBtn) {
        clearLogsBtn.addEventListener('click', async () => {
            try {
                console.log('清空使用日志');
                const confirmed = await confirm('确定要清空所有使用日志吗？', {
                    title: 'Prompt Manager',
                    type: 'warning'
                });
                
                if (confirmed) {
                    // TODO: 调用后端API清空日志
                    await new Promise(resolve => setTimeout(resolve, 500)); // 模拟API调用
                    
                    // 重新加载日志
                    loadUsageLogs();
                    
                    await message('日志已清空', { title: 'Prompt Manager', type: 'info' });
                }
            } catch (error) {
                console.error('清空日志失败:', error);
                await message(`清空失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    } else {
        console.error('无法找到clear-logs-btn元素');
    }

    // 搜索使用日志
    const logsSearchInput = document.getElementById('logs-search');
    const logsSearchBtn = document.getElementById('logs-search-btn');
    if (logsSearchInput && logsSearchBtn) {
        logsSearchBtn.addEventListener('click', () => {
            const keyword = logsSearchInput.value.trim();
            console.log('搜索使用日志:', keyword);
            // TODO: 实现搜索功能
        });
        
        logsSearchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                logsSearchBtn.click();
            }
        });
    } else {
        console.error('无法找到日志搜索相关元素');
    }

    // 加载使用日志
    async function loadUsageLogs() {
        try {
            console.log('加载使用日志');
            // 显示加载状态
            const logsList = document.getElementById('logs-list');
            if (logsList) {
                logsList.innerHTML = '<div class="loading">加载中...</div>';
            }
            
            // TODO: 调用后端API获取使用日志
            // 这里暂时使用模拟数据
            await new Promise(resolve => setTimeout(resolve, 500)); // 模拟网络延迟
            
            // 显示使用日志
            renderUsageLogs([]);
        } catch (error) {
            console.error('加载使用日志失败:', error);
            await message(`加载失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            
            // 显示错误状态
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
        
        // TODO: 实现日志渲染逻辑
        logsList.innerHTML = '<div class="empty-state"><p>功能开发中...</p></div>';
    }

    // 初始化时加载使用日志
    loadUsageLogs();

    // 加载提示词列表
    async function loadPrompts() {
        try {
            console.log('开始加载提示词列表');
            const promptList = document.querySelector('.prompt-list');
            if (promptList) {
                // 显示加载状态
                promptList.innerHTML = '<div class="loading">加载中...</div>';
            }
            
            const prompts = await invoke('get_all_prompts');
            console.log('成功获取提示词列表，数量:', prompts.length);
            renderPrompts(prompts);
        } catch (error) {
            console.error('加载提示词列表失败:', error);
            // 添加用户友好的错误提示
            await message(`加载提示词失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            
            // 显示错误状态
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
        
        // 清空现有内容
        promptList.innerHTML = '';
        
        // 添加提示词项
        prompts.forEach(prompt => {
            const promptItem = document.createElement('div');
            promptItem.className = 'prompt-item';
            promptItem.dataset.promptId = prompt.id;
            
            // 处理标签
            let tagsHtml = '';
            if (prompt.tags && prompt.tags.length > 0) {
                tagsHtml = prompt.tags.map(tag => 
                    `<span class="tag">${tag}</span>`
                ).join('');
            }
            
            // 处理更新时间
            let updateTime = '未知时间';
            if (prompt.updated_at) {
                // 简单处理时间显示
                updateTime = new Date(prompt.updated_at).toLocaleString('zh-CN');
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
        // 创建模态框
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
                    <button class="secondary-btn" id="cancel-btn">取消</button>
                    <button class="primary-btn" id="save-btn">${prompt ? '更新' : '保存'}</button>
                </div>
            </div>
        `;
        
        // 添加到页面
        document.body.appendChild(modal);
        
        // 获取模态框元素
        const closeBtn = modal.querySelector('.close-btn');
        const cancelBtn = modal.querySelector('#cancel-btn');
        const saveBtn = modal.querySelector('#save-btn');
        
        // 关闭模态框函数
        function closeModal() {
            document.body.removeChild(modal);
        }
        
        // 绑定事件
        closeBtn.addEventListener('click', closeModal);
        cancelBtn.addEventListener('click', closeModal);
        
        // 保存提示词
        saveBtn.addEventListener('click', async () => {
            const name = document.getElementById('prompt-name').value;
            const tags = document.getElementById('prompt-tags').value;
            const content = document.getElementById('prompt-content').value;
            
            if (!name || !content) {
                await message('请填写名称和内容', { title: 'Prompt Manager', type: 'warning' });
                return;
            }
            
            // 处理标签
            let tagArray = null;
            if (tags) {
                tagArray = tags.split(',').map(tag => tag.trim()).filter(tag => tag.length > 0);
                if (tagArray.length === 0) tagArray = null;
            }
            
            try {
                if (prompt) {
                    // 更新提示词
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
                    
                    await invoke('update_prompt', { prompt: updatedPrompt });
                    await message('提示词已更新', { title: 'Prompt Manager', type: 'info' });
                } else {
                    // 创建新提示词
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
                        updated_at: null
                    };
                    
                    await invoke('create_prompt', { prompt: newPrompt });
                    await message('提示词已创建', { title: 'Prompt Manager', type: 'info' });
                }
                
                // 关闭模态框
                closeModal();
                
                // 重新加载提示词列表
                loadPrompts();
            } catch (error) {
                console.error('保存提示词失败:', error);
                await message(`保存失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    }
    
    // 初始化提示词列表
    loadPrompts();
    
    async function initSettingsUI() {
        try {
            const settings = await invoke('get_settings');
            // settings: { hotkey, uia_mode }
            const hkInput = document.getElementById('hotkey-input');
            if (hkInput && settings.hotkey) {
                hkInput.value = settings.hotkey;
            }
            const modeSel = document.getElementById('injection-mode');
            if (modeSel && settings.uia_mode) {
                // 后端使用 overwrite/append，前端 select 使用 replace/append
                const frontValue = settings.uia_mode === 'overwrite' ? 'replace' : settings.uia_mode;
                if ([...modeSel.options].some(o => o.value === frontValue)) {
                    modeSel.value = frontValue;
                }
            }
        } catch (e) {
            console.error('初始化设置UI失败:', e);
        }
    }
});