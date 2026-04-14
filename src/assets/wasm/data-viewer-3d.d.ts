/* tslint:disable */
/* eslint-disable */
export class EulerRotationDeg {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(pitch: number, yaw: number, roll: number): EulerRotationDeg;
  pitch: number;
  yaw: number;
  roll: number;
}
export class Orientation {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(zoom: number, translation: Translation, rotation: EulerRotationDeg): Orientation;
  zoom: number;
  translation: Translation;
  rotation: EulerRotationDeg;
}
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
  texture: number;
}
export class Translation {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(x: number, y: number): Translation;
  x: number;
  y: number;
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
  set_topology(data: Uint8Array): Promise<void>;
  set_texture(data: Uint8Array): Promise<void>;
  get_pixel_value(): Promise<PixelValue>;
  capture_image(): Promise<Uint8Array>;
  set_height_shader(): void;
  set_texture_shader(): void;
  set_turbo_colormap_shader(): void;
  set_overlays(overlays: Overlay[]): void;
  clear_overlays(): void;
  zoom_in(): void;
  zoom_out(): void;
  set_orientation(orientation: Orientation): void;
  reset_orientation(): void;
  set_percentile(percentile: number): void;
  set_texture_range(start: number, end: number): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_orientation_free: (a: number, b: number) => void;
  readonly __wbg_get_orientation_zoom: (a: number) => number;
  readonly __wbg_set_orientation_zoom: (a: number, b: number) => void;
  readonly __wbg_get_orientation_translation: (a: number) => number;
  readonly __wbg_set_orientation_translation: (a: number, b: number) => void;
  readonly __wbg_get_orientation_rotation: (a: number) => number;
  readonly __wbg_set_orientation_rotation: (a: number, b: number) => void;
  readonly orientation_new: (a: number, b: number, c: number) => number;
  readonly __wbg_wasmbindgenpixelrange_free: (a: number, b: number) => void;
  readonly wasmbindgenpixelrange_new: (a: number, b: number) => number;
  readonly __wbg_overlaycolor_free: (a: number, b: number) => void;
  readonly overlaycolor_new: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_overlay_free: (a: number, b: number) => void;
  readonly overlay_new: (a: number, b: number, c: number) => number;
  readonly __wbg_translation_free: (a: number, b: number) => void;
  readonly __wbg_get_translation_x: (a: number) => number;
  readonly __wbg_set_translation_x: (a: number, b: number) => void;
  readonly __wbg_get_translation_y: (a: number) => number;
  readonly __wbg_set_translation_y: (a: number, b: number) => void;
  readonly translation_new: (a: number, b: number) => number;
  readonly main: (a: number, b: number) => number;
  readonly __wbg_pixelvalue_free: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_x: (a: number) => number;
  readonly __wbg_set_pixelvalue_x: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_y: (a: number) => number;
  readonly __wbg_set_pixelvalue_y: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_texture: (a: number) => number;
  readonly __wbg_set_pixelvalue_texture: (a: number, b: number) => void;
  readonly __wbg_eulerrotationdeg_free: (a: number, b: number) => void;
  readonly __wbg_get_eulerrotationdeg_pitch: (a: number) => number;
  readonly __wbg_set_eulerrotationdeg_pitch: (a: number, b: number) => void;
  readonly __wbg_get_eulerrotationdeg_yaw: (a: number) => number;
  readonly __wbg_set_eulerrotationdeg_yaw: (a: number, b: number) => void;
  readonly __wbg_get_eulerrotationdeg_roll: (a: number) => number;
  readonly __wbg_set_eulerrotationdeg_roll: (a: number, b: number) => void;
  readonly eulerrotationdeg_new: (a: number, b: number, c: number) => number;
  readonly __wbg_wasmviewer_free: (a: number, b: number) => void;
  readonly wasmviewer_new: (a: number, b: number) => [number, number, number];
  readonly wasmviewer_run: (a: number) => [number, number];
  readonly wasmviewer_reset_view: (a: number) => any;
  readonly wasmviewer_set_topology: (a: number, b: number, c: number) => any;
  readonly wasmviewer_set_texture: (a: number, b: number, c: number) => any;
  readonly wasmviewer_get_pixel_value: (a: number) => any;
  readonly wasmviewer_capture_image: (a: number) => any;
  readonly wasmviewer_set_height_shader: (a: number) => [number, number];
  readonly wasmviewer_set_texture_shader: (a: number) => [number, number];
  readonly wasmviewer_set_turbo_colormap_shader: (a: number) => [number, number];
  readonly wasmviewer_set_overlays: (a: number, b: number, c: number) => [number, number];
  readonly wasmviewer_clear_overlays: (a: number) => [number, number];
  readonly wasmviewer_zoom_in: (a: number) => [number, number];
  readonly wasmviewer_zoom_out: (a: number) => [number, number];
  readonly wasmviewer_set_orientation: (a: number, b: number) => [number, number];
  readonly wasmviewer_reset_orientation: (a: number) => [number, number];
  readonly wasmviewer_set_percentile: (a: number, b: number) => [number, number];
  readonly wasmviewer_set_texture_range: (a: number, b: number, c: number) => [number, number];
  readonly __wbg_set_pixelvalue_z: (a: number, b: number) => void;
  readonly __wbg_get_pixelvalue_z: (a: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_1: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h9111653ea2b7a9f5: (a: number, b: number) => void;
  readonly closure688_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure695_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly closure756_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure777_externref_shim: (a: number, b: number, c: any, d: any) => void;
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
