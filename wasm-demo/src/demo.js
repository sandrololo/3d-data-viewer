/**
 * 3D Data Viewer - WebAssembly Demo
 * 
 * This demo initializes and runs the Rust-based 3D data viewer
 * compiled to WebAssembly using wgpu and wasm-bindgen.
 */

import init, {
    Overlay,
    OverlayColor,
    WasmBindgenPixelRange,
    WasmViewer
} from './assets/wasm/data-viewer-3d.js';

// DOM Elements
const loadingOverlay = document.getElementById('loading-overlay');
const errorOverlay = document.getElementById('error-overlay');
const errorMessage = document.getElementById('error-message');
const statusWebGPU = document.getElementById('status-webgpu');
const statusWasm = document.getElementById('status-wasm');
const pixelX = document.getElementById('pixel-x');
const pixelY = document.getElementById('pixel-y');
const pixelZ = document.getElementById('pixel-z');
const pixelA = document.getElementById('pixel-a');

// Control buttons
const btnHeight = document.getElementById('btn-height');
const btnAmplitude = document.getElementById('btn-amplitude');
const btnReset = document.getElementById('btn-reset');
const btnSetOverlay = document.getElementById('btn-set-overlay');
const btnClearOverlay = document.getElementById('btn-clear-overlay');
const btnDownloadImage = document.getElementById('btn-download-image');

// State
let wasmModule = null;
let wasmViewer = null;
let isHeightMode = true;
let isOverlayVisible = false;
let isPollingEnabled = false;
let isPolling = false;
let overlayDefinitions = null;

const OVERLAY_DATA_PATH = './src/assets/data/overlay.json';

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
 * Download current canvas image
 */
