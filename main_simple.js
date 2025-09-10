
async function submitPrompt() {
    const name = document.getElementById('prompt-name')?.value?.trim();
    const content = document.getElementById('prompt-content')?.value?.trim();
    const tagsInput = document.getElementById('prompt-tags')?.value?.trim();

    if (!name || !content) {
        alert('请填写提示词名称和内容');
        return;
    }

    // 将标签字符串转换为数组
    const tags = tagsInput ? tagsInput.split(',').map(tag => tag.trim()).filter(tag => tag) : [];

    const newPrompt = {
        name,
        content,
        tags,
    };

    try {
        // 调用 Tauri 后端创建提示词
        const result = await safeInvoke('create_prompt', { prompt: newPrompt });
        updateDebugInfo(`创建提示词成功: ${result}`);
        alert('提示词已成功添加！');
        
        // 关闭模态框并刷新提示词列表
        closeAddPromptModal();
        loadPrompts();
    } catch (error) {
        updateDebugInfo(`创建提示词失败: ${error}`);
        alert(`创建提示词失败: ${error}`);
    }
}
