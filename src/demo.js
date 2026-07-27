/**
 * 3D Data Viewer - WebAssembly Demo
 * 
 * This demo initializes and runs the Rust-based 3D data viewer
 * compiled to WebAssembly using wgpu and wasm-bindgen.
 */

import init, {
    Orientation,
    Region,
    RegionColor,
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
const shaderSelect = document.getElementById('shader-select');
const btnReset = document.getElementById('btn-reset');
const btnSetOverlay = document.getElementById('btn-set-overlay');
const btnClearOverlay = document.getElementById('btn-clear-overlay');
const btnDownloadImage = document.getElementById('btn-download-image');
const btnToggleGrid = document.getElementById('btn-toggle-grid');

const DEFAULT_ORIENTATION = () =>
    Orientation.new(0.8, 0.0, 0.0, 70, 0, -45);

// State
let wasmModule = null;
let wasmViewer = null;
let currentShader = 'height';
let isOverlayVisible = false;
let isGridVisible = true;
let isPollingEnabled = false;
let isPolling = false;
let overlayDefinitions = null;
let imageWidth = 0;
let imageHeight = 0;

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
 * Parse image dimensions from a TIFF ArrayBuffer
 */
function parseTiffDimensions(buffer) {
    const view = new DataView(buffer);
    const littleEndian = view.getUint16(0) === 0x4949; // 'II' = little-endian
    const ifdOffset = view.getUint32(4, littleEndian);
    const numEntries = view.getUint16(ifdOffset, littleEndian);
    let w = 0, h = 0;
    for (let i = 0; i < numEntries; i++) {
        const entryOffset = ifdOffset + 2 + i * 12;
        const tag = view.getUint16(entryOffset, littleEndian);
        if (tag === 256) w = view.getUint32(entryOffset + 8, littleEndian);
        else if (tag === 257) h = view.getUint32(entryOffset + 8, littleEndian);
    }
    return { width: w, height: h };
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
    const updateOverlayButtons = (mode) => {
        const disabled = mode !== 'height';
        const title = disabled ? 'Only available in Height shader mode' : '';
        btnSetOverlay.disabled = disabled;
        btnClearOverlay.disabled = disabled;
        btnSetOverlay.title = title;
        btnClearOverlay.title = title;
    };

    const setShader = (mode) => {
        if (!wasmViewer || mode === currentShader) {
            return;
        }
        currentShader = mode;
        shaderSelect.value = mode;
        updateOverlayButtons(mode);
        switch (mode) {
            case 'height':
                wasmViewer.set_height_shader();
                break;
            case 'texture':
                wasmViewer.set_texture_shader();
                break;
            case 'turbo':
                wasmViewer.set_turbo_colormap_shader();
                break;
        }
    };

    const cycleShader = () => {
        if (!wasmViewer) {
            return;
        }
        const modes = ['height', 'texture', 'turbo'];
        const idx = modes.indexOf(currentShader);
        setShader(modes[(idx + 1) % modes.length]);
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

        try {
            wasmViewer.set_orientation(DEFAULT_ORIENTATION());
        } catch (error) {
            console.error('Failed to reset view:', error);
        }
    };

    // Shader mode dropdown
    shaderSelect.addEventListener('change', (event) => {
        setShader(event.target.value);
    });

    // Reset view - call viewer method directly
    btnReset.addEventListener('click', () => {
        resetView();
    });

    // Toggle grid visibility
    const toggleGrid = () => {
        if (!wasmViewer) return;
        isGridVisible = !isGridVisible;
        wasmViewer.display_grid(isGridVisible);
    };

    btnToggleGrid.addEventListener('click', () => {
        toggleGrid();
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
            cycleShader();
        } else if (key === 't') {
            event.preventDefault();
            void toggleOverlay();
        } else if (key === 'r') {
            event.preventDefault();
            resetView();
        } else if (key === 'g') {
            event.preventDefault();
            toggleGrid();
        } else if (event.key === '+' || event.key === '=') {
            event.preventDefault();
            wasmViewer.zoom_in();
        } else if (event.key === '-') {
            event.preventDefault();
            wasmViewer.zoom_out();
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
            // Silence "Pixel out of bounds" — this is the sentinel returned when the
            // cursor is not over valid geometry (x/y == u32::MAX).
            if (!String(err).includes('out of bounds')) {
                console.error('Failed to fetch pixel (WASM error):', err);
            }
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
            const dims = parseTiffDimensions(surfaceData.buffer);
            imageWidth = dims.width;
            imageHeight = dims.height;
            if (wasmViewer && typeof wasmViewer.set_topology === 'function') {
                await wasmViewer.set_topology(surfaceData);
                await wasmViewer.set_texture(amplitudeData);
                wasmViewer.set_texture_range(0, 100);
                wasmViewer.set_orientation(DEFAULT_ORIENTATION());
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

    // Parse, validate and sort raw ranges
    const valid = [];
    for (const range of ranges) {
        if (!Array.isArray(range) || range.length < 2) continue;
        const start = Number(range[0]);
        const end = Number(range[1]);
        if (!Number.isFinite(start) || !Number.isFinite(end) || start >= end) continue;
        valid.push([Math.trunc(start), Math.trunc(end)]);
    }
    if (valid.length === 0) return null;

    // Merge adjacent and overlapping ranges.
    // SortedRanges requires non-zero gaps between consecutive ranges, so
    // touching ranges (end of one == start of next) must be merged first.
    valid.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    const merged = [valid[0].slice()];
    for (let i = 1; i < valid.length; i++) {
        const last = merged[merged.length - 1];
        const [s, e] = valid[i];
        if (s <= last[1]) {
            last[1] = Math.max(last[1], e); // merge
        } else {
            merged.push([s, e]);
        }
    }

    const pixelRanges = merged.map(([s, e]) =>
        WasmBindgenPixelRange.new(BigInt(s), BigInt(e))
    );

    const [r, g, b, a] = colorRgba;
    const color = RegionColor.new(r, g, b, a);
    return Region.new(pixelRanges, color, imageWidth, imageHeight);
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
