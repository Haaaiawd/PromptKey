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
// Setup event listeners
function setupEventListeners() {
    // Petal click (Target the clickable inner area)
    petals.forEach((petal, index) => {
        const petalInner = petal.querySelector('.petal-inner');
        
        // Handle click on the visual wedge
        if (petalInner) {
            petalInner.addEventListener('click', () => handlePetalClick(index));
            
            // Hover effects for center text
            petalInner.addEventListener('mouseenter', () => {
                const name = petal.dataset.promptName;
                if (name && name !== '-') {
                    centerText.textContent = name;
                }
            });
            
            petalInner.addEventListener('mouseleave', () => {
                centerText.textContent = 'PromptWheel';
            });
        }
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
    const petal = petals[index];
    const inner = petal.querySelector('.petal-inner');
    if (inner) {
        inner.style.background = 'var(--active-color, #00ffc8)'; // Fallback/Dynamic
        setTimeout(() => inner.style.background = '', 300);
    }
}

// Keyboard shortcuts
// Keyboard shortcuts
function handleKeyPress(event) {
    const key = event.key;
    const code = event.code;
    
    // Number keys 1-6 (Row and Numpad)
    // Check both key value and code for robustness
    let index = -1;
    
    if (key >= '1' && key <= '6') {
        index = parseInt(key) - 1;
    } else if (code.startsWith('Numpad') && code.length === 7) {
        const num = parseInt(code[6]);
        if (num >= 1 && num <= 6) {
            index = num - 1;
        }
    }

    if (index !== -1) {
        handlePetalClick(index);
        return;
    }
    
    // PageUp/PageDown for pagination
    if ((key === 'PageUp' || key === 'ArrowLeft') && currentPage > 0) {
        event.preventDefault();
        currentPage--;
        loadPrompts();
        return;
    }
    
    if ((key === 'PageDown' || key === 'ArrowRight') && currentPage < totalPages - 1) {
        event.preventDefault();
        currentPage++;
        loadPrompts();
        return;
    }
    
    // Escape to close
    if (key === 'Escape') {
        window.__TAURI__.window.getCurrentWindow().hide();
    }
}

// Export for potential external use
window.wheelApp = {
    loadPrompts,
    currentPage,
    prompts
};
