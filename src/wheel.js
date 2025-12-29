// TW011: PromptWheel JavaScript Logic
// Handles data loading, keyboard navigation, and interaction

const { invoke } = window.__TAURI__.core;

// State
let currentPage = 0;
let totalPages = 1;
let prompts = [];
const PER_PAGE = 6;

// DOM Elements
const petals = document.querySelectorAll('.petal');
const centerText = document.querySelector('.center-text');
const pageIndicator = document.getElementById('page-indicator');
const prevBtn = document.getElementById('prev-page');
const nextBtn = document.getElementById('next-page');

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    loadPrompts();
    setupEventListeners();
});

// Load prompts from backend
async function loadPrompts() {
    try {
        const result = await invoke('get_top_prompts_paginated', {
            page: currentPage,
            perPage: PER_PAGE
        });

        prompts = result.prompts;
        totalPages = result.total_pages;
        
        renderPrompts();
        updatePaginationUI();
        
        console.log(`Loaded ${prompts.length} prompts (Page ${currentPage + 1}/${totalPages})`);
    } catch (error) {
        console.error('Failed to load prompts:', error);
        centerText.textContent = 'Error loading prompts';
    }
}

// Render prompts into petals
function renderPrompts() {
    petals.forEach((petal, index) => {
        const prompt = prompts[index];
        const nameSpan = petal.querySelector('.petal-name');
        
        if (prompt) {
            nameSpan.textContent = prompt.name;
            petal.dataset.promptId = prompt.id;
            petal.dataset.promptName = prompt.name;
            petal.dataset.promptContent = prompt.content;
            petal.style.opacity = '1';
            petal.style.pointerEvents = 'auto';
        } else {
            // Empty petal
            nameSpan.textContent = '-';
            petal.dataset.promptId = '';
            petal.style.opacity = '0.3';
            petal.style.pointerEvents = 'none';
        }
    });
}

// Update pagination UI
function updatePaginationUI() {
    pageIndicator.textContent = `Page ${currentPage + 1} / ${Math.max(totalPages, 1)}`;
    prevBtn.disabled = currentPage === 0;
    nextBtn.disabled = currentPage >= totalPages - 1 || totalPages === 0;
}

// Setup event listeners
function setupEventListeners() {
    // Petal click
    petals.forEach((petal, index) => {
        petal.addEventListener('click', () => handlePetalClick(index));
        
        // Hover: show full name in center
        petal.addEventListener('mouseenter', () => {
            const name = petal.dataset.promptName;
            if (name && name !== '-') {
                centerText.textContent = name;
            }
        });
        
        petal.addEventListener('mouseleave', () => {
            centerText.textContent = 'PromptWheel';
        });
    });
    
    // Pagination buttons
    prevBtn.addEventListener('click', () => {
        if (currentPage > 0) {
            currentPage--;
            loadPrompts();
        }
    });
    
    nextBtn.addEventListener('click', () => {
        if (currentPage < totalPages - 1) {
            currentPage++;
            loadPrompts();
        }
    });
    
    // Keyboard navigation
    document.addEventListener('keydown', handleKeyPress);
}

// Handle petal click
async function handlePetalClick(index) {
    const prompt = prompts[index];
    if (!prompt) return;
    
    console.log(`Petal ${index + 1} clicked: ${prompt.name} (ID: ${prompt.id})`);
    
    try {
        // Trigger injection
        await invoke('trigger_wheel_injection', { promptId: prompt.id });
        console.log(`✅ Injection triggered for prompt ID ${prompt.id}`);
        
        // Visual feedback
        highlightPetal(index);
        
        // TW014: Explicitly hide window after selection for better UX
        const { getCurrentWindow } = window.__TAURI__.window;
        await getCurrentWindow().hide();
    } catch (error) {
        console.error('Failed to trigger injection:', error);
        alert(`Injection failed: ${error}`);
    }
}

// Visual feedback for selection
function highlightPetal(index) {
    petals.forEach((p, i) => {
        if (i === index) {
            p.classList.add('active');
            setTimeout(() => p.classList.remove('active'), 500);
        }
    });
}

// Keyboard shortcuts
function handleKeyPress(event) {
    const key = event.key;
    
    // Number keys 1-6 for petal selection
    if (key >= '1' && key <= '6') {
        const index = parseInt(key) - 1;
        handlePetalClick(index);
        return;
    }
    
    // PageUp/PageDown for pagination
    if (key === 'PageUp' && currentPage > 0) {
        event.preventDefault();
        currentPage--;
        loadPrompts();
        return;
    }
    
    if (key === 'PageDown' && currentPage < totalPages - 1) {
        event.preventDefault();
        currentPage++;
        loadPrompts();
        return;
    }
    
    // Escape to close (optional, window config will handle this)
    if (key === 'Escape') {
        console.log('Escape pressed, window should close');
        // Window will auto-hide on blur (configured in Tauri)
    }
}

// Export for potential external use
window.wheelApp = {
    loadPrompts,
    currentPage,
    prompts
};
