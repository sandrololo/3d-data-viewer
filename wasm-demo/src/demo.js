/**
 * 3D Data Viewer - WebAssembly Demo
 * 
 * This demo initializes and runs the Rust-based 3D data viewer
 * compiled to WebAssembly using wgpu and wasm-bindgen.
 */

import init, {
    viewer_back_to_origin,
    viewer_set_amplitude_shader,
    viewer_set_height_shader,
    viewer_set_overlays,
    viewer_clear_overlays
} from './assets/wasm/data-viewer-3d.js';

// DOM Elements
const loadingOverlay = document.getElementById('loading-overlay');
const errorOverlay = document.getElementById('error-overlay');
const errorMessage = document.getElementById('error-message');
const statusWebGPU = document.getElementById('status-webgpu');
const statusWasm = document.getElementById('status-wasm');
const statusFps = document.getElementById('status-fps');

// Control buttons
const btnHeight = document.getElementById('btn-height');
const btnAmplitude = document.getElementById('btn-amplitude');
const btnReset = document.getElementById('btn-reset');
const btnSetOverlay = document.getElementById('btn-set-overlay');
const btnClearOverlay = document.getElementById('btn-clear-overlay');

// State
let wasmModule = null;
let isHeightMode = true;

/**
 * Check if WebGPU is available
 */
async function checkWebGPU() {
    if (!navigator.gpu) {
        return { available: false, reason: 'WebGPU is not supported in this browser.' };
    }

    try {
        const adapter = await navigator.gpu.requestAdapter();
        if (!adapter) {
            return { available: false, reason: 'No WebGPU adapter found. Your GPU may not support WebGPU.' };
        }

        // Get adapter info - handle different API versions
        let info = null;
        try {
            // Try the newer API first
            if (typeof adapter.requestAdapterInfo === 'function') {
                info = await adapter.requestAdapterInfo();
            } else if (adapter.info) {
                // Fallback to sync property if available
                info = adapter.info;
            }
        } catch (infoErr) {
            // Adapter info is optional, continue without it
            console.log('Could not get adapter info:', infoErr);
        }

        return {
            available: true,
            adapter: adapter,
            info: info
        };
    } catch (e) {
        return { available: false, reason: `WebGPU error: ${e.message}` };
    }
}

/**
 * Update loading status text
 */
function updateLoadingText(text) {
    const loadingText = loadingOverlay.querySelector('.loading-text');
    if (loadingText) {
        loadingText.textContent = text;
    }
}

/**
 * Show error state
 */
function showError(message) {
    loadingOverlay.classList.add('hidden');
    errorOverlay.classList.add('visible');
    errorMessage.textContent = message;
    statusWebGPU.textContent = 'Error';
    statusWebGPU.classList.add('error');
}

/**
 * Hide loading overlay
 */
function hideLoading() {
    loadingOverlay.classList.add('hidden');
}

/**
 * Initialize the WASM module
 * The run_web() function is called automatically via #[wasm_bindgen(start)]
 */
async function initWasm() {
    updateLoadingText('Loading WebAssembly module...');
    statusWasm.textContent = 'Loading...';

    try {
        // Initialize the WASM module - run_web() is called automatically
        // via the #[wasm_bindgen(start)] attribute in Rust
        wasmModule = await init();

        statusWasm.textContent = 'Ready';
        statusWasm.classList.add('success');

        console.log('WASM module initialized, run_web() executed:', wasmModule);
        return true;
    } catch (e) {
        // winit uses exceptions for control flow on web - this is expected!
        if (e.message && e.message.includes("Using exceptions for control flow")) {
            console.log('WASM event loop started (this is normal)');
            statusWasm.textContent = 'Running';
            statusWasm.classList.add('success');
            return true;
        }

        console.error('Failed to load WASM:', e);
        statusWasm.textContent = 'Failed';
        statusWasm.classList.add('error');
        showError(`Failed to load WebAssembly module: ${e.message}`);
        return false;
    }
}

/**
 * Set up button event handlers
 */
function setupControls() {
    // Shader mode buttons - call Rust functions directly
    btnHeight.addEventListener('click', () => {
        if (!isHeightMode) {
            isHeightMode = true;
            btnHeight.classList.add('active');
            btnAmplitude.classList.remove('active');
            viewer_set_height_shader();
        }
    });

    btnAmplitude.addEventListener('click', () => {
        if (isHeightMode) {
            isHeightMode = false;
            btnAmplitude.classList.add('active');
            btnHeight.classList.remove('active');
            viewer_set_amplitude_shader();
        }
    });

    // Reset view - call Rust function directly
    btnReset.addEventListener('click', () => {
        viewer_back_to_origin();
    });

    // Set overlay - call Rust function directly
    btnSetOverlay.addEventListener('click', () => {
        viewer_set_overlays();
    });

    // Clear overlay - call Rust function directly
    btnClearOverlay.addEventListener('click', () => {
        viewer_clear_overlays();
    });
}

/**
 * FPS counter
 */
function setupFpsCounter() {
    let frameCount = 0;
    let lastTime = performance.now();

    function updateFps() {
        frameCount++;
        const now = performance.now();
        const delta = now - lastTime;

        if (delta >= 1000) {
            const fps = Math.round((frameCount * 1000) / delta);
            statusFps.textContent = `${fps}`;
            frameCount = 0;
            lastTime = now;
        }

        requestAnimationFrame(updateFps);
    }

    requestAnimationFrame(updateFps);
}

/**
 * Main initialization
 */
async function main() {
    console.log('🚀 Starting 3D Data Viewer Demo');

    // Check WebGPU support
    updateLoadingText('Checking WebGPU support...');
    const gpuCheck = await checkWebGPU();

    if (!gpuCheck.available) {
        showError(gpuCheck.reason);
        return;
    }

    statusWebGPU.textContent = 'Available';
    statusWebGPU.classList.add('success');
    console.log('✅ WebGPU available:', gpuCheck.info);

    // Initialize WASM
    const wasmReady = await initWasm();

    if (!wasmReady) {
        return;
    }

    // Set up controls
    setupControls();

    // Start FPS counter
    setupFpsCounter();

    // Hide loading overlay
    updateLoadingText('Starting renderer...');

    // Small delay to allow WASM to initialize rendering
    setTimeout(() => {
        hideLoading();
        console.log('✅ 3D Data Viewer ready!');
    }, 500);
}

// Handle canvas resize
function handleResize() {
    const canvas = document.getElementById('canvas');
    const container = canvas.parentElement;

    if (canvas && container) {
        canvas.width = container.clientWidth;
        canvas.height = container.clientHeight;
    }
}

// Set up resize observer
const resizeObserver = new ResizeObserver(handleResize);
const canvasContainer = document.querySelector('.canvas-container');
if (canvasContainer) {
    resizeObserver.observe(canvasContainer);
}

// Initial resize
handleResize();

// Start the application
main().catch(e => {
    console.error('Fatal error:', e);
    showError(`An unexpected error occurred: ${e.message}`);
});

