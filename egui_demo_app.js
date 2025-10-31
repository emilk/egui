let wasm_bindgen;
(function() {
    const __exports = {};
    let script_src;
    if (typeof document !== 'undefined' && document.currentScript !== null) {
        script_src = new URL(document.currentScript.src, location.href).toString();
    }
    let wasm = undefined;

    function isLikeNone(x) {
        return x === undefined || x === null;
    }

    function addToExternrefTable0(obj) {
        const idx = wasm.__externref_table_alloc();
        wasm.__wbindgen_export_1.set(idx, obj);
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
            const idx = addToExternrefTable0(e);
            wasm.__wbindgen_exn_store(idx);
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

    const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(state => {
        wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b)
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
                    wasm.__wbindgen_export_6.get(state.dtor)(a, state.b);
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
    function __wbg_adapter_34(arg0, arg1, arg2) {
        wasm.closure1188_externref_shim(arg0, arg1, arg2);
    }

    function takeFromExternrefTable0(idx) {
        const value = wasm.__wbindgen_export_1.get(idx);
        wasm.__externref_table_dealloc(idx);
        return value;
    }
    function __wbg_adapter_37(arg0, arg1) {
        const ret = wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h3f60b534b8335bd1_multivalue_shim(arg0, arg1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }

    function __wbg_adapter_42(arg0, arg1, arg2) {
        wasm.closure1408_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_646(arg0, arg1, arg2, arg3) {
        wasm.closure1448_externref_shim(arg0, arg1, arg2, arg3);
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
            const ret = wasm.webhandle_start(this.__wbg_ptr, canvas);
            return ret;
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
            const ret = wasm.webhandle_panic_message(this.__wbg_ptr);
            let v1;
            if (ret[0] !== 0) {
                v1 = getStringFromWasm0(ret[0], ret[1]).slice();
                wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
            }
            return v1;
        }
        /**
         * @returns {string | undefined}
         */
        panic_callstack() {
            const ret = wasm.webhandle_panic_callstack(this.__wbg_ptr);
            let v1;
            if (ret[0] !== 0) {
                v1 = getStringFromWasm0(ret[0], ret[1]).slice();
                wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
            }
            return v1;
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
        imports.wbg.__wbg_activeElement_367599fdfa7ad115 = function(arg0) {
            const ret = arg0.activeElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_activeElement_7cabba30de7b6b67 = function(arg0) {
            const ret = arg0.activeElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_activeTexture_0f19d8acfa0a14c2 = function(arg0, arg1) {
            arg0.activeTexture(arg1 >>> 0);
        };
        imports.wbg.__wbg_activeTexture_460f2e367e813fb0 = function(arg0, arg1) {
            arg0.activeTexture(arg1 >>> 0);
        };
        imports.wbg.__wbg_addEventListener_84ae3eac6e15480a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments) };
        imports.wbg.__wbg_altKey_c33c03aed82e4275 = function(arg0) {
            const ret = arg0.altKey;
            return ret;
        };
        imports.wbg.__wbg_altKey_d7495666df921121 = function(arg0) {
            const ret = arg0.altKey;
            return ret;
        };
        imports.wbg.__wbg_appendChild_8204974b7328bf98 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_arrayBuffer_d1b44c4390db422f = function() { return handleError(function (arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_arrayBuffer_f18c144cd0125f07 = function(arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        };
        imports.wbg.__wbg_at_7d852dd9f194d43e = function(arg0, arg1) {
            const ret = arg0.at(arg1);
            return ret;
        };
        imports.wbg.__wbg_attachShader_3d4eb6af9e3e7bd1 = function(arg0, arg1, arg2) {
            arg0.attachShader(arg1, arg2);
        };
        imports.wbg.__wbg_attachShader_94e758c8b5283eb2 = function(arg0, arg1, arg2) {
            arg0.attachShader(arg1, arg2);
        };
        imports.wbg.__wbg_bindBuffer_309c9a6c21826cf5 = function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindBuffer_f32f587f1c2962a7 = function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindTexture_a6e795697f49ebd1 = function(arg0, arg1, arg2) {
            arg0.bindTexture(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindTexture_bc8eb316247f739d = function(arg0, arg1, arg2) {
            arg0.bindTexture(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindVertexArrayOES_da8e7059b789629e = function(arg0, arg1) {
            arg0.bindVertexArrayOES(arg1);
        };
        imports.wbg.__wbg_bindVertexArray_6b4b88581064b71f = function(arg0, arg1) {
            arg0.bindVertexArray(arg1);
        };
        imports.wbg.__wbg_blendEquationSeparate_c1aa26a9a5c5267e = function(arg0, arg1, arg2) {
            arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendEquationSeparate_f3d422e981d86339 = function(arg0, arg1, arg2) {
            arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendFuncSeparate_483be8d4dd635340 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_blendFuncSeparate_dafeabfc1680b2ee = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_blockSize_1490803190b57a34 = function(arg0) {
            const ret = arg0.blockSize;
            return ret;
        };
        imports.wbg.__wbg_blur_c2ad8cc71bac3974 = function() { return handleError(function (arg0) {
            arg0.blur();
        }, arguments) };
        imports.wbg.__wbg_body_942ea927546a04ba = function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_bottom_79b03e9c3d6f4e1e = function(arg0) {
            const ret = arg0.bottom;
            return ret;
        };
        imports.wbg.__wbg_bufferData_3261d3e1dd6fc903 = function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        };
        imports.wbg.__wbg_bufferData_33c59bf909ea6fd3 = function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        };
        imports.wbg.__wbg_buffer_609cc3eee51ed158 = function(arg0) {
            const ret = arg0.buffer;
            return ret;
        };
        imports.wbg.__wbg_button_f75c56aec440ea04 = function(arg0) {
            const ret = arg0.button;
            return ret;
        };
        imports.wbg.__wbg_call_672a4d21634d4a24 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_call_7cccdd69e0791ae2 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_cancelAnimationFrame_089b48301c362fde = function() { return handleError(function (arg0, arg1) {
            arg0.cancelAnimationFrame(arg1);
        }, arguments) };
        imports.wbg.__wbg_cancel_ec9f8196f0b0eb21 = function(arg0) {
            arg0.cancel();
        };
        imports.wbg.__wbg_changedTouches_3654bea4294f2e86 = function(arg0) {
            const ret = arg0.changedTouches;
            return ret;
        };
        imports.wbg.__wbg_clearColor_d39507085c98a678 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearColor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_clearColor_f0fa029dfbcc1982 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearColor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_clearInterval_ad2594253cc39c4b = function(arg0, arg1) {
            arg0.clearInterval(arg1);
        };
        imports.wbg.__wbg_clear_62b9037b892f6988 = function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        };
        imports.wbg.__wbg_clear_f8d5f3c348d37d95 = function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        };
        imports.wbg.__wbg_clientX_5eb380a5f1fec6fd = function(arg0) {
            const ret = arg0.clientX;
            return ret;
        };
        imports.wbg.__wbg_clientX_687c1a16e03e1f58 = function(arg0) {
            const ret = arg0.clientX;
            return ret;
        };
        imports.wbg.__wbg_clientY_78d0605ac74642c2 = function(arg0) {
            const ret = arg0.clientY;
            return ret;
        };
        imports.wbg.__wbg_clientY_d8b9c7f0c4e2e677 = function(arg0) {
            const ret = arg0.clientY;
            return ret;
        };
        imports.wbg.__wbg_clipboardData_04bd9c1b0935d7e6 = function(arg0) {
            const ret = arg0.clipboardData;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_clipboard_93f8aa8cc426db44 = function(arg0) {
            const ret = arg0.clipboard;
            return ret;
        };
        imports.wbg.__wbg_colorMask_5e7c60b9c7a57a2e = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        };
        imports.wbg.__wbg_colorMask_6dac12039c7145ae = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        };
        imports.wbg.__wbg_compileShader_0ad770bbdbb9de21 = function(arg0, arg1) {
            arg0.compileShader(arg1);
        };
        imports.wbg.__wbg_compileShader_2307c9d370717dd5 = function(arg0, arg1) {
            arg0.compileShader(arg1);
        };
        imports.wbg.__wbg_contentBoxSize_638692469db816f2 = function(arg0) {
            const ret = arg0.contentBoxSize;
            return ret;
        };
        imports.wbg.__wbg_contentRect_81407eb60e52248f = function(arg0) {
            const ret = arg0.contentRect;
            return ret;
        };
        imports.wbg.__wbg_createBuffer_7a9ec3d654073660 = function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createBuffer_9886e84a67b68c89 = function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createElement_8c9931a732ee2fea = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createProgram_8ff56c485f3233d0 = function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createProgram_da203074cafb1038 = function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createShader_4a256a8cc9c1ce4f = function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createShader_983150fb1243ee56 = function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createTexture_9c536c79b635fdef = function(arg0) {
            const ret = arg0.createTexture();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createTexture_bfaa54c0cd22e367 = function(arg0) {
            const ret = arg0.createTexture();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createVertexArrayOES_991b44f100f93329 = function(arg0) {
            const ret = arg0.createVertexArrayOES();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createVertexArray_e435029ae2660efd = function(arg0) {
            const ret = arg0.createVertexArray();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_ctrlKey_1e826e468105ac11 = function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        };
        imports.wbg.__wbg_ctrlKey_cdbe8154dfb00d1f = function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        };
        imports.wbg.__wbg_dataTransfer_86283b0702a1aff1 = function(arg0) {
            const ret = arg0.dataTransfer;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_data_e77bd5c125ecc8a8 = function(arg0, arg1) {
            const ret = arg1.data;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_debug_dd144233080535a9 = function(arg0, arg1) {
            console.debug(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_deleteBuffer_7ed96e1bf7c02e87 = function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        };
        imports.wbg.__wbg_deleteBuffer_a7822433fc95dfb8 = function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        };
        imports.wbg.__wbg_deleteProgram_3fa626bbc0001eb7 = function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        };
        imports.wbg.__wbg_deleteProgram_71a133c6d053e272 = function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        };
        imports.wbg.__wbg_deleteShader_8d42f169deda58ac = function(arg0, arg1) {
            arg0.deleteShader(arg1);
        };
        imports.wbg.__wbg_deleteShader_c65a44796c5004d8 = function(arg0, arg1) {
            arg0.deleteShader(arg1);
        };
        imports.wbg.__wbg_deleteTexture_a30f5ca0163c4110 = function(arg0, arg1) {
            arg0.deleteTexture(arg1);
        };
        imports.wbg.__wbg_deleteTexture_bb82c9fec34372ba = function(arg0, arg1) {
            arg0.deleteTexture(arg1);
        };
        imports.wbg.__wbg_deleteVertexArrayOES_1ee7a06a4b23ec8c = function(arg0, arg1) {
            arg0.deleteVertexArrayOES(arg1);
        };
        imports.wbg.__wbg_deleteVertexArray_77fe73664a3332ae = function(arg0, arg1) {
            arg0.deleteVertexArray(arg1);
        };
        imports.wbg.__wbg_deltaMode_9bfd9fe3f6b4b240 = function(arg0) {
            const ret = arg0.deltaMode;
            return ret;
        };
        imports.wbg.__wbg_deltaX_5c1121715746e4b7 = function(arg0) {
            const ret = arg0.deltaX;
            return ret;
        };
        imports.wbg.__wbg_deltaY_f9318542caea0c36 = function(arg0) {
            const ret = arg0.deltaY;
            return ret;
        };
        imports.wbg.__wbg_detachShader_ab39d8a19811cfa6 = function(arg0, arg1, arg2) {
            arg0.detachShader(arg1, arg2);
        };
        imports.wbg.__wbg_detachShader_cd3ab294e635ff90 = function(arg0, arg1, arg2) {
            arg0.detachShader(arg1, arg2);
        };
        imports.wbg.__wbg_devicePixelContentBoxSize_a6de82cb30d70825 = function(arg0) {
            const ret = arg0.devicePixelContentBoxSize;
            return ret;
        };
        imports.wbg.__wbg_devicePixelRatio_68c391265f05d093 = function(arg0) {
            const ret = arg0.devicePixelRatio;
            return ret;
        };
        imports.wbg.__wbg_disableVertexAttribArray_452cc9815fced7e4 = function(arg0, arg1) {
            arg0.disableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_disableVertexAttribArray_afd097fb465dc100 = function(arg0, arg1) {
            arg0.disableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_disable_2702df5b5da5dd21 = function(arg0, arg1) {
            arg0.disable(arg1 >>> 0);
        };
        imports.wbg.__wbg_disable_8b53998501a7a85b = function(arg0, arg1) {
            arg0.disable(arg1 >>> 0);
        };
        imports.wbg.__wbg_disconnect_ac3f4ba550970c76 = function(arg0) {
            arg0.disconnect();
        };
        imports.wbg.__wbg_document_d249400bd7bd996d = function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_done_769e5ede4b31c67b = function(arg0) {
            const ret = arg0.done;
            return ret;
        };
        imports.wbg.__wbg_drawArrays_6acaa2669c105f3a = function(arg0, arg1, arg2, arg3) {
            arg0.drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_drawArrays_6d29ea2ebc0c72a2 = function(arg0, arg1, arg2, arg3) {
            arg0.drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_drawElements_16199ef1cc58b16a = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_drawElements_65cb4b099bd7d4ac = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_elementFromPoint_be6286b8ec1ae1a2 = function(arg0, arg1, arg2) {
            const ret = arg0.elementFromPoint(arg1, arg2);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_elementFromPoint_e788840a5168e09e = function(arg0, arg1, arg2) {
            const ret = arg0.elementFromPoint(arg1, arg2);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_enableVertexAttribArray_607be07574298e5e = function(arg0, arg1) {
            arg0.enableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_enableVertexAttribArray_93c3d406a41ad6c7 = function(arg0, arg1) {
            arg0.enableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbg_enable_51114837e05ee280 = function(arg0, arg1) {
            arg0.enable(arg1 >>> 0);
        };
        imports.wbg.__wbg_enable_d183fef39258803f = function(arg0, arg1) {
            arg0.enable(arg1 >>> 0);
        };
        imports.wbg.__wbg_error_524f506f44df1645 = function(arg0) {
            console.error(arg0);
        };
        imports.wbg.__wbg_error_8175373a88db6004 = function(arg0, arg1) {
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
        imports.wbg.__wbg_fetch_b7bf320f681242d2 = function(arg0, arg1) {
            const ret = arg0.fetch(arg1);
            return ret;
        };
        imports.wbg.__wbg_files_5f07ac9b6f9116a7 = function(arg0) {
            const ret = arg0.files;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_focus_7d08b55eba7b368d = function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments) };
        imports.wbg.__wbg_force_6e5acfdea2af0a4f = function(arg0) {
            const ret = arg0.force;
            return ret;
        };
        imports.wbg.__wbg_generateMipmap_13d3d6406de35b14 = function(arg0, arg1) {
            arg0.generateMipmap(arg1 >>> 0);
        };
        imports.wbg.__wbg_generateMipmap_e3b21a330b500089 = function(arg0, arg1) {
            arg0.generateMipmap(arg1 >>> 0);
        };
        imports.wbg.__wbg_getAttribLocation_959c0150cdd39cac = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getAttribLocation(arg1, getStringFromWasm0(arg2, arg3));
            return ret;
        };
        imports.wbg.__wbg_getAttribLocation_9db82d01924fa43d = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getAttribLocation(arg1, getStringFromWasm0(arg2, arg3));
            return ret;
        };
        imports.wbg.__wbg_getBoundingClientRect_9073b0ff7574d76b = function(arg0) {
            const ret = arg0.getBoundingClientRect();
            return ret;
        };
        imports.wbg.__wbg_getComputedStyle_046dd6472f8e7f1d = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getComputedStyle(arg1);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getContext_e9cf379449413580 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getData_84cc441a50843727 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getData(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getError_578ee28e31637d2f = function(arg0) {
            const ret = arg0.getError();
            return ret;
        };
        imports.wbg.__wbg_getError_d749701e28a45150 = function(arg0) {
            const ret = arg0.getError();
            return ret;
        };
        imports.wbg.__wbg_getExtension_e6c97409b224b5dc = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getExtension_ff0fb1398bcf28c3 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getItem_17f98dee3b43fa7e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getParameter_1f0887a2b88e6d19 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getParameter(arg1 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getParameter_e3429f024018310f = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getParameter(arg1 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getProgramInfoLog_631c180b1b21c8ed = function(arg0, arg1, arg2) {
            const ret = arg1.getProgramInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getProgramInfoLog_a998105a680059db = function(arg0, arg1, arg2) {
            const ret = arg1.getProgramInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getProgramParameter_0c411f0cd4185c5b = function(arg0, arg1, arg2) {
            const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_getProgramParameter_360f95ff07ac068d = function(arg0, arg1, arg2) {
            const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_getPropertyValue_e623c23a05dfb30c = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getPropertyValue(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getRootNode_f59bcfa355239af5 = function(arg0) {
            const ret = arg0.getRootNode();
            return ret;
        };
        imports.wbg.__wbg_getShaderInfoLog_7e7b38fb910ec534 = function(arg0, arg1, arg2) {
            const ret = arg1.getShaderInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getShaderInfoLog_f59c3112acc6e039 = function(arg0, arg1, arg2) {
            const ret = arg1.getShaderInfoLog(arg2);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_getShaderParameter_511b5f929074fa31 = function(arg0, arg1, arg2) {
            const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_getShaderParameter_6dbe0b8558dc41fd = function(arg0, arg1, arg2) {
            const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_getSupportedExtensions_3938cc3251d21f05 = function(arg0) {
            const ret = arg0.getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_getSupportedExtensions_8c007dbb54905635 = function(arg0) {
            const ret = arg0.getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_getTime_46267b1c24877e30 = function(arg0) {
            const ret = arg0.getTime();
            return ret;
        };
        imports.wbg.__wbg_getTimezoneOffset_6b5752021c499c47 = function(arg0) {
            const ret = arg0.getTimezoneOffset();
            return ret;
        };
        imports.wbg.__wbg_getUniformLocation_657a2b6d102bd126 = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_getUniformLocation_838363001c74dc21 = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_get_3091cb4339203d1a = function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_get_4095561f3d5ec806 = function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_get_67b2ba62fc30de12 = function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_get_8edd839202d9f4db = function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_get_b9b93047fe3cf45b = function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        };
        imports.wbg.__wbg_hash_dd4b49269c385c8a = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.hash;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_headers_7852a8ea641c1379 = function(arg0) {
            const ret = arg0.headers;
            return ret;
        };
        imports.wbg.__wbg_headers_9cb51cfd2ac780a4 = function(arg0) {
            const ret = arg0.headers;
            return ret;
        };
        imports.wbg.__wbg_height_1f8226c8f6875110 = function(arg0) {
            const ret = arg0.height;
            return ret;
        };
        imports.wbg.__wbg_height_838cee19ba8597db = function(arg0) {
            const ret = arg0.height;
            return ret;
        };
        imports.wbg.__wbg_hidden_d5c02c79a2b77bb6 = function(arg0) {
            const ret = arg0.hidden;
            return ret;
        };
        imports.wbg.__wbg_host_9bd7b5dc07c48606 = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.host;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_hostname_8d7204884eb7378b = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.hostname;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_href_87d60a783a012377 = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_id_c65402eae48fb242 = function(arg0, arg1) {
            const ret = arg1.id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_identifier_59e0705aef81ff93 = function(arg0) {
            const ret = arg0.identifier;
            return ret;
        };
        imports.wbg.__wbg_info_836c31b66e63847b = function(arg0, arg1) {
            console.info(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_inlineSize_8ff96b3ec1b24423 = function(arg0) {
            const ret = arg0.inlineSize;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Document_917b7ac52e42682e = function(arg0) {
            let result;
            try {
                result = arg0 instanceof Document;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Element_0af65443936d5154 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_HtmlElement_51378c201250b16c = function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_HtmlInputElement_12d71bf2d15dd19e = function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLInputElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_ResizeObserverEntry_cb85a268a84783ba = function(arg0) {
            let result;
            try {
                result = arg0 instanceof ResizeObserverEntry;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_ResizeObserverSize_4138fd53d59e1653 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof ResizeObserverSize;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Response_f2cc20d9f7dfd644 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_ShadowRoot_726578bcd7fa418a = function(arg0) {
            let result;
            try {
                result = arg0 instanceof ShadowRoot;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_TypeError_896f9e5789610ec3 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof TypeError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_WebGl2RenderingContext_2b6045efeb76568d = function(arg0) {
            let result;
            try {
                result = arg0 instanceof WebGL2RenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_WebGlRenderingContext_b9cbe798424f6d4c = function(arg0) {
            let result;
            try {
                result = arg0 instanceof WebGLRenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_Window_def73ea0955fc569 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_isComposing_36511555ff1869a4 = function(arg0) {
            const ret = arg0.isComposing;
            return ret;
        };
        imports.wbg.__wbg_isComposing_6e36768c82fd5a4f = function(arg0) {
            const ret = arg0.isComposing;
            return ret;
        };
        imports.wbg.__wbg_isSecureContext_aedcf3816338189a = function(arg0) {
            const ret = arg0.isSecureContext;
            return ret;
        };
        imports.wbg.__wbg_is_c7481c65e7e5df9e = function(arg0, arg1) {
            const ret = Object.is(arg0, arg1);
            return ret;
        };
        imports.wbg.__wbg_item_aea4b8432b5457be = function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_items_89c2afbece3a5d13 = function(arg0) {
            const ret = arg0.items;
            return ret;
        };
        imports.wbg.__wbg_iterator_9a24c88df860dc65 = function() {
            const ret = Symbol.iterator;
            return ret;
        };
        imports.wbg.__wbg_keyCode_237a8d1a040910b8 = function(arg0) {
            const ret = arg0.keyCode;
            return ret;
        };
        imports.wbg.__wbg_key_7b5c6cb539be8e13 = function(arg0, arg1) {
            const ret = arg1.key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_lastModified_7a9e61b3961224b8 = function(arg0) {
            const ret = arg0.lastModified;
            return ret;
        };
        imports.wbg.__wbg_left_e46801720267b66d = function(arg0) {
            const ret = arg0.left;
            return ret;
        };
        imports.wbg.__wbg_length_1d5c829e9b2319d6 = function(arg0) {
            const ret = arg0.length;
            return ret;
        };
        imports.wbg.__wbg_length_802483321c8130cf = function(arg0) {
            const ret = arg0.length;
            return ret;
        };
        imports.wbg.__wbg_length_a446193dc22c12f8 = function(arg0) {
            const ret = arg0.length;
            return ret;
        };
        imports.wbg.__wbg_length_cfc862ec0ccc7ca0 = function(arg0) {
            const ret = arg0.length;
            return ret;
        };
        imports.wbg.__wbg_length_e2d2a49132c1b256 = function(arg0) {
            const ret = arg0.length;
            return ret;
        };
        imports.wbg.__wbg_linkProgram_067ee06739bdde81 = function(arg0, arg1) {
            arg0.linkProgram(arg1);
        };
        imports.wbg.__wbg_linkProgram_e002979fe36e5b2a = function(arg0, arg1) {
            arg0.linkProgram(arg1);
        };
        imports.wbg.__wbg_localStorage_1406c99c39728187 = function() { return handleError(function (arg0) {
            const ret = arg0.localStorage;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_location_350d99456c2f3693 = function(arg0) {
            const ret = arg0.location;
            return ret;
        };
        imports.wbg.__wbg_matchMedia_bf8807a841d930c1 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.matchMedia(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_matches_e9ca73fbf8a3a104 = function(arg0) {
            const ret = arg0.matches;
            return ret;
        };
        imports.wbg.__wbg_metaKey_0b25f7848e014cc8 = function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        };
        imports.wbg.__wbg_metaKey_e1dd47d709a80ce5 = function(arg0) {
            const ret = arg0.metaKey;
            return ret;
        };
        imports.wbg.__wbg_name_28c43f147574bf08 = function(arg0, arg1) {
            const ret = arg1.name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_navigator_1577371c070c8947 = function(arg0) {
            const ret = arg0.navigator;
            return ret;
        };
        imports.wbg.__wbg_new0_f788a2397c7ca929 = function() {
            const ret = new Date();
            return ret;
        };
        imports.wbg.__wbg_new_23a2665fac83c611 = function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return __wbg_adapter_646(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = state0.b = 0;
            }
        };
        imports.wbg.__wbg_new_31a97dac4f10fab7 = function(arg0) {
            const ret = new Date(arg0);
            return ret;
        };
        imports.wbg.__wbg_new_3a326de758934c0f = function() {
            const ret = new Error();
            return ret;
        };
        imports.wbg.__wbg_new_405e22f390576ce2 = function() {
            const ret = new Object();
            return ret;
        };
        imports.wbg.__wbg_new_5f34cc0c99fcc488 = function() { return handleError(function (arg0) {
            const ret = new ResizeObserver(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_new_78feb108b6472713 = function() {
            const ret = new Array();
            return ret;
        };
        imports.wbg.__wbg_new_a12002a7f91c75be = function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        };
        imports.wbg.__wbg_newnoargs_105ed471475aaf50 = function(arg0, arg1) {
            const ret = new Function(getStringFromWasm0(arg0, arg1));
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_840f3c038856d4e9 = function(arg0, arg1, arg2) {
            const ret = new Int8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_999332a180064b59 = function(arg0, arg1, arg2) {
            const ret = new Int32Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_d4a86622320ea258 = function(arg0, arg1, arg2) {
            const ret = new Uint16Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_d97e637ebe145a9a = function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_e6b7e69acd4c7354 = function(arg0, arg1, arg2) {
            const ret = new Float32Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_f1dead44d1fc7212 = function(arg0, arg1, arg2) {
            const ret = new Uint32Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_f254047f7e80e7ff = function(arg0, arg1, arg2) {
            const ret = new Int16Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_newwithrecordfromstrtoblobpromise_53d3e3611a048f1e = function() { return handleError(function (arg0) {
            const ret = new ClipboardItem(arg0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithstrandinit_06c535e0a867c635 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithtext_4beba0b832dd9cc1 = function() { return handleError(function (arg0, arg1) {
            const ret = new SpeechSynthesisUtterance(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_newwithu8arraysequenceandoptions_068570c487f69127 = function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_next_25feadfc0913fea9 = function(arg0) {
            const ret = arg0.next;
            return ret;
        };
        imports.wbg.__wbg_next_6574e1a8a62d1055 = function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_now_2c95c9de01293173 = function(arg0) {
            const ret = arg0.now();
            return ret;
        };
        imports.wbg.__wbg_now_d18023d54d4e5500 = function(arg0) {
            const ret = arg0.now();
            return ret;
        };
        imports.wbg.__wbg_observe_ed4adb1c245103c5 = function(arg0, arg1, arg2) {
            arg0.observe(arg1, arg2);
        };
        imports.wbg.__wbg_of_2eaf5a02d443ef03 = function(arg0) {
            const ret = Array.of(arg0);
            return ret;
        };
        imports.wbg.__wbg_offsetTop_de8d0722bd1b211d = function(arg0) {
            const ret = arg0.offsetTop;
            return ret;
        };
        imports.wbg.__wbg_ok_3aaf32d069979723 = function(arg0) {
            const ret = arg0.ok;
            return ret;
        };
        imports.wbg.__wbg_open_6c3f5ef5a0204c5d = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_origin_7c5d649acdace3ea = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.origin;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_performance_7a3ffd0b17f663ad = function(arg0) {
            const ret = arg0.performance;
            return ret;
        };
        imports.wbg.__wbg_performance_c185c0cdc2766575 = function(arg0) {
            const ret = arg0.performance;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_pixelStorei_6aba5d04cdcaeaf6 = function(arg0, arg1, arg2) {
            arg0.pixelStorei(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_pixelStorei_c8520e4b46f4a973 = function(arg0, arg1, arg2) {
            arg0.pixelStorei(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_port_008e0061f421df1d = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.port;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_preventDefault_c2314fd813c02b3c = function(arg0) {
            arg0.preventDefault();
        };
        imports.wbg.__wbg_protocol_faa0494a9b2554cb = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.protocol;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_push_737cfc8c1432c2c6 = function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        };
        imports.wbg.__wbg_queueMicrotask_97d92b4fcc8a61c5 = function(arg0) {
            queueMicrotask(arg0);
        };
        imports.wbg.__wbg_queueMicrotask_d3219def82552485 = function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        };
        imports.wbg.__wbg_readPixels_51a0c02cdee207a5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments) };
        imports.wbg.__wbg_readPixels_a6cbb21794452142 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments) };
        imports.wbg.__wbg_readPixels_cd64c5a7b0343355 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments) };
        imports.wbg.__wbg_removeEventListener_056dfe8c3d6c58f9 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments) };
        imports.wbg.__wbg_remove_e2d2659f3128c045 = function(arg0) {
            arg0.remove();
        };
        imports.wbg.__wbg_requestAnimationFrame_d7fd890aaefc3246 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_resolve_4851785c9c5f573d = function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        };
        imports.wbg.__wbg_right_54416a875852cab1 = function(arg0) {
            const ret = arg0.right;
            return ret;
        };
        imports.wbg.__wbg_scissor_e917a332f67a5d30 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.scissor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_scissor_eb177ca33bf24a44 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.scissor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_search_c1c3bfbeadd96c47 = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.search;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_setAttribute_2704501201f15687 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setItem_212ecc915942ab0a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setProperty_f2cf326652b9a713 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_set_11cd83f45504cedf = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_set_65595bdd868b3009 = function(arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        };
        imports.wbg.__wbg_set_bb8cecf6a62b9f46 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_setautofocus_6ca6f0ab5a566c21 = function() { return handleError(function (arg0, arg1) {
            arg0.autofocus = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setbody_5923b78a95eedf29 = function(arg0, arg1) {
            arg0.body = arg1;
        };
        imports.wbg.__wbg_setbox_2786f3ccea97cac4 = function(arg0, arg1) {
            arg0.box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
        };
        imports.wbg.__wbg_setheight_da683a33fa99843c = function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        };
        imports.wbg.__wbg_setmethod_3c5280fe5d890842 = function(arg0, arg1, arg2) {
            arg0.method = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setmode_5dc300b865044b65 = function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_RequestMode[arg1];
        };
        imports.wbg.__wbg_setonce_0cb80aea26303a35 = function(arg0, arg1) {
            arg0.once = arg1 !== 0;
        };
        imports.wbg.__wbg_setpitch_5f1e968545051707 = function(arg0, arg1) {
            arg0.pitch = arg1;
        };
        imports.wbg.__wbg_setrate_e0aa4bfe9a720dc5 = function(arg0, arg1) {
            arg0.rate = arg1;
        };
        imports.wbg.__wbg_settabIndex_31adfec3c7eafbce = function(arg0, arg1) {
            arg0.tabIndex = arg1;
        };
        imports.wbg.__wbg_settype_2a902a4a235bb64a = function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_settype_39ed370d3edd403c = function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setvalue_6ad9ef6c692ea746 = function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setvolume_791fef19f3df9b00 = function(arg0, arg1) {
            arg0.volume = arg1;
        };
        imports.wbg.__wbg_setwidth_c5fed9f5e7f0b406 = function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        };
        imports.wbg.__wbg_shaderSource_72d3e8597ef85b67 = function(arg0, arg1, arg2, arg3) {
            arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_shaderSource_ad0087e637a35191 = function(arg0, arg1, arg2, arg3) {
            arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_shiftKey_2bebb3b703254f47 = function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        };
        imports.wbg.__wbg_shiftKey_86e737105bab1a54 = function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        };
        imports.wbg.__wbg_size_3808d41635a9c259 = function(arg0) {
            const ret = arg0.size;
            return ret;
        };
        imports.wbg.__wbg_speak_edb998564c00bb2a = function(arg0, arg1) {
            arg0.speak(arg1);
        };
        imports.wbg.__wbg_speechSynthesis_74e411ffcf3fc3c7 = function() { return handleError(function (arg0) {
            const ret = arg0.speechSynthesis;
            return ret;
        }, arguments) };
        imports.wbg.__wbg_stack_7ab0e52d303d5f54 = function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_static_accessor_GLOBAL_88a902d13a557d07 = function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0 = function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_static_accessor_SELF_37c5d418e4bf5819 = function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_static_accessor_WINDOW_5de37043a91a9c40 = function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_statusText_207754230b39e67c = function(arg0, arg1) {
            const ret = arg1.statusText;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_status_f6360336ca686bf0 = function(arg0) {
            const ret = arg0.status;
            return ret;
        };
        imports.wbg.__wbg_stopPropagation_11d220a858e5e0fb = function(arg0) {
            arg0.stopPropagation();
        };
        imports.wbg.__wbg_style_fb30c14e5815805c = function(arg0) {
            const ret = arg0.style;
            return ret;
        };
        imports.wbg.__wbg_texImage2D_57483314967bdd11 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texImage2D_5f2835f02b1d1077 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texImage2D_b8edcb5692f65f88 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texParameteri_8112b26b3c360b7e = function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_texParameteri_ef50743cb94d507e = function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_texSubImage2D_c7951ed97252bdff = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_d52d1a0d3654c60b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_fbdf91268228c757 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_then_44b73946d2fb3e7d = function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        };
        imports.wbg.__wbg_then_48b406749878a531 = function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        };
        imports.wbg.__wbg_top_ec9fceb1f030f2ea = function(arg0) {
            const ret = arg0.top;
            return ret;
        };
        imports.wbg.__wbg_touches_6831ee0099511603 = function(arg0) {
            const ret = arg0.touches;
            return ret;
        };
        imports.wbg.__wbg_type_00566e0d2e337e2e = function(arg0, arg1) {
            const ret = arg1.type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_type_20c7c49b2fbe0023 = function(arg0, arg1) {
            const ret = arg1.type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_uniform1f_21390b04609a9fa5 = function(arg0, arg1, arg2) {
            arg0.uniform1f(arg1, arg2);
        };
        imports.wbg.__wbg_uniform1f_dc009a0e7f7e5977 = function(arg0, arg1, arg2) {
            arg0.uniform1f(arg1, arg2);
        };
        imports.wbg.__wbg_uniform1i_5ddd9d8ccbd390bb = function(arg0, arg1, arg2) {
            arg0.uniform1i(arg1, arg2);
        };
        imports.wbg.__wbg_uniform1i_ed95b6129dce4d84 = function(arg0, arg1, arg2) {
            arg0.uniform1i(arg1, arg2);
        };
        imports.wbg.__wbg_uniform2f_56af4e1731d87421 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2f(arg1, arg2, arg3);
        };
        imports.wbg.__wbg_uniform2f_b69b5369bc019bd5 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2f(arg1, arg2, arg3);
        };
        imports.wbg.__wbg_url_ae10c34ca209681d = function(arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_useProgram_473bf913989b6089 = function(arg0, arg1) {
            arg0.useProgram(arg1);
        };
        imports.wbg.__wbg_useProgram_9b2660f7bb210471 = function(arg0, arg1) {
            arg0.useProgram(arg1);
        };
        imports.wbg.__wbg_userAgent_12e9d8e62297563f = function() { return handleError(function (arg0, arg1) {
            const ret = arg1.userAgent;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_value_91cbf0dd3ab84c1e = function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_value_cd1ffa7b1ab794f1 = function(arg0) {
            const ret = arg0.value;
            return ret;
        };
        imports.wbg.__wbg_vertexAttribPointer_550dc34903e3d1ea = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        };
        imports.wbg.__wbg_vertexAttribPointer_7a2a506cdbe3aebc = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        };
        imports.wbg.__wbg_viewport_a1b4d71297ba89af = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.viewport(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_viewport_e615e98f676f2d39 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.viewport(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_warn_536ec8b9339ed738 = function(arg0, arg1) {
            console.warn(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_width_5dde457d606ba683 = function(arg0) {
            const ret = arg0.width;
            return ret;
        };
        imports.wbg.__wbg_width_cdaf02311c1621d1 = function(arg0) {
            const ret = arg0.width;
            return ret;
        };
        imports.wbg.__wbg_writeText_51c338e8ae4b85b9 = function(arg0, arg1, arg2) {
            const ret = arg0.writeText(getStringFromWasm0(arg1, arg2));
            return ret;
        };
        imports.wbg.__wbg_write_e357400b06c0ccf5 = function(arg0, arg1) {
            const ret = arg0.write(arg1);
            return ret;
        };
        imports.wbg.__wbindgen_boolean_get = function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
            return ret;
        };
        imports.wbg.__wbindgen_cb_drop = function(arg0) {
            const obj = arg0.original;
            if (obj.cnt-- == 1) {
                obj.a = 0;
                return true;
            }
            const ret = false;
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper4143 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1189, __wbg_adapter_34);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper4145 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1189, __wbg_adapter_37);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper4147 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1189, __wbg_adapter_34);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper4701 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1409, __wbg_adapter_42);
            return ret;
        };
        imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbindgen_in = function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        };
        imports.wbg.__wbindgen_init_externref_table = function() {
            const table = wasm.__wbindgen_export_1;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
            ;
        };
        imports.wbg.__wbindgen_is_function = function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        };
        imports.wbg.__wbindgen_is_object = function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        };
        imports.wbg.__wbindgen_is_undefined = function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        };
        imports.wbg.__wbindgen_memory = function() {
            const ret = wasm.memory;
            return ret;
        };
        imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        };
        imports.wbg.__wbindgen_number_new = function(arg0) {
            const ret = arg0;
            return ret;
        };
        imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
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


        wasm.__wbindgen_start();
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
