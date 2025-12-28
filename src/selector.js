/**
 * Quick Selector Panel - Frontend Logic
 * Implements fuzzy search, keyboard navigation, and prompt selection
 */

// Import Tauri APIs
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// Global state
let allPrompts = [];
let fuseInstance = null;
let currentFocus = 0;
let lastQuery = '';

// DOM elements (initialized after DOM loads)
let searchInput;
let resultsContainer;
let statsContent;

/**
 * T1-015: Data Loading Module
 * Initialize the selector panel with Prompts data and Fuse.js search
 */
async function init() {
    console.log('[Selector] Initializing...');
    
    // Get DOM elements
    searchInput = document.getElementById('search-box');
    resultsContainer = document.getElementById('results-container');
    statsContent = document.getElementById('stats-content');
    
    try {
        // 1. Load Prompts data
        allPrompts = await invoke('get_all_prompts_for_selector');
        console.log(`[Selector] Loaded ${allPrompts.length} prompts`);
        
        // 2. Initialize Fuse.js for fuzzy search
        const fuseOptions = {
            keys: [
                { name: 'name', weight: 0.6 },
                { name: 'tags', weight: 0.3 },
                { name: 'category', weight: 0.1 }
            ],
            threshold: 0.3,          // 30% similarity to match
            includeScore: true,
            minMatchCharLength: 1,
            ignoreLocation: true,    // Search anywhere in the string
        };
        fuseInstance = new Fuse(allPrompts, fuseOptions);
        
        // 3. Load statistics data
        try {
            const stats = await invoke('get_selector_stats');
            renderStats(stats);
        } catch (e) {
            console.warn('[Selector] Failed to load stats:', e);
            statsContent.textContent = '暂无数据';
        }
        
        // 4. Initial display: Top 10 by usage
        const initialResults = sortByUsage(allPrompts).slice(0, 10);
        renderResults(initialResults);
        
        // 5. Register event listeners
        searchInput.addEventListener('input', handleSearch);
        document.addEventListener('keydown', handleKeyboard);
        
        // 6. Listen for reset-state event from Tauri backend
        await listen('reset-state', resetUI);
        
        console.log('[Selector] ✅ Initialization complete');
        
    } catch (error) {
        console.error('[Selector] Init failed:', error);
        showError('无法加载Prompts,请重启应用');
    }
}

/**
 * T1-016: Search Logic
 * Handle search input with Fuse.js filtering and PRD-compliant sorting
 */
function handleSearch(e) {
    const query = e.target.value.trim();
    lastQuery = query;
    
    let results;
    
    if (query === '') {
        // Empty query: show Top 10 by usage
        results = sortByUsage(allPrompts).slice(0, 10);
    } else {
        // Fuzzy search with Fuse.js
        const fuseResults = fuseInstance.search(query);
        
        // Apply PRD sorting: relevance → recency → id
        results = fuseResults
            .map(r => ({
                ...r.item,
                _score: r.score
            }))
            .sort((a, b) => {
                // 1. Primary sort: relevance score (lower is better for Fuse.js)
                if (Math.abs(a._score - b._score) > 0.01) {
                    return a._score - b._score;
                }
                // 2. Secondary sort: last used time (newer first)
                const timeA = a.last_used_at || 0;
                const timeB = b.last_used_at || 0;
                if (timeA !== timeB) {
                    return timeB - timeA; // descending
                }
                // 3. Fallback sort: id (ascending)
                return a.id - b.id;
            })
            .slice(0, 10); // Top 10 results
    }
    
    renderResults(results);
    
    // Reset focus to first item
    currentFocus = 0;
    updateFocusStyle();
}

/**
 * T1-017: Keyboard Navigation
 * Handle ↑↓Enter ESC with focus cycling
 */
function handleKeyboard(e) {
    const results = document.querySelectorAll('.result-item');
    
    if (results.length === 0) return;
    
    switch(e.key) {
        case 'ArrowDown':
            e.preventDefault();
            currentFocus = (currentFocus + 1) % results.length; // Wrap around
            updateFocusStyle();
            break;
        
        case 'ArrowUp':
            e.preventDefault();
            currentFocus = (currentFocus - 1 + results.length) % results.length;
            updateFocusStyle();
            break;
        
        case 'Enter':
            e.preventDefault();
            const focusedItem = results[currentFocus];
            if (focusedItem) {
                const promptId = parseInt(focusedItem.dataset.id);
                selectPrompt(promptId);
            }
            break;
        
        case 'Escape':
            e.preventDefault();
            hideWindow();
            break;
    }
}

/**
 * Update visual focus indicator
 */
function updateFocusStyle() {
    const results = document.querySelectorAll('.result-item');
    results.forEach((item, index) => {
        if (index === currentFocus) {
            item.classList.add('focused');
            item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        } else {
            item.classList.remove('focused');
        }
    });
}

/**
 * T1-018: Clipboard & Logging
 * Copy selected prompt to clipboard and log usage
 */
