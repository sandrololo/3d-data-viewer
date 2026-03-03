/* tslint:disable */
/* eslint-disable */
export class Overlay {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(pixelrange: WasmBindgenPixelRange[], color: OverlayColor): Overlay;
}
export class OverlayColor {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(r: number, g: number, b: number, a: number): OverlayColor;
}
export class PixelValue {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  x: number;
  y: number;
  z: number;
  amplitude: number;
}
export class WasmBindgenPixelRange {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(start: number, end: number): WasmBindgenPixelRange;
}
export class WasmViewer {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(canvas_id: string): WasmViewer;
  run(): void;
  reset_view(): Promise<void>;
  set_surface(data: Uint8Array): Promise<void>;
  set_amplitude(data: Uint8Array): Promise<void>;
  get_pixel_value(): Promise<PixelValue>;
  capture_image(): Promise<Uint8Array>;
  set_height_shader(): void;
  set_amplitude_shader(): void;
  set_overlays(overlays: Overlay[]): void;
  clear_overlays(): void;
  zoom_in(): void;
  zoom_out(): void;
  reset_orientation(): void;
  set_percentile(percentile: number): void;
  set_amplitude_range(start: number, end: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmbindgenpixelrange_free: (a: number, b: number) => void;
  readonly wasmbindgenpixelrange_new: (a: number, b: number) => number;
  readonly __wbg_overlaycolor_free: (a: number, b: number) => void;
  readonly overlaycolor_new: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_overlay_free: (a: number, b: number) => void;
  readonly overlay_new: (a: number, b: number, c: number) => number;
  readonly __wbg_pixelvalue_free: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_x: (a: number) => number;
  readonly __wbg_set_pixelvalue_x: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_y: (a: number) => number;
  readonly __wbg_set_pixelvalue_y: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_z: (a: number) => number;
  readonly __wbg_set_pixelvalue_z: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_amplitude: (a: number) => number;
  readonly __wbg_set_pixelvalue_amplitude: (a: number, b: number) => void;
  readonly __wbg_wasmviewer_free: (a: number, b: number) => void;
  readonly wasmviewer_new: (a: number, b: number) => [number, number, number];
  readonly wasmviewer_run: (a: number) => [number, number];
  readonly wasmviewer_reset_view: (a: number) => any;
  readonly wasmviewer_set_surface: (a: number, b: number, c: number) => any;
  readonly wasmviewer_set_amplitude: (a: number, b: number, c: number) => any;
  readonly wasmviewer_get_pixel_value: (a: number) => any;
  readonly wasmviewer_capture_image: (a: number) => any;
  readonly wasmviewer_set_height_shader: (a: number) => [number, number];
  readonly wasmviewer_set_amplitude_shader: (a: number) => [number, number];
  readonly wasmviewer_set_overlays: (a: number, b: number, c: number) => [number, number];
  readonly wasmviewer_clear_overlays: (a: number) => [number, number];
  readonly wasmviewer_zoom_in: (a: number) => [number, number];
  readonly wasmviewer_zoom_out: (a: number) => [number, number];
  readonly wasmviewer_reset_orientation: (a: number) => [number, number];
  readonly wasmviewer_set_percentile: (a: number, b: number) => [number, number];
  readonly wasmviewer_set_amplitude_range: (a: number, b: number, c: number) => [number, number];
  readonly main: (a: number, b: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_1: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure691_externref_shim: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h9111653ea2b7a9f5: (a: number, b: number) => void;
  readonly closure698_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly closure758_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure779_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