async function downloadCurrentImage() {
    if (wasmViewer && typeof wasmViewer.capture_image === 'function') {
        try {
            const bytes = await wasmViewer.capture_image();
            const bytesCopy = new Uint8Array(bytes);
            const blob = new Blob([bytesCopy], { type: 'image/png' });
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'capture.png';
            a.click();
            URL.revokeObjectURL(url);
            return;
        } catch (error) {
            console.error('Failed to capture image via WASM capture_image:', error);
        }
    }

    const canvas = document.getElementById('canvas');

    if (!canvas) {
        console.warn('Canvas element not found');
        return;
    }

    const filename = `3d-viewer-${new Date().toISOString().replace(/[:.]/g, '-')}.png`;

    canvas.toBlob((blob) => {
        if (!blob) {
            console.error('Failed to export canvas image');
            return;
        }

        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        link.remove();
        URL.revokeObjectURL(url);
    }, 'image/png');
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
function renderPixelReadout(x, y, z, amplitude) {
    if (!pixelX || !pixelY || !pixelZ || !pixelA) {
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
    pixelA.textContent = formatValue(amplitude, true);
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
        wasmViewer = WasmViewer.new('canvas');
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
    const setHeightShader = () => {
        if (wasmViewer && !isHeightMode) {
            isHeightMode = true;
            btnHeight.classList.add('active');
            btnAmplitude.classList.remove('active');
            wasmViewer.set_height_shader();
        }
    };

    const setAmplitudeShader = () => {
        if (wasmViewer && isHeightMode) {
            isHeightMode = false;
            btnAmplitude.classList.add('active');
            btnHeight.classList.remove('active');
            wasmViewer.set_texture_shader();
        }
    };

    const toggleShader = () => {
        if (!wasmViewer) {
            return;
        }

        if (isHeightMode) {
            setAmplitudeShader();
        } else {
            setHeightShader();
        }
    };

    const setOverlay = async () => {
        if (wasmViewer) {
            const overlays = await example_overlays();
            if (overlays.length === 0) {
                console.warn('No overlays loaded from overlay.json');
                return;
            }

            wasmViewer.set_overlays(overlays);
            isOverlayVisible = true;
        }
    };

    const clearOverlay = () => {
        if (wasmViewer) {
            wasmViewer.clear_overlays();
            isOverlayVisible = false;
        }
    };

    const toggleOverlay = async () => {
        if (!wasmViewer) {
            return;
        }

        if (isOverlayVisible) {
            clearOverlay();
        } else {
            await setOverlay();
        }
    };

    const resetView = () => {
        if (!wasmViewer) {
            return;
        }

        if (typeof wasmViewer.reset_orientation === 'function') {
            try {
                wasmViewer.reset_orientation();
            } catch (error) {
                console.error('Failed to reset view:', error);
            }
        }
    };

    // Shader mode buttons - call viewer methods directly
    btnHeight.addEventListener('click', () => {
        setHeightShader();
    });

    btnAmplitude.addEventListener('click', () => {
        setAmplitudeShader();
    });

    // Reset view - call viewer method directly
    btnReset.addEventListener('click', () => {
        resetView();
    });

    // Set overlay - call viewer method directly
    btnSetOverlay.addEventListener('click', () => {
        void setOverlay();
    });

    // Clear overlay - call viewer method directly
    btnClearOverlay.addEventListener('click', () => {
        clearOverlay();
    });

    // Download current canvas image
    btnDownloadImage.addEventListener('click', () => {
        downloadCurrentImage();
    });

    // Keyboard shortcuts
    document.addEventListener('keydown', (event) => {
        if (event.repeat) {
            return;
        }

        const activeEl = document.activeElement;
        const tag = activeEl && activeEl.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA' || (activeEl && activeEl.isContentEditable)) {
            return;
        }

        const key = event.key.toLowerCase();
        if (key === 's') {
            event.preventDefault();
            toggleShader();
        } else if (key === 't') {
            event.preventDefault();
            void toggleOverlay();
        } else if (key === 'r') {
            event.preventDefault();
            resetView();
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

function toFiniteNumber(value) {
    if (typeof value === 'number') {
        return Number.isFinite(value) ? value : Number.NaN;
    }

    if (typeof value === 'bigint') {
        const converted = Number(value);
        return Number.isFinite(converted) ? converted : Number.NaN;
    }

    if (typeof value === 'string') {
        const trimmed = value.trim();
        if (!trimmed) {
            return Number.NaN;
        }
        const converted = Number(trimmed);
        return Number.isFinite(converted) ? converted : Number.NaN;
    }

    return Number.NaN;
}

function readNumericMember(source, key) {
    if (!source || typeof source !== 'object') {
        return Number.NaN;
    }

    const capitalizedKey = key.length > 0 ? `${key[0].toUpperCase()}${key.slice(1)}` : key;
    const candidates = [
        key,
        `get_${key}`,
        `get${capitalizedKey}`,
    ];

    for (const candidate of candidates) {
        let value;
        try {
            value = source[candidate];
        } catch (_) {
            continue;
        }

        if (typeof value === 'function') {
            try {
                value = value.call(source);
            } catch (_) {
                continue;
            }
        }

        const numeric = toFiniteNumber(value);
        if (Number.isFinite(numeric)) {
            return numeric;
        }
    }

    return Number.NaN;
}

function parsePixelResult(result) {
    if (result) {
        const x = readNumericMember(result, 'x')
        const y = readNumericMember(result, 'y')
        const z = readNumericMember(result, 'z')
        const amplitude = readNumericMember(result, 'texture')
        console.log(result)
        if (Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(z)) {
            return { x, y, z, amplitude };
        }
    }
    return null;
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

            const parsed = parsePixelResult(result);
            if (parsed) {
                renderPixelReadout(parsed.x, parsed.y, parsed.z, parsed.amplitude);
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

    // Hide loading overlay
    updateLoadingText('Starting renderer...');

    setTimeout(async () => {
        // Load surface data and set it in WASM
        updateLoadingText('Loading surface data...');
        try {
            const surfaceData = await loadSurfaceData();
            const amplitudeData = await loadAmplitudeData();
            if (wasmViewer && typeof wasmViewer.set_topology === 'function') {
                await wasmViewer.set_topology(surfaceData);
                await wasmViewer.set_texture(amplitudeData);
                wasmViewer.set_texture_range(0, 100);
                console.log('✅ Surface data set in WASM viewer');
            } else {
                console.warn('set_topology method not available on wasmViewer');
            }
        } catch (error) {
            console.error('Failed to load surface data:', error);
            // Continue without surface data rather than failing completely
        }
        hideLoading();
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

async function loadOverlayDefinitions() {
    if (overlayDefinitions) {
        return overlayDefinitions;
    }

    try {
        const response = await fetch(OVERLAY_DATA_PATH);
        if (!response.ok) {
            throw new Error(`Failed to load overlay data: ${response.status} ${response.statusText}`);
        }

        const parsed = await response.json();
        overlayDefinitions = parsed;
        return overlayDefinitions;
    } catch (error) {
        console.error('Failed to load overlay definitions:', error);
        return null;
    }
}

function buildOverlay(ranges, colorRgba) {
    if (!Array.isArray(ranges) || ranges.length === 0) {
        return null;
    }

    const pixelRanges = [];
    for (const range of ranges) {
        if (!Array.isArray(range) || range.length < 2) {
            continue;
        }

        const start = Number(range[0]);
        const end = Number(range[1]);
        if (!Number.isFinite(start) || !Number.isFinite(end)) {
            continue;
        }

        pixelRanges.push(WasmBindgenPixelRange.new(Math.trunc(start), Math.trunc(end)));
    }

    if (pixelRanges.length === 0) {
        return null;
    }

    const [r, g, b, a] = colorRgba;
    const color = OverlayColor.new(r, g, b, a);
    return Overlay.new(pixelRanges, color);
}

async function example_overlays() {
    const definitions = await loadOverlayDefinitions();
    if (!definitions || typeof definitions !== 'object') {
        return [];
    }

    const firstOverlay = buildOverlay(definitions.overlay1, [255, 64, 64, 220]);
    const secondOverlay = buildOverlay(definitions.overlay2, [64, 196, 64, 220]);

    return [firstOverlay, secondOverlay].filter(Boolean);
}
