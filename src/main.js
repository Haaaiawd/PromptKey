// 现代化GUI交互逻辑
import { invoke } from '@tauri-apps/api/core';
import { confirm, message } from '@tauri-apps/plugin-dialog';

document.addEventListener('DOMContentLoaded', () => {
    console.log('DOM加载完成，开始初始化GUI');
    
    // 导航切换
    const navButtons = document.querySelectorAll('.nav-btn');
    const panels = document.querySelectorAll('.panel');
    
    navButtons.forEach(button => {
        button.addEventListener('click', () => {
            const targetPanel = button.id.replace('-btn', '') + '-panel';
            
            // 更新导航按钮状态
            navButtons.forEach(btn => btn.classList.remove('active'));
            button.classList.add('active');
            
            // 显示目标面板
            panels.forEach(panel => {
                panel.classList.remove('active');
                if (panel.id === targetPanel) {
                    panel.classList.add('active');
                }
            });
        });
    });
    
    // 服务控制功能
    const serviceStatus = document.getElementById('service-status');
    const toggleServiceBtn = document.getElementById('toggle-service-btn');
    
    // 检查服务状态
    async function checkServiceStatus() {
        try {
            if (!serviceStatus) {
                console.error('无法找到service-status元素');
                return;
            }
            
            console.log('正在检查服务状态...');
            const isRunning = await invoke('check_service_status');
            console.log('服务状态:', isRunning);
            
            if (isRunning) {
                serviceStatus.textContent = '运行中';
                serviceStatus.className = 'status-running';
                if (toggleServiceBtn) {
                    toggleServiceBtn.textContent = '停止服务';
                }
            } else {
                serviceStatus.textContent = '已停止';
                serviceStatus.className = 'status-stopped';
                if (toggleServiceBtn) {
                    toggleServiceBtn.textContent = '启动服务';
                }
            }
        } catch (error) {
            console.error('检查服务状态时出错:', error);
            if (serviceStatus) {
                serviceStatus.textContent = '未知';
                serviceStatus.className = 'status-unknown';
            }
        }
    }
    
    // 初始化服务状态
    console.log('初始化服务状态...');
    checkServiceStatus();
    
    // 切换服务状态
    if (toggleServiceBtn) {
        toggleServiceBtn.addEventListener('click', async () => {
            try {
                console.log('切换服务状态...');
                const isRunning = await invoke('check_service_status');
                
                if (isRunning) {
                    // 停止服务
                    console.log('正在停止服务...');
                    await invoke('stop_service');
                    await message('服务已停止', { title: 'Prompt Manager', type: 'info' });
                } else {
                    // 启动服务
                    console.log('正在启动服务...');
                    await invoke('start_service');
                    await message('服务已启动', { title: 'Prompt Manager', type: 'info' });
                }
                
                // 更新状态显示
                checkServiceStatus();
            } catch (error) {
                console.error('切换服务状态时出错:', error);
                await message(`操作失败: ${error}`, { title: 'Prompt Manager', type: 'error' });
            }
        });
    } else {
        console.error('无法找到toggle-service-btn元素');
    }
    
    // 快捷键录制功能
    const hotkeyInput = document.getElementById('hotkey-input');
    const hotkeyRecordBtn = document.getElementById('hotkey-record-btn');
    
    // 设置默认快捷键（如果还没有设置）
    if (hotkeyInput && !hotkeyInput.value) {
        hotkeyInput.value = 'Ctrl+Alt+Space';
    }
    
    if (hotkeyRecordBtn && hotkeyInput) {
        hotkeyRecordBtn.addEventListener('click', () => {
            hotkeyInput.value = '请按键...';
            hotkeyInput.focus();
            
            const handleKeyDown = (e) => {
                e.preventDefault();
                
                let hotkey = '';
                if (e.ctrlKey) hotkey += 'Ctrl+';
                if (e.shiftKey) hotkey += 'Shift+';
                if (e.altKey) hotkey += 'Alt+';
                
                // 处理特殊键
                switch (e.key) {
                    case ' ':
                        hotkey += 'Space';
                        break;
                    case 'Escape':
                        hotkey += 'Esc';
                        break;
                    case 'Control':
                    case 'Shift':
                    case 'Alt':
                        // 仅修饰键不处理
                        break;
                    default:
                        hotkey += e.key;
                }
                
                // 只有当有实际按键时才更新
                if (hotkey && !['Control', 'Shift', 'Alt'].includes(e.key)) {
                    hotkeyInput.value = hotkey;
                    document.removeEventListener('keydown', handleKeyDown);
                }
            };
            
            document.addEventListener('keydown', handleKeyDown);
            
            // 5秒后自动移除监听器
            setTimeout(() => {
                document.removeEventListener('keydown', handleKeyDown);
                if (hotkeyInput.value === '请按键...') {
                    // 恢复默认值或之前的值
                    hotkeyInput.value = 'Ctrl+Alt+Space';
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
                await invoke('apply_settings', { hotkey, uiaMode });
                await message('设置已保存并已重启服务', { title: 'Prompt Manager', type: 'info' });
                // 刷新服务状态
                checkServiceStatus();
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
            const confirmed = await confirm('确定要重置所有设置吗？', {
                title: 'Prompt Manager',
                type: 'warning'
            });
            
            if (confirmed) {
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
                await message('设置已重置', { title: 'Prompt Manager', type: 'info' });
                // 这里应该实现实际的重置逻辑
            }
        });
    } else {
        console.error('无法找到reset-settings-btn元素');
    }
    
    // 加载提示词列表
    async function loadPrompts() {
        try {
            const prompts = await invoke('get_all_prompts');
            renderPrompts(prompts);
        } catch (error) {
            console.error('加载提示词列表失败:', error);
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
            } else {
                tagsHtml = '<span class="tag">无标签</span>';
            }
            
            promptItem.innerHTML = `
                <div class="prompt-info">
                    <h3>${prompt.name}</h3>
                    <p class="prompt-content">${prompt.content}</p>
                    <div class="prompt-meta">
                        ${tagsHtml}
                        <span class="last-used">ID: ${prompt.id}</span>
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
    
    // 定期检查服务状态
    setInterval(checkServiceStatus, 5000);
});