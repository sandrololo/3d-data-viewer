/* tslint:disable */
/* eslint-disable */

export class Orientation {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static new(zoom: number, x: number, y: number, pitch: number, yaw: number, roll: number): Orientation;
}

export class PixelValue {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    texture: number;
    x: number;
    y: number;
    z: number;
}

export class Region {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static new(pixelrange: WasmBindgenPixelRange[], color: RegionColor, image_width: number, image_height: number): Region;
}

export class RegionColor {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static new(r: number, g: number, b: number, a: number): RegionColor;
}

export class WasmBindgenPixelRange {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static new(start: bigint, end: bigint): WasmBindgenPixelRange;
}

export class WasmViewer {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    capture_image(): Promise<Uint8Array>;
    clear_overlays(): void;
    display_grid(visible: boolean): void;
    get_pixel_value(): Promise<PixelValue>;
    static new(canvas_id: string): WasmViewer;
    reset_orientation(): void;
    reset_view(): Promise<void>;
    run(): void;
    set_height_shader(): void;
    set_mip_override(level?: number | null): void;
    set_orientation(orientation: Orientation): void;
    set_overlays(overlays: Region[]): void;
    set_percentile(percentile: number): void;
    set_texture(data: Uint8Array): Promise<void>;
    set_texture_range(start: number, end: number): void;
    set_texture_shader(): void;
    set_topology(data: Uint8Array): Promise<void>;
    /**
     * Sets a topology with a validity mask. The mask is a flat array of bytes (0=invalid, 1=valid)
     * with the same dimensions (width*height) as the topology image.
     * Invalid pixels will create holes in the rendered mesh.
     */
    set_topology_masked(data: Uint8Array, mask: Uint8Array): Promise<void>;
    set_turbo_colormap_shader(): void;
    zoom_in(): void;
    zoom_out(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly main: (a: number, b: number) => number;
    readonly __wbg_get_pixelvalue_texture: (a: number) => number;
    readonly __wbg_get_pixelvalue_x: (a: number) => number;
    readonly __wbg_get_pixelvalue_y: (a: number) => number;
    readonly __wbg_get_pixelvalue_z: (a: number) => number;
    readonly __wbg_orientation_free: (a: number, b: number) => void;
    readonly __wbg_pixelvalue_free: (a: number, b: number) => void;
    readonly __wbg_region_free: (a: number, b: number) => void;
    readonly __wbg_regioncolor_free: (a: number, b: number) => void;
    readonly __wbg_set_pixelvalue_texture: (a: number, b: number) => void;
    readonly __wbg_set_pixelvalue_x: (a: number, b: number) => void;
    readonly __wbg_set_pixelvalue_y: (a: number, b: number) => void;
    readonly __wbg_set_pixelvalue_z: (a: number, b: number) => void;
    readonly __wbg_wasmbindgenpixelrange_free: (a: number, b: number) => void;
    readonly __wbg_wasmviewer_free: (a: number, b: number) => void;
    readonly orientation_new: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly region_new: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly regioncolor_new: (a: number, b: number, c: number, d: number) => number;
    readonly wasmbindgenpixelrange_new: (a: bigint, b: bigint) => [number, number, number];
    readonly wasmviewer_capture_image: (a: number) => any;
    readonly wasmviewer_clear_overlays: (a: number) => [number, number];
    readonly wasmviewer_display_grid: (a: number, b: number) => [number, number];
    readonly wasmviewer_get_pixel_value: (a: number) => any;
    readonly wasmviewer_new: (a: number, b: number) => [number, number, number];
    readonly wasmviewer_reset_orientation: (a: number) => [number, number];
    readonly wasmviewer_reset_view: (a: number) => any;
    readonly wasmviewer_run: (a: number) => [number, number];
    readonly wasmviewer_set_height_shader: (a: number) => [number, number];
    readonly wasmviewer_set_mip_override: (a: number, b: number) => [number, number];
    readonly wasmviewer_set_orientation: (a: number, b: number) => [number, number];
    readonly wasmviewer_set_overlays: (a: number, b: number, c: number) => [number, number];
    readonly wasmviewer_set_percentile: (a: number, b: number) => [number, number];
    readonly wasmviewer_set_texture: (a: number, b: number, c: number) => any;
    readonly wasmviewer_set_texture_range: (a: number, b: number, c: number) => [number, number];
    readonly wasmviewer_set_texture_shader: (a: number) => [number, number];
    readonly wasmviewer_set_topology: (a: number, b: number, c: number) => any;
    readonly wasmviewer_set_topology_masked: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmviewer_set_turbo_colormap_shader: (a: number) => [number, number];
    readonly wasmviewer_zoom_in: (a: number) => [number, number];
    readonly wasmviewer_zoom_out: (a: number) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h7d04f668e53cbf20: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h517b456bb5d26d33: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h03e77ef33c6d57af: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h16d2b2e46ab70735: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_4: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_5: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_6: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_7: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_8: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_9: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h019f26cd9a989750_10: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hb0acd68b345ce37f: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
