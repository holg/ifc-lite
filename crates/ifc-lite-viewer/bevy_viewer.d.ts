/* tslint:disable */
/* eslint-disable */

/**
 * Run the viewer on a canvas element (WASM)
 */
export function run_on_canvas(canvas_selector: string): void;

/**
 * WASM entry point
 */
export function wasm_start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly run_on_canvas: (a: number, b: number) => void;
  readonly wasm_start: () => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___js_sys_629e2361c07d1d11___Array_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__js_sys_629e2361c07d1d11___Array____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___wasm_bindgen_13092f1e5f52fe7e___JsValue_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__wasm_bindgen_13092f1e5f52fe7e___JsValue____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___web_sys_2419c9479ddfc543___features__gen_PointerEvent__PointerEvent_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__web_sys_2419c9479ddfc543___features__gen_PointerEvent__PointerEvent____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___web_sys_2419c9479ddfc543___features__gen_WheelEvent__WheelEvent_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__web_sys_2419c9479ddfc543___features__gen_WheelEvent__WheelEvent____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___web_sys_2419c9479ddfc543___features__gen_FocusEvent__FocusEvent_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__web_sys_2419c9479ddfc543___features__gen_FocusEvent__FocusEvent____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___web_sys_2419c9479ddfc543___features__gen_PageTransitionEvent__PageTransitionEvent_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__web_sys_2419c9479ddfc543___features__gen_PageTransitionEvent__PageTransitionEvent____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___web_sys_2419c9479ddfc543___features__gen_KeyboardEvent__KeyboardEvent_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__web_sys_2419c9479ddfc543___features__gen_KeyboardEvent__KeyboardEvent____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___web_sys_2419c9479ddfc543___features__gen_Event__Event_____: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__web_sys_2419c9479ddfc543___features__gen_Event__Event____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut_____Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___js_sys_629e2361c07d1d11___Array__web_sys_2419c9479ddfc543___features__gen_ResizeObserver__ResizeObserver_____: (a: number, b: number, c: any, d: any) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___closure__destroy___dyn_core_67558b4ca73dc0a8___ops__function__FnMut__js_sys_629e2361c07d1d11___Array__web_sys_2419c9479ddfc543___features__gen_ResizeObserver__ResizeObserver___Output_______: (a: number, b: number) => void;
  readonly wasm_bindgen_13092f1e5f52fe7e___convert__closures_____invoke___bool_: (a: number, b: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
