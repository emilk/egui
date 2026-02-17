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

    let cachedFloat32ArrayMemory0 = null;

    function getFloat32ArrayMemory0() {
        if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
            cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
        }
        return cachedFloat32ArrayMemory0;
    }

    function getArrayF32FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
    }

    let cachedInt32ArrayMemory0 = null;

    function getInt32ArrayMemory0() {
        if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
            cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
        }
        return cachedInt32ArrayMemory0;
    }

    function getArrayI32FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
    }

    let cachedUint32ArrayMemory0 = null;

    function getUint32ArrayMemory0() {
        if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
            cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
        }
        return cachedUint32ArrayMemory0;
    }

    function getArrayU32FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
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

    function takeFromExternrefTable0(idx) {
        const value = wasm.__wbindgen_export_1.get(idx);
        wasm.__externref_table_dealloc(idx);
        return value;
    }
    function __wbg_adapter_36(arg0, arg1) {
        const ret = wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h5bbec6e54db8da61_multivalue_shim(arg0, arg1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }

    function __wbg_adapter_39(arg0, arg1, arg2) {
        wasm.closure1270_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_44(arg0, arg1, arg2) {
        wasm.closure2238_externref_shim(arg0, arg1, arg2);
    }

    function __wbg_adapter_1408(arg0, arg1, arg2, arg3) {
        wasm.closure4163_externref_shim(arg0, arg1, arg2, arg3);
    }

    const __wbindgen_enum_GpuAddressMode = ["clamp-to-edge", "repeat", "mirror-repeat"];

    const __wbindgen_enum_GpuBlendFactor = ["zero", "one", "src", "one-minus-src", "src-alpha", "one-minus-src-alpha", "dst", "one-minus-dst", "dst-alpha", "one-minus-dst-alpha", "src-alpha-saturated", "constant", "one-minus-constant", "src1", "one-minus-src1", "src1-alpha", "one-minus-src1-alpha"];

    const __wbindgen_enum_GpuBlendOperation = ["add", "subtract", "reverse-subtract", "min", "max"];

    const __wbindgen_enum_GpuBufferBindingType = ["uniform", "storage", "read-only-storage"];

    const __wbindgen_enum_GpuCanvasAlphaMode = ["opaque", "premultiplied"];

    const __wbindgen_enum_GpuCompareFunction = ["never", "less", "equal", "less-equal", "greater", "not-equal", "greater-equal", "always"];

    const __wbindgen_enum_GpuCullMode = ["none", "front", "back"];

    const __wbindgen_enum_GpuFilterMode = ["nearest", "linear"];

    const __wbindgen_enum_GpuFrontFace = ["ccw", "cw"];

    const __wbindgen_enum_GpuIndexFormat = ["uint16", "uint32"];

    const __wbindgen_enum_GpuLoadOp = ["load", "clear"];

    const __wbindgen_enum_GpuMipmapFilterMode = ["nearest", "linear"];

    const __wbindgen_enum_GpuPowerPreference = ["low-power", "high-performance"];

    const __wbindgen_enum_GpuPrimitiveTopology = ["point-list", "line-list", "line-strip", "triangle-list", "triangle-strip"];

    const __wbindgen_enum_GpuSamplerBindingType = ["filtering", "non-filtering", "comparison"];

    const __wbindgen_enum_GpuStencilOperation = ["keep", "zero", "replace", "invert", "increment-clamp", "decrement-clamp", "increment-wrap", "decrement-wrap"];

    const __wbindgen_enum_GpuStorageTextureAccess = ["write-only", "read-only", "read-write"];

    const __wbindgen_enum_GpuStoreOp = ["store", "discard"];

    const __wbindgen_enum_GpuTextureAspect = ["all", "stencil-only", "depth-only"];

    const __wbindgen_enum_GpuTextureDimension = ["1d", "2d", "3d"];

    const __wbindgen_enum_GpuTextureFormat = ["r8unorm", "r8snorm", "r8uint", "r8sint", "r16uint", "r16sint", "r16float", "rg8unorm", "rg8snorm", "rg8uint", "rg8sint", "r32uint", "r32sint", "r32float", "rg16uint", "rg16sint", "rg16float", "rgba8unorm", "rgba8unorm-srgb", "rgba8snorm", "rgba8uint", "rgba8sint", "bgra8unorm", "bgra8unorm-srgb", "rgb9e5ufloat", "rgb10a2uint", "rgb10a2unorm", "rg11b10ufloat", "rg32uint", "rg32sint", "rg32float", "rgba16uint", "rgba16sint", "rgba16float", "rgba32uint", "rgba32sint", "rgba32float", "stencil8", "depth16unorm", "depth24plus", "depth24plus-stencil8", "depth32float", "depth32float-stencil8", "bc1-rgba-unorm", "bc1-rgba-unorm-srgb", "bc2-rgba-unorm", "bc2-rgba-unorm-srgb", "bc3-rgba-unorm", "bc3-rgba-unorm-srgb", "bc4-r-unorm", "bc4-r-snorm", "bc5-rg-unorm", "bc5-rg-snorm", "bc6h-rgb-ufloat", "bc6h-rgb-float", "bc7-rgba-unorm", "bc7-rgba-unorm-srgb", "etc2-rgb8unorm", "etc2-rgb8unorm-srgb", "etc2-rgb8a1unorm", "etc2-rgb8a1unorm-srgb", "etc2-rgba8unorm", "etc2-rgba8unorm-srgb", "eac-r11unorm", "eac-r11snorm", "eac-rg11unorm", "eac-rg11snorm", "astc-4x4-unorm", "astc-4x4-unorm-srgb", "astc-5x4-unorm", "astc-5x4-unorm-srgb", "astc-5x5-unorm", "astc-5x5-unorm-srgb", "astc-6x5-unorm", "astc-6x5-unorm-srgb", "astc-6x6-unorm", "astc-6x6-unorm-srgb", "astc-8x5-unorm", "astc-8x5-unorm-srgb", "astc-8x6-unorm", "astc-8x6-unorm-srgb", "astc-8x8-unorm", "astc-8x8-unorm-srgb", "astc-10x5-unorm", "astc-10x5-unorm-srgb", "astc-10x6-unorm", "astc-10x6-unorm-srgb", "astc-10x8-unorm", "astc-10x8-unorm-srgb", "astc-10x10-unorm", "astc-10x10-unorm-srgb", "astc-12x10-unorm", "astc-12x10-unorm-srgb", "astc-12x12-unorm", "astc-12x12-unorm-srgb"];

    const __wbindgen_enum_GpuTextureSampleType = ["float", "unfilterable-float", "depth", "sint", "uint"];

    const __wbindgen_enum_GpuTextureViewDimension = ["1d", "2d", "2d-array", "cube", "cube-array", "3d"];

    const __wbindgen_enum_GpuVertexFormat = ["uint8", "uint8x2", "uint8x4", "sint8", "sint8x2", "sint8x4", "unorm8", "unorm8x2", "unorm8x4", "snorm8", "snorm8x2", "snorm8x4", "uint16", "uint16x2", "uint16x4", "sint16", "sint16x2", "sint16x4", "unorm16", "unorm16x2", "unorm16x4", "snorm16", "snorm16x2", "snorm16x4", "float16", "float16x2", "float16x4", "float32", "float32x2", "float32x3", "float32x4", "uint32", "uint32x2", "uint32x3", "uint32x4", "sint32", "sint32x2", "sint32x3", "sint32x4", "unorm10-10-10-2", "unorm8x4-bgra"];

    const __wbindgen_enum_GpuVertexStepMode = ["vertex", "instance"];

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
         *
         * # Errors
         * Returns an error if the app could not start.
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
        imports.wbg.__wbg_Window_6419f7513544dd0b = function(arg0) {
            const ret = arg0.Window;
            return ret;
        };
        imports.wbg.__wbg_WorkerGlobalScope_147f18e856464ee4 = function(arg0) {
            const ret = arg0.WorkerGlobalScope;
            return ret;
        };
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
        imports.wbg.__wbg_beginQuery_6af0b28414b16c07 = function(arg0, arg1, arg2) {
            arg0.beginQuery(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_beginRenderPass_5959b1e03e4f545c = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.beginRenderPass(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_bindAttribLocation_40da4b3e84cc7bd5 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.bindAttribLocation(arg1, arg2 >>> 0, getStringFromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_bindAttribLocation_ce2730e29976d230 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.bindAttribLocation(arg1, arg2 >>> 0, getStringFromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_bindBufferRange_454f90f2b1781982 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.bindBufferRange(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
        };
        imports.wbg.__wbg_bindBuffer_309c9a6c21826cf5 = function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindBuffer_f32f587f1c2962a7 = function(arg0, arg1, arg2) {
            arg0.bindBuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindFramebuffer_bd02c8cc707d670f = function(arg0, arg1, arg2) {
            arg0.bindFramebuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindFramebuffer_e48e83c0f973944d = function(arg0, arg1, arg2) {
            arg0.bindFramebuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindRenderbuffer_53eedd88e52b4cb5 = function(arg0, arg1, arg2) {
            arg0.bindRenderbuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindRenderbuffer_55e205fecfddbb8c = function(arg0, arg1, arg2) {
            arg0.bindRenderbuffer(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_bindSampler_9f59cf2eaa22eee0 = function(arg0, arg1, arg2) {
            arg0.bindSampler(arg1 >>> 0, arg2);
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
        imports.wbg.__wbg_blendColor_15ba1eff44560932 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendColor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_blendColor_6446fba673f64ff0 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendColor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_blendEquationSeparate_c1aa26a9a5c5267e = function(arg0, arg1, arg2) {
            arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendEquationSeparate_f3d422e981d86339 = function(arg0, arg1, arg2) {
            arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendEquation_c23d111ad6d268ff = function(arg0, arg1) {
            arg0.blendEquation(arg1 >>> 0);
        };
        imports.wbg.__wbg_blendEquation_cec7bc41f3e5704c = function(arg0, arg1) {
            arg0.blendEquation(arg1 >>> 0);
        };
        imports.wbg.__wbg_blendFuncSeparate_483be8d4dd635340 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_blendFuncSeparate_dafeabfc1680b2ee = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_blendFunc_9454884a3cfd2911 = function(arg0, arg1, arg2) {
            arg0.blendFunc(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blendFunc_c3b74be5a39c665f = function(arg0, arg1, arg2) {
            arg0.blendFunc(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_blitFramebuffer_7303bdff77cfe967 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            arg0.blitFramebuffer(arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0);
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
        imports.wbg.__wbg_bufferData_463178757784fcac = function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        };
        imports.wbg.__wbg_bufferData_d99b6b4eb5283f20 = function(arg0, arg1, arg2, arg3) {
            arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        };
        imports.wbg.__wbg_bufferSubData_4e973eefe9236d04 = function(arg0, arg1, arg2, arg3) {
            arg0.bufferSubData(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_bufferSubData_dcd4d16031a60345 = function(arg0, arg1, arg2, arg3) {
            arg0.bufferSubData(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_buffer_09165b52af8c5237 = function(arg0) {
            const ret = arg0.buffer;
            return ret;
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
        imports.wbg.__wbg_clearBufferfv_65ea413f7f2554a2 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearBufferfv(arg1 >>> 0, arg2, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_clearBufferiv_c003c27b77a0245b = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearBufferiv(arg1 >>> 0, arg2, getArrayI32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_clearBufferuiv_8c285072f2026a37 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearBufferuiv(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_clearDepth_17cfee5be8476fae = function(arg0, arg1) {
            arg0.clearDepth(arg1);
        };
        imports.wbg.__wbg_clearDepth_670d19914a501259 = function(arg0, arg1) {
            arg0.clearDepth(arg1);
        };
        imports.wbg.__wbg_clearInterval_ad2594253cc39c4b = function(arg0, arg1) {
            arg0.clearInterval(arg1);
        };
        imports.wbg.__wbg_clearStencil_4323424f1acca0df = function(arg0, arg1) {
            arg0.clearStencil(arg1);
        };
        imports.wbg.__wbg_clearStencil_7addd3b330b56b27 = function(arg0, arg1) {
            arg0.clearStencil(arg1);
        };
        imports.wbg.__wbg_clear_62b9037b892f6988 = function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        };
        imports.wbg.__wbg_clear_f8d5f3c348d37d95 = function(arg0, arg1) {
            arg0.clear(arg1 >>> 0);
        };
        imports.wbg.__wbg_clientWaitSync_6930890a42bd44c0 = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.clientWaitSync(arg1, arg2 >>> 0, arg3 >>> 0);
            return ret;
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
        imports.wbg.__wbg_compressedTexSubImage2D_71877eec950ca069 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8, arg9);
        };
        imports.wbg.__wbg_compressedTexSubImage2D_99abf4cfdb7c3fd8 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            arg0.compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8);
        };
        imports.wbg.__wbg_compressedTexSubImage2D_d66dcfcb2422e703 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            arg0.compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8);
        };
        imports.wbg.__wbg_compressedTexSubImage3D_58506392da46b927 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            arg0.compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10);
        };
        imports.wbg.__wbg_compressedTexSubImage3D_81477746675a4017 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10, arg11);
        };
        imports.wbg.__wbg_configure_8d74ee79dc392b1f = function() { return handleError(function (arg0, arg1) {
            arg0.configure(arg1);
        }, arguments) };
        imports.wbg.__wbg_contentBoxSize_638692469db816f2 = function(arg0) {
            const ret = arg0.contentBoxSize;
            return ret;
        };
        imports.wbg.__wbg_contentRect_81407eb60e52248f = function(arg0) {
            const ret = arg0.contentRect;
            return ret;
        };
        imports.wbg.__wbg_copyBufferSubData_9469a965478e33b5 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.copyBufferSubData(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
        };
        imports.wbg.__wbg_copyTexSubImage2D_05e7e8df6814a705 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            arg0.copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
        };
        imports.wbg.__wbg_copyTexSubImage2D_607ad28606952982 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            arg0.copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
        };
        imports.wbg.__wbg_copyTexSubImage3D_32e92c94044e58ca = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.copyTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9);
        };
        imports.wbg.__wbg_copyTextureToBuffer_739b5accd0131afa = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.copyTextureToBuffer(arg1, arg2, arg3);
        }, arguments) };
        imports.wbg.__wbg_createBindGroupLayout_37b290868edc95c3 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBindGroupLayout(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createBindGroup_9e48ec0df6021806 = function(arg0, arg1) {
            const ret = arg0.createBindGroup(arg1);
            return ret;
        };
        imports.wbg.__wbg_createBuffer_301327852bcb0fc9 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBuffer(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createBuffer_7a9ec3d654073660 = function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createBuffer_9886e84a67b68c89 = function(arg0) {
            const ret = arg0.createBuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createCommandEncoder_f91fd6a7bbb31da6 = function(arg0, arg1) {
            const ret = arg0.createCommandEncoder(arg1);
            return ret;
        };
        imports.wbg.__wbg_createElement_8c9931a732ee2fea = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createFramebuffer_7824f69bba778885 = function(arg0) {
            const ret = arg0.createFramebuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createFramebuffer_c8d70ebc4858051e = function(arg0) {
            const ret = arg0.createFramebuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createPipelineLayout_e218679853a4ec90 = function(arg0, arg1) {
            const ret = arg0.createPipelineLayout(arg1);
            return ret;
        };
        imports.wbg.__wbg_createProgram_8ff56c485f3233d0 = function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createProgram_da203074cafb1038 = function(arg0) {
            const ret = arg0.createProgram();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createQuery_5ed5e770ec1009c1 = function(arg0) {
            const ret = arg0.createQuery();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createRenderPipeline_01226de8ac511c31 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createRenderPipeline(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_createRenderbuffer_d88aa9403faa38ea = function(arg0) {
            const ret = arg0.createRenderbuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createRenderbuffer_fd347ae14f262eaa = function(arg0) {
            const ret = arg0.createRenderbuffer();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createSampler_dd08c9ffd5b1afa4 = function(arg0, arg1) {
            const ret = arg0.createSampler(arg1);
            return ret;
        };
        imports.wbg.__wbg_createSampler_f76e29d7522bec9e = function(arg0) {
            const ret = arg0.createSampler();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createShaderModule_a7e2ac8c2d5bd874 = function(arg0, arg1) {
            const ret = arg0.createShaderModule(arg1);
            return ret;
        };
        imports.wbg.__wbg_createShader_4a256a8cc9c1ce4f = function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createShader_983150fb1243ee56 = function(arg0, arg1) {
            const ret = arg0.createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_createTexture_47efd1fcfeeaeac8 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createTexture(arg1);
            return ret;
        }, arguments) };
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
        imports.wbg.__wbg_createView_bb87ba5802a138dc = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createView(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_ctrlKey_1e826e468105ac11 = function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        };
        imports.wbg.__wbg_ctrlKey_cdbe8154dfb00d1f = function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        };
        imports.wbg.__wbg_cullFace_187079e6e20a464d = function(arg0, arg1) {
            arg0.cullFace(arg1 >>> 0);
        };
        imports.wbg.__wbg_cullFace_fbae6dd4d5e61ba4 = function(arg0, arg1) {
            arg0.cullFace(arg1 >>> 0);
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
        imports.wbg.__wbg_debug_21bee8b7f5110d62 = function(arg0, arg1) {
            console.debug(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_deleteBuffer_7ed96e1bf7c02e87 = function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        };
        imports.wbg.__wbg_deleteBuffer_a7822433fc95dfb8 = function(arg0, arg1) {
            arg0.deleteBuffer(arg1);
        };
        imports.wbg.__wbg_deleteFramebuffer_66853fb7101488cb = function(arg0, arg1) {
            arg0.deleteFramebuffer(arg1);
        };
        imports.wbg.__wbg_deleteFramebuffer_cd3285ee5a702a7a = function(arg0, arg1) {
            arg0.deleteFramebuffer(arg1);
        };
        imports.wbg.__wbg_deleteProgram_3fa626bbc0001eb7 = function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        };
        imports.wbg.__wbg_deleteProgram_71a133c6d053e272 = function(arg0, arg1) {
            arg0.deleteProgram(arg1);
        };
        imports.wbg.__wbg_deleteQuery_6a2b7cd30074b20b = function(arg0, arg1) {
            arg0.deleteQuery(arg1);
        };
        imports.wbg.__wbg_deleteRenderbuffer_59f4369653485031 = function(arg0, arg1) {
            arg0.deleteRenderbuffer(arg1);
        };
        imports.wbg.__wbg_deleteRenderbuffer_8808192853211567 = function(arg0, arg1) {
            arg0.deleteRenderbuffer(arg1);
        };
        imports.wbg.__wbg_deleteSampler_7f02bb003ba547f0 = function(arg0, arg1) {
            arg0.deleteSampler(arg1);
        };
        imports.wbg.__wbg_deleteShader_8d42f169deda58ac = function(arg0, arg1) {
            arg0.deleteShader(arg1);
        };
        imports.wbg.__wbg_deleteShader_c65a44796c5004d8 = function(arg0, arg1) {
            arg0.deleteShader(arg1);
        };
        imports.wbg.__wbg_deleteSync_5a3fbe5d6b742398 = function(arg0, arg1) {
            arg0.deleteSync(arg1);
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
        imports.wbg.__wbg_depthFunc_2906916f4536d5d7 = function(arg0, arg1) {
            arg0.depthFunc(arg1 >>> 0);
        };
        imports.wbg.__wbg_depthFunc_f34449ae87cc4e3e = function(arg0, arg1) {
            arg0.depthFunc(arg1 >>> 0);
        };
        imports.wbg.__wbg_depthMask_5fe84e2801488eda = function(arg0, arg1) {
            arg0.depthMask(arg1 !== 0);
        };
        imports.wbg.__wbg_depthMask_76688a8638b2f321 = function(arg0, arg1) {
            arg0.depthMask(arg1 !== 0);
        };
        imports.wbg.__wbg_depthRange_3cd6b4dc961d9116 = function(arg0, arg1, arg2) {
            arg0.depthRange(arg1, arg2);
        };
        imports.wbg.__wbg_depthRange_f9c084ff3d81fd7b = function(arg0, arg1, arg2) {
            arg0.depthRange(arg1, arg2);
        };
        imports.wbg.__wbg_destroy_1fb0841289b41ab7 = function(arg0) {
            arg0.destroy();
        };
        imports.wbg.__wbg_destroy_c98dc18b3a071e98 = function(arg0) {
            arg0.destroy();
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
        imports.wbg.__wbg_drawArraysInstancedANGLE_342ee6b5236d9702 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawArraysInstancedANGLE(arg1 >>> 0, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_drawArraysInstanced_622ea9f149b0b80c = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_drawArrays_6acaa2669c105f3a = function(arg0, arg1, arg2, arg3) {
            arg0.drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_drawArrays_6d29ea2ebc0c72a2 = function(arg0, arg1, arg2, arg3) {
            arg0.drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_drawBuffersWEBGL_9fdbdf3d4cbd3aae = function(arg0, arg1) {
            arg0.drawBuffersWEBGL(arg1);
        };
        imports.wbg.__wbg_drawBuffers_e729b75c5a50d760 = function(arg0, arg1) {
            arg0.drawBuffers(arg1);
        };
        imports.wbg.__wbg_drawElementsInstancedANGLE_096b48ab8686c5cf = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawElementsInstancedANGLE(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_drawElementsInstanced_f874e87d0b4e95e9 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawElementsInstanced(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_drawIndexed_3cb778da4c5793f5 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.drawIndexed(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5 >>> 0);
        };
        imports.wbg.__wbg_draw_35bd445973b180dc = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.draw(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
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
        imports.wbg.__wbg_endQuery_17aac36532ca7d47 = function(arg0, arg1) {
            arg0.endQuery(arg1 >>> 0);
        };
        imports.wbg.__wbg_end_ddc7a483fce32eed = function(arg0) {
            arg0.end();
        };
        imports.wbg.__wbg_error_524f506f44df1645 = function(arg0) {
            console.error(arg0);
        };
        imports.wbg.__wbg_error_541113e32ba1ecbd = function(arg0, arg1) {
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
        imports.wbg.__wbg_fenceSync_02d142d21e315da6 = function(arg0, arg1, arg2) {
            const ret = arg0.fenceSync(arg1 >>> 0, arg2 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_fetch_c7cf5ae5e39ce3a5 = function(arg0) {
            const ret = fetch(arg0);
            return ret;
        };
        imports.wbg.__wbg_files_5f07ac9b6f9116a7 = function(arg0) {
            const ret = arg0.files;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_finish_7c3e136077cc2230 = function(arg0) {
            const ret = arg0.finish();
            return ret;
        };
        imports.wbg.__wbg_finish_db51f74029254467 = function(arg0, arg1) {
            const ret = arg0.finish(arg1);
            return ret;
        };
        imports.wbg.__wbg_flush_4150080f65c49208 = function(arg0) {
            arg0.flush();
        };
        imports.wbg.__wbg_flush_987c35de09e06fd6 = function(arg0) {
            arg0.flush();
        };
        imports.wbg.__wbg_focus_7d08b55eba7b368d = function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments) };
        imports.wbg.__wbg_force_6e5acfdea2af0a4f = function(arg0) {
            const ret = arg0.force;
            return ret;
        };
        imports.wbg.__wbg_framebufferRenderbuffer_2fdd12e89ad81eb9 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_framebufferRenderbuffer_8b88592753b54715 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_framebufferTexture2D_81a565732bd5d8fe = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_framebufferTexture2D_ed855d0b097c557a = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_framebufferTextureLayer_5e6bd1b0cb45d815 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.framebufferTextureLayer(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
        };
        imports.wbg.__wbg_framebufferTextureMultiviewOVR_e54f936c3cc382cb = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.framebufferTextureMultiviewOVR(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5, arg6);
        };
        imports.wbg.__wbg_frontFace_289c9d7a8569c4f2 = function(arg0, arg1) {
            arg0.frontFace(arg1 >>> 0);
        };
        imports.wbg.__wbg_frontFace_4d4936cfaeb8b7df = function(arg0, arg1) {
            arg0.frontFace(arg1 >>> 0);
        };
        imports.wbg.__wbg_getBindGroupLayout_d087f5d30b56cb41 = function(arg0, arg1) {
            const ret = arg0.getBindGroupLayout(arg1 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_getBoundingClientRect_9073b0ff7574d76b = function(arg0) {
            const ret = arg0.getBoundingClientRect();
            return ret;
        };
        imports.wbg.__wbg_getBufferSubData_8ab2dcc5fcf5770f = function(arg0, arg1, arg2, arg3) {
            arg0.getBufferSubData(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_getComputedStyle_046dd6472f8e7f1d = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getComputedStyle(arg1);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getContext_3ae09aaa73194801 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2), arg3);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getContext_e9cf379449413580 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getContext_f65a0debd1e8f8e8 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getContext_fc19859df6331073 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2), arg3);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getCurrentTexture_b82524d31095411f = function() { return handleError(function (arg0) {
            const ret = arg0.getCurrentTexture();
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getData_84cc441a50843727 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getData(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getExtension_ff0fb1398bcf28c3 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_getIndexedParameter_f9211edc36533919 = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getIndexedParameter(arg1 >>> 0, arg2 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getItem_17f98dee3b43fa7e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments) };
        imports.wbg.__wbg_getMappedRange_98acf7ad62c501ee = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getMappedRange(arg1, arg2);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getParameter_1f0887a2b88e6d19 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getParameter(arg1 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getParameter_e3429f024018310f = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.getParameter(arg1 >>> 0);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_getPreferredCanvasFormat_92cc631581256e43 = function(arg0) {
            const ret = arg0.getPreferredCanvasFormat();
            return (__wbindgen_enum_GpuTextureFormat.indexOf(ret) + 1 || 96) - 1;
        };
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
        imports.wbg.__wbg_getQueryParameter_8921497e1d1561c1 = function(arg0, arg1, arg2) {
            const ret = arg0.getQueryParameter(arg1, arg2 >>> 0);
            return ret;
        };
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
        imports.wbg.__wbg_getSupportedExtensions_8c007dbb54905635 = function(arg0) {
            const ret = arg0.getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_getSupportedProfiles_10d2a4d32a128384 = function(arg0) {
            const ret = arg0.getSupportedProfiles();
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_getSyncParameter_7cb8461f5891606c = function(arg0, arg1, arg2) {
            const ret = arg0.getSyncParameter(arg1, arg2 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_getTime_46267b1c24877e30 = function(arg0) {
            const ret = arg0.getTime();
            return ret;
        };
        imports.wbg.__wbg_getTimezoneOffset_6b5752021c499c47 = function(arg0) {
            const ret = arg0.getTimezoneOffset();
            return ret;
        };
        imports.wbg.__wbg_getUniformBlockIndex_288fdc31528171ca = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.getUniformBlockIndex(arg1, getStringFromWasm0(arg2, arg3));
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
        imports.wbg.__wbg_get_e27dfaeb6f46bd45 = function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        };
        imports.wbg.__wbg_gpu_4b2187814fd587ca = function(arg0) {
            const ret = arg0.gpu;
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
        imports.wbg.__wbg_includes_937486a108ec147b = function(arg0, arg1, arg2) {
            const ret = arg0.includes(arg1, arg2);
            return ret;
        };
        imports.wbg.__wbg_info_2e7d618e9cb88d77 = function(arg0, arg1) {
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
        imports.wbg.__wbg_instanceof_GpuAdapter_5e451ad6596e2784 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUAdapter;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_GpuCanvasContext_f70ee27f49f4f884 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof GPUCanvasContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        };
        imports.wbg.__wbg_instanceof_HtmlCanvasElement_2ea67072a7624ac5 = function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLCanvasElement;
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
        imports.wbg.__wbg_invalidateFramebuffer_83f643d2a4936456 = function() { return handleError(function (arg0, arg1, arg2) {
            arg0.invalidateFramebuffer(arg1 >>> 0, arg2);
        }, arguments) };
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
        imports.wbg.__wbg_label_8296b38115112ca4 = function(arg0, arg1) {
            const ret = arg1.label;
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
        imports.wbg.__wbg_limits_b79b8275a12805b2 = function(arg0) {
            const ret = arg0.limits;
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
        imports.wbg.__wbg_mapAsync_2dba5c7b48d2e598 = function(arg0, arg1, arg2, arg3) {
            const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
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
        imports.wbg.__wbg_maxBindGroups_af2c64a371bc64b2 = function(arg0) {
            const ret = arg0.maxBindGroups;
            return ret;
        };
        imports.wbg.__wbg_maxBindingsPerBindGroup_430f6510523172d9 = function(arg0) {
            const ret = arg0.maxBindingsPerBindGroup;
            return ret;
        };
        imports.wbg.__wbg_maxBufferSize_68b45c1b69c22207 = function(arg0) {
            const ret = arg0.maxBufferSize;
            return ret;
        };
        imports.wbg.__wbg_maxColorAttachmentBytesPerSample_cbfce6f5737b4853 = function(arg0) {
            const ret = arg0.maxColorAttachmentBytesPerSample;
            return ret;
        };
        imports.wbg.__wbg_maxColorAttachments_70e7c33a58d9fc56 = function(arg0) {
            const ret = arg0.maxColorAttachments;
            return ret;
        };
        imports.wbg.__wbg_maxComputeInvocationsPerWorkgroup_4ad21bf35b7bd17f = function(arg0) {
            const ret = arg0.maxComputeInvocationsPerWorkgroup;
            return ret;
        };
        imports.wbg.__wbg_maxComputeWorkgroupSizeX_854c87a3ea2e5a00 = function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeX;
            return ret;
        };
        imports.wbg.__wbg_maxComputeWorkgroupSizeY_965ebcb7fee4acf5 = function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeY;
            return ret;
        };
        imports.wbg.__wbg_maxComputeWorkgroupSizeZ_3bf468106936874c = function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeZ;
            return ret;
        };
        imports.wbg.__wbg_maxComputeWorkgroupStorageSize_b9cab4f75b0f03e3 = function(arg0) {
            const ret = arg0.maxComputeWorkgroupStorageSize;
            return ret;
        };
        imports.wbg.__wbg_maxComputeWorkgroupsPerDimension_f4664066d76015da = function(arg0) {
            const ret = arg0.maxComputeWorkgroupsPerDimension;
            return ret;
        };
        imports.wbg.__wbg_maxDynamicStorageBuffersPerPipelineLayout_6b7faf56a6e328ad = function(arg0) {
            const ret = arg0.maxDynamicStorageBuffersPerPipelineLayout;
            return ret;
        };
        imports.wbg.__wbg_maxDynamicUniformBuffersPerPipelineLayout_22a38cc27e2f4626 = function(arg0) {
            const ret = arg0.maxDynamicUniformBuffersPerPipelineLayout;
            return ret;
        };
        imports.wbg.__wbg_maxSampledTexturesPerShaderStage_97c70c39fb197a2b = function(arg0) {
            const ret = arg0.maxSampledTexturesPerShaderStage;
            return ret;
        };
        imports.wbg.__wbg_maxSamplersPerShaderStage_a148c7e536a3807c = function(arg0) {
            const ret = arg0.maxSamplersPerShaderStage;
            return ret;
        };
        imports.wbg.__wbg_maxStorageBufferBindingSize_bfaa9c302ad157e3 = function(arg0) {
            const ret = arg0.maxStorageBufferBindingSize;
            return ret;
        };
        imports.wbg.__wbg_maxStorageBuffersPerShaderStage_463d04005d78f248 = function(arg0) {
            const ret = arg0.maxStorageBuffersPerShaderStage;
            return ret;
        };
        imports.wbg.__wbg_maxStorageTexturesPerShaderStage_3fe774bbe6ad1371 = function(arg0) {
            const ret = arg0.maxStorageTexturesPerShaderStage;
            return ret;
        };
        imports.wbg.__wbg_maxTextureArrayLayers_6b1a7b0b3b4c0556 = function(arg0) {
            const ret = arg0.maxTextureArrayLayers;
            return ret;
        };
        imports.wbg.__wbg_maxTextureDimension1D_e79117695a706815 = function(arg0) {
            const ret = arg0.maxTextureDimension1D;
            return ret;
        };
        imports.wbg.__wbg_maxTextureDimension2D_cbb3e7343bea93d1 = function(arg0) {
            const ret = arg0.maxTextureDimension2D;
            return ret;
        };
        imports.wbg.__wbg_maxTextureDimension3D_7ac996fb8fe18286 = function(arg0) {
            const ret = arg0.maxTextureDimension3D;
            return ret;
        };
        imports.wbg.__wbg_maxUniformBufferBindingSize_22c4f55b73d306cf = function(arg0) {
            const ret = arg0.maxUniformBufferBindingSize;
            return ret;
        };
        imports.wbg.__wbg_maxUniformBuffersPerShaderStage_65e2b2eaf78ef4e1 = function(arg0) {
            const ret = arg0.maxUniformBuffersPerShaderStage;
            return ret;
        };
        imports.wbg.__wbg_maxVertexAttributes_a6c97c2dc4a8d443 = function(arg0) {
            const ret = arg0.maxVertexAttributes;
            return ret;
        };
        imports.wbg.__wbg_maxVertexBufferArrayStride_305ba73c4de05f82 = function(arg0) {
            const ret = arg0.maxVertexBufferArrayStride;
            return ret;
        };
        imports.wbg.__wbg_maxVertexBuffers_df4a4911d2c540d8 = function(arg0) {
            const ret = arg0.maxVertexBuffers;
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
        imports.wbg.__wbg_minStorageBufferOffsetAlignment_12d731adbf75fd21 = function(arg0) {
            const ret = arg0.minStorageBufferOffsetAlignment;
            return ret;
        };
        imports.wbg.__wbg_minUniformBufferOffsetAlignment_2a0a0d2e84c280a7 = function(arg0) {
            const ret = arg0.minUniformBufferOffsetAlignment;
            return ret;
        };
        imports.wbg.__wbg_name_28c43f147574bf08 = function(arg0, arg1) {
            const ret = arg1.name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_navigator_0a9bf1120e24fec2 = function(arg0) {
            const ret = arg0.navigator;
            return ret;
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
                        return __wbg_adapter_1408(a, state0.b, arg0, arg1);
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
        imports.wbg.__wbg_new_358bba68c164c0c7 = function() {
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
        imports.wbg.__wbg_onSubmittedWorkDone_22f709e16b81d1c2 = function(arg0) {
            const ret = arg0.onSubmittedWorkDone();
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
        imports.wbg.__wbg_polygonOffset_773fe0017b2c8f51 = function(arg0, arg1, arg2) {
            arg0.polygonOffset(arg1, arg2);
        };
        imports.wbg.__wbg_polygonOffset_8c11c066486216c4 = function(arg0, arg1, arg2) {
            arg0.polygonOffset(arg1, arg2);
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
        imports.wbg.__wbg_queryCounterEXT_7aed85645b7ec1da = function(arg0, arg1, arg2) {
            arg0.queryCounterEXT(arg1, arg2 >>> 0);
        };
        imports.wbg.__wbg_querySelectorAll_40998fd748f057ef = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments) };
        imports.wbg.__wbg_querySelector_c69f8b573958906b = function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments) };
        imports.wbg.__wbg_queueMicrotask_97d92b4fcc8a61c5 = function(arg0) {
            queueMicrotask(arg0);
        };
        imports.wbg.__wbg_queueMicrotask_d3219def82552485 = function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        };
        imports.wbg.__wbg_queue_e7ab52ab0880dce9 = function(arg0) {
            const ret = arg0.queue;
            return ret;
        };
        imports.wbg.__wbg_readBuffer_1c35b1e4939f881d = function(arg0, arg1) {
            arg0.readBuffer(arg1 >>> 0);
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
        imports.wbg.__wbg_renderbufferStorageMultisample_13fbd5e58900c6fe = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.renderbufferStorageMultisample(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_renderbufferStorage_73e01ea83b8afab4 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
        };
        imports.wbg.__wbg_renderbufferStorage_f010012bd3566942 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
        };
        imports.wbg.__wbg_requestAdapter_127118e33ef3f15e = function(arg0) {
            const ret = arg0.requestAdapter();
            return ret;
        };
        imports.wbg.__wbg_requestAdapter_eb00393b717ebb9c = function(arg0, arg1) {
            const ret = arg0.requestAdapter(arg1);
            return ret;
        };
        imports.wbg.__wbg_requestAnimationFrame_d7fd890aaefc3246 = function() { return handleError(function (arg0, arg1) {
            const ret = arg0.requestAnimationFrame(arg1);
            return ret;
        }, arguments) };
        imports.wbg.__wbg_requestDevice_1be6e30ff9d67933 = function(arg0, arg1) {
            const ret = arg0.requestDevice(arg1);
            return ret;
        };
        imports.wbg.__wbg_resolve_4851785c9c5f573d = function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        };
        imports.wbg.__wbg_right_54416a875852cab1 = function(arg0) {
            const ret = arg0.right;
            return ret;
        };
        imports.wbg.__wbg_samplerParameterf_909baf50360c94d4 = function(arg0, arg1, arg2, arg3) {
            arg0.samplerParameterf(arg1, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_samplerParameteri_d5c292172718da63 = function(arg0, arg1, arg2, arg3) {
            arg0.samplerParameteri(arg1, arg2 >>> 0, arg3);
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
        imports.wbg.__wbg_setBindGroup_0ae63a01a1ed4c73 = function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_setBindGroup_d906e4c5d8533957 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments) };
        imports.wbg.__wbg_setIndexBuffer_c7ecba3588b25ce2 = function(arg0, arg1, arg2, arg3) {
            arg0.setIndexBuffer(arg1, __wbindgen_enum_GpuIndexFormat[arg2], arg3);
        };
        imports.wbg.__wbg_setIndexBuffer_db41507e5114fad4 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setIndexBuffer(arg1, __wbindgen_enum_GpuIndexFormat[arg2], arg3, arg4);
        };
        imports.wbg.__wbg_setItem_212ecc915942ab0a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setPipeline_b010841b1ab020c5 = function(arg0, arg1) {
            arg0.setPipeline(arg1);
        };
        imports.wbg.__wbg_setProperty_f2cf326652b9a713 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments) };
        imports.wbg.__wbg_setScissorRect_48aad86f2b04be65 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setScissorRect(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_setVertexBuffer_da6ef21c06e9c5ac = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_setVertexBuffer_f209d2bcc82ece37 = function(arg0, arg1, arg2, arg3) {
            arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_setViewport_bee857cbfc17f5bf = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setViewport(arg1, arg2, arg3, arg4, arg5, arg6);
        };
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
        imports.wbg.__wbg_seta_004bf5b9918b7a9d = function(arg0, arg1) {
            arg0.a = arg1;
        };
        imports.wbg.__wbg_setaccess_615d472480b556e8 = function(arg0, arg1) {
            arg0.access = __wbindgen_enum_GpuStorageTextureAccess[arg1];
        };
        imports.wbg.__wbg_setaddressmodeu_f8c82bdfe28ff814 = function(arg0, arg1) {
            arg0.addressModeU = __wbindgen_enum_GpuAddressMode[arg1];
        };
        imports.wbg.__wbg_setaddressmodev_15cc0a4331c8a793 = function(arg0, arg1) {
            arg0.addressModeV = __wbindgen_enum_GpuAddressMode[arg1];
        };
        imports.wbg.__wbg_setaddressmodew_b3ede4a69eef8df8 = function(arg0, arg1) {
            arg0.addressModeW = __wbindgen_enum_GpuAddressMode[arg1];
        };
        imports.wbg.__wbg_setalpha_7c9ec1b9552caf33 = function(arg0, arg1) {
            arg0.alpha = arg1;
        };
        imports.wbg.__wbg_setalphamode_d776091480150822 = function(arg0, arg1) {
            arg0.alphaMode = __wbindgen_enum_GpuCanvasAlphaMode[arg1];
        };
        imports.wbg.__wbg_setalphatocoverageenabled_97c65e8e0f0f97f0 = function(arg0, arg1) {
            arg0.alphaToCoverageEnabled = arg1 !== 0;
        };
        imports.wbg.__wbg_setarraylayercount_4b8708bd126ac758 = function(arg0, arg1) {
            arg0.arrayLayerCount = arg1 >>> 0;
        };
        imports.wbg.__wbg_setarraystride_89addb9ef89545a3 = function(arg0, arg1) {
            arg0.arrayStride = arg1;
        };
        imports.wbg.__wbg_setaspect_e672528231f771cb = function(arg0, arg1) {
            arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        };
        imports.wbg.__wbg_setattributes_2ab28c57eed0dc3a = function(arg0, arg1) {
            arg0.attributes = arg1;
        };
        imports.wbg.__wbg_setautofocus_6ca6f0ab5a566c21 = function() { return handleError(function (arg0, arg1) {
            arg0.autofocus = arg1 !== 0;
        }, arguments) };
        imports.wbg.__wbg_setb_b2b86286be8253f1 = function(arg0, arg1) {
            arg0.b = arg1;
        };
        imports.wbg.__wbg_setbasearraylayer_a3268c17b424196f = function(arg0, arg1) {
            arg0.baseArrayLayer = arg1 >>> 0;
        };
        imports.wbg.__wbg_setbasemiplevel_7ac60a20e24c81b1 = function(arg0, arg1) {
            arg0.baseMipLevel = arg1 >>> 0;
        };
        imports.wbg.__wbg_setbeginningofpasswriteindex_87e36fb6887d3c1c = function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        };
        imports.wbg.__wbg_setbindgrouplayouts_7fedf360e81319eb = function(arg0, arg1) {
            arg0.bindGroupLayouts = arg1;
        };
        imports.wbg.__wbg_setbinding_030f427cbe0e3a55 = function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        };
        imports.wbg.__wbg_setbinding_69fdec34b16b327b = function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        };
        imports.wbg.__wbg_setblend_c6896375c7f0119c = function(arg0, arg1) {
            arg0.blend = arg1;
        };
        imports.wbg.__wbg_setbody_5923b78a95eedf29 = function(arg0, arg1) {
            arg0.body = arg1;
        };
        imports.wbg.__wbg_setbox_2786f3ccea97cac4 = function(arg0, arg1) {
            arg0.box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
        };
        imports.wbg.__wbg_setbuffer_b70ef3f40d503e25 = function(arg0, arg1) {
            arg0.buffer = arg1;
        };
        imports.wbg.__wbg_setbuffer_b79f2efcb24ba844 = function(arg0, arg1) {
            arg0.buffer = arg1;
        };
        imports.wbg.__wbg_setbuffer_c23b131bfa95f222 = function(arg0, arg1) {
            arg0.buffer = arg1;
        };
        imports.wbg.__wbg_setbuffers_14ec06929ea541ec = function(arg0, arg1) {
            arg0.buffers = arg1;
        };
        imports.wbg.__wbg_setbytesperrow_279f81f686787a9f = function(arg0, arg1) {
            arg0.bytesPerRow = arg1 >>> 0;
        };
        imports.wbg.__wbg_setbytesperrow_fbb55671d2ba86f2 = function(arg0, arg1) {
            arg0.bytesPerRow = arg1 >>> 0;
        };
        imports.wbg.__wbg_setclearvalue_829dfd0db30aaeac = function(arg0, arg1) {
            arg0.clearValue = arg1;
        };
        imports.wbg.__wbg_setcode_09748e5373b711b2 = function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setcolor_96b2f28b4f51fceb = function(arg0, arg1) {
            arg0.color = arg1;
        };
        imports.wbg.__wbg_setcolorattachments_ee51f860224ee6dd = function(arg0, arg1) {
            arg0.colorAttachments = arg1;
        };
        imports.wbg.__wbg_setcompare_61125878543846d0 = function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        };
        imports.wbg.__wbg_setcompare_eb86f2890782b20b = function(arg0, arg1) {
            arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
        };
        imports.wbg.__wbg_setcount_4d43f3f3ab7f952d = function(arg0, arg1) {
            arg0.count = arg1 >>> 0;
        };
        imports.wbg.__wbg_setcullmode_4e0bb3799474c091 = function(arg0, arg1) {
            arg0.cullMode = __wbindgen_enum_GpuCullMode[arg1];
        };
        imports.wbg.__wbg_setdepthbias_ea8b79f02442c9c7 = function(arg0, arg1) {
            arg0.depthBias = arg1;
        };
        imports.wbg.__wbg_setdepthbiasclamp_5375d337b8b35cd8 = function(arg0, arg1) {
            arg0.depthBiasClamp = arg1;
        };
        imports.wbg.__wbg_setdepthbiasslopescale_0493feedbe6ad438 = function(arg0, arg1) {
            arg0.depthBiasSlopeScale = arg1;
        };
        imports.wbg.__wbg_setdepthclearvalue_20534499c6507e19 = function(arg0, arg1) {
            arg0.depthClearValue = arg1;
        };
        imports.wbg.__wbg_setdepthcompare_00e8b65c01d4bf03 = function(arg0, arg1) {
            arg0.depthCompare = __wbindgen_enum_GpuCompareFunction[arg1];
        };
        imports.wbg.__wbg_setdepthfailop_765de27464903fd0 = function(arg0, arg1) {
            arg0.depthFailOp = __wbindgen_enum_GpuStencilOperation[arg1];
        };
        imports.wbg.__wbg_setdepthloadop_33c128108a7dc8f1 = function(arg0, arg1) {
            arg0.depthLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        };
        imports.wbg.__wbg_setdepthorarraylayers_58d45a4c8cd4f655 = function(arg0, arg1) {
            arg0.depthOrArrayLayers = arg1 >>> 0;
        };
        imports.wbg.__wbg_setdepthreadonly_60990818c939df42 = function(arg0, arg1) {
            arg0.depthReadOnly = arg1 !== 0;
        };
        imports.wbg.__wbg_setdepthstencil_2e141a5dfe91878d = function(arg0, arg1) {
            arg0.depthStencil = arg1;
        };
        imports.wbg.__wbg_setdepthstencilattachment_47273ec480dd9bb3 = function(arg0, arg1) {
            arg0.depthStencilAttachment = arg1;
        };
        imports.wbg.__wbg_setdepthstoreop_9cf32660e51edb87 = function(arg0, arg1) {
            arg0.depthStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        };
        imports.wbg.__wbg_setdepthwriteenabled_2757b4106a089684 = function(arg0, arg1) {
            arg0.depthWriteEnabled = arg1 !== 0;
        };
        imports.wbg.__wbg_setdevice_c2cb3231e445ef7c = function(arg0, arg1) {
            arg0.device = arg1;
        };
        imports.wbg.__wbg_setdimension_0bc5536bd1965aea = function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureDimension[arg1];
        };
        imports.wbg.__wbg_setdimension_c7429fee9721a104 = function(arg0, arg1) {
            arg0.dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        };
        imports.wbg.__wbg_setdstfactor_976f0a83fd6ab733 = function(arg0, arg1) {
            arg0.dstFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        };
        imports.wbg.__wbg_setendofpasswriteindex_3cc5a7a3f6819a03 = function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        };
        imports.wbg.__wbg_setentries_01031c155d815ef1 = function(arg0, arg1) {
            arg0.entries = arg1;
        };
        imports.wbg.__wbg_setentries_8f49811ca79d7dbf = function(arg0, arg1) {
            arg0.entries = arg1;
        };
        imports.wbg.__wbg_setentrypoint_1da27599bf796782 = function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setentrypoint_670e208336b80723 = function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setexternaltexture_66700d1d2537a6de = function(arg0, arg1) {
            arg0.externalTexture = arg1;
        };
        imports.wbg.__wbg_setfailop_9de9bf69ac6682e3 = function(arg0, arg1) {
            arg0.failOp = __wbindgen_enum_GpuStencilOperation[arg1];
        };
        imports.wbg.__wbg_setformat_10a5222e02236027 = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        };
        imports.wbg.__wbg_setformat_37627c6070d0ecfc = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        };
        imports.wbg.__wbg_setformat_3c7d4bce3fb94de5 = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        };
        imports.wbg.__wbg_setformat_47fd2845afca8e1a = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        };
        imports.wbg.__wbg_setformat_72e1ce883fb57e05 = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        };
        imports.wbg.__wbg_setformat_877a89e3431cb656 = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuVertexFormat[arg1];
        };
        imports.wbg.__wbg_setformat_ee418ce830040f4d = function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        };
        imports.wbg.__wbg_setfragment_616c1d1c0db9abd4 = function(arg0, arg1) {
            arg0.fragment = arg1;
        };
        imports.wbg.__wbg_setfrontface_a1a0e940bd9fa3d0 = function(arg0, arg1) {
            arg0.frontFace = __wbindgen_enum_GpuFrontFace[arg1];
        };
        imports.wbg.__wbg_setg_9ab482dfe9422850 = function(arg0, arg1) {
            arg0.g = arg1;
        };
        imports.wbg.__wbg_sethasdynamicoffset_21302a736944b6d9 = function(arg0, arg1) {
            arg0.hasDynamicOffset = arg1 !== 0;
        };
        imports.wbg.__wbg_setheight_433680330c9420c3 = function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        };
        imports.wbg.__wbg_setheight_cd4d12f9029588ee = function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        };
        imports.wbg.__wbg_setheight_da683a33fa99843c = function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        };
        imports.wbg.__wbg_setlabel_0b21604c6a585153 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_1b7e4bc9d67c38b4 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_2e55e1407bac5ba2 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_407c8b09134f4f1d = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_5dc53fac7117f697 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_8e88157a8e30ddcd = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_a56a46194be79e8d = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_a6c76bf653812d73 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_ae972d3c351c79ec = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_b1b0d28716686810 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_cabc4eccde1e89fd = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_d90e07589bdb8f1a = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlabel_e69d774bf38947d2 = function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setlayout_3a36319a5990c8b7 = function(arg0, arg1) {
            arg0.layout = arg1;
        };
        imports.wbg.__wbg_setlayout_ac044d38ca30f520 = function(arg0, arg1) {
            arg0.layout = arg1;
        };
        imports.wbg.__wbg_setloadop_d48e31970a7bdf9b = function(arg0, arg1) {
            arg0.loadOp = __wbindgen_enum_GpuLoadOp[arg1];
        };
        imports.wbg.__wbg_setlodmaxclamp_150813b458d7989c = function(arg0, arg1) {
            arg0.lodMaxClamp = arg1;
        };
        imports.wbg.__wbg_setlodminclamp_444adbc1645f8521 = function(arg0, arg1) {
            arg0.lodMinClamp = arg1;
        };
        imports.wbg.__wbg_setmagfilter_4ce311d0e097cca4 = function(arg0, arg1) {
            arg0.magFilter = __wbindgen_enum_GpuFilterMode[arg1];
        };
        imports.wbg.__wbg_setmappedatcreation_34e7f793131eefbb = function(arg0, arg1) {
            arg0.mappedAtCreation = arg1 !== 0;
        };
        imports.wbg.__wbg_setmask_a51cdf9e56393e94 = function(arg0, arg1) {
            arg0.mask = arg1 >>> 0;
        };
        imports.wbg.__wbg_setmaxanisotropy_5be6e383b6e6632b = function(arg0, arg1) {
            arg0.maxAnisotropy = arg1;
        };
        imports.wbg.__wbg_setmethod_3c5280fe5d890842 = function(arg0, arg1, arg2) {
            arg0.method = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setminbindingsize_f9a65ac1a20ab955 = function(arg0, arg1) {
            arg0.minBindingSize = arg1;
        };
        imports.wbg.__wbg_setminfilter_87ee94d6dcfdc3d8 = function(arg0, arg1) {
            arg0.minFilter = __wbindgen_enum_GpuFilterMode[arg1];
        };
        imports.wbg.__wbg_setmiplevel_2d7e962e91fd1c33 = function(arg0, arg1) {
            arg0.mipLevel = arg1 >>> 0;
        };
        imports.wbg.__wbg_setmiplevelcount_32bbfdc1aebc8dd3 = function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        };
        imports.wbg.__wbg_setmiplevelcount_79f47bf6140098e5 = function(arg0, arg1) {
            arg0.mipLevelCount = arg1 >>> 0;
        };
        imports.wbg.__wbg_setmipmapfilter_1739c7c215847dc1 = function(arg0, arg1) {
            arg0.mipmapFilter = __wbindgen_enum_GpuMipmapFilterMode[arg1];
        };
        imports.wbg.__wbg_setmode_5dc300b865044b65 = function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_RequestMode[arg1];
        };
        imports.wbg.__wbg_setmodule_8ff6ea5431317fde = function(arg0, arg1) {
            arg0.module = arg1;
        };
        imports.wbg.__wbg_setmodule_dae95bb56c7d6ee9 = function(arg0, arg1) {
            arg0.module = arg1;
        };
        imports.wbg.__wbg_setmultisample_156e854358e208ff = function(arg0, arg1) {
            arg0.multisample = arg1;
        };
        imports.wbg.__wbg_setmultisampled_775f1e38d554a0f4 = function(arg0, arg1) {
            arg0.multisampled = arg1 !== 0;
        };
        imports.wbg.__wbg_setoffset_25f624abc0979ae4 = function(arg0, arg1) {
            arg0.offset = arg1;
        };
        imports.wbg.__wbg_setoffset_9cf47ca05ec82222 = function(arg0, arg1) {
            arg0.offset = arg1;
        };
        imports.wbg.__wbg_setoffset_9ed8011d53037f93 = function(arg0, arg1) {
            arg0.offset = arg1;
        };
        imports.wbg.__wbg_setoffset_d27243aad0b0b017 = function(arg0, arg1) {
            arg0.offset = arg1;
        };
        imports.wbg.__wbg_setonce_0cb80aea26303a35 = function(arg0, arg1) {
            arg0.once = arg1 !== 0;
        };
        imports.wbg.__wbg_setoperation_2ad26b5d94a70e63 = function(arg0, arg1) {
            arg0.operation = __wbindgen_enum_GpuBlendOperation[arg1];
        };
        imports.wbg.__wbg_setorigin_142f4ec35ba3f8da = function(arg0, arg1) {
            arg0.origin = arg1;
        };
        imports.wbg.__wbg_setpassop_25209e5db7ec5d4b = function(arg0, arg1) {
            arg0.passOp = __wbindgen_enum_GpuStencilOperation[arg1];
        };
        imports.wbg.__wbg_setpitch_5f1e968545051707 = function(arg0, arg1) {
            arg0.pitch = arg1;
        };
        imports.wbg.__wbg_setpowerpreference_2f983dce6d983584 = function(arg0, arg1) {
            arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        };
        imports.wbg.__wbg_setprimitive_cc91060b2752c577 = function(arg0, arg1) {
            arg0.primitive = arg1;
        };
        imports.wbg.__wbg_setqueryset_e258abc9e7072a65 = function(arg0, arg1) {
            arg0.querySet = arg1;
        };
        imports.wbg.__wbg_setr_4943e4c720ff77ca = function(arg0, arg1) {
            arg0.r = arg1;
        };
        imports.wbg.__wbg_setrate_e0aa4bfe9a720dc5 = function(arg0, arg1) {
            arg0.rate = arg1;
        };
        imports.wbg.__wbg_setrequiredfeatures_52447a9e50ed9b36 = function(arg0, arg1) {
            arg0.requiredFeatures = arg1;
        };
        imports.wbg.__wbg_setresolvetarget_28603a69bca08e48 = function(arg0, arg1) {
            arg0.resolveTarget = arg1;
        };
        imports.wbg.__wbg_setresource_0b72a17db4105dcc = function(arg0, arg1) {
            arg0.resource = arg1;
        };
        imports.wbg.__wbg_setrowsperimage_2388f2cfec4ea946 = function(arg0, arg1) {
            arg0.rowsPerImage = arg1 >>> 0;
        };
        imports.wbg.__wbg_setrowsperimage_d6b2e6d0385b8e27 = function(arg0, arg1) {
            arg0.rowsPerImage = arg1 >>> 0;
        };
        imports.wbg.__wbg_setsamplecount_1cd165278e1081cb = function(arg0, arg1) {
            arg0.sampleCount = arg1 >>> 0;
        };
        imports.wbg.__wbg_setsampler_9559ad3dd242f711 = function(arg0, arg1) {
            arg0.sampler = arg1;
        };
        imports.wbg.__wbg_setsampletype_5656761d1d13c084 = function(arg0, arg1) {
            arg0.sampleType = __wbindgen_enum_GpuTextureSampleType[arg1];
        };
        imports.wbg.__wbg_setshaderlocation_2ee098966925fd00 = function(arg0, arg1) {
            arg0.shaderLocation = arg1 >>> 0;
        };
        imports.wbg.__wbg_setsize_a43ef8b3ef024e2c = function(arg0, arg1) {
            arg0.size = arg1;
        };
        imports.wbg.__wbg_setsize_d3baf773adcc6357 = function(arg0, arg1) {
            arg0.size = arg1;
        };
        imports.wbg.__wbg_setsize_fadeb2bddc7e6f67 = function(arg0, arg1) {
            arg0.size = arg1;
        };
        imports.wbg.__wbg_setsrcfactor_ebc4adbcb746fedc = function(arg0, arg1) {
            arg0.srcFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        };
        imports.wbg.__wbg_setstencilback_51d5377faff8840b = function(arg0, arg1) {
            arg0.stencilBack = arg1;
        };
        imports.wbg.__wbg_setstencilclearvalue_21847cbc9881e39b = function(arg0, arg1) {
            arg0.stencilClearValue = arg1 >>> 0;
        };
        imports.wbg.__wbg_setstencilfront_115e8b375153cc55 = function(arg0, arg1) {
            arg0.stencilFront = arg1;
        };
        imports.wbg.__wbg_setstencilloadop_3531e7e23b9c735e = function(arg0, arg1) {
            arg0.stencilLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        };
        imports.wbg.__wbg_setstencilreadmask_6022bedf9e54ec0d = function(arg0, arg1) {
            arg0.stencilReadMask = arg1 >>> 0;
        };
        imports.wbg.__wbg_setstencilreadonly_beb27fbf4ca9b6e4 = function(arg0, arg1) {
            arg0.stencilReadOnly = arg1 !== 0;
        };
        imports.wbg.__wbg_setstencilstoreop_7b3259ed6b9d76ca = function(arg0, arg1) {
            arg0.stencilStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        };
        imports.wbg.__wbg_setstencilwritemask_294d575eb0e2fd6f = function(arg0, arg1) {
            arg0.stencilWriteMask = arg1 >>> 0;
        };
        imports.wbg.__wbg_setstepmode_5b6d687e55df5dd0 = function(arg0, arg1) {
            arg0.stepMode = __wbindgen_enum_GpuVertexStepMode[arg1];
        };
        imports.wbg.__wbg_setstoragetexture_b2963724a23aca9b = function(arg0, arg1) {
            arg0.storageTexture = arg1;
        };
        imports.wbg.__wbg_setstoreop_e1b7633c5612534a = function(arg0, arg1) {
            arg0.storeOp = __wbindgen_enum_GpuStoreOp[arg1];
        };
        imports.wbg.__wbg_setstripindexformat_6d0c95e2646c52d1 = function(arg0, arg1) {
            arg0.stripIndexFormat = __wbindgen_enum_GpuIndexFormat[arg1];
        };
        imports.wbg.__wbg_settabIndex_31adfec3c7eafbce = function(arg0, arg1) {
            arg0.tabIndex = arg1;
        };
        imports.wbg.__wbg_settargets_9f867a93d09515a9 = function(arg0, arg1) {
            arg0.targets = arg1;
        };
        imports.wbg.__wbg_settexture_08516f643ed9f7ef = function(arg0, arg1) {
            arg0.texture = arg1;
        };
        imports.wbg.__wbg_settexture_fbeffa5f2e57db49 = function(arg0, arg1) {
            arg0.texture = arg1;
        };
        imports.wbg.__wbg_settimestampwrites_94da76b5f3fee792 = function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        };
        imports.wbg.__wbg_settopology_0ef9190b0c51fc78 = function(arg0, arg1) {
            arg0.topology = __wbindgen_enum_GpuPrimitiveTopology[arg1];
        };
        imports.wbg.__wbg_settype_2a902a4a235bb64a = function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_settype_39ed370d3edd403c = function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_settype_657cd6d704dbc037 = function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuBufferBindingType[arg1];
        };
        imports.wbg.__wbg_settype_c9565dd4ebe21c60 = function(arg0, arg1) {
            arg0.type = __wbindgen_enum_GpuSamplerBindingType[arg1];
        };
        imports.wbg.__wbg_setunclippeddepth_936bc9a32a318b94 = function(arg0, arg1) {
            arg0.unclippedDepth = arg1 !== 0;
        };
        imports.wbg.__wbg_setusage_500c45ebe8b0bbf2 = function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        };
        imports.wbg.__wbg_setusage_9c6ccd6bcc15f735 = function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        };
        imports.wbg.__wbg_setusage_b84e5d16af27594a = function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        };
        imports.wbg.__wbg_setusage_e2790ec1205a5e27 = function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        };
        imports.wbg.__wbg_setvalue_6ad9ef6c692ea746 = function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        };
        imports.wbg.__wbg_setvertex_9c9752039687305f = function(arg0, arg1) {
            arg0.vertex = arg1;
        };
        imports.wbg.__wbg_setview_5aa6ed9f881b63f2 = function(arg0, arg1) {
            arg0.view = arg1;
        };
        imports.wbg.__wbg_setview_820375e4a740874f = function(arg0, arg1) {
            arg0.view = arg1;
        };
        imports.wbg.__wbg_setviewdimension_6ba3ac8e6bedbcb4 = function(arg0, arg1) {
            arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        };
        imports.wbg.__wbg_setviewdimension_95e6461d131f7086 = function(arg0, arg1) {
            arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        };
        imports.wbg.__wbg_setviewformats_6533614c7017475e = function(arg0, arg1) {
            arg0.viewFormats = arg1;
        };
        imports.wbg.__wbg_setviewformats_ff46db459c40096d = function(arg0, arg1) {
            arg0.viewFormats = arg1;
        };
        imports.wbg.__wbg_setvisibility_deca18896989c982 = function(arg0, arg1) {
            arg0.visibility = arg1 >>> 0;
        };
        imports.wbg.__wbg_setvolume_791fef19f3df9b00 = function(arg0, arg1) {
            arg0.volume = arg1;
        };
        imports.wbg.__wbg_setwidth_07eabc802de7b030 = function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        };
        imports.wbg.__wbg_setwidth_660ca581e3fbe279 = function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        };
        imports.wbg.__wbg_setwidth_c5fed9f5e7f0b406 = function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        };
        imports.wbg.__wbg_setwritemask_122c167c45bb2d8e = function(arg0, arg1) {
            arg0.writeMask = arg1 >>> 0;
        };
        imports.wbg.__wbg_setx_cc281962ce68ef00 = function(arg0, arg1) {
            arg0.x = arg1 >>> 0;
        };
        imports.wbg.__wbg_sety_7d6f1f0a01ce4000 = function(arg0, arg1) {
            arg0.y = arg1 >>> 0;
        };
        imports.wbg.__wbg_setz_b316da2a41e7822f = function(arg0, arg1) {
            arg0.z = arg1 >>> 0;
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
        imports.wbg.__wbg_size_beea1890c315fb17 = function(arg0) {
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
        imports.wbg.__wbg_stack_d87a83f5bc721084 = function(arg0, arg1) {
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
        imports.wbg.__wbg_stencilFuncSeparate_91700dcf367ae07e = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
        };
        imports.wbg.__wbg_stencilFuncSeparate_c1a6fa2005ca0aaf = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
        };
        imports.wbg.__wbg_stencilMaskSeparate_4f1a2defc8c10956 = function(arg0, arg1, arg2) {
            arg0.stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_stencilMaskSeparate_f8a0cfb5c2994d4a = function(arg0, arg1, arg2) {
            arg0.stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_stencilMask_1e602ef63f5b4144 = function(arg0, arg1) {
            arg0.stencilMask(arg1 >>> 0);
        };
        imports.wbg.__wbg_stencilMask_cd8ca0a55817e599 = function(arg0, arg1) {
            arg0.stencilMask(arg1 >>> 0);
        };
        imports.wbg.__wbg_stencilOpSeparate_1fa08985e79e1627 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_stencilOpSeparate_ff6683bbe3838ae6 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        };
        imports.wbg.__wbg_stopPropagation_11d220a858e5e0fb = function(arg0) {
            arg0.stopPropagation();
        };
        imports.wbg.__wbg_style_fb30c14e5815805c = function(arg0) {
            const ret = arg0.style;
            return ret;
        };
        imports.wbg.__wbg_submit_3ecd36be9abeba75 = function(arg0, arg1) {
            arg0.submit(arg1);
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
        imports.wbg.__wbg_texImage3D_921b54d09bf45af0 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            arg0.texImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8 >>> 0, arg9 >>> 0, arg10);
        }, arguments) };
        imports.wbg.__wbg_texImage3D_a00b7a4df48cf757 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            arg0.texImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8 >>> 0, arg9 >>> 0, arg10);
        }, arguments) };
        imports.wbg.__wbg_texParameteri_8112b26b3c360b7e = function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_texParameteri_ef50743cb94d507e = function(arg0, arg1, arg2, arg3) {
            arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_texStorage2D_fbda848497f3674e = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.texStorage2D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_texStorage3D_fd7a7ca30e7981d1 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.texStorage3D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5, arg6);
        };
        imports.wbg.__wbg_texSubImage2D_061605071aad9d2c = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_aa9a084093764796 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_c7951ed97252bdff = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_d52d1a0d3654c60b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_dd9cac68ad5fe0b6 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_e6d34f5bb062e404 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_f39ea52a2d4bd2f7 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage2D_fbdf91268228c757 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_04731251d7cecc83 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_37f0045d16871670 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_3a871f6405d2f183 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_66acd67f56e3b214 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_a051de089266fa1b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_b28c55f839bbec41 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments) };
        imports.wbg.__wbg_texSubImage3D_f18bf091cd48774c = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
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
        imports.wbg.__wbg_uniform1ui_66e092b67a21c84d = function(arg0, arg1, arg2) {
            arg0.uniform1ui(arg1, arg2 >>> 0);
        };
        imports.wbg.__wbg_uniform2fv_656fce9525420996 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2fv(arg1, getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform2fv_d8bd2a36da7ce440 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2fv(arg1, getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform2iv_4d39fc5a26f03f55 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2iv(arg1, getArrayI32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform2iv_e967139a28017a99 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2iv(arg1, getArrayI32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform2uiv_4c340c9e8477bb07 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform2uiv(arg1, getArrayU32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform3fv_7d828b7c4c91138e = function(arg0, arg1, arg2, arg3) {
            arg0.uniform3fv(arg1, getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform3fv_8153c834ce667125 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform3fv(arg1, getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform3iv_58662d914661aa10 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform3iv(arg1, getArrayI32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform3iv_f30d27ec224b4b24 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform3iv(arg1, getArrayI32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform3uiv_38673b825dc755f6 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform3uiv(arg1, getArrayU32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform4f_36b8f9be15064aa7 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.uniform4f(arg1, arg2, arg3, arg4, arg5);
        };
        imports.wbg.__wbg_uniform4f_f7ea07febf8b5108 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.uniform4f(arg1, arg2, arg3, arg4, arg5);
        };
        imports.wbg.__wbg_uniform4fv_8827081a7585145b = function(arg0, arg1, arg2, arg3) {
            arg0.uniform4fv(arg1, getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform4fv_c01fbc6c022abac3 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform4fv(arg1, getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform4iv_7fe05be291899f06 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform4iv(arg1, getArrayI32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform4iv_84fdf80745e7ff26 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform4iv(arg1, getArrayI32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniform4uiv_9de55998fbfef236 = function(arg0, arg1, arg2, arg3) {
            arg0.uniform4uiv(arg1, getArrayU32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniformBlockBinding_18117f4bda07115b = function(arg0, arg1, arg2, arg3) {
            arg0.uniformBlockBinding(arg1, arg2 >>> 0, arg3 >>> 0);
        };
        imports.wbg.__wbg_uniformMatrix2fv_98681e400347369c = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix2fv_bc019eb4784a3b8c = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix2x3fv_6421f8d6f7f4d144 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix2x3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix2x4fv_27d807767d7aadc6 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix2x4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix3fv_3d6ad3a1e0b0b5b6 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix3fv_3df529aab93cf902 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix3x2fv_79357317e9637d05 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix3x2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix3x4fv_9d1a88b5abfbd64b = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix3x4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix4fv_da94083874f202ad = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix4fv_e87383507ae75670 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix4x2fv_aa507d918a0b5a62 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix4x2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_uniformMatrix4x3fv_6712c7a3b4276fb4 = function(arg0, arg1, arg2, arg3, arg4) {
            arg0.uniformMatrix4x3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_unmap_2903d5b193373f12 = function(arg0) {
            arg0.unmap();
        };
        imports.wbg.__wbg_url_ae10c34ca209681d = function(arg0, arg1) {
            const ret = arg1.url;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        };
        imports.wbg.__wbg_usage_7b00ab14a235fa77 = function(arg0) {
            const ret = arg0.usage;
            return ret;
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
        imports.wbg.__wbg_vertexAttribDivisorANGLE_11e909d332960413 = function(arg0, arg1, arg2) {
            arg0.vertexAttribDivisorANGLE(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_vertexAttribDivisor_4d361d77ffb6d3ff = function(arg0, arg1, arg2) {
            arg0.vertexAttribDivisor(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_vertexAttribIPointer_d0c67543348c90ce = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.vertexAttribIPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
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
        imports.wbg.__wbg_warn_11b4e4f7bff9ffb7 = function(arg0, arg1) {
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
        imports.wbg.__wbg_writeBuffer_1897edb8e6677e9a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.writeBuffer(arg1, arg2, arg3, arg4, arg5);
        }, arguments) };
        imports.wbg.__wbg_writeText_51c338e8ae4b85b9 = function(arg0, arg1, arg2) {
            const ret = arg0.writeText(getStringFromWasm0(arg1, arg2));
            return ret;
        };
        imports.wbg.__wbg_writeTexture_e6008247063eadbf = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.writeTexture(arg1, arg2, arg3, arg4);
        }, arguments) };
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
        imports.wbg.__wbindgen_closure_wrapper4693 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1267, __wbg_adapter_36);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper4695 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1267, __wbg_adapter_39);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper4697 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 1267, __wbg_adapter_39);
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper6752 = function(arg0, arg1, arg2) {
            const ret = makeMutClosure(arg0, arg1, 2239, __wbg_adapter_44);
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
        imports.wbg.__wbindgen_is_null = function(arg0) {
            const ret = arg0 === null;
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
        cachedFloat32ArrayMemory0 = null;
        cachedInt32ArrayMemory0 = null;
        cachedUint32ArrayMemory0 = null;
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