async function selectPrompt(promptId) {
    const prompt = allPrompts.find(p => p.id === promptId);
    if (!prompt) {
        console.error('[Selector] Prompt not found:', promptId);
        return;
    }
    
    try {
        // 1. Copy to clipboard (with fallback)
        try {
            // Try Web Clipboard API first
            await navigator.clipboard.writeText(prompt.content);
        } catch (clipError) {
            console.warn('[Selector] Web Clipboard failed, trying Tauri plugin:', clipError);
            // Fallback to Tauri clipboard plugin (if available)
            if (window.__TAURI_PLUGIN_CLIPBOARD__) {
                const { writeText } = window.__TAURI_PLUGIN_CLIPBOARD__;
                await writeText(prompt.content);
            } else {
                throw new Error('No clipboard method available');
            }
        }
        console.log(`[Selector] ✅ Copied prompt ${prompt.id} to clipboard`);
        
        // 2. Log usage event
        await invoke('log_selector_usage', {
            promptId: prompt.id,
            promptName: prompt.name,
            query: lastQuery || null
        });
        console.log(`[Selector] ✅ Logged usage for prompt ${prompt.id}`);
        
        // 3. Hide window (auto-close after selection)
        hideWindow();
        
    } catch (error) {
        console.error('[Selector] Select failed:', error);
        showToast('复制失败,请重试');
    }
}

/**
 * Hide the selector window
 */
async function hideWindow() {
    try {
        const { getCurrent } = window.__TAURI__.window;
       const currentWindow = getCurrent();
        await currentWindow.hide();
        console.log('[Selector] Window hidden');
    } catch (e) {
        console.error('[Selector] Failed to hide window:', e);
    }
}

/**
 * T1-019: DOM Rendering
 * Render results list with prompt details
 */
function renderResults(prompts) {
    if (prompts.length === 0) {
        resultsContainer.innerHTML = `
            <div class="empty-state">
                <div class="empty-state-icon">🔍</div>
                <div class="empty-state-text">未找到匹配的Prompts</div>
            </div>
        `;
        return;
    }
    
    resultsContainer.innerHTML = prompts.map((prompt, index) => {
        const preview = prompt.content.substring(0, 50) + (prompt.content.length > 50 ? '...' : '');
        const tags = prompt.tags || [];
        const category = prompt.category || '';
        const usageCount = prompt.usage_count || 0;
        const lastUsed = prompt.last_used_at ? formatTimestamp(prompt.last_used_at) : '';
        
        return `
            <div class="result-item ${index === 0 ? 'focused' : ''}" 
                 data-id="${prompt.id}"
                 onclick="selectPrompt(${prompt.id})">
                <div class="result-name">${escapeHtml(prompt.name)}</div>
                <div class="result-preview">${escapeHtml(preview)}</div>
                <div class="result-meta">
                    ${category ? `<span class="tag category-tag">${escapeHtml(category)}</span>` : ''}
                    ${tags.slice(1).map(tag => `<span class="tag">${escapeHtml(tag)}</span>`).join('')}
                    ${usageCount > 0 ? `<span class="usage-info">使用 ${usageCount} 次${lastUsed ? ` · ${lastUsed}` : ''}</span>` : ''}
                </div>
            </div>
        `;
    }).join('');
    
    // Reset focus to first item
    currentFocus = 0;
    updateFocusStyle();
}

/**
 * T1-019: Stats Bar Rendering
 * Display Top 2 most-used prompts
 */
function renderStats(stats) {
    if (!stats || !stats.top_prompts || stats.top_prompts.length === 0) {
        statsContent.textContent = '暂无数据';
        return;
    }
    
    const statItems = stats.top_prompts.map(item => 
        `<span class="stat-item">
            <span class="stat-name">${escapeHtml(item.name)}</span>: 
            <span class="stat-count">${item.usage_count}次</span>
        </span>`
    ).join('');
    
    statsContent.innerHTML = statItems;
}

/**
 * Reset UI state (called on window show)
 */
function resetUI() {
    console.log('[Selector] Resetting UI state');
    searchInput.value = '';
    lastQuery = '';
    currentFocus = 0;
    
    // Re-render initial state
    const initialResults = sortByUsage(allPrompts).slice(0, 10);
    renderResults(initialResults);
    
    // Focus on search box
    searchInput.focus();
}

/**
 * Utility: Sort prompts by usage count (descending)
 */
function sortByUsage(prompts) {
    return [...prompts].sort((a, b) => {
        // Primary: usage count (descending)
        if (b.usage_count !== a.usage_count) {
            return b.usage_count - a.usage_count;
        }
        // Secondary: last used time (newer first)
        const timeA = a.last_used_at || 0;
        const timeB = b.last_used_at || 0;
        if (timeB !== timeA) {
            return timeB - timeA;
        }
        // Fallback: id (ascending)
        return a.id - b.id;
    });
}

/**
 * Utility: Format Unix timestamp to relative time
 */
function formatTimestamp(ms) {
    const now = Date.now();
    const diff = now - ms;
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    
    if (days > 0) return `${days}天前`;
    if (hours > 0) return `${hours}小时前`;
    if (minutes > 0) return `${minutes}分钟前`;
    return '刚刚';
}

/**
 * Utility: Escape HTML to prevent XSS
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Utility: Show error message
 */
function showError(message) {
    resultsContainer.innerHTML = `
        <div class="empty-state">
            <div class="empty-state-icon">⚠️</div>
            <div class="empty-state-text">${escapeHtml(message)}</div>
        </div>
    `;
}

/**
 * Utility: Show toast notification (simple version)
 */
function showToast(message) {
    // Simple console.error for now
    // TODO: Implement proper toast UI
    console.error('[Toast]', message);
    alert(message);
}

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', init);
