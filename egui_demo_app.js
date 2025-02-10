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

    function isLikeNone(x) {
        return x === undefined || x === null;
    }

    let heap_next = heap.length;

    function addHeapObject(obj) {
        if (heap_next === heap.length) heap.push(heap.length + 1);
        const idx = heap_next;
        heap_next = heap[idx];

        heap[idx] = obj;
        return idx;
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

    function handleError(f, args) {
        try {
            return f.apply(this, args);
        } catch (e) {
            wasm.__wbindgen_exn_store(addHeapObject(e));
        }
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

    let cachedDataViewMemory0 = null;

    function getDataViewMemory0() {
        if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
            cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
        }
        return cachedDataViewMemory0;
    }

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

    const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(state => {
        wasm.__wbindgen_export_4.get(state.dtor)(state.a, state.b)
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
                    wasm.__wbindgen_export_4.get(state.dtor)(a, state.b);
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
        if (builtInMatches && builtInMatches.length > 1) {
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
    function __wbg_adapter_32(arg0, arg1, arg2) {
        wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h9d2fc152aefa7656(arg0, arg1, addHeapObject(arg2));
    }

    function __wbg_adapter_37(arg0, arg1) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hb22e6bd94257b68d(retptr, arg0, arg1);
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
        wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h0cae73e9ad435632(arg0, arg1, addHeapObject(arg2));
    }

    function __wbg_adapter_636(arg0, arg1, arg2, arg3) {
        wasm.wasm_bindgen__convert__closures__invoke2_mut__h732120dd70b4239b(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
    }

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
        imports.wbg.__wbg_activeElement_d1a1f2b334adf636 = function(arg0) {
            const ret = getObject(arg0).activeElement;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_activeTexture_24b42b03041f3428 = function(arg0, arg1) {
            getObject(arg0).activeTexture(arg1 >>> 0);
        };
        imports.wbg.__wbg_activeTexture_47928613532be667 = function(arg0, arg1) {
            getObject(arg0).activeTexture(arg1 >>> 0);
        };
        imports.wbg.__wbg_addEventListener_562dd6708dd0467d = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
        }, arguments) };
        imports.wbg.__wbg_altKey_56dd0987e7ccbbf2 = function(arg0) {
            const ret = getObject(arg0).altKey;
            return ret;
        };
        imports.wbg.__wbg_altKey_583c79ba3f4fce1e = function(arg0) {
            const ret = getObject(arg0).altKey;
            return ret;
        };
        imports.wbg.__wbg_appendChild_805222aed73feea9 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).appendChild(getObject(arg1));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_arrayBuffer_d004045654bdb712 = function() { return handleError(function (arg0) {
            const ret = getObject(arg0).arrayBuffer();
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_arrayBuffer_f6066fa96bb3b4ec = function(arg0) {
            const ret = getObject(arg0).arrayBuffer();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_at_7551c28a1fce0709 = function(arg0, arg1) {
            const ret = getObject(arg0).at(arg1);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_attachShader_81000b0c4da5b206 = function(arg0, arg1, arg2) {
            getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_attachShader_ccc35921e866b2bf = function(arg0, arg1, arg2) {
            getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_bindBuffer_0fedb16582ffeee6 = function(arg0, arg1, arg2) {
            getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_bindBuffer_c6e5f29d60e90c01 = function(arg0, arg1, arg2) {
            getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_bindTexture_6478edbb238b9c73 = function(arg0, arg1, arg2) {
            getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_bindTexture_9b177c97248ed4d9 = function(arg0, arg1, arg2) {
            getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_bindVertexArrayOES_e81fca007d08d0db = function(arg0, arg1) {
            getObject(arg0).bindVertexArrayOES(getObject(arg1));
        };
        imports.wbg.__wbg_bindVertexArray_d9082254ff3bcc13 = function(arg0, arg1) {
            getObject(arg0).bindVertexArray(getObject(arg1));
        };
        imports.wbg.__wbg_blendEquationSeparate_3dbbe20b0a9fa818 = function(arg0, arg1, arg2) {
            getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendEquationSeparate_d360393d3b1557cd = function(arg0, arg1, arg2) {
            getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendFuncSeparate_2cc7ac2397290a15 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_blendFuncSeparate_c6c035b0094bd58f = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_blockSize_e0006fb003814895 = function(arg0) {
            const ret = getObject(arg0).blockSize;
            return ret;
        };
        imports.wbg.__wbg_blur_5de3b295415a90b1 = function() { return handleError(function (arg0) {
            getObject(arg0).blur();
        }, arguments) };
        imports.wbg.__wbg_body_83d4bc4961a422aa = function(arg0) {
            const ret = getObject(arg0).body;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_bottom_c88ccf856db329d7 = function(arg0) {
            const ret = getObject(arg0).bottom;
            return ret;
        };
        imports.wbg.__wbg_bufferData_5b85d77150f6520a = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
        };
        imports.wbg.__wbg_bufferData_b03654fb80052afe = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
        };
        imports.wbg.__wbg_buffer_6e1d53ff183194fc = function(arg0) {
            const ret = getObject(arg0).buffer;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_button_db48f93638c59f95 = function(arg0) {
            const ret = getObject(arg0).button;
            return ret;
        };
        imports.wbg.__wbg_call_0411c0c3c424db9a = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_call_3114932863209ca6 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).call(getObject(arg1));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_cancelAnimationFrame_f1ad512e269ea165 = function() { return handleError(function (arg0, arg1) {
            getObject(arg0).cancelAnimationFrame(arg1);
        }, arguments) };
        imports.wbg.__wbg_cancel_cad909b29551da17 = function(arg0) {
            getObject(arg0).cancel();
        };
        imports.wbg.__wbg_changedTouches_1120694ede4321bc = function(arg0) {
            const ret = getObject(arg0).changedTouches;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_clearColor_a5d4f51509d11942 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_clearColor_efddd2ad0120f9e5 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_clearInterval_5bbcdf9491cea345 = function(arg0, arg1) {
            getObject(arg0).clearInterval(arg1);
        };
        imports.wbg.__wbg_clear_4e68091e616c29ad = function(arg0, arg1) {
            getObject(arg0).clear(arg1 >>> 0);
        };
        imports.wbg.__wbg_clear_af7641961d766f51 = function(arg0, arg1) {
            getObject(arg0).clear(arg1 >>> 0);
        };
        imports.wbg.__wbg_clientX_505ff93b1712c529 = function(arg0) {
            const ret = getObject(arg0).clientX;
            return ret;
        };
        imports.wbg.__wbg_clientX_f02129d888351eb1 = function(arg0) {
            const ret = getObject(arg0).clientX;
            return ret;
        };
        imports.wbg.__wbg_clientY_3169d28f891e219e = function(arg0) {
            const ret = getObject(arg0).clientY;
            return ret;
        };
        imports.wbg.__wbg_clientY_373d758473493bb9 = function(arg0) {
            const ret = getObject(arg0).clientY;
            return ret;
        };
        imports.wbg.__wbg_clipboardData_066a3f804f1ac3f5 = function(arg0) {
            const ret = getObject(arg0).clipboardData;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_clipboard_3b6f50e23cac9cfb = function(arg0) {
            const ret = getObject(arg0).clipboard;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_colorMask_22d850d91e89df9f = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        };
        imports.wbg.__wbg_colorMask_a6068fae89c17ceb = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        };
        imports.wbg.__wbg_compileShader_5e41ecd397194c21 = function(arg0, arg1) {
            getObject(arg0).compileShader(getObject(arg1));
        };
        imports.wbg.__wbg_compileShader_6868fa6a842f0911 = function(arg0, arg1) {
            getObject(arg0).compileShader(getObject(arg1));
        };
        imports.wbg.__wbg_contentBoxSize_1ffe0adfed1a4ba0 = function(arg0) {
            const ret = getObject(arg0).contentBoxSize;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_contentRect_7aaa87e16fd2882d = function(arg0) {
            const ret = getObject(arg0).contentRect;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_createBuffer_1e646d14cebbb438 = function(arg0) {
            const ret = getObject(arg0).createBuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createBuffer_2a696fb8c0d07970 = function(arg0) {
            const ret = getObject(arg0).createBuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createElement_22b48bfb31a0c20e = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_createProgram_5a5a92d23fdc2f1a = function(arg0) {
            const ret = getObject(arg0).createProgram();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createProgram_b75025f0f1a4ef55 = function(arg0) {
            const ret = getObject(arg0).createProgram();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createShader_29c8e06db7e7211f = function(arg0, arg1) {
            const ret = getObject(arg0).createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createShader_8c3053457f874cdd = function(arg0, arg1) {
            const ret = getObject(arg0).createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createTexture_20f63b261993f581 = function(arg0) {
            const ret = getObject(arg0).createTexture();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createTexture_facd2df68d8d0276 = function(arg0) {
            const ret = getObject(arg0).createTexture();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createVertexArrayOES_89f45f4a4fde395f = function(arg0) {
            const ret = getObject(arg0).createVertexArrayOES();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createVertexArray_70d7c4c62e00613d = function(arg0) {
            const ret = getObject(arg0).createVertexArray();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_ctrlKey_60b29e015a543678 = function(arg0) {
            const ret = getObject(arg0).ctrlKey;
            return ret;
        };
        imports.wbg.__wbg_ctrlKey_ab341328ab202d37 = function(arg0) {
            const ret = getObject(arg0).ctrlKey;
            return ret;
        };
        imports.wbg.__wbg_dataTransfer_e55d95fe65ed3f67 = function(arg0) {
            const ret = getObject(arg0).dataTransfer;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_data_955678973a75e5ba = function(arg0, arg1) {
            const ret = getObject(arg1).data;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_debug_d3ae65bab60caa5b = function(arg0, arg1) {
            console.debug(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_deleteBuffer_9d705222a7e77a7b = function(arg0, arg1) {
            getObject(arg0).deleteBuffer(getObject(arg1));
        };
        imports.wbg.__wbg_deleteBuffer_db0d7ecbaa97fa56 = function(arg0, arg1) {
            getObject(arg0).deleteBuffer(getObject(arg1));
        };
        imports.wbg.__wbg_deleteProgram_cb18e0020d488bad = function(arg0, arg1) {
            getObject(arg0).deleteProgram(getObject(arg1));
        };
        imports.wbg.__wbg_deleteProgram_e1a6c4b922e202e3 = function(arg0, arg1) {
            getObject(arg0).deleteProgram(getObject(arg1));
        };
        imports.wbg.__wbg_deleteShader_257caf93b24ac555 = function(arg0, arg1) {
            getObject(arg0).deleteShader(getObject(arg1));
        };
        imports.wbg.__wbg_deleteShader_fb86028e46cb069b = function(arg0, arg1) {
            getObject(arg0).deleteShader(getObject(arg1));
        };
        imports.wbg.__wbg_deleteTexture_6913f682a09c8171 = function(arg0, arg1) {
            getObject(arg0).deleteTexture(getObject(arg1));
        };
        imports.wbg.__wbg_deleteTexture_a29655740b1cbe33 = function(arg0, arg1) {
            getObject(arg0).deleteTexture(getObject(arg1));
        };
        imports.wbg.__wbg_deleteVertexArrayOES_2def7ce37f8e89f2 = function(arg0, arg1) {
            getObject(arg0).deleteVertexArrayOES(getObject(arg1));
        };
        imports.wbg.__wbg_deleteVertexArray_a3bca8e15204ffed = function(arg0, arg1) {
            getObject(arg0).deleteVertexArray(getObject(arg1));
        };
        imports.wbg.__wbg_deltaMode_a4cc321212f87817 = function(arg0) {
            const ret = getObject(arg0).deltaMode;
            return ret;
        };
        imports.wbg.__wbg_deltaX_27e2939a1af8c940 = function(arg0) {
            const ret = getObject(arg0).deltaX;
            return ret;
        };
        imports.wbg.__wbg_deltaY_4bb52a4f0a7ad28b = function(arg0) {
            const ret = getObject(arg0).deltaY;
            return ret;
        };
        imports.wbg.__wbg_detachShader_04216326458dc0a9 = function(arg0, arg1, arg2) {
            getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_detachShader_88bd32c18892fac1 = function(arg0, arg1, arg2) {
            getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_devicePixelContentBoxSize_1ea2c6145730b8c0 = function(arg0) {
            const ret = getObject(arg0).devicePixelContentBoxSize;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_devicePixelRatio_f4eb7cbe3a812de0 = function(arg0) {
            const ret = getObject(arg0).devicePixelRatio;
            return ret;
        };
        imports.wbg.__wbg_disableVertexAttribArray_c513f1fff3cb73f2 = function(arg0, arg1) {
            getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_disableVertexAttribArray_e7ff41dc0c3eaf1f = function(arg0, arg1) {
            getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_disable_4c1cedffa6646166 = function(arg0, arg1) {
            getObject(arg0).disable(arg1 >>> 0);
        };
        imports.wbg.__wbg_disable_5320561e5cb15f08 = function(arg0, arg1) {
            getObject(arg0).disable(arg1 >>> 0);
        };
        imports.wbg.__wbg_disconnect_c45e8044053eddf3 = function(arg0) {
            getObject(arg0).disconnect();
        };
        imports.wbg.__wbg_document_c488ca7509cc6938 = function(arg0) {
            const ret = getObject(arg0).document;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_done_adfd3f40364def50 = function(arg0) {
            const ret = getObject(arg0).done;
            return ret;
        };
        imports.wbg.__wbg_drawArrays_87e9bb0e2fb3e0fa = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_drawArrays_f5b0f0a0dc392c3f = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_drawElements_4711582129f6c013 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_drawElements_f8a2f5716d2414ff = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_elementFromPoint_6e4e1f5c8a377d85 = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).elementFromPoint(arg1, arg2);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_enableVertexAttribArray_0ce3052ae5f3f84a = function(arg0, arg1) {
            getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_enableVertexAttribArray_2bb681a583bf0dbe = function(arg0, arg1) {
            getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_enable_8d6ea7489b31dabd = function(arg0, arg1) {
            getObject(arg0).enable(arg1 >>> 0);
        };
        imports.wbg.__wbg_enable_bb868e19d5c88d56 = function(arg0, arg1) {
            getObject(arg0).enable(arg1 >>> 0);
        };
        imports.wbg.__wbg_error_2a6b93fdada7ff11 = function(arg0) {
            console.error(getObject(arg0));
        };
        imports.wbg.__wbg_error_40892570b9472688 = function(arg0, arg1) {
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
        imports.wbg.__wbg_fetch_ffad8c569a5e9c85 = function(arg0, arg1) {
            const ret = getObject(arg0).fetch(getObject(arg1));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_files_7925b63b783cb707 = function(arg0) {
            const ret = getObject(arg0).files;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_focus_c71947fc3fe22147 = function() { return handleError(function (arg0) {
            getObject(arg0).focus();
        }, arguments) };
        imports.wbg.__wbg_force_fd468d8bd1105322 = function(arg0) {
            const ret = getObject(arg0).force;
            return ret;
        };
        imports.wbg.__wbg_generateMipmap_76bf688783f59912 = function(arg0, arg1) {
            getObject(arg0).generateMipmap(arg1 >>> 0);
        };
        imports.wbg.__wbg_generateMipmap_ed4bc72d0d683666 = function(arg0, arg1) {
            getObject(arg0).generateMipmap(arg1 >>> 0);
        };
        imports.wbg.__wbg_getAttribLocation_2213adf3127f5256 = function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return ret;
        };
        imports.wbg.__wbg_getAttribLocation_69fb47a6468250a6 = function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return ret;
        };
        imports.wbg.__wbg_getBoundingClientRect_d5aa7383cf5c9a73 = function(arg0) {
            const ret = getObject(arg0).getBoundingClientRect();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_getComputedStyle_c3a9de7674a38310 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).getComputedStyle(getObject(arg1));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_getContext_24d4414b979c1bbd = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_getData_6beb356aa81b2753 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = getObject(arg1).getData(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getError_2ade39a55f13ea88 = function(arg0) {
            const ret = getObject(arg0).getError();
            return ret;
        };
        imports.wbg.__wbg_getError_f3261aa0f84ecd29 = function(arg0) {
            const ret = getObject(arg0).getError();
            return ret;
        };
        imports.wbg.__wbg_getExtension_28666bdc87d23aca = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_getExtension_d73649e3cf75a45f = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_getItem_561976eef304cebe = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = getObject(arg1).getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getParameter_304cffb9a759dc04 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).getParameter(arg1 >>> 0);
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_getParameter_fd65bc6ff1b0ffd9 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).getParameter(arg1 >>> 0);
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_getProgramInfoLog_032aac3e6f3a253c = function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getProgramInfoLog_039168c2aed8d3fe = function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getProgramParameter_70b22019524689fa = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_getProgramParameter_9b3bdf8d90159edb = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_getPropertyValue_e87121b8549f72d5 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = getObject(arg1).getPropertyValue(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getShaderInfoLog_5c7d45bafe3be3dd = function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getShaderInfoLog_d2cc881ce343a733 = function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getShaderParameter_6d0578dd9f58b684 = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_getShaderParameter_c50fbeadf9ef6879 = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_getSupportedExtensions_e153f28bc47a72f0 = function(arg0) {
            const ret = getObject(arg0).getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_getSupportedExtensions_fca342bac23691db = function(arg0) {
            const ret = getObject(arg0).getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_getTime_701326a7a826723f = function(arg0) {
            const ret = getObject(arg0).getTime();
            return ret;
        };
        imports.wbg.__wbg_getTimezoneOffset_e564c972d25502d1 = function(arg0) {
            const ret = getObject(arg0).getTimezoneOffset();
            return ret;
        };
        imports.wbg.__wbg_getUniformLocation_852fbe42afe106ff = function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_getUniformLocation_9d46a65011600cce = function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_get_62193fadfa67e6bc = function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_get_68aa371864aa301a = function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_get_92a4780a3beb5fe9 = function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_get_af324f3e968d37f8 = function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_get_d517571ff6ca648d = function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_globalThis_1e2ac1d6eee845b3 = function() { return handleError(function () {
            const ret = globalThis.globalThis;
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_global_f25a574ae080367c = function() { return handleError(function () {
            const ret = global.global;
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_hash_7f9b669d9748278e = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).hash;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_headers_1d68d1929689e9ed = function(arg0) {
            const ret = getObject(arg0).headers;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_headers_a5edfea2425875b2 = function(arg0) {
            const ret = getObject(arg0).headers;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_height_4065e49e5ec4c4c1 = function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        };
        imports.wbg.__wbg_height_e509816ec3fdf5b1 = function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        };
        imports.wbg.__wbg_hidden_62b8112083edecbf = function(arg0) {
            const ret = getObject(arg0).hidden;
            return ret;
        };
        imports.wbg.__wbg_host_7b8d981c6ad88028 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).host;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_hostname_dce7b3f0f39588c4 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).hostname;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_href_e702fa00b4409c7a = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_id_fe55568da8117231 = function(arg0, arg1) {
            const ret = getObject(arg1).id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_identifier_190ff6fc4b8c412f = function(arg0) {
            const ret = getObject(arg0).identifier;
            return ret;
        };
        imports.wbg.__wbg_info_bc5786afd6182908 = function(arg0, arg1) {
            console.info(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_inlineSize_6f8d0983462c2919 = function(arg0) {
            const ret = getObject(arg0).inlineSize;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Element_8d48056f7dc3afd9 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_HtmlElement_cf88a4b73702ca50 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_HtmlInputElement_d01f8554d1afb4b9 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof HTMLInputElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_ResizeObserverEntry_3b8a451fd881e4ee = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof ResizeObserverEntry;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_ResizeObserverSize_4e9c7f5bcb3f64bf = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof ResizeObserverSize;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Response_0ec26bd2f8a75ca2 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_TypeError_a07add5eaa7ffa60 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof TypeError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_WebGl2RenderingContext_888701598b82d45d = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof WebGL2RenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_WebGlRenderingContext_40ca2e1fd0dd70a8 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof WebGLRenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Window_a959820eb267fe22 = function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_isComposing_8a94b9b44a611f9b = function(arg0) {
            const ret = getObject(arg0).isComposing;
            return ret;
        };
        imports.wbg.__wbg_isComposing_8bc0758f907b31f6 = function(arg0) {
            const ret = getObject(arg0).isComposing;
            return ret;
        };
        imports.wbg.__wbg_isSecureContext_be7df9481b21ad0d = function(arg0) {
            const ret = getObject(arg0).isSecureContext;
            return ret;
        };
        imports.wbg.__wbg_is_20768e55ad2a7c3f = function(arg0, arg1) {
            const ret = Object.is(getObject(arg0), getObject(arg1));
            return ret;
        };
        imports.wbg.__wbg_item_4ab9e42b03a389fb = function(arg0, arg1) {
            const ret = getObject(arg0).item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_items_9b6cd46552011b58 = function(arg0) {
            const ret = getObject(arg0).items;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_iterator_7a20c20ce22add0f = function() {
            const ret = Symbol.iterator;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_keyCode_9fa1dd4d4dbccacc = function(arg0) {
            const ret = getObject(arg0).keyCode;
            return ret;
        };
        imports.wbg.__wbg_key_02315cd3f595756b = function(arg0, arg1) {
            const ret = getObject(arg1).key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_lastModified_b79e9e83d7e1059b = function(arg0) {
            const ret = getObject(arg0).lastModified;
            return ret;
        };
        imports.wbg.__wbg_left_20475bbabd8b02a8 = function(arg0) {
            const ret = getObject(arg0).left;
            return ret;
        };
        imports.wbg.__wbg_length_2e63ba34c4121df5 = function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        };
        imports.wbg.__wbg_length_2f85adaf7e2cf83e = function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        };
        imports.wbg.__wbg_length_6b0a67aa2ca7671a = function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        };
        imports.wbg.__wbg_length_a01c8a0710cec6f4 = function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        };
        imports.wbg.__wbg_length_e74df4881604f1d9 = function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        };
        imports.wbg.__wbg_linkProgram_575f761eda0a14bc = function(arg0, arg1) {
            getObject(arg0).linkProgram(getObject(arg1));
        };
        imports.wbg.__wbg_linkProgram_5eee13e603e9af41 = function(arg0, arg1) {
            getObject(arg0).linkProgram(getObject(arg1));
        };
        imports.wbg.__wbg_localStorage_05bfbeeb8946b5bf = function() { return handleError(function (arg0) {
            const ret = getObject(arg0).localStorage;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_location_54d35e8c85dcfb9c = function(arg0) {
            const ret = getObject(arg0).location;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_matchMedia_0be65181eeae951c = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_matches_254463383aee4688 = function(arg0) {
            const ret = getObject(arg0).matches;
            return ret;
        };
        imports.wbg.__wbg_matches_43eecfbacd820ac4 = function(arg0) {
            const ret = getObject(arg0).matches;
            return ret;
        };
        imports.wbg.__wbg_metaKey_34d5658170ffb3ee = function(arg0) {
            const ret = getObject(arg0).metaKey;
            return ret;
        };
        imports.wbg.__wbg_metaKey_6c8e9228e8dda152 = function(arg0) {
            const ret = getObject(arg0).metaKey;
            return ret;
        };
        imports.wbg.__wbg_name_1abd3f68be202781 = function(arg0, arg1) {
            const ret = getObject(arg1).name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_navigator_da495c9e52e160b1 = function(arg0) {
            const ret = getObject(arg0).navigator;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new0_207938728f108bf6 = function() {
            const ret = new Date();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_076cac58bb698dd4 = function() {
            const ret = new Object();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_0af70c8101def809 = function() {
            const ret = new Error();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_0c28e72025e00594 = function() {
            const ret = new Array();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_1e8ca58d170d6ad0 = function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return __wbg_adapter_636(a, state0.b, arg0, arg1);
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
        imports.wbg.__wbg_new_23362fa370a0a372 = function(arg0) {
            const ret = new Uint8Array(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_2f2bde6bba4a5707 = function(arg0) {
            const ret = new Date(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_4b15073a88792687 = function() { return handleError(function (arg0) {
            const ret = new ResizeObserver(getObject(arg0));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_newnoargs_19a249f4eceaaac3 = function(arg0, arg1) {
            const ret = new Function(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_24ff09a6b37a856f = function(arg0, arg1, arg2) {
            const ret = new Int16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_25d3cac011b6e2d5 = function(arg0, arg1, arg2) {
            const ret = new Uint32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_457c61bfe0fb7b8c = function(arg0, arg1, arg2) {
            const ret = new Int32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_9f48300371c8802a = function(arg0, arg1, arg2) {
            const ret = new Uint16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_a6087a94c7bfea61 = function(arg0, arg1, arg2) {
            const ret = new Int8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_c280c15b00e018cd = function(arg0, arg1, arg2) {
            const ret = new Float32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_ee8def7000b7b2be = function(arg0, arg1, arg2) {
            const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithrecordfromstrtoblobpromise_ae5831f3c12a27d6 = function() { return handleError(function (arg0) {
            const ret = new ClipboardItem(getObject(arg0));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_newwithstrandinit_ee1418802d8d481c = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_newwithtext_a3a54927289a7f87 = function() { return handleError(function (arg0, arg1) {
            const ret = new SpeechSynthesisUtterance(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_newwithu8arraysequenceandoptions_eca6efa84137af3c = function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_next_c591766a7286b02a = function() { return handleError(function (arg0) {
            const ret = getObject(arg0).next();
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_next_f387ecc56a94ba00 = function(arg0) {
            const ret = getObject(arg0).next;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_now_2c95c9de01293173 = function(arg0) {
            const ret = getObject(arg0).now();
            return ret;
        };
        imports.wbg.__wbg_now_5b0cbad8de553ec4 = function(arg0) {
            const ret = getObject(arg0).now();
            return ret;
        };
        imports.wbg.__wbg_observe_fd48955513eca909 = function(arg0, arg1, arg2) {
            getObject(arg0).observe(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_of_5ae3a2d893e18853 = function(arg0) {
            const ret = Array.of(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_offsetTop_a63a585c4631bbce = function(arg0) {
            const ret = getObject(arg0).offsetTop;
            return ret;
        };
        imports.wbg.__wbg_ok_4844a29ac7f98955 = function(arg0) {
            const ret = getObject(arg0).ok;
            return ret;
        };
        imports.wbg.__wbg_open_111256ae00fddfa1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_origin_e6426cdc04ec89f8 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).origin;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_performance_7a3ffd0b17f663ad = function(arg0) {
            const ret = getObject(arg0).performance;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_performance_ade89c628a3e4597 = function(arg0) {
            const ret = getObject(arg0).performance;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_pixelStorei_198b92c3e346678a = function(arg0, arg1, arg2) {
            getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_pixelStorei_9c4cb0a4b040b41d = function(arg0, arg1, arg2) {
            getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_port_2aa4001cc751af01 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).port;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_preventDefault_faafffcaad92972d = function(arg0) {
            getObject(arg0).preventDefault();
        };
        imports.wbg.__wbg_protocol_217a6f279ad0fa8c = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).protocol;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_push_3e9ce81246ef1d1b = function(arg0, arg1) {
            const ret = getObject(arg0).push(getObject(arg1));
            return ret;
        };
        imports.wbg.__wbg_queueMicrotask_5a8a9131f3f0b37b = function(arg0) {
            const ret = getObject(arg0).queueMicrotask;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_queueMicrotask_6d79674585219521 = function(arg0) {
            queueMicrotask(getObject(arg0));
        };
        imports.wbg.__wbg_readPixels_48fc96a447cda9aa = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
        }, arguments) };
        imports.wbg.__wbg_readPixels_7b1022930a9026d1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments) };
        imports.wbg.__wbg_readPixels_da6e94b84b4cfd41 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
        }, arguments) };
        imports.wbg.__wbg_removeEventListener_d14a328308e427ba = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
        }, arguments) };
        imports.wbg.__wbg_remove_7dd176d7be8b9e3a = function(arg0) {
            getObject(arg0).remove();
        };
        imports.wbg.__wbg_requestAnimationFrame_e8ca543d07df528e = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_resolve_6a311e8bb26423ab = function(arg0) {
            const ret = Promise.resolve(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_right_d476e01e3a36fd76 = function(arg0) {
            const ret = getObject(arg0).right;
            return ret;
        };
        imports.wbg.__wbg_scissor_4c06926fa8af817c = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).scissor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_scissor_608c4f610141e6df = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).scissor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_search_4c8c4c416a168e55 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).search;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_self_ac4343e4047b83cc = function() { return handleError(function () {
            const ret = self.self;
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_setAttribute_e5d83ecaf7f586d5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setItem_7a9a3aaeafde3c1f = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setProperty_b11b0bad191551d1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_set_421385e996a16e02 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_set_6e304ccd9a757a67 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_set_7b70226104a82921 = function(arg0, arg1, arg2) {
            getObject(arg0).set(getObject(arg1), arg2 >>> 0);
        };
        imports.wbg.__wbg_setautofocus_4268b2ccf2a3269c = function() { return handleError(function (arg0, arg1) {
            getObject(arg0).autofocus = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setbody_a548052400c35526 = function(arg0, arg1) {
            getObject(arg0).body = getObject(arg1);
        };
        imports.wbg.__wbg_setbox_f664fc1447c0b2bb = function(arg0, arg1) {
            getObject(arg0).box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
        };
        imports.wbg.__wbg_setheight_4286b13b9186d39f = function(arg0, arg1) {
            getObject(arg0).height = arg1 >>> 0;
        };
        imports.wbg.__wbg_setmethod_c704d56d480d8580 = function(arg0, arg1, arg2) {
            getObject(arg0).method = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setmode_26f3e7a9f55ddb2d = function(arg0, arg1) {
            getObject(arg0).mode = __wbindgen_enum_RequestMode[arg1];
        };
        imports.wbg.__wbg_setonce_fc9746c79ec638d1 = function(arg0, arg1) {
            getObject(arg0).once = arg1 !== 0;
        };
        imports.wbg.__wbg_setpitch_de7ce55d794d12dc = function(arg0, arg1) {
            getObject(arg0).pitch = arg1;
        };
        imports.wbg.__wbg_setrate_cc099905628dbb50 = function(arg0, arg1) {
            getObject(arg0).rate = arg1;
        };
        imports.wbg.__wbg_settabIndex_bc37dd560b089902 = function(arg0, arg1) {
            getObject(arg0).tabIndex = arg1;
        };
        imports.wbg.__wbg_settype_202db174d92fe493 = function(arg0, arg1, arg2) {
            getObject(arg0).type = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_settype_407738d1ed7fb627 = function(arg0, arg1, arg2) {
            getObject(arg0).type = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setvalue_c3b8653f179bcfd8 = function(arg0, arg1, arg2) {
            getObject(arg0).value = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setvolume_7744effc83619788 = function(arg0, arg1) {
            getObject(arg0).volume = arg1;
        };
        imports.wbg.__wbg_setwidth_5e43e6e177d3e2ec = function(arg0, arg1) {
            getObject(arg0).width = arg1 >>> 0;
        };
        imports.wbg.__wbg_shaderSource_7d9e91c6b9aaf864 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_shaderSource_b7db90958962e1f7 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_shiftKey_570898b1142a9898 = function(arg0) {
            const ret = getObject(arg0).shiftKey;
            return ret;
        };
        imports.wbg.__wbg_shiftKey_e90da27a3092777e = function(arg0) {
            const ret = getObject(arg0).shiftKey;
            return ret;
        };
        imports.wbg.__wbg_size_965da315036ee58c = function(arg0) {
            const ret = getObject(arg0).size;
            return ret;
        };
        imports.wbg.__wbg_speak_90b6f925e65b9380 = function(arg0, arg1) {
            getObject(arg0).speak(getObject(arg1));
        };
        imports.wbg.__wbg_speechSynthesis_4177f4a5fb6ed1b9 = function() { return handleError(function (arg0) {
            const ret = getObject(arg0).speechSynthesis;
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_stack_6d08a71c4c0cb36c = function(arg0, arg1) {
            const ret = getObject(arg1).stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_statusText_9d3182a125c063c7 = function(arg0, arg1) {
            const ret = getObject(arg1).statusText;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_status_5f9868b7ed8dd175 = function(arg0) {
            const ret = getObject(arg0).status;
            return ret;
        };
        imports.wbg.__wbg_stopPropagation_0ac50def48a51d8a = function(arg0) {
            getObject(arg0).stopPropagation();
        };
        imports.wbg.__wbg_style_e7c4e0938a7565b2 = function(arg0) {
            const ret = getObject(arg0).style;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_texImage2D_0054f31782d533db = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texImage2D_102612af3b3ea301 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments) };
        imports.wbg.__wbg_texImage2D_38f7a3dc4dcf0183 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments) };
        imports.wbg.__wbg_texParameteri_2cc96bb59a67d4c2 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_texParameteri_8e4109b7fbd3b875 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_texSubImage2D_3c9a9ceac3c27fe7 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_746d9e75d2dd12d1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_da8455e8da280cee = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments) };
        imports.wbg.__wbg_then_5c6469c1e1da9e59 = function(arg0, arg1) {
            const ret = getObject(arg0).then(getObject(arg1));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_then_faeb8aed8c1629b7 = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_top_6105791de23fffbe = function(arg0) {
            const ret = getObject(arg0).top;
            return ret;
        };
        imports.wbg.__wbg_touches_aeefd32ebb91cffb = function(arg0) {
            const ret = getObject(arg0).touches;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_type_a977b04d482f3f35 = function(arg0, arg1) {
            const ret = getObject(arg1).type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_type_de2569f85494aa87 = function(arg0, arg1) {
            const ret = getObject(arg1).type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_uniform1f_163e6ffe0d9805a4 = function(arg0, arg1, arg2) {
            getObject(arg0).uniform1f(getObject(arg1), arg2);
        };
        imports.wbg.__wbg_uniform1f_bb85eb8ed9248e52 = function(arg0, arg1, arg2) {
            getObject(arg0).uniform1f(getObject(arg1), arg2);
        };
        imports.wbg.__wbg_uniform1i_9fe01b91ff85aa85 = function(arg0, arg1, arg2) {
            getObject(arg0).uniform1i(getObject(arg1), arg2);
        };
        imports.wbg.__wbg_uniform1i_da7c764279d55bb5 = function(arg0, arg1, arg2) {
            getObject(arg0).uniform1i(getObject(arg1), arg2);
        };
        imports.wbg.__wbg_uniform2f_2cd6d040eb7c91e1 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
        };
        imports.wbg.__wbg_uniform2f_9a0ac4e03e84a890 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
        };
        imports.wbg.__wbg_url_ba6c16bbafb59895 = function(arg0, arg1) {
            const ret = getObject(arg1).url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_useProgram_795e70e5047fcb65 = function(arg0, arg1) {
            getObject(arg0).useProgram(getObject(arg1));
        };
        imports.wbg.__wbg_useProgram_e84b53bf74bbe9b3 = function(arg0, arg1) {
            getObject(arg0).useProgram(getObject(arg1));
        };
        imports.wbg.__wbg_userAgent_bfd54e5c60738678 = function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).userAgent;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_value_30db1d77772f3236 = function(arg0) {
            const ret = getObject(arg0).value;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_value_e88c0b5368388056 = function(arg0, arg1) {
            const ret = getObject(arg1).value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_vertexAttribPointer_1738c34c1c0d57a0 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        };
        imports.wbg.__wbg_vertexAttribPointer_34a2b143ee35746f = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        };
        imports.wbg.__wbg_viewport_04c48fc077486d94 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).viewport(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_viewport_301bba26f65246ed = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).viewport(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_warn_764c6c1d8f1e9651 = function(arg0, arg1) {
            console.warn(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_width_826b25a505a0b357 = function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        };
        imports.wbg.__wbg_width_dfc6149b0c4d8821 = function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        };
        imports.wbg.__wbg_window_1a23defd102c72f4 = function() { return handleError(function () {
            const ret = window.window;
            return addHeapObject(ret);
        }, arguments) };
        imports.wbg.__wbg_writeText_4abbbcc0bb5d06fb = function(arg0, arg1, arg2) {
            const ret = getObject(arg0).writeText(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_write_6271f5aa65db9f3f = function(arg0, arg1) {
            const ret = getObject(arg0).write(getObject(arg1));
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_boolean_get = function(arg0) {
            const v = getObject(arg0);
            const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
            return ret;
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
        imports.wbg.__wbindgen_closure_wrapper2997 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_32);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper2999 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_32);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper3001 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 906, __wbg_adapter_37);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper3566 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1172, __wbg_adapter_40);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
            const ret = debugString(getObject(arg1));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbindgen_in = function(arg0, arg1) {
            const ret = getObject(arg0) in getObject(arg1);
            return ret;
        };
        imports.wbg.__wbindgen_is_function = function(arg0) {
            const ret = typeof(getObject(arg0)) === 'function';
            return ret;
        };
        imports.wbg.__wbindgen_is_object = function(arg0) {
            const val = getObject(arg0);
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        };
        imports.wbg.__wbindgen_is_undefined = function(arg0) {
            const ret = getObject(arg0) === undefined;
            return ret;
        };
        imports.wbg.__wbindgen_memory = function() {
            const ret = wasm.memory;
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
            const obj = getObject(arg1);
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        };
        imports.wbg.__wbindgen_number_new = function(arg0) {
            const ret = arg0;
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
            const ret = getObject(arg0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
            takeObject(arg0);
        };
        imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
            const obj = getObject(arg1);
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
            const ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_throw = function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
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
