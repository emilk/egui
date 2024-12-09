let wasm_bindgen;
(function() {
    const __exports = {};
    let script_src;
    if (typeof document !== 'undefined' && document.currentScript !== null) {
        script_src = new URL(document.currentScript.src, location.href).toString();
    }
    let wasm = undefined;

    const heap = new Array(128).fill(undefined);

    heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_2.get(state.dtor)(state.a, state.b)
});

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}
function __wbg_adapter_32(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h5e3cbecfb7616b4e(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_37(arg0, arg1) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hea41d664f9393da0(retptr, arg0, arg1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wbg_adapter_40(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hc62e9caef7094c4f(arg0, arg1, addHeapObject(arg2));
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_612(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h18bd98d686c97fd5(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const __wbindgen_enum_BinaryType = ["blob", "arraybuffer"];

const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];

const __wbindgen_enum_ResizeObserverBoxOptions = ["border-box", "content-box", "device-pixel-content-box"];

const WebHandleFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_webhandle_free(ptr >>> 0, 1));
/**
 * Our handle to the web app from JavaScript.
 */
class WebHandle {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WebHandleFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_webhandle_free(ptr, 0);
    }
    /**
     * Installs a panic hook, then returns.
     */
    constructor() {
        const ret = wasm.webhandle_new();
        this.__wbg_ptr = ret >>> 0;
        WebHandleFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Call this once from JavaScript to start your app.
     * @param {HTMLCanvasElement} canvas
     * @returns {Promise<void>}
     */
    start(canvas) {
        const ret = wasm.webhandle_start(this.__wbg_ptr, addHeapObject(canvas));
        return takeObject(ret);
    }
    destroy() {
        wasm.webhandle_destroy(this.__wbg_ptr);
    }
    /**
     * Example on how to call into your app from JavaScript.
     */
    example() {
        wasm.webhandle_example(this.__wbg_ptr);
    }
    /**
     * The JavaScript can check whether or not your app has crashed:
     * @returns {boolean}
     */
    has_panicked() {
        const ret = wasm.webhandle_has_panicked(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {string | undefined}
     */
    panic_message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.webhandle_panic_message(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            let v1;
            if (r0 !== 0) {
                v1 = getStringFromWasm0(r0, r1).slice();
                wasm.__wbindgen_free(r0, r1 * 1, 1);
            }
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * @returns {string | undefined}
     */
    panic_callstack() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.webhandle_panic_callstack(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            let v1;
            if (r0 !== 0) {
                v1 = getStringFromWasm0(r0, r1).slice();
                wasm.__wbindgen_free(r0, r1 * 1, 1);
            }
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
__exports.WebHandle = WebHandle;

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_error_ecb8b2ef9f17fff0 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_new_5a0c9fd4e0a0fcbe = function() {
        const ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_stack_97b8ae9669382e90 = function(arg0, arg1) {
        const ret = getObject(arg1).stack;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = getObject(arg0) in getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbg_debug_3f6d1d5368ceacfc = function(arg0, arg1) {
        console.debug(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_info_8771fc1eb3dce37c = function(arg0, arg1) {
        console.info(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_warn_bcd15d6888e2b773 = function(arg0, arg1) {
        console.warn(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_queueMicrotask_c5419c06eab41e73 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_queueMicrotask_848aa4969108a57e = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbg_performance_a1b8bde2ee512264 = function(arg0) {
        const ret = getObject(arg0).performance;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_now_abd80e969af37148 = function(arg0) {
        const ret = getObject(arg0).now();
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbg_instanceof_WebGl2RenderingContext_8dbe5170d8fdea28 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof WebGL2RenderingContext;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_bindVertexArray_9971ca458d8940ea = function(arg0, arg1) {
        getObject(arg0).bindVertexArray(getObject(arg1));
    };
    imports.wbg.__wbg_bufferData_97b16c4aedab785a = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
    };
    imports.wbg.__wbg_createVertexArray_ec08b54b9f8c74ea = function(arg0) {
        const ret = getObject(arg0).createVertexArray();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_deleteVertexArray_112dd9bcd72ec608 = function(arg0, arg1) {
        getObject(arg0).deleteVertexArray(getObject(arg1));
    };
    imports.wbg.__wbg_texImage2D_05363c5a13ee70f9 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
    }, arguments) };
    imports.wbg.__wbg_texImage2D_1dbd69fed57d8de3 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
    }, arguments) };
    imports.wbg.__wbg_texSubImage2D_97bed542c038dfb5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
    }, arguments) };
    imports.wbg.__wbg_texSubImage2D_74255449b4229fd1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
    }, arguments) };
    imports.wbg.__wbg_activeTexture_a2e9931456fe92b4 = function(arg0, arg1) {
        getObject(arg0).activeTexture(arg1 >>> 0);
    };
    imports.wbg.__wbg_attachShader_299671ccaa78592c = function(arg0, arg1, arg2) {
        getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
    };
    imports.wbg.__wbg_bindBuffer_70e5a7ef4920142a = function(arg0, arg1, arg2) {
        getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
    };
    imports.wbg.__wbg_bindTexture_78210066cfdda8ac = function(arg0, arg1, arg2) {
        getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
    };
    imports.wbg.__wbg_blendEquationSeparate_f31b2648426dff95 = function(arg0, arg1, arg2) {
        getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
    };
    imports.wbg.__wbg_blendFuncSeparate_79ff089d1b7d8fdd = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    };
    imports.wbg.__wbg_clear_678615798766f804 = function(arg0, arg1) {
        getObject(arg0).clear(arg1 >>> 0);
    };
    imports.wbg.__wbg_clearColor_0af942e0c8c453eb = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
    };
    imports.wbg.__wbg_colorMask_88c579e312b0fdcf = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
    };
    imports.wbg.__wbg_compileShader_9680f4f1d833586c = function(arg0, arg1) {
        getObject(arg0).compileShader(getObject(arg1));
    };
    imports.wbg.__wbg_createBuffer_478457cb9beff1a3 = function(arg0) {
        const ret = getObject(arg0).createBuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createProgram_48b8a105fd0cfb35 = function(arg0) {
        const ret = getObject(arg0).createProgram();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createShader_f956a5ec67a77964 = function(arg0, arg1) {
        const ret = getObject(arg0).createShader(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createTexture_3ebc81a77f42cd4b = function(arg0) {
        const ret = getObject(arg0).createTexture();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_deleteBuffer_4ab8b253a2ff7ec7 = function(arg0, arg1) {
        getObject(arg0).deleteBuffer(getObject(arg1));
    };
    imports.wbg.__wbg_deleteProgram_ef8d37545b8ab3ce = function(arg0, arg1) {
        getObject(arg0).deleteProgram(getObject(arg1));
    };
    imports.wbg.__wbg_deleteShader_c65ef8df50ff2e29 = function(arg0, arg1) {
        getObject(arg0).deleteShader(getObject(arg1));
    };
    imports.wbg.__wbg_deleteTexture_05e26b0508f0589d = function(arg0, arg1) {
        getObject(arg0).deleteTexture(getObject(arg1));
    };
    imports.wbg.__wbg_detachShader_edfa1d6365e56336 = function(arg0, arg1, arg2) {
        getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
    };
    imports.wbg.__wbg_disable_d0317155c2bda795 = function(arg0, arg1) {
        getObject(arg0).disable(arg1 >>> 0);
    };
    imports.wbg.__wbg_disableVertexAttribArray_58aa0d2748ca82d4 = function(arg0, arg1) {
        getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
    };
    imports.wbg.__wbg_drawArrays_af53529e509d0c8b = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
    };
    imports.wbg.__wbg_drawElements_a40e0aeb70716911 = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
    };
    imports.wbg.__wbg_enable_b73a997042de6e09 = function(arg0, arg1) {
        getObject(arg0).enable(arg1 >>> 0);
    };
    imports.wbg.__wbg_enableVertexAttribArray_08b992ae13fe30a9 = function(arg0, arg1) {
        getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
    };
    imports.wbg.__wbg_generateMipmap_eed46cfd38713244 = function(arg0, arg1) {
        getObject(arg0).generateMipmap(arg1 >>> 0);
    };
    imports.wbg.__wbg_getAttribLocation_c498bc242afbf700 = function(arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return ret;
    };
    imports.wbg.__wbg_getError_8ad2027c9905d389 = function(arg0) {
        const ret = getObject(arg0).getError();
        return ret;
    };
    imports.wbg.__wbg_getExtension_811520f1db50ca11 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getParameter_1b7c85c782ee0a5e = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).getParameter(arg1 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getProgramInfoLog_16c69289b6a9c98e = function(arg0, arg1, arg2) {
        const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getProgramParameter_4c981ddc3b62dda8 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getShaderInfoLog_afb2baaac4baaff5 = function(arg0, arg1, arg2) {
        const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getShaderParameter_e21fb00f8255b86b = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getSupportedExtensions_ae0473d2b21281af = function(arg0) {
        const ret = getObject(arg0).getSupportedExtensions();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_getUniformLocation_74149153bba4c4cb = function(arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_linkProgram_983c5972b815b0de = function(arg0, arg1) {
        getObject(arg0).linkProgram(getObject(arg1));
    };
    imports.wbg.__wbg_pixelStorei_1077f1f904f1a03d = function(arg0, arg1, arg2) {
        getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
    };
    imports.wbg.__wbg_scissor_3cdd53b98aa49fb5 = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).scissor(arg1, arg2, arg3, arg4);
    };
    imports.wbg.__wbg_shaderSource_c36f18b5114855e7 = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
    };
    imports.wbg.__wbg_texParameteri_a73df30f47a92fec = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
    };
    imports.wbg.__wbg_uniform1f_d2ba9f3d60c3859c = function(arg0, arg1, arg2) {
        getObject(arg0).uniform1f(getObject(arg1), arg2);
    };
    imports.wbg.__wbg_uniform1i_b7abcc7b3b4aee52 = function(arg0, arg1, arg2) {
        getObject(arg0).uniform1i(getObject(arg1), arg2);
    };
    imports.wbg.__wbg_uniform2f_42bc6b11055323f9 = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
    };
    imports.wbg.__wbg_useProgram_8232847dbf97643a = function(arg0, arg1) {
        getObject(arg0).useProgram(getObject(arg1));
    };
    imports.wbg.__wbg_vertexAttribPointer_f602d22ecb0758f6 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
    };
    imports.wbg.__wbg_viewport_e333f63662d91f3a = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).viewport(arg1, arg2, arg3, arg4);
    };
    imports.wbg.__wbg_instanceof_Window_6575cd7f1322f82f = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_document_d7fa2c739c2b191a = function(arg0) {
        const ret = getObject(arg0).document;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_location_72721055fbff81f2 = function(arg0) {
        const ret = getObject(arg0).location;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_navigator_3d3836196a5d8e62 = function(arg0) {
        const ret = getObject(arg0).navigator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_devicePixelRatio_5d0556383aa83231 = function(arg0) {
        const ret = getObject(arg0).devicePixelRatio;
        return ret;
    };
    imports.wbg.__wbg_speechSynthesis_1200ffd9a328556b = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).speechSynthesis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_localStorage_6026615061e890bf = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).localStorage;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_performance_8efa15a3e0d18099 = function(arg0) {
        const ret = getObject(arg0).performance;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_isSecureContext_8a5cdec3d92171bf = function(arg0) {
        const ret = getObject(arg0).isSecureContext;
        return ret;
    };
    imports.wbg.__wbg_getComputedStyle_ec7e113b79b74e98 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).getComputedStyle(getObject(arg1));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_matchMedia_2c5a513148e49e4a = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_245c3e57ba96efce = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_cancelAnimationFrame_f802bc3f3a9b2e5c = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).cancelAnimationFrame(arg1);
    }, arguments) };
    imports.wbg.__wbg_requestAnimationFrame_8c3436f4ac89bc48 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_clearInterval_538773bf49791f6f = function(arg0, arg1) {
        getObject(arg0).clearInterval(arg1);
    };
    imports.wbg.__wbg_fetch_bb5ee426272994d9 = function(arg0, arg1) {
        const ret = getObject(arg0).fetch(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_body_8e909b791b1745d3 = function(arg0) {
        const ret = getObject(arg0).body;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_hidden_9ae94f885363245b = function(arg0) {
        const ret = getObject(arg0).hidden;
        return ret;
    };
    imports.wbg.__wbg_activeElement_4ab2bc6dcf8da330 = function(arg0) {
        const ret = getObject(arg0).activeElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createElement_e4523490bd0ae51d = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_elementFromPoint_7ebcb9cd80c2cbb6 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).elementFromPoint(arg1, arg2);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_Element_1a81366cc90e70e2 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Element;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_id_7e3fad2f1278550e = function(arg0, arg1) {
        const ret = getObject(arg1).id;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getBoundingClientRect_5ad16be1e2955e83 = function(arg0) {
        const ret = getObject(arg0).getBoundingClientRect();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_setAttribute_2a8f647a8d92c712 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_remove_d7a18d9f46bc50fd = function(arg0) {
        getObject(arg0).remove();
    };
    imports.wbg.__wbg_settabIndex_a11c34f55118ce28 = function(arg0, arg1) {
        getObject(arg0).tabIndex = arg1;
    };
    imports.wbg.__wbg_style_04eb1488bc2ceffc = function(arg0) {
        const ret = getObject(arg0).style;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_offsetTop_b9521f886d70cd98 = function(arg0) {
        const ret = getObject(arg0).offsetTop;
        return ret;
    };
    imports.wbg.__wbg_blur_d7e0bcc31c40e996 = function() { return handleError(function (arg0) {
        getObject(arg0).blur();
    }, arguments) };
    imports.wbg.__wbg_focus_6b6181f7644f6dbc = function() { return handleError(function (arg0) {
        getObject(arg0).focus();
    }, arguments) };
    imports.wbg.__wbg_instanceof_WebGlRenderingContext_39ef06dc49edcd67 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof WebGLRenderingContext;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_bufferData_11bf0e7b1bcebb55 = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
    };
    imports.wbg.__wbg_texImage2D_12005a1c57d665bb = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
    }, arguments) };
    imports.wbg.__wbg_texSubImage2D_e784b7363b6c212d = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
    }, arguments) };
    imports.wbg.__wbg_activeTexture_b0bb95e7b2c13666 = function(arg0, arg1) {
        getObject(arg0).activeTexture(arg1 >>> 0);
    };
    imports.wbg.__wbg_attachShader_4a6cb7b86d7531b8 = function(arg0, arg1, arg2) {
        getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
    };
    imports.wbg.__wbg_bindBuffer_87bece1307f4836f = function(arg0, arg1, arg2) {
        getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
    };
    imports.wbg.__wbg_bindTexture_578ab0356afb56df = function(arg0, arg1, arg2) {
        getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
    };
    imports.wbg.__wbg_blendEquationSeparate_4dba689f460b83c7 = function(arg0, arg1, arg2) {
        getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
    };
    imports.wbg.__wbg_blendFuncSeparate_54734c3d5f7ec376 = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    };
    imports.wbg.__wbg_clear_c5af939d0a44a025 = function(arg0, arg1) {
        getObject(arg0).clear(arg1 >>> 0);
    };
    imports.wbg.__wbg_clearColor_f7a4d2d6a8d8cdf6 = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
    };
    imports.wbg.__wbg_colorMask_f1fbf32fb9ff5f55 = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
    };
    imports.wbg.__wbg_compileShader_48a677cac607634b = function(arg0, arg1) {
        getObject(arg0).compileShader(getObject(arg1));
    };
    imports.wbg.__wbg_createBuffer_2f1b069b0fbe4db7 = function(arg0) {
        const ret = getObject(arg0).createBuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createProgram_1510c4697aed8d2f = function(arg0) {
        const ret = getObject(arg0).createProgram();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createShader_3edd95eb001d29c9 = function(arg0, arg1) {
        const ret = getObject(arg0).createShader(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createTexture_01a5bbc5d52164d2 = function(arg0) {
        const ret = getObject(arg0).createTexture();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_deleteBuffer_2b49293fc295ccea = function(arg0, arg1) {
        getObject(arg0).deleteBuffer(getObject(arg1));
    };
    imports.wbg.__wbg_deleteProgram_16d8257cfae7ddbe = function(arg0, arg1) {
        getObject(arg0).deleteProgram(getObject(arg1));
    };
    imports.wbg.__wbg_deleteShader_a49077cc02f9d75c = function(arg0, arg1) {
        getObject(arg0).deleteShader(getObject(arg1));
    };
    imports.wbg.__wbg_deleteTexture_f72079e46289ccf8 = function(arg0, arg1) {
        getObject(arg0).deleteTexture(getObject(arg1));
    };
    imports.wbg.__wbg_detachShader_11435bbfadd38efd = function(arg0, arg1, arg2) {
        getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
    };
    imports.wbg.__wbg_disable_a342a9330a0cd452 = function(arg0, arg1) {
        getObject(arg0).disable(arg1 >>> 0);
    };
    imports.wbg.__wbg_disableVertexAttribArray_636452904a337436 = function(arg0, arg1) {
        getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
    };
    imports.wbg.__wbg_drawArrays_bb3d6e0af7dcb469 = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
    };
    imports.wbg.__wbg_drawElements_f761454e5306de80 = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
    };
    imports.wbg.__wbg_enable_d120ad9b31220426 = function(arg0, arg1) {
        getObject(arg0).enable(arg1 >>> 0);
    };
    imports.wbg.__wbg_enableVertexAttribArray_a12ed0a684959868 = function(arg0, arg1) {
        getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
    };
    imports.wbg.__wbg_generateMipmap_952d412cb9b50adb = function(arg0, arg1) {
        getObject(arg0).generateMipmap(arg1 >>> 0);
    };
    imports.wbg.__wbg_getAttribLocation_6389196ac5e58c5c = function(arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return ret;
    };
    imports.wbg.__wbg_getError_919f5a6e84ab2e46 = function(arg0) {
        const ret = getObject(arg0).getError();
        return ret;
    };
    imports.wbg.__wbg_getExtension_e54e6eac6f420939 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getParameter_21bd0c7970e3e51c = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).getParameter(arg1 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getProgramInfoLog_2ebf87ded3a93ef1 = function(arg0, arg1, arg2) {
        const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getProgramParameter_2fc04fee21ea5036 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getShaderInfoLog_eabc357ae8803006 = function(arg0, arg1, arg2) {
        const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getShaderParameter_e5207a499ce4b3a1 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getSupportedExtensions_d46f11b06ebefa0d = function(arg0) {
        const ret = getObject(arg0).getSupportedExtensions();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_getUniformLocation_f600c2277dd826b4 = function(arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_linkProgram_b4246295077a3859 = function(arg0, arg1) {
        getObject(arg0).linkProgram(getObject(arg1));
    };
    imports.wbg.__wbg_pixelStorei_86e41250cf27c77f = function(arg0, arg1, arg2) {
        getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
    };
    imports.wbg.__wbg_scissor_f1b8dd095e3fa77a = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).scissor(arg1, arg2, arg3, arg4);
    };
    imports.wbg.__wbg_shaderSource_f8f569556926b597 = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
    };
    imports.wbg.__wbg_texParameteri_72793934be86cdcf = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
    };
    imports.wbg.__wbg_uniform1f_800970b4947e87c4 = function(arg0, arg1, arg2) {
        getObject(arg0).uniform1f(getObject(arg1), arg2);
    };
    imports.wbg.__wbg_uniform1i_55c667a20431c589 = function(arg0, arg1, arg2) {
        getObject(arg0).uniform1i(getObject(arg1), arg2);
    };
    imports.wbg.__wbg_uniform2f_41690ab5f9e5c2b2 = function(arg0, arg1, arg2, arg3) {
        getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
    };
    imports.wbg.__wbg_useProgram_0f0a7b123a5eba79 = function(arg0, arg1) {
        getObject(arg0).useProgram(getObject(arg1));
    };
    imports.wbg.__wbg_vertexAttribPointer_6e1de5dfe082f820 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
    };
    imports.wbg.__wbg_viewport_567a7a444dd3650b = function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).viewport(arg1, arg2, arg3, arg4);
    };
    imports.wbg.__wbg_bindVertexArrayOES_f7ae803496f6f82f = function(arg0, arg1) {
        getObject(arg0).bindVertexArrayOES(getObject(arg1));
    };
    imports.wbg.__wbg_createVertexArrayOES_6e8273e64e878af6 = function(arg0) {
        const ret = getObject(arg0).createVertexArrayOES();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_deleteVertexArrayOES_f24bf9fab17be367 = function(arg0, arg1) {
        getObject(arg0).deleteVertexArrayOES(getObject(arg1));
    };
    imports.wbg.__wbg_new_bc395d17a25f9f2f = function() { return handleError(function (arg0) {
        const ret = new ResizeObserver(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_disconnect_91f6e3e629264c78 = function(arg0) {
        getObject(arg0).disconnect();
    };
    imports.wbg.__wbg_observe_e05a589c42476328 = function(arg0, arg1, arg2) {
        getObject(arg0).observe(getObject(arg1), getObject(arg2));
    };
    imports.wbg.__wbg_error_53abcd6a461f73d8 = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_items_f84b3a3510533fc9 = function(arg0) {
        const ret = getObject(arg0).items;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_files_194ca113571d995f = function(arg0) {
        const ret = getObject(arg0).files;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_getData_d687ebb5e1da9d6e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg1).getData(getStringFromWasm0(arg2, arg3));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_appendChild_bc4a0deae90a5164 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).appendChild(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_addEventListener_4357f9b7b3826784 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_removeEventListener_4c13d11156153514 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_instanceof_HtmlInputElement_ee25196edbacced9 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof HTMLInputElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_setautofocus_2c3f132eaaf0526a = function(arg0, arg1) {
        getObject(arg0).autofocus = arg1 !== 0;
    };
    imports.wbg.__wbg_settype_8179afd52dd717a2 = function(arg0, arg1, arg2) {
        getObject(arg0).type = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_value_0cffd86fb9a5a18d = function(arg0, arg1) {
        const ret = getObject(arg1).value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_setvalue_36bcf6f86c998f0a = function(arg0, arg1, arg2) {
        getObject(arg0).value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_getPropertyValue_b199c95cfeadebdf = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg1).getPropertyValue(getStringFromWasm0(arg2, arg3));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_setProperty_b9a2384cbfb499fb = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_width_45de62653cf1c40c = function(arg0) {
        const ret = getObject(arg0).width;
        return ret;
    };
    imports.wbg.__wbg_height_333816c9b207333d = function(arg0) {
        const ret = getObject(arg0).height;
        return ret;
    };
    imports.wbg.__wbg_top_5f4586313f3e086f = function(arg0) {
        const ret = getObject(arg0).top;
        return ret;
    };
    imports.wbg.__wbg_right_f7d64a961b5dc7fb = function(arg0) {
        const ret = getObject(arg0).right;
        return ret;
    };
    imports.wbg.__wbg_bottom_e9f75f36a7551e7a = function(arg0) {
        const ret = getObject(arg0).bottom;
        return ret;
    };
    imports.wbg.__wbg_left_324ad4ce0086311f = function(arg0) {
        const ret = getObject(arg0).left;
        return ret;
    };
    imports.wbg.__wbg_preventDefault_eecc4a63e64c4526 = function(arg0) {
        getObject(arg0).preventDefault();
    };
    imports.wbg.__wbg_stopPropagation_8a8fc87824cc6f0b = function(arg0) {
        getObject(arg0).stopPropagation();
    };
    imports.wbg.__wbg_clientX_a8eebf094c107e43 = function(arg0) {
        const ret = getObject(arg0).clientX;
        return ret;
    };
    imports.wbg.__wbg_clientY_ffe0a79af8089cd4 = function(arg0) {
        const ret = getObject(arg0).clientY;
        return ret;
    };
    imports.wbg.__wbg_ctrlKey_4015247a39aa9410 = function(arg0) {
        const ret = getObject(arg0).ctrlKey;
        return ret;
    };
    imports.wbg.__wbg_shiftKey_6d843f3032fd0366 = function(arg0) {
        const ret = getObject(arg0).shiftKey;
        return ret;
    };
    imports.wbg.__wbg_altKey_c9401b041949ea90 = function(arg0) {
        const ret = getObject(arg0).altKey;
        return ret;
    };
    imports.wbg.__wbg_metaKey_5d680933661ea1ea = function(arg0) {
        const ret = getObject(arg0).metaKey;
        return ret;
    };
    imports.wbg.__wbg_button_d8226b772c8cf494 = function(arg0) {
        const ret = getObject(arg0).button;
        return ret;
    };
    imports.wbg.__wbg_identifier_b858c904e1c72507 = function(arg0) {
        const ret = getObject(arg0).identifier;
        return ret;
    };
    imports.wbg.__wbg_clientX_0e075d664eb70517 = function(arg0) {
        const ret = getObject(arg0).clientX;
        return ret;
    };
    imports.wbg.__wbg_clientY_32b24b7be6b2e79d = function(arg0) {
        const ret = getObject(arg0).clientY;
        return ret;
    };
    imports.wbg.__wbg_force_5af67f6cd0b9c097 = function(arg0) {
        const ret = getObject(arg0).force;
        return ret;
    };
    imports.wbg.__wbg_deltaX_10154f810008c0a0 = function(arg0) {
        const ret = getObject(arg0).deltaX;
        return ret;
    };
    imports.wbg.__wbg_deltaY_afd77a1b9e0d9ccd = function(arg0) {
        const ret = getObject(arg0).deltaY;
        return ret;
    };
    imports.wbg.__wbg_deltaMode_f31810d86a9defec = function(arg0) {
        const ret = getObject(arg0).deltaMode;
        return ret;
    };
    imports.wbg.__wbg_setbox_0540f4f0ed4733b6 = function(arg0, arg1) {
        getObject(arg0).box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
    };
    imports.wbg.__wbg_size_51012fd85f29376f = function(arg0) {
        const ret = getObject(arg0).size;
        return ret;
    };
    imports.wbg.__wbg_type_ad7415f0526f4291 = function(arg0, arg1) {
        const ret = getObject(arg1).type;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_arrayBuffer_b399ae237d35f149 = function(arg0) {
        const ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_name_e30efb33291e0016 = function(arg0, arg1) {
        const ret = getObject(arg1).name;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_lastModified_bbe64ae9081c28ac = function(arg0) {
        const ret = getObject(arg0).lastModified;
        return ret;
    };
    imports.wbg.__wbg_headers_b848f4e3b0f8b7b9 = function(arg0) {
        const ret = getObject(arg0).headers;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithstrandinit_4b92c89af0a8e383 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_setbody_aa8b691bec428bf4 = function(arg0, arg1) {
        getObject(arg0).body = getObject(arg1);
    };
    imports.wbg.__wbg_setmethod_ce2da76000b02f6a = function(arg0, arg1, arg2) {
        getObject(arg0).method = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setmode_4919fd636102c586 = function(arg0, arg1) {
        getObject(arg0).mode = __wbindgen_enum_RequestMode[arg1];
    };
    imports.wbg.__wbg_set_55d5ebe3df237c46 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_instanceof_Response_3c0e210a57ff751d = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_url_58af972663531d16 = function(arg0, arg1) {
        const ret = getObject(arg1).url;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_status_5f4e900d22140a18 = function(arg0) {
        const ret = getObject(arg0).status;
        return ret;
    };
    imports.wbg.__wbg_ok_abdcc292e508a59f = function(arg0) {
        const ret = getObject(arg0).ok;
        return ret;
    };
    imports.wbg.__wbg_statusText_a3640f5ebf06d87c = function(arg0, arg1) {
        const ret = getObject(arg1).statusText;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_headers_1b9bf90c73fae600 = function(arg0) {
        const ret = getObject(arg0).headers;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_arrayBuffer_144729e09879650e = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_touches_092e96ce3221acbc = function(arg0) {
        const ret = getObject(arg0).touches;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_changedTouches_ee3dabea7d95ebf2 = function(arg0) {
        const ret = getObject(arg0).changedTouches;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_1b6ac4894265d4e6 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_item_b480aa49bad2197f = function(arg0, arg1) {
        const ret = getObject(arg0).item(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_get_4d863ed1d42a2b7d = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_data_86e77dc14916d155 = function(arg0, arg1) {
        const ret = getObject(arg1).data;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_dataTransfer_b898af73237a967c = function(arg0) {
        const ret = getObject(arg0).dataTransfer;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_keyCode_b40a8c053bb90ca7 = function(arg0) {
        const ret = getObject(arg0).keyCode;
        return ret;
    };
    imports.wbg.__wbg_altKey_ebf03e2308f51c08 = function(arg0) {
        const ret = getObject(arg0).altKey;
        return ret;
    };
    imports.wbg.__wbg_ctrlKey_f592192d87040d94 = function(arg0) {
        const ret = getObject(arg0).ctrlKey;
        return ret;
    };
    imports.wbg.__wbg_shiftKey_cb120edc9c25950d = function(arg0) {
        const ret = getObject(arg0).shiftKey;
        return ret;
    };
    imports.wbg.__wbg_metaKey_0735ca81e2ec6c72 = function(arg0) {
        const ret = getObject(arg0).metaKey;
        return ret;
    };
    imports.wbg.__wbg_isComposing_527cd20b8f4c31d2 = function(arg0) {
        const ret = getObject(arg0).isComposing;
        return ret;
    };
    imports.wbg.__wbg_key_001eb20ba3b3d2fd = function(arg0, arg1) {
        const ret = getObject(arg1).key;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_instanceof_ResizeObserverSize_24861b0b596f2d13 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ResizeObserverSize;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_inlineSize_bdd9c1683673d79e = function(arg0) {
        const ret = getObject(arg0).inlineSize;
        return ret;
    };
    imports.wbg.__wbg_blockSize_5d28da4852a3728e = function(arg0) {
        const ret = getObject(arg0).blockSize;
        return ret;
    };
    imports.wbg.__wbg_getItem_cc312d333f535f07 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg1).getItem(getStringFromWasm0(arg2, arg3));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_setItem_5afc04d5b6287c76 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_writeText_9d976569ea57aa0d = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).writeText(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_href_a78089b3b726e0af = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).href;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_origin_1830c25dfb01148b = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).origin;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_protocol_39dcf7495862d01b = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).protocol;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_host_0dbedd515e7d7dff = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).host;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_hostname_f0edc9340c081d0e = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).hostname;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_port_42a0d83797247d86 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).port;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_search_aaeccdb8c45f3efa = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).search;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_hash_acef7ae4422b13b0 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).hash;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_matches_7df350fbe7beb09f = function(arg0) {
        const ret = getObject(arg0).matches;
        return ret;
    };
    imports.wbg.__wbg_now_d3cbc9581625f686 = function(arg0) {
        const ret = getObject(arg0).now();
        return ret;
    };
    imports.wbg.__wbg_setvolume_84e0086877ae1798 = function(arg0, arg1) {
        getObject(arg0).volume = arg1;
    };
    imports.wbg.__wbg_setrate_ed5ab0f6462d491a = function(arg0, arg1) {
        getObject(arg0).rate = arg1;
    };
    imports.wbg.__wbg_setpitch_66c9969ab3e5dd4a = function(arg0, arg1) {
        getObject(arg0).pitch = arg1;
    };
    imports.wbg.__wbg_newwithtext_83fd503a64146fd2 = function() { return handleError(function (arg0, arg1) {
        const ret = new SpeechSynthesisUtterance(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_type_7dfeed312f915bc7 = function(arg0, arg1) {
        const ret = getObject(arg1).type;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_length_3ee0cb6d6e03fafa = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_get_12f95c0ea9bf1d84 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_length_21a3493916831b15 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_get_4d667686e5e4fa4f = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_matches_2534cd63e08d2272 = function(arg0) {
        const ret = getObject(arg0).matches;
        return ret;
    };
    imports.wbg.__wbg_cancel_b83cef45dde56318 = function(arg0) {
        getObject(arg0).cancel();
    };
    imports.wbg.__wbg_speak_32acc7cc354355f3 = function(arg0, arg1) {
        getObject(arg0).speak(getObject(arg1));
    };
    imports.wbg.__wbg_clipboardData_6c07151d2898ce82 = function(arg0) {
        const ret = getObject(arg0).clipboardData;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_width_cd62a064492c4489 = function(arg0) {
        const ret = getObject(arg0).width;
        return ret;
    };
    imports.wbg.__wbg_setwidth_23bf2deedd907275 = function(arg0, arg1) {
        getObject(arg0).width = arg1 >>> 0;
    };
    imports.wbg.__wbg_height_f9f3ea69baf38ed4 = function(arg0) {
        const ret = getObject(arg0).height;
        return ret;
    };
    imports.wbg.__wbg_setheight_239dc283bbe50da4 = function(arg0, arg1) {
        getObject(arg0).height = arg1 >>> 0;
    };
    imports.wbg.__wbg_getContext_bf8985355a4d22ca = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_isComposing_67ecb026b0e0555e = function(arg0) {
        const ret = getObject(arg0).isComposing;
        return ret;
    };
    imports.wbg.__wbg_clipboard_e43b3472696043c3 = function(arg0) {
        const ret = getObject(arg0).clipboard;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_userAgent_b433f0f22dfedec5 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).userAgent;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_instanceof_ResizeObserverEntry_c62bf203457900ce = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ResizeObserverEntry;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_contentRect_0ff902e25b5b4c7a = function(arg0) {
        const ret = getObject(arg0).contentRect;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_contentBoxSize_407a04b6289dbaa7 = function(arg0) {
        const ret = getObject(arg0).contentBoxSize;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_devicePixelContentBoxSize_67d2874a12290f0b = function(arg0) {
        const ret = getObject(arg0).devicePixelContentBoxSize;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_5419cf6b954aa11d = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_f217bbbf7e8e4df4 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_newnoargs_1ede4bf2ebbaaf43 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbg_next_13b477da1eaa3897 = function(arg0) {
        const ret = getObject(arg0).next;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_next_b06e115d1b01e10b = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_done_983b5ffcaec8c583 = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_value_2ab8a198c834c26a = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_iterator_695d699a44d6234c = function() {
        const ret = Symbol.iterator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_ef828680c64da212 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_a9ef466721e824f2 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_e69b5f66fda8f13c = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_self_bf91bf94d9e04084 = function() { return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_52dd9f07d03fd5f8 = function() { return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_globalThis_05c129bf37fcf1be = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_3eca19bb09e9c484 = function() { return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_at_2c39eacdcce73361 = function(arg0, arg1) {
        const ret = getObject(arg0).at(arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_call_3bfa248576352471 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getTime_41225036a0393d63 = function(arg0) {
        const ret = getObject(arg0).getTime();
        return ret;
    };
    imports.wbg.__wbg_getTimezoneOffset_93f7d384c8ade3be = function(arg0) {
        const ret = getObject(arg0).getTimezoneOffset();
        return ret;
    };
    imports.wbg.__wbg_new_6fb55f037293191b = function(arg0) {
        const ret = new Date(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new0_218ada33b570be35 = function() {
        const ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_is_4b64bc96710d6a0f = function(arg0, arg1) {
        const ret = Object.is(getObject(arg0), getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_instanceof_TypeError_4510504e6f11a93c = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof TypeError;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_new_1073970097e5a420 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_612(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return addHeapObject(ret);
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_resolve_0aad7c1484731c99 = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_748f75edfb032440 = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_4866a7d9f55d8f3e = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_ccaed51a635d8a2d = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_a477014f6b279082 = function(arg0, arg1, arg2) {
        const ret = new Int8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_2162229fb032f49b = function(arg0, arg1, arg2) {
        const ret = new Int16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_98f18acc088b651f = function(arg0, arg1, arg2) {
        const ret = new Int32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_7e3eb787208af730 = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_fec2611eb9180f95 = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_ec2fcf81bc573fd9 = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_length_9254c4bd3b9f23c4 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_e74b33a1f7565139 = function(arg0, arg1, arg2) {
        const ret = new Uint16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_5f67057565ba35bf = function(arg0, arg1, arg2) {
        const ret = new Uint32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_fc445c2d308275d0 = function(arg0, arg1, arg2) {
        const ret = new Float32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper3301 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1141, __wbg_adapter_32);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper3303 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1141, __wbg_adapter_32);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper3305 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1141, __wbg_adapter_37);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper3476 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1202, __wbg_adapter_40);
        return addHeapObject(ret);
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;



    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined' && typeof script_src !== 'undefined') {
        module_or_path = script_src.replace(/\.js$/, '_bg.wasm');
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

wasm_bindgen = Object.assign(__wbg_init, { initSync }, __exports);

})();
