/**
 * 3D Data Viewer - WebAssembly Demo
 * 
 * This demo initializes and runs the Rust-based 3D data viewer
 * compiled to WebAssembly using wgpu and wasm-bindgen.
 */

import init, {
    WasmViewer
} from './assets/wasm/data-viewer-3d.js';

// DOM Elements
const loadingOverlay = document.getElementById('loading-overlay');
const errorOverlay = document.getElementById('error-overlay');
const errorMessage = document.getElementById('error-message');
const statusWebGPU = document.getElementById('status-webgpu');
const statusWasm = document.getElementById('status-wasm');
const statusFps = document.getElementById('status-fps');
const pixelX = document.getElementById('pixel-x');
const pixelY = document.getElementById('pixel-y');
const pixelZ = document.getElementById('pixel-z');

// Control buttons
const btnHeight = document.getElementById('btn-height');
const btnAmplitude = document.getElementById('btn-amplitude');
const btnReset = document.getElementById('btn-reset');
const btnSetOverlay = document.getElementById('btn-set-overlay');
const btnClearOverlay = document.getElementById('btn-clear-overlay');

// State
let wasmModule = null;
let wasmViewer = null;
let isHeightMode = true;
let isPollingEnabled = false;
let isPolling = false;

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
 * Load surface data from assets/data/surface.tiff
 */
async function loadSurfaceData() {
    try {
        console.log('Loading surface.tiff...');
        const response = await fetch('./src/assets/data/surface.tiff');

        if (!response.ok) {
            throw new Error(`Failed to load surface.tiff: ${response.status} ${response.statusText}`);
        }

        const arrayBuffer = await response.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        console.log('Surface data loaded:', uint8Array.length, 'bytes');
        return uint8Array;
    } catch (error) {
        console.error('Error loading surface data:', error);
        throw error;
    }
}

async function loadAmplitudeData() {
    try {
        console.log('Loading amplitude.tiff...');
        const response = await fetch('./src/assets/data/amplitude.tiff');

        if (!response.ok) {
            throw new Error(`Failed to load amplitude.tiff: ${response.status} ${response.statusText}`);
        }

        const arrayBuffer = await response.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        console.log('Amplitude data loaded:', uint8Array.length, 'bytes');
        return uint8Array;
    } catch (error) {
        console.error('Error loading amplitude data:', error);
        throw error;
    }
}

/**
 * Update the pixel readout in the UI
 */
function renderPixelReadout(x, y, z) {
    if (!pixelX || !pixelY || !pixelZ) {
        return;
    }

    const formatValue = (value, roundToInt, decimals) => {
        if (!Number.isFinite(value)) {
            return '--';
        }
        if (roundToInt) {
            return Math.round(value).toString();
        }
        return decimals !== undefined ? value.toFixed(decimals) : value.toString();
    };

    pixelX.textContent = formatValue(x, true);
    pixelY.textContent = formatValue(y, true);
    pixelZ.textContent = formatValue(z, false, 2);
}

/**
 * Initialize the WASM module and create the viewer
 */
async function initWasm() {
    updateLoadingText('Loading WebAssembly module...');
    statusWasm.textContent = 'Loading...';

    try {
        // Initialize the WASM module and create the viewer instance
        console.log('Calling init()...');
        wasmModule = await init();
        wasmViewer = WasmViewer.new();
        wasmViewer.run()
        console.log('init() completed successfully');
        console.log('wasmModule after init:', !!wasmModule);

        statusWasm.textContent = 'Ready';
        statusWasm.classList.add('success');

        console.log('WASM module initialized:', wasmModule);
        return true;
    } catch (e) {
        console.log('Exception caught during init:', e.message);

        // winit uses exceptions for control flow on web when starting the event loop
        if (e.message && e.message.includes("Using exceptions for control flow")) {
            console.log('WASM viewer started (this is normal - winit uses exceptions for control flow)');
            console.log('wasmModule after exception:', !!wasmModule);
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
    // Shader mode buttons - call viewer methods directly
    btnHeight.addEventListener('click', () => {
        if (!isHeightMode && wasmViewer) {
            isHeightMode = true;
            btnHeight.classList.add('active');
            btnAmplitude.classList.remove('active');
            wasmViewer.set_height_shader();
        }
    });

    btnAmplitude.addEventListener('click', () => {
        if (isHeightMode && wasmViewer) {
            isHeightMode = false;
            btnAmplitude.classList.add('active');
            btnHeight.classList.remove('active');
            wasmViewer.set_amplitude_shader();
        }
    });

    // Reset view - call viewer method directly
    btnReset.addEventListener('click', () => {
        if (wasmViewer) {
            wasmViewer.back_to_origin();
        }
    });

    // Set overlay - call viewer method directly
    btnSetOverlay.addEventListener('click', () => {
        if (wasmViewer) {
            wasmViewer.set_overlays();
        }
    });

    // Clear overlay - call viewer method directly
    btnClearOverlay.addEventListener('click', () => {
        if (wasmViewer) {
            wasmViewer.clear_overlays();
        }
    });

    // Set up mouse movement tracking
    const canvas = document.getElementById('canvas');
    if (canvas) {
        console.log('Setting up canvas mouse tracking');
        canvas.addEventListener('mousemove', () => {
            if (wasmViewer) {
                isPollingEnabled = true;
                startPixelPolling();
            }
        });

        canvas.addEventListener('mouseleave', () => {
            console.log('Canvas mouseleave');
            isPollingEnabled = false;
        });
    } else {
        console.warn('Canvas element not found');
    }
}

/**
 * Continuously poll the current pixel from WASM and update the panel
 */
function startPixelPolling() {
    // Prevent multiple polling loops
    if (isPolling) {
        return;
    }

    console.log('Starting pixel polling');
    isPolling = true;

    async function pollOnce() {
        if (!wasmViewer) {
            console.log('Polling stopped - wasmViewer:', !!wasmViewer);
            isPolling = false;
            return;
        }

        try {
            const result = await wasmViewer.get_pixel_value();

            // Handle both Array and TypedArray (Float32Array, etc.)
            if (result && (Array.isArray(result) || ArrayBuffer.isView(result)) && result.length >= 3) {
                const x = result[0];
                const y = result[1];
                const z = result[2];
                renderPixelReadout(x, y, z);
            } else {
                console.log('Invalid result format:', result);
            }
        } catch (err) {
            console.error('Failed to fetch pixel (WASM error):', err);
        }

        // Continue polling
        setTimeout(pollOnce, 100);
    }

    pollOnce();
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

    setTimeout(async () => {
        // Load surface data and set it in WASM
        updateLoadingText('Loading surface data...');
        try {
            const surfaceData = await loadSurfaceData();
            const amplitudeData = await loadAmplitudeData();
            if (wasmViewer && typeof wasmViewer.set_surface === 'function') {
                wasmViewer.set_surface(surfaceData);
                wasmViewer.set_amplitude(amplitudeData);
                console.log('✅ Surface data set in WASM viewer');
                hideLoading();
            } else {
                console.warn('set_surface method not available on wasmViewer');
            }
        } catch (error) {
            console.error('Failed to load surface data:', error);
            // Continue without surface data rather than failing completely
        }
        console.log('✅ 3D Data Viewer ready!');
        console.log('wasmModule available:', !!wasmModule);
        console.log('wasmViewer available:', !!wasmViewer);

        // Start polling if wasmViewer is available
        if (wasmViewer) {
            console.log('Starting polling');
            isPollingEnabled = true;
            startPixelPolling();
        } else {
            console.warn('wasmViewer not available yet');
        }
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

