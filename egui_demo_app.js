let wasm_bindgen = (function(exports) {
    let script_src;
    if (typeof document !== 'undefined' && document.currentScript !== null) {
        script_src = new URL(document.currentScript.src, location.href).toString();
    }

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
         * Installs a panic hook, then returns.
         */
        constructor() {
            const ret = wasm.webhandle_new();
            this.__wbg_ptr = ret >>> 0;
            WebHandleFinalization.register(this, this.__wbg_ptr, this);
            return this;
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
    }
    if (Symbol.dispose) WebHandle.prototype[Symbol.dispose] = WebHandle.prototype.free;
    exports.WebHandle = WebHandle;

    function __wbg_get_imports() {
        const import0 = {
            __proto__: null,
            __wbg_Window_130f91dd096c52b7: function(arg0) {
                const ret = arg0.Window;
                return ret;
            },
            __wbg_WorkerGlobalScope_09ad241aadeb3432: function(arg0) {
                const ret = arg0.WorkerGlobalScope;
                return ret;
            },
            __wbg___wbindgen_boolean_get_bbbb1c18aa2f5e25: function(arg0) {
                const v = arg0;
                const ret = typeof(v) === 'boolean' ? v : undefined;
                return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
            },
            __wbg___wbindgen_debug_string_0bc8482c6e3508ae: function(arg0, arg1) {
                const ret = debugString(arg1);
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg___wbindgen_in_47fa6863be6f2f25: function(arg0, arg1) {
                const ret = arg0 in arg1;
                return ret;
            },
            __wbg___wbindgen_is_function_0095a73b8b156f76: function(arg0) {
                const ret = typeof(arg0) === 'function';
                return ret;
            },
            __wbg___wbindgen_is_null_ac34f5003991759a: function(arg0) {
                const ret = arg0 === null;
                return ret;
            },
            __wbg___wbindgen_is_object_5ae8e5880f2c1fbd: function(arg0) {
                const val = arg0;
                const ret = typeof(val) === 'object' && val !== null;
                return ret;
            },
            __wbg___wbindgen_is_undefined_9e4d92534c42d778: function(arg0) {
                const ret = arg0 === undefined;
                return ret;
            },
            __wbg___wbindgen_number_get_8ff4255516ccad3e: function(arg0, arg1) {
                const obj = arg1;
                const ret = typeof(obj) === 'number' ? obj : undefined;
                getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
            },
            __wbg___wbindgen_string_get_72fb696202c56729: function(arg0, arg1) {
                const obj = arg1;
                const ret = typeof(obj) === 'string' ? obj : undefined;
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg___wbindgen_throw_be289d5034ed271b: function(arg0, arg1) {
                throw new Error(getStringFromWasm0(arg0, arg1));
            },
            __wbg__wbg_cb_unref_d9b87ff7982e3b21: function(arg0) {
                arg0._wbg_cb_unref();
            },
            __wbg_activeElement_1554b6917654f8d6: function(arg0) {
                const ret = arg0.activeElement;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_activeElement_d9d2a80dfafa67ed: function(arg0) {
                const ret = arg0.activeElement;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_activeTexture_6f9a710514686c24: function(arg0, arg1) {
                arg0.activeTexture(arg1 >>> 0);
            },
            __wbg_activeTexture_7e39cb8fdf4b6d5a: function(arg0, arg1) {
                arg0.activeTexture(arg1 >>> 0);
            },
            __wbg_addEventListener_c917b5aafbcf493f: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3, arg4);
            }, arguments); },
            __wbg_altKey_73c1173ba53073d5: function(arg0) {
                const ret = arg0.altKey;
                return ret;
            },
            __wbg_altKey_8155c319c215e3aa: function(arg0) {
                const ret = arg0.altKey;
                return ret;
            },
            __wbg_appendChild_dea38765a26d346d: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.appendChild(arg1);
                return ret;
            }, arguments); },
            __wbg_arrayBuffer_05ce1af23e9064e8: function(arg0) {
                const ret = arg0.arrayBuffer();
                return ret;
            },
            __wbg_arrayBuffer_bb54076166006c39: function() { return handleError(function (arg0) {
                const ret = arg0.arrayBuffer();
                return ret;
            }, arguments); },
            __wbg_at_dfc235641cc0e40c: function(arg0, arg1) {
                const ret = arg0.at(arg1);
                return ret;
            },
            __wbg_attachShader_32114efcf2744eb6: function(arg0, arg1, arg2) {
                arg0.attachShader(arg1, arg2);
            },
            __wbg_attachShader_b36058e5c9eeaf54: function(arg0, arg1, arg2) {
                arg0.attachShader(arg1, arg2);
            },
            __wbg_beginQuery_0fdf154e1da0e73d: function(arg0, arg1, arg2) {
                arg0.beginQuery(arg1 >>> 0, arg2);
            },
            __wbg_beginRenderPass_d629bb2679d62ed6: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.beginRenderPass(arg1);
                return ret;
            }, arguments); },
            __wbg_bindAttribLocation_5cfc7fa688df5051: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.bindAttribLocation(arg1, arg2 >>> 0, getStringFromWasm0(arg3, arg4));
            },
            __wbg_bindAttribLocation_ce78bfb13019dbe6: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.bindAttribLocation(arg1, arg2 >>> 0, getStringFromWasm0(arg3, arg4));
            },
            __wbg_bindBufferRange_009d206fe9e4151e: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.bindBufferRange(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
            },
            __wbg_bindBuffer_69a7a0b8f3f9b9cf: function(arg0, arg1, arg2) {
                arg0.bindBuffer(arg1 >>> 0, arg2);
            },
            __wbg_bindBuffer_c9068e8712a034f5: function(arg0, arg1, arg2) {
                arg0.bindBuffer(arg1 >>> 0, arg2);
            },
            __wbg_bindFramebuffer_031c73ba501cb8f6: function(arg0, arg1, arg2) {
                arg0.bindFramebuffer(arg1 >>> 0, arg2);
            },
            __wbg_bindFramebuffer_7815ca611abb057f: function(arg0, arg1, arg2) {
                arg0.bindFramebuffer(arg1 >>> 0, arg2);
            },
            __wbg_bindRenderbuffer_8a2aa4e3d1fb5443: function(arg0, arg1, arg2) {
                arg0.bindRenderbuffer(arg1 >>> 0, arg2);
            },
            __wbg_bindRenderbuffer_db37c1bac9ed4da0: function(arg0, arg1, arg2) {
                arg0.bindRenderbuffer(arg1 >>> 0, arg2);
            },
            __wbg_bindSampler_96f0e90e7bc31da9: function(arg0, arg1, arg2) {
                arg0.bindSampler(arg1 >>> 0, arg2);
            },
            __wbg_bindTexture_b2b7b1726a83f93e: function(arg0, arg1, arg2) {
                arg0.bindTexture(arg1 >>> 0, arg2);
            },
            __wbg_bindTexture_ec13ddcb9dc8e032: function(arg0, arg1, arg2) {
                arg0.bindTexture(arg1 >>> 0, arg2);
            },
            __wbg_bindVertexArrayOES_c2610602f7485b3f: function(arg0, arg1) {
                arg0.bindVertexArrayOES(arg1);
            },
            __wbg_bindVertexArray_78220d1edb1d2382: function(arg0, arg1) {
                arg0.bindVertexArray(arg1);
            },
            __wbg_blendColor_1d50ac87d9a2794b: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.blendColor(arg1, arg2, arg3, arg4);
            },
            __wbg_blendColor_e799d452ab2a5788: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.blendColor(arg1, arg2, arg3, arg4);
            },
            __wbg_blendEquationSeparate_1b12c43928cc7bc1: function(arg0, arg1, arg2) {
                arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_blendEquationSeparate_a8094fbec94cf80e: function(arg0, arg1, arg2) {
                arg0.blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_blendEquation_82202f34c4c00e50: function(arg0, arg1) {
                arg0.blendEquation(arg1 >>> 0);
            },
            __wbg_blendEquation_e9b99928ed1494ad: function(arg0, arg1) {
                arg0.blendEquation(arg1 >>> 0);
            },
            __wbg_blendFuncSeparate_95465944f788a092: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
            },
            __wbg_blendFuncSeparate_f366c170c5097fbe: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
            },
            __wbg_blendFunc_2ef59299d10c662d: function(arg0, arg1, arg2) {
                arg0.blendFunc(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_blendFunc_446658e7231ab9c8: function(arg0, arg1, arg2) {
                arg0.blendFunc(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_blitFramebuffer_d730a23ab4db248e: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
                arg0.blitFramebuffer(arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0);
            },
            __wbg_blockSize_ef9a626745d7dfac: function(arg0) {
                const ret = arg0.blockSize;
                return ret;
            },
            __wbg_blur_07f34335e06e5234: function() { return handleError(function (arg0) {
                arg0.blur();
            }, arguments); },
            __wbg_body_f67922363a220026: function(arg0) {
                const ret = arg0.body;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_bottom_c7ec510a18034add: function(arg0) {
                const ret = arg0.bottom;
                return ret;
            },
            __wbg_bufferData_1be8450fab534758: function(arg0, arg1, arg2, arg3) {
                arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
            },
            __wbg_bufferData_32d26eba0c74a53c: function(arg0, arg1, arg2, arg3) {
                arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
            },
            __wbg_bufferData_52235e85894af988: function(arg0, arg1, arg2, arg3) {
                arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
            },
            __wbg_bufferData_98f6c413a8f0f139: function(arg0, arg1, arg2, arg3) {
                arg0.bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
            },
            __wbg_bufferSubData_33eebcc173094f6a: function(arg0, arg1, arg2, arg3) {
                arg0.bufferSubData(arg1 >>> 0, arg2, arg3);
            },
            __wbg_bufferSubData_3e902f031adf13fd: function(arg0, arg1, arg2, arg3) {
                arg0.bufferSubData(arg1 >>> 0, arg2, arg3);
            },
            __wbg_button_d86841d0a03adc44: function(arg0) {
                const ret = arg0.button;
                return ret;
            },
            __wbg_call_389efe28435a9388: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.call(arg1);
                return ret;
            }, arguments); },
            __wbg_call_4708e0c13bdc8e95: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.call(arg1, arg2);
                return ret;
            }, arguments); },
            __wbg_cancelAnimationFrame_cd35895d78cf4510: function() { return handleError(function (arg0, arg1) {
                arg0.cancelAnimationFrame(arg1);
            }, arguments); },
            __wbg_cancel_8f4e3a220b2f0fe1: function(arg0) {
                arg0.cancel();
            },
            __wbg_changedTouches_b6ab7be7b1aed8d6: function(arg0) {
                const ret = arg0.changedTouches;
                return ret;
            },
            __wbg_clearBufferfv_ac87d92e2f45d80c: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.clearBufferfv(arg1 >>> 0, arg2, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_clearBufferiv_69ff24bb52ec4c88: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.clearBufferiv(arg1 >>> 0, arg2, getArrayI32FromWasm0(arg3, arg4));
            },
            __wbg_clearBufferuiv_8ad59a8219aafaca: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.clearBufferuiv(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4));
            },
            __wbg_clearDepth_2b109f644a783a53: function(arg0, arg1) {
                arg0.clearDepth(arg1);
            },
            __wbg_clearDepth_670099db422a4f91: function(arg0, arg1) {
                arg0.clearDepth(arg1);
            },
            __wbg_clearInterval_c75df0651e74fbb8: function(arg0, arg1) {
                arg0.clearInterval(arg1);
            },
            __wbg_clearStencil_5d243d0dff03c315: function(arg0, arg1) {
                arg0.clearStencil(arg1);
            },
            __wbg_clearStencil_aa65955bb39d8c18: function(arg0, arg1) {
                arg0.clearStencil(arg1);
            },
            __wbg_clear_4d801d0d054c3579: function(arg0, arg1) {
                arg0.clear(arg1 >>> 0);
            },
            __wbg_clear_7187030f892c5ca0: function(arg0, arg1) {
                arg0.clear(arg1 >>> 0);
            },
            __wbg_clientWaitSync_21865feaeb76a9a5: function(arg0, arg1, arg2, arg3) {
                const ret = arg0.clientWaitSync(arg1, arg2 >>> 0, arg3 >>> 0);
                return ret;
            },
            __wbg_clientX_a3c5f4ff30e91264: function(arg0) {
                const ret = arg0.clientX;
                return ret;
            },
            __wbg_clientX_ed7d2827ca30c165: function(arg0) {
                const ret = arg0.clientX;
                return ret;
            },
            __wbg_clientY_79ab4711d0597b2c: function(arg0) {
                const ret = arg0.clientY;
                return ret;
            },
            __wbg_clientY_e28509acb9b4a42a: function(arg0) {
                const ret = arg0.clientY;
                return ret;
            },
            __wbg_clipboardData_018789e461e23aaa: function(arg0) {
                const ret = arg0.clipboardData;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_clipboard_98c5a32249fa8416: function(arg0) {
                const ret = arg0.clipboard;
                return ret;
            },
            __wbg_colorMask_177d9762658e5e28: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
            },
            __wbg_colorMask_7a8dbc86e7376a9b: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
            },
            __wbg_compileShader_63b824e86bb00b8f: function(arg0, arg1) {
                arg0.compileShader(arg1);
            },
            __wbg_compileShader_94718a93495d565d: function(arg0, arg1) {
                arg0.compileShader(arg1);
            },
            __wbg_compressedTexSubImage2D_215bb115facd5e48: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
                arg0.compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8);
            },
            __wbg_compressedTexSubImage2D_684350eb62830032: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
                arg0.compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8);
            },
            __wbg_compressedTexSubImage2D_d8fbae93bb8c4cc9: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8, arg9);
            },
            __wbg_compressedTexSubImage3D_16afa3a47bf1d979: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
                arg0.compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10);
            },
            __wbg_compressedTexSubImage3D_778008a6293f15ab: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10, arg11);
            },
            __wbg_configure_66c0acbad612270b: function() { return handleError(function (arg0, arg1) {
                arg0.configure(arg1);
            }, arguments); },
            __wbg_contentBoxSize_328a5cd3e7d063a9: function(arg0) {
                const ret = arg0.contentBoxSize;
                return ret;
            },
            __wbg_contentRect_79b98e4d4f4728a4: function(arg0) {
                const ret = arg0.contentRect;
                return ret;
            },
            __wbg_copyBufferSubData_a4f9815861ff0ae9: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.copyBufferSubData(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
            },
            __wbg_copyTexSubImage2D_417a65926e3d2490: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
                arg0.copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
            },
            __wbg_copyTexSubImage2D_91ebcd9cd1908265: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
                arg0.copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
            },
            __wbg_copyTexSubImage3D_f62ef4c4eeb9a7dc: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.copyTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9);
            },
            __wbg_copyTextureToBuffer_97ee3fcb8f6b7e53: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                arg0.copyTextureToBuffer(arg1, arg2, arg3);
            }, arguments); },
            __wbg_createBindGroupLayout_dfced374d0e633c8: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.createBindGroupLayout(arg1);
                return ret;
            }, arguments); },
            __wbg_createBindGroup_71b999f263f53ae3: function(arg0, arg1) {
                const ret = arg0.createBindGroup(arg1);
                return ret;
            },
            __wbg_createBuffer_14e3024bab893efd: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.createBuffer(arg1);
                return ret;
            }, arguments); },
            __wbg_createBuffer_26534c05e01b8559: function(arg0) {
                const ret = arg0.createBuffer();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createBuffer_c4ec897aacc1b91c: function(arg0) {
                const ret = arg0.createBuffer();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createCommandEncoder_ceb8552b0a4a295b: function(arg0, arg1) {
                const ret = arg0.createCommandEncoder(arg1);
                return ret;
            },
            __wbg_createElement_49f60fdcaae809c8: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
                return ret;
            }, arguments); },
            __wbg_createFramebuffer_41512c38358a41c4: function(arg0) {
                const ret = arg0.createFramebuffer();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createFramebuffer_b88ffa8e0fd262c4: function(arg0) {
                const ret = arg0.createFramebuffer();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createPipelineLayout_399e3cceac893a43: function(arg0, arg1) {
                const ret = arg0.createPipelineLayout(arg1);
                return ret;
            },
            __wbg_createProgram_98aaa91f7c81c5e2: function(arg0) {
                const ret = arg0.createProgram();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createProgram_9b7710a1f2701c2c: function(arg0) {
                const ret = arg0.createProgram();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createQuery_7988050efd7e4c48: function(arg0) {
                const ret = arg0.createQuery();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createRenderPipeline_6eadb20d6db4159a: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.createRenderPipeline(arg1);
                return ret;
            }, arguments); },
            __wbg_createRenderbuffer_1e567f2f4d461710: function(arg0) {
                const ret = arg0.createRenderbuffer();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createRenderbuffer_a601226a6a680dbe: function(arg0) {
                const ret = arg0.createRenderbuffer();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createSampler_cbc9e6c8b1de65ba: function(arg0, arg1) {
                const ret = arg0.createSampler(arg1);
                return ret;
            },
            __wbg_createSampler_da6bb96c9ffaaa27: function(arg0) {
                const ret = arg0.createSampler();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createShaderModule_cc902174d32e28fb: function(arg0, arg1) {
                const ret = arg0.createShaderModule(arg1);
                return ret;
            },
            __wbg_createShader_e3ac08ed8c5b14b2: function(arg0, arg1) {
                const ret = arg0.createShader(arg1 >>> 0);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createShader_f2b928ca9a426b14: function(arg0, arg1) {
                const ret = arg0.createShader(arg1 >>> 0);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createTexture_16d2c8a3d7d4a75a: function(arg0) {
                const ret = arg0.createTexture();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createTexture_1d91bb9e10a9bd61: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.createTexture(arg1);
                return ret;
            }, arguments); },
            __wbg_createTexture_f9451a82c7527ce2: function(arg0) {
                const ret = arg0.createTexture();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createVertexArrayOES_bd76ceee6ab9b95e: function(arg0) {
                const ret = arg0.createVertexArrayOES();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createVertexArray_ad5294951ae57497: function(arg0) {
                const ret = arg0.createVertexArray();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_createView_eef09e8ca0c88b84: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.createView(arg1);
                return ret;
            }, arguments); },
            __wbg_ctrlKey_09a1b54d77dea92b: function(arg0) {
                const ret = arg0.ctrlKey;
                return ret;
            },
            __wbg_ctrlKey_96ff94f8b18636a3: function(arg0) {
                const ret = arg0.ctrlKey;
                return ret;
            },
            __wbg_cullFace_39500f654c67a205: function(arg0, arg1) {
                arg0.cullFace(arg1 >>> 0);
            },
            __wbg_cullFace_e7e711a14d2c3f48: function(arg0, arg1) {
                arg0.cullFace(arg1 >>> 0);
            },
            __wbg_dataTransfer_d924a622fbe51b06: function(arg0) {
                const ret = arg0.dataTransfer;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_data_acd149571f3b741a: function(arg0, arg1) {
                const ret = arg1.data;
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_debug_9779a2a9ffdc6eec: function(arg0, arg1) {
                console.debug(getStringFromWasm0(arg0, arg1));
            },
            __wbg_deleteBuffer_22fcc93912cbf659: function(arg0, arg1) {
                arg0.deleteBuffer(arg1);
            },
            __wbg_deleteBuffer_ab099883c168644d: function(arg0, arg1) {
                arg0.deleteBuffer(arg1);
            },
            __wbg_deleteFramebuffer_8de1ca41ac87cfd9: function(arg0, arg1) {
                arg0.deleteFramebuffer(arg1);
            },
            __wbg_deleteFramebuffer_9738f3bb85c1ab35: function(arg0, arg1) {
                arg0.deleteFramebuffer(arg1);
            },
            __wbg_deleteProgram_9298fb3e3c1d3a78: function(arg0, arg1) {
                arg0.deleteProgram(arg1);
            },
            __wbg_deleteProgram_f354e79b8cae8076: function(arg0, arg1) {
                arg0.deleteProgram(arg1);
            },
            __wbg_deleteQuery_ea8bf1954febd774: function(arg0, arg1) {
                arg0.deleteQuery(arg1);
            },
            __wbg_deleteRenderbuffer_096edada57729468: function(arg0, arg1) {
                arg0.deleteRenderbuffer(arg1);
            },
            __wbg_deleteRenderbuffer_0f565f0727b341fc: function(arg0, arg1) {
                arg0.deleteRenderbuffer(arg1);
            },
            __wbg_deleteSampler_c6b68c4071841afa: function(arg0, arg1) {
                arg0.deleteSampler(arg1);
            },
            __wbg_deleteShader_aaf3b520a64d5d9d: function(arg0, arg1) {
                arg0.deleteShader(arg1);
            },
            __wbg_deleteShader_ff70ca962883e241: function(arg0, arg1) {
                arg0.deleteShader(arg1);
            },
            __wbg_deleteSync_c8e4a9c735f71d18: function(arg0, arg1) {
                arg0.deleteSync(arg1);
            },
            __wbg_deleteTexture_2be78224e5584a8b: function(arg0, arg1) {
                arg0.deleteTexture(arg1);
            },
            __wbg_deleteTexture_9d411c0e60ffa324: function(arg0, arg1) {
                arg0.deleteTexture(arg1);
            },
            __wbg_deleteVertexArrayOES_197df47ef9684195: function(arg0, arg1) {
                arg0.deleteVertexArrayOES(arg1);
            },
            __wbg_deleteVertexArray_7bc7f92769862f93: function(arg0, arg1) {
                arg0.deleteVertexArray(arg1);
            },
            __wbg_deltaMode_a1d1df711e44cefc: function(arg0) {
                const ret = arg0.deltaMode;
                return ret;
            },
            __wbg_deltaX_f0ca9116db5f7bc1: function(arg0) {
                const ret = arg0.deltaX;
                return ret;
            },
            __wbg_deltaY_eb94120160ac821c: function(arg0) {
                const ret = arg0.deltaY;
                return ret;
            },
            __wbg_depthFunc_eb3aa05361dd2eaa: function(arg0, arg1) {
                arg0.depthFunc(arg1 >>> 0);
            },
            __wbg_depthFunc_f670d4cbb9cd0913: function(arg0, arg1) {
                arg0.depthFunc(arg1 >>> 0);
            },
            __wbg_depthMask_103091329ca1a750: function(arg0, arg1) {
                arg0.depthMask(arg1 !== 0);
            },
            __wbg_depthMask_75a36d0065471a4b: function(arg0, arg1) {
                arg0.depthMask(arg1 !== 0);
            },
            __wbg_depthRange_337bf254e67639bb: function(arg0, arg1, arg2) {
                arg0.depthRange(arg1, arg2);
            },
            __wbg_depthRange_5579d448b9d7de57: function(arg0, arg1, arg2) {
                arg0.depthRange(arg1, arg2);
            },
            __wbg_description_5b9c632e5babd331: function(arg0, arg1) {
                const ret = arg1.description;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_destroy_1ea1ace6353a5e0f: function(arg0) {
                arg0.destroy();
            },
            __wbg_destroy_eff086e5aa506cbf: function(arg0) {
                arg0.destroy();
            },
            __wbg_devicePixelContentBoxSize_8f39437eab7f03ea: function(arg0) {
                const ret = arg0.devicePixelContentBoxSize;
                return ret;
            },
            __wbg_devicePixelRatio_5c458affc89fc209: function(arg0) {
                const ret = arg0.devicePixelRatio;
                return ret;
            },
            __wbg_disableVertexAttribArray_24a020060006b10f: function(arg0, arg1) {
                arg0.disableVertexAttribArray(arg1 >>> 0);
            },
            __wbg_disableVertexAttribArray_4bac633c27bae599: function(arg0, arg1) {
                arg0.disableVertexAttribArray(arg1 >>> 0);
            },
            __wbg_disable_7fe6fb3e97717f88: function(arg0, arg1) {
                arg0.disable(arg1 >>> 0);
            },
            __wbg_disable_bd37bdcca1764aea: function(arg0, arg1) {
                arg0.disable(arg1 >>> 0);
            },
            __wbg_disconnect_5202f399852258c0: function(arg0) {
                arg0.disconnect();
            },
            __wbg_document_ee35a3d3ae34ef6c: function(arg0) {
                const ret = arg0.document;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_done_57b39ecd9addfe81: function(arg0) {
                const ret = arg0.done;
                return ret;
            },
            __wbg_drawArraysInstancedANGLE_9e4cc507eae8b24d: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.drawArraysInstancedANGLE(arg1 >>> 0, arg2, arg3, arg4);
            },
            __wbg_drawArraysInstanced_ec30adc616ec58d5: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
            },
            __wbg_drawArrays_075228181299b824: function(arg0, arg1, arg2, arg3) {
                arg0.drawArrays(arg1 >>> 0, arg2, arg3);
            },
            __wbg_drawArrays_2be89c369a29f30b: function(arg0, arg1, arg2, arg3) {
                arg0.drawArrays(arg1 >>> 0, arg2, arg3);
            },
            __wbg_drawBuffersWEBGL_447bc0a21f8ef22d: function(arg0, arg1) {
                arg0.drawBuffersWEBGL(arg1);
            },
            __wbg_drawBuffers_5eccfaacc6560299: function(arg0, arg1) {
                arg0.drawBuffers(arg1);
            },
            __wbg_drawElementsInstancedANGLE_6f9da0b845ac6c4e: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.drawElementsInstancedANGLE(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
            },
            __wbg_drawElementsInstanced_d41fc920ae24717c: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.drawElementsInstanced(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
            },
            __wbg_drawIndexed_e3db09f636339744: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.drawIndexed(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5 >>> 0);
            },
            __wbg_draw_a8206b1671491bc7: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.draw(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
            },
            __wbg_elementFromPoint_c626cb9a65328c63: function(arg0, arg1, arg2) {
                const ret = arg0.elementFromPoint(arg1, arg2);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_elementFromPoint_fcddd007465b6e73: function(arg0, arg1, arg2) {
                const ret = arg0.elementFromPoint(arg1, arg2);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_enableVertexAttribArray_475e06c31777296d: function(arg0, arg1) {
                arg0.enableVertexAttribArray(arg1 >>> 0);
            },
            __wbg_enableVertexAttribArray_aa6e40408261eeb9: function(arg0, arg1) {
                arg0.enableVertexAttribArray(arg1 >>> 0);
            },
            __wbg_enable_d1ac04dfdd2fb3ae: function(arg0, arg1) {
                arg0.enable(arg1 >>> 0);
            },
            __wbg_enable_fee40f19b7053ea3: function(arg0, arg1) {
                arg0.enable(arg1 >>> 0);
            },
            __wbg_endQuery_54f0627d4c931318: function(arg0, arg1) {
                arg0.endQuery(arg1 >>> 0);
            },
            __wbg_end_1595543b445014be: function(arg0) {
                arg0.end();
            },
            __wbg_error_720e2e25f7f4aaab: function(arg0, arg1) {
                let deferred0_0;
                let deferred0_1;
                try {
                    deferred0_0 = arg0;
                    deferred0_1 = arg1;
                    console.error(getStringFromWasm0(arg0, arg1));
                } finally {
                    wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
                }
            },
            __wbg_error_9a7fe3f932034cde: function(arg0) {
                console.error(arg0);
            },
            __wbg_fenceSync_c52a4e24eabfa0d3: function(arg0, arg1, arg2) {
                const ret = arg0.fenceSync(arg1 >>> 0, arg2 >>> 0);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_fetch_913ac1857674e36f: function(arg0) {
                const ret = fetch(arg0);
                return ret;
            },
            __wbg_files_c7608e3fb8eb4d07: function(arg0) {
                const ret = arg0.files;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_finish_62f2c753601f524b: function(arg0, arg1) {
                const ret = arg0.finish(arg1);
                return ret;
            },
            __wbg_finish_6c763d6b0e965746: function(arg0) {
                const ret = arg0.finish();
                return ret;
            },
            __wbg_flush_7777597fd43065db: function(arg0) {
                arg0.flush();
            },
            __wbg_flush_e322496f5412e567: function(arg0) {
                arg0.flush();
            },
            __wbg_focus_128ff465f65677cc: function() { return handleError(function (arg0) {
                arg0.focus();
            }, arguments); },
            __wbg_force_6acda126382fc3c0: function(arg0) {
                const ret = arg0.force;
                return ret;
            },
            __wbg_framebufferRenderbuffer_850811ed6e26475e: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4);
            },
            __wbg_framebufferRenderbuffer_cd9d55a68a2300ea: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4);
            },
            __wbg_framebufferTexture2D_8adf6bdfc3c56dee: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5);
            },
            __wbg_framebufferTexture2D_c283e928186aa542: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4, arg5);
            },
            __wbg_framebufferTextureLayer_c8328828c8d5eb60: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.framebufferTextureLayer(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
            },
            __wbg_framebufferTextureMultiviewOVR_16d049b41d692b91: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.framebufferTextureMultiviewOVR(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5, arg6);
            },
            __wbg_frontFace_027e2ec7a7bc347c: function(arg0, arg1) {
                arg0.frontFace(arg1 >>> 0);
            },
            __wbg_frontFace_d4a6507ad2939b5c: function(arg0, arg1) {
                arg0.frontFace(arg1 >>> 0);
            },
            __wbg_getBindGroupLayout_01629f013fda472b: function(arg0, arg1) {
                const ret = arg0.getBindGroupLayout(arg1 >>> 0);
                return ret;
            },
            __wbg_getBoundingClientRect_b5c8c34d07878818: function(arg0) {
                const ret = arg0.getBoundingClientRect();
                return ret;
            },
            __wbg_getBufferSubData_4fc54b4fbb1462d7: function(arg0, arg1, arg2, arg3) {
                arg0.getBufferSubData(arg1 >>> 0, arg2, arg3);
            },
            __wbg_getComputedStyle_2d1f9dfe4ee7e0b9: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.getComputedStyle(arg1);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_getContext_2966500392030d63: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_getContext_2a5764d48600bc43: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_getContext_b28d2db7bd648242: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                const ret = arg0.getContext(getStringFromWasm0(arg1, arg2), arg3);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_getContext_de810d9f187f29ca: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                const ret = arg0.getContext(getStringFromWasm0(arg1, arg2), arg3);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_getCurrentTexture_b0d53fd9275326fe: function() { return handleError(function (arg0) {
                const ret = arg0.getCurrentTexture();
                return ret;
            }, arguments); },
            __wbg_getData_2aada4ab05d445e3: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                const ret = arg1.getData(getStringFromWasm0(arg2, arg3));
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_getExtension_3c0cb5ae01bb4b17: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.getExtension(getStringFromWasm0(arg1, arg2));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_getIndexedParameter_ca1693c768bc4934: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.getIndexedParameter(arg1 >>> 0, arg2 >>> 0);
                return ret;
            }, arguments); },
            __wbg_getItem_0c792d344808dcf5: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_getMappedRange_bacd5f5901104d49: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.getMappedRange(arg1, arg2);
                return ret;
            }, arguments); },
            __wbg_getParameter_1ecb910cfdd21f88: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.getParameter(arg1 >>> 0);
                return ret;
            }, arguments); },
            __wbg_getParameter_2e1f97ecaab76274: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.getParameter(arg1 >>> 0);
                return ret;
            }, arguments); },
            __wbg_getPreferredCanvasFormat_282539e2d81caacd: function(arg0) {
                const ret = arg0.getPreferredCanvasFormat();
                return (__wbindgen_enum_GpuTextureFormat.indexOf(ret) + 1 || 96) - 1;
            },
            __wbg_getProgramInfoLog_2ffa30e3abb8b5c2: function(arg0, arg1, arg2) {
                const ret = arg1.getProgramInfoLog(arg2);
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_getProgramInfoLog_dbfda4b6e7eb1b37: function(arg0, arg1, arg2) {
                const ret = arg1.getProgramInfoLog(arg2);
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_getProgramParameter_43fbc6d2613c08b3: function(arg0, arg1, arg2) {
                const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
                return ret;
            },
            __wbg_getProgramParameter_92e4540ca9da06b2: function(arg0, arg1, arg2) {
                const ret = arg0.getProgramParameter(arg1, arg2 >>> 0);
                return ret;
            },
            __wbg_getPropertyValue_d6911b2a1f9acba9: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                const ret = arg1.getPropertyValue(getStringFromWasm0(arg2, arg3));
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_getQueryParameter_5d6af051438ae479: function(arg0, arg1, arg2) {
                const ret = arg0.getQueryParameter(arg1, arg2 >>> 0);
                return ret;
            },
            __wbg_getRootNode_193e544ed534e810: function(arg0) {
                const ret = arg0.getRootNode();
                return ret;
            },
            __wbg_getShaderInfoLog_9991e9e77b0c6805: function(arg0, arg1, arg2) {
                const ret = arg1.getShaderInfoLog(arg2);
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_getShaderInfoLog_9e0b96da4b13ae49: function(arg0, arg1, arg2) {
                const ret = arg1.getShaderInfoLog(arg2);
                var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_getShaderParameter_786fd84f85720ca8: function(arg0, arg1, arg2) {
                const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
                return ret;
            },
            __wbg_getShaderParameter_afa4a3dd9dd397c1: function(arg0, arg1, arg2) {
                const ret = arg0.getShaderParameter(arg1, arg2 >>> 0);
                return ret;
            },
            __wbg_getSupportedExtensions_57142a6b598d7787: function(arg0) {
                const ret = arg0.getSupportedExtensions();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_getSupportedProfiles_1f728bc32003c4d0: function(arg0) {
                const ret = arg0.getSupportedProfiles();
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_getSyncParameter_7d11ab875b41617e: function(arg0, arg1, arg2) {
                const ret = arg0.getSyncParameter(arg1, arg2 >>> 0);
                return ret;
            },
            __wbg_getTime_1e3cd1391c5c3995: function(arg0) {
                const ret = arg0.getTime();
                return ret;
            },
            __wbg_getUniformBlockIndex_1ee7e922e6d96d7e: function(arg0, arg1, arg2, arg3) {
                const ret = arg0.getUniformBlockIndex(arg1, getStringFromWasm0(arg2, arg3));
                return ret;
            },
            __wbg_getUniformLocation_71c070e6644669ad: function(arg0, arg1, arg2, arg3) {
                const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_getUniformLocation_d06b3a5b3c60e95c: function(arg0, arg1, arg2, arg3) {
                const ret = arg0.getUniformLocation(arg1, getStringFromWasm0(arg2, arg3));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_get_4fe487fe39ff3573: function(arg0, arg1) {
                const ret = arg0[arg1 >>> 0];
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_get_5bd55a138a9e899f: function(arg0, arg1) {
                const ret = arg0[arg1 >>> 0];
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_get_9a1f194723936140: function(arg0, arg1) {
                const ret = arg0[arg1 >>> 0];
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_get_9b94d73e6221f75c: function(arg0, arg1) {
                const ret = arg0[arg1 >>> 0];
                return ret;
            },
            __wbg_get_b3ed3ad4be2bc8ac: function() { return handleError(function (arg0, arg1) {
                const ret = Reflect.get(arg0, arg1);
                return ret;
            }, arguments); },
            __wbg_get_d8db2ad31d529ff8: function(arg0, arg1) {
                const ret = arg0[arg1 >>> 0];
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_gpu_d3ce5fec0560b53f: function(arg0) {
                const ret = arg0.gpu;
                return ret;
            },
            __wbg_hash_90eadad0e1447454: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.hash;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_headers_59a2938db9f80985: function(arg0) {
                const ret = arg0.headers;
                return ret;
            },
            __wbg_headers_5a897f7fee9a0571: function(arg0) {
                const ret = arg0.headers;
                return ret;
            },
            __wbg_height_38750dc6de41ee75: function(arg0) {
                const ret = arg0.height;
                return ret;
            },
            __wbg_height_c2027cf67d1c9b11: function(arg0) {
                const ret = arg0.height;
                return ret;
            },
            __wbg_hidden_8ce6a98b8c12451c: function(arg0) {
                const ret = arg0.hidden;
                return ret;
            },
            __wbg_host_92d607209031b72c: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.host;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_hostname_0c450e33386895ba: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.hostname;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_href_67854c3dd511f6f3: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.href;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_id_ff64a5892a30d4e9: function(arg0, arg1) {
                const ret = arg1.id;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_identifier_5feaba602edf9981: function(arg0) {
                const ret = arg0.identifier;
                return ret;
            },
            __wbg_includes_32215c836f1cd3fb: function(arg0, arg1, arg2) {
                const ret = arg0.includes(arg1, arg2);
                return ret;
            },
            __wbg_info_0ccdff43e587b087: function(arg0) {
                const ret = arg0.info;
                return ret;
            },
            __wbg_info_f06a459f3a80dcde: function(arg0, arg1) {
                console.info(getStringFromWasm0(arg0, arg1));
            },
            __wbg_inlineSize_3e4e7e8c813884fd: function(arg0) {
                const ret = arg0.inlineSize;
                return ret;
            },
            __wbg_instanceof_Document_50f5ff170c1a7826: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof Document;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_Element_9e662f49ab6c6beb: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof Element;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_GpuAdapter_dc7e13c1676da9bd: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof GPUAdapter;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_GpuCanvasContext_c2609c698a76a6b6: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof GPUCanvasContext;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_HtmlCanvasElement_3f2f6e1edb1c9792: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof HTMLCanvasElement;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_HtmlElement_5abfac207260fd6f: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof HTMLElement;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_HtmlInputElement_c10b7260b4e0710a: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof HTMLInputElement;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_ResizeObserverEntry_16bca25646e32231: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof ResizeObserverEntry;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_ResizeObserverSize_cee71be747d9d29e: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof ResizeObserverSize;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_Response_ee1d54d79ae41977: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof Response;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_ShadowRoot_5285adde3587c73e: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof ShadowRoot;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_TypeError_45484a0407e7f588: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof TypeError;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_WebGl2RenderingContext_4a08a94517ed5240: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof WebGL2RenderingContext;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_instanceof_Window_ed49b2db8df90359: function(arg0) {
                let result;
                try {
                    result = arg0 instanceof Window;
                } catch (_) {
                    result = false;
                }
                const ret = result;
                return ret;
            },
            __wbg_invalidateFramebuffer_b17b7e1da3051745: function() { return handleError(function (arg0, arg1, arg2) {
                arg0.invalidateFramebuffer(arg1 >>> 0, arg2);
            }, arguments); },
            __wbg_isComposing_1eafc5b1376f01d1: function(arg0) {
                const ret = arg0.isComposing;
                return ret;
            },
            __wbg_isComposing_9323fa62320f5fc0: function(arg0) {
                const ret = arg0.isComposing;
                return ret;
            },
            __wbg_isSecureContext_1e186b850f07cfb3: function(arg0) {
                const ret = arg0.isSecureContext;
                return ret;
            },
            __wbg_is_f29129f676e5410c: function(arg0, arg1) {
                const ret = Object.is(arg0, arg1);
                return ret;
            },
            __wbg_item_98b174cdde606b25: function(arg0, arg1) {
                const ret = arg0.item(arg1 >>> 0);
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_items_4130211600bde9a4: function(arg0) {
                const ret = arg0.items;
                return ret;
            },
            __wbg_iterator_6ff6560ca1568e55: function() {
                const ret = Symbol.iterator;
                return ret;
            },
            __wbg_keyCode_155291a11654466e: function(arg0) {
                const ret = arg0.keyCode;
                return ret;
            },
            __wbg_key_d41e8e825e6bb0e9: function(arg0, arg1) {
                const ret = arg1.key;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_label_a845e6103c78476f: function(arg0, arg1) {
                const ret = arg1.label;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_lastModified_a5cfce993c651681: function(arg0) {
                const ret = arg0.lastModified;
                return ret;
            },
            __wbg_left_3b7c3c1030d5ca7a: function(arg0) {
                const ret = arg0.left;
                return ret;
            },
            __wbg_length_25b2ccd77d48ecb1: function(arg0) {
                const ret = arg0.length;
                return ret;
            },
            __wbg_length_32ed9a279acd054c: function(arg0) {
                const ret = arg0.length;
                return ret;
            },
            __wbg_length_35a7bace40f36eac: function(arg0) {
                const ret = arg0.length;
                return ret;
            },
            __wbg_length_9efde69e99cd464e: function(arg0) {
                const ret = arg0.length;
                return ret;
            },
            __wbg_length_dd7a84decbd9cde7: function(arg0) {
                const ret = arg0.length;
                return ret;
            },
            __wbg_limits_c18490a2121161fa: function(arg0) {
                const ret = arg0.limits;
                return ret;
            },
            __wbg_linkProgram_6600dd2c0863bbfd: function(arg0, arg1) {
                arg0.linkProgram(arg1);
            },
            __wbg_linkProgram_be6b825cf66d177b: function(arg0, arg1) {
                arg0.linkProgram(arg1);
            },
            __wbg_localStorage_a22d31b9eacc4594: function() { return handleError(function (arg0) {
                const ret = arg0.localStorage;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_location_df7ca06c93e51763: function(arg0) {
                const ret = arg0.location;
                return ret;
            },
            __wbg_mapAsync_fa3ea90b32de5224: function(arg0, arg1, arg2, arg3) {
                const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
                return ret;
            },
            __wbg_matchMedia_91d4fc9729dc3c84: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.matchMedia(getStringFromWasm0(arg1, arg2));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_matches_4b5c22bd830f7bb3: function(arg0) {
                const ret = arg0.matches;
                return ret;
            },
            __wbg_maxBindGroups_f2a0264eb0828e5d: function(arg0) {
                const ret = arg0.maxBindGroups;
                return ret;
            },
            __wbg_maxBindingsPerBindGroup_6832c52651aaad9e: function(arg0) {
                const ret = arg0.maxBindingsPerBindGroup;
                return ret;
            },
            __wbg_maxBufferSize_9130347c76ae64c0: function(arg0) {
                const ret = arg0.maxBufferSize;
                return ret;
            },
            __wbg_maxColorAttachmentBytesPerSample_a3fb9526356315fc: function(arg0) {
                const ret = arg0.maxColorAttachmentBytesPerSample;
                return ret;
            },
            __wbg_maxColorAttachments_0ef7219b5a9840df: function(arg0) {
                const ret = arg0.maxColorAttachments;
                return ret;
            },
            __wbg_maxComputeInvocationsPerWorkgroup_87bcd8de0824a71c: function(arg0) {
                const ret = arg0.maxComputeInvocationsPerWorkgroup;
                return ret;
            },
            __wbg_maxComputeWorkgroupSizeX_c92b1adc90ed5412: function(arg0) {
                const ret = arg0.maxComputeWorkgroupSizeX;
                return ret;
            },
            __wbg_maxComputeWorkgroupSizeY_db0e992a727cda06: function(arg0) {
                const ret = arg0.maxComputeWorkgroupSizeY;
                return ret;
            },
            __wbg_maxComputeWorkgroupSizeZ_6f9561e5e2eba971: function(arg0) {
                const ret = arg0.maxComputeWorkgroupSizeZ;
                return ret;
            },
            __wbg_maxComputeWorkgroupStorageSize_7fb6a2ed96304c7e: function(arg0) {
                const ret = arg0.maxComputeWorkgroupStorageSize;
                return ret;
            },
            __wbg_maxComputeWorkgroupsPerDimension_d43b02728c7b6d2d: function(arg0) {
                const ret = arg0.maxComputeWorkgroupsPerDimension;
                return ret;
            },
            __wbg_maxDynamicStorageBuffersPerPipelineLayout_47367d4119e49f2a: function(arg0) {
                const ret = arg0.maxDynamicStorageBuffersPerPipelineLayout;
                return ret;
            },
            __wbg_maxDynamicUniformBuffersPerPipelineLayout_408dfe96dc22c0ad: function(arg0) {
                const ret = arg0.maxDynamicUniformBuffersPerPipelineLayout;
                return ret;
            },
            __wbg_maxInterStageShaderVariables_d9cac70b842dc03b: function(arg0) {
                const ret = arg0.maxInterStageShaderVariables;
                return ret;
            },
            __wbg_maxSampledTexturesPerShaderStage_409e36afbd8c218d: function(arg0) {
                const ret = arg0.maxSampledTexturesPerShaderStage;
                return ret;
            },
            __wbg_maxSamplersPerShaderStage_79f8fae6e0a93bf3: function(arg0) {
                const ret = arg0.maxSamplersPerShaderStage;
                return ret;
            },
            __wbg_maxStorageBufferBindingSize_3bd37af2519c45c7: function(arg0) {
                const ret = arg0.maxStorageBufferBindingSize;
                return ret;
            },
            __wbg_maxStorageBuffersPerShaderStage_f157c8890b35866f: function(arg0) {
                const ret = arg0.maxStorageBuffersPerShaderStage;
                return ret;
            },
            __wbg_maxStorageTexturesPerShaderStage_ac89cc2cee6ecb5e: function(arg0) {
                const ret = arg0.maxStorageTexturesPerShaderStage;
                return ret;
            },
            __wbg_maxTextureArrayLayers_9e79b924dc01f9ac: function(arg0) {
                const ret = arg0.maxTextureArrayLayers;
                return ret;
            },
            __wbg_maxTextureDimension1D_71235f57d9e94b11: function(arg0) {
                const ret = arg0.maxTextureDimension1D;
                return ret;
            },
            __wbg_maxTextureDimension2D_59a7cebc0a04c70c: function(arg0) {
                const ret = arg0.maxTextureDimension2D;
                return ret;
            },
            __wbg_maxTextureDimension3D_d1d4d04c7c1cc184: function(arg0) {
                const ret = arg0.maxTextureDimension3D;
                return ret;
            },
            __wbg_maxUniformBufferBindingSize_258b21f5bb5d812c: function(arg0) {
                const ret = arg0.maxUniformBufferBindingSize;
                return ret;
            },
            __wbg_maxUniformBuffersPerShaderStage_b84ed7a4f29f2dbd: function(arg0) {
                const ret = arg0.maxUniformBuffersPerShaderStage;
                return ret;
            },
            __wbg_maxVertexAttributes_a459e0528f92d023: function(arg0) {
                const ret = arg0.maxVertexAttributes;
                return ret;
            },
            __wbg_maxVertexBufferArrayStride_8c5305383fc96c2b: function(arg0) {
                const ret = arg0.maxVertexBufferArrayStride;
                return ret;
            },
            __wbg_maxVertexBuffers_763175203c3282f9: function(arg0) {
                const ret = arg0.maxVertexBuffers;
                return ret;
            },
            __wbg_metaKey_374999c340f70626: function(arg0) {
                const ret = arg0.metaKey;
                return ret;
            },
            __wbg_metaKey_67113fb40365d736: function(arg0) {
                const ret = arg0.metaKey;
                return ret;
            },
            __wbg_minStorageBufferOffsetAlignment_277cddbb47dd6de2: function(arg0) {
                const ret = arg0.minStorageBufferOffsetAlignment;
                return ret;
            },
            __wbg_minUniformBufferOffsetAlignment_d3724c1f21b12c5f: function(arg0) {
                const ret = arg0.minUniformBufferOffsetAlignment;
                return ret;
            },
            __wbg_name_171cddfde96a29c8: function(arg0, arg1) {
                const ret = arg1.name;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_navigator_43be698ba96fc088: function(arg0) {
                const ret = arg0.navigator;
                return ret;
            },
            __wbg_navigator_4478931f32ebca57: function(arg0) {
                const ret = arg0.navigator;
                return ret;
            },
            __wbg_new_0_73afc35eb544e539: function() {
                const ret = new Date();
                return ret;
            },
            __wbg_new_2e2be9617c4407d5: function() { return handleError(function (arg0) {
                const ret = new ResizeObserver(arg0);
                return ret;
            }, arguments); },
            __wbg_new_361308b2356cecd0: function() {
                const ret = new Object();
                return ret;
            },
            __wbg_new_3eb36ae241fe6f44: function() {
                const ret = new Array();
                return ret;
            },
            __wbg_new_b5d9e2fb389fef91: function(arg0, arg1) {
                try {
                    var state0 = {a: arg0, b: arg1};
                    var cb0 = (arg0, arg1) => {
                        const a = state0.a;
                        state0.a = 0;
                        try {
                            return wasm_bindgen__convert__closures_____invoke__h67b96ab6d28eb353(a, state0.b, arg0, arg1);
                        } finally {
                            state0.a = a;
                        }
                    };
                    const ret = new Promise(cb0);
                    return ret;
                } finally {
                    state0.a = state0.b = 0;
                }
            },
            __wbg_new_c155239f1b113b68: function(arg0, arg1) {
                const ret = new Intl.DateTimeFormat(arg0, arg1);
                return ret;
            },
            __wbg_new_db61162c22800cbc: function() {
                const ret = new Error();
                return ret;
            },
            __wbg_new_dd2b680c8bf6ae29: function(arg0) {
                const ret = new Uint8Array(arg0);
                return ret;
            },
            __wbg_new_from_slice_a3d2629dc1826784: function(arg0, arg1) {
                const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
                return ret;
            },
            __wbg_new_no_args_1c7c842f08d00ebb: function(arg0, arg1) {
                const ret = new Function(getStringFromWasm0(arg0, arg1));
                return ret;
            },
            __wbg_new_with_byte_offset_and_length_aa261d9c9da49eb1: function(arg0, arg1, arg2) {
                const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
                return ret;
            },
            __wbg_new_with_record_from_str_to_blob_promise_17d3b40dbba6c99d: function() { return handleError(function (arg0) {
                const ret = new ClipboardItem(arg0);
                return ret;
            }, arguments); },
            __wbg_new_with_str_and_init_a61cbc6bdef21614: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
                return ret;
            }, arguments); },
            __wbg_new_with_text_c4caedd46c3e5f01: function() { return handleError(function (arg0, arg1) {
                const ret = new SpeechSynthesisUtterance(getStringFromWasm0(arg0, arg1));
                return ret;
            }, arguments); },
            __wbg_new_with_u8_array_sequence_and_options_cc0f8f2c1ef62e68: function() { return handleError(function (arg0, arg1) {
                const ret = new Blob(arg0, arg1);
                return ret;
            }, arguments); },
            __wbg_next_3482f54c49e8af19: function() { return handleError(function (arg0) {
                const ret = arg0.next();
                return ret;
            }, arguments); },
            __wbg_next_418f80d8f5303233: function(arg0) {
                const ret = arg0.next;
                return ret;
            },
            __wbg_now_2c95c9de01293173: function(arg0) {
                const ret = arg0.now();
                return ret;
            },
            __wbg_now_ebffdf7e580f210d: function(arg0) {
                const ret = arg0.now();
                return ret;
            },
            __wbg_observe_1ae37077cf10b11b: function(arg0, arg1, arg2) {
                arg0.observe(arg1, arg2);
            },
            __wbg_of_f915f7cd925b21a5: function(arg0) {
                const ret = Array.of(arg0);
                return ret;
            },
            __wbg_offsetTop_e3d5b0a34b3200fc: function(arg0) {
                const ret = arg0.offsetTop;
                return ret;
            },
            __wbg_ok_87f537440a0acf85: function(arg0) {
                const ret = arg0.ok;
                return ret;
            },
            __wbg_onSubmittedWorkDone_9f0181ba120b0ac1: function(arg0) {
                const ret = arg0.onSubmittedWorkDone();
                return ret;
            },
            __wbg_open_ea44acde696d3b0c: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                const ret = arg0.open(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_origin_a9c891fa602b4d40: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.origin;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_performance_06f12ba62483475d: function(arg0) {
                const ret = arg0.performance;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_performance_7a3ffd0b17f663ad: function(arg0) {
                const ret = arg0.performance;
                return ret;
            },
            __wbg_pixelStorei_2a65936c11b710fe: function(arg0, arg1, arg2) {
                arg0.pixelStorei(arg1 >>> 0, arg2);
            },
            __wbg_pixelStorei_f7cc498f52d523f1: function(arg0, arg1, arg2) {
                arg0.pixelStorei(arg1 >>> 0, arg2);
            },
            __wbg_polygonOffset_24a8059deb03be92: function(arg0, arg1, arg2) {
                arg0.polygonOffset(arg1, arg2);
            },
            __wbg_polygonOffset_4b3158d8ed028862: function(arg0, arg1, arg2) {
                arg0.polygonOffset(arg1, arg2);
            },
            __wbg_port_dc56bc76d55c2b55: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.port;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_preventDefault_cdcfcd7e301b9702: function(arg0) {
                arg0.preventDefault();
            },
            __wbg_protocol_4c3b13957de7d079: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.protocol;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_prototypesetcall_bdcdcc5842e4d77d: function(arg0, arg1, arg2) {
                Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
            },
            __wbg_push_8ffdcb2063340ba5: function(arg0, arg1) {
                const ret = arg0.push(arg1);
                return ret;
            },
            __wbg_queryCounterEXT_b578f07c30420446: function(arg0, arg1, arg2) {
                arg0.queryCounterEXT(arg1, arg2 >>> 0);
            },
            __wbg_querySelectorAll_1283aae52043a951: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
                return ret;
            }, arguments); },
            __wbg_querySelector_c3b0df2d58eec220: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            }, arguments); },
            __wbg_queueMicrotask_0aa0a927f78f5d98: function(arg0) {
                const ret = arg0.queueMicrotask;
                return ret;
            },
            __wbg_queueMicrotask_5bb536982f78a56f: function(arg0) {
                queueMicrotask(arg0);
            },
            __wbg_queue_e3b0b96c5ab7e74d: function(arg0) {
                const ret = arg0.queue;
                return ret;
            },
            __wbg_readBuffer_9eb461d6857295f0: function(arg0, arg1) {
                arg0.readBuffer(arg1 >>> 0);
            },
            __wbg_readPixels_55b18304384e073d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
                arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
            }, arguments); },
            __wbg_readPixels_6ea8e288a8673282: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
                arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
            }, arguments); },
            __wbg_readPixels_95b2464a7bb863a2: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
                arg0.readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
            }, arguments); },
            __wbg_removeEventListener_e63328781a5b9af9: function() { return handleError(function (arg0, arg1, arg2, arg3) {
                arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
            }, arguments); },
            __wbg_remove_31c39325eee968fc: function(arg0) {
                arg0.remove();
            },
            __wbg_renderbufferStorageMultisample_bc0ae08a7abb887a: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.renderbufferStorageMultisample(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
            },
            __wbg_renderbufferStorage_1bc02383614b76b2: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
            },
            __wbg_renderbufferStorage_6348154d30979c44: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
            },
            __wbg_requestAdapter_7b611d4e9585dcf3: function(arg0, arg1) {
                const ret = arg0.requestAdapter(arg1);
                return ret;
            },
            __wbg_requestAdapter_93aa461d903e7829: function(arg0) {
                const ret = arg0.requestAdapter();
                return ret;
            },
            __wbg_requestAnimationFrame_43682f8e1c5e5348: function() { return handleError(function (arg0, arg1) {
                const ret = arg0.requestAnimationFrame(arg1);
                return ret;
            }, arguments); },
            __wbg_requestDevice_0e969b74ad8b475a: function(arg0, arg1) {
                const ret = arg0.requestDevice(arg1);
                return ret;
            },
            __wbg_resolve_002c4b7d9d8f6b64: function(arg0) {
                const ret = Promise.resolve(arg0);
                return ret;
            },
            __wbg_resolvedOptions_4c36dbfa1c4ba2bf: function(arg0) {
                const ret = arg0.resolvedOptions();
                return ret;
            },
            __wbg_right_154af6c2b1bf0c89: function(arg0) {
                const ret = arg0.right;
                return ret;
            },
            __wbg_samplerParameterf_f070d2b69b1e2d46: function(arg0, arg1, arg2, arg3) {
                arg0.samplerParameterf(arg1, arg2 >>> 0, arg3);
            },
            __wbg_samplerParameteri_8e4c4bcead0ee669: function(arg0, arg1, arg2, arg3) {
                arg0.samplerParameteri(arg1, arg2 >>> 0, arg3);
            },
            __wbg_scissor_2ff8f18f05a6d408: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.scissor(arg1, arg2, arg3, arg4);
            },
            __wbg_scissor_b870b1434a9c25b4: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.scissor(arg1, arg2, arg3, arg4);
            },
            __wbg_search_1b385e665c888780: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.search;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_setAttribute_cc8e4c8a2a008508: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            }, arguments); },
            __wbg_setBindGroup_423040c1a83b1b9d: function(arg0, arg1, arg2) {
                arg0.setBindGroup(arg1 >>> 0, arg2);
            },
            __wbg_setBindGroup_7471ce5e9441a9bf: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
            }, arguments); },
            __wbg_setIndexBuffer_144c120e04a9967c: function(arg0, arg1, arg2, arg3) {
                arg0.setIndexBuffer(arg1, __wbindgen_enum_GpuIndexFormat[arg2], arg3);
            },
            __wbg_setIndexBuffer_3813312cbe3bac8f: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.setIndexBuffer(arg1, __wbindgen_enum_GpuIndexFormat[arg2], arg3, arg4);
            },
            __wbg_setItem_cf340bb2edbd3089: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            }, arguments); },
            __wbg_setPipeline_dc786871d6cbbdf2: function(arg0, arg1) {
                arg0.setPipeline(arg1);
            },
            __wbg_setProperty_cbb25c4e74285b39: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            }, arguments); },
            __wbg_setScissorRect_5218eacccfbd3c9b: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.setScissorRect(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
            },
            __wbg_setVertexBuffer_4bcc73f3179aec9a: function(arg0, arg1, arg2, arg3) {
                arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3);
            },
            __wbg_setVertexBuffer_806357ef7fb4b1af: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.setVertexBuffer(arg1 >>> 0, arg2, arg3, arg4);
            },
            __wbg_setViewport_339ed506b656ae0c: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.setViewport(arg1, arg2, arg3, arg4, arg5, arg6);
            },
            __wbg_set_25cf9deff6bf0ea8: function(arg0, arg1, arg2) {
                arg0.set(arg1, arg2 >>> 0);
            },
            __wbg_set_6cb8631f80447a67: function() { return handleError(function (arg0, arg1, arg2) {
                const ret = Reflect.set(arg0, arg1, arg2);
                return ret;
            }, arguments); },
            __wbg_set_a_8183648bc4372304: function(arg0, arg1) {
                arg0.a = arg1;
            },
            __wbg_set_access_60e80ecbbb46631c: function(arg0, arg1) {
                arg0.access = __wbindgen_enum_GpuStorageTextureAccess[arg1];
            },
            __wbg_set_address_mode_u_83951b319ac8afc0: function(arg0, arg1) {
                arg0.addressModeU = __wbindgen_enum_GpuAddressMode[arg1];
            },
            __wbg_set_address_mode_v_18ae0b572ce0f9f1: function(arg0, arg1) {
                arg0.addressModeV = __wbindgen_enum_GpuAddressMode[arg1];
            },
            __wbg_set_address_mode_w_c75d3d65931004e7: function(arg0, arg1) {
                arg0.addressModeW = __wbindgen_enum_GpuAddressMode[arg1];
            },
            __wbg_set_alpha_c4d99941e6befb52: function(arg0, arg1) {
                arg0.alpha = arg1;
            },
            __wbg_set_alpha_mode_a5c986ab3ae87b6c: function(arg0, arg1) {
                arg0.alphaMode = __wbindgen_enum_GpuCanvasAlphaMode[arg1];
            },
            __wbg_set_alpha_to_coverage_enabled_8fcffd00f4b80e85: function(arg0, arg1) {
                arg0.alphaToCoverageEnabled = arg1 !== 0;
            },
            __wbg_set_array_layer_count_249d840890c875ac: function(arg0, arg1) {
                arg0.arrayLayerCount = arg1 >>> 0;
            },
            __wbg_set_array_stride_216572351e30dcdd: function(arg0, arg1) {
                arg0.arrayStride = arg1;
            },
            __wbg_set_aspect_19acdd2dd1d2d4a7: function(arg0, arg1) {
                arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
            },
            __wbg_set_aspect_65b305d88b305ad5: function(arg0, arg1) {
                arg0.aspect = __wbindgen_enum_GpuTextureAspect[arg1];
            },
            __wbg_set_attributes_fb89bde07aa89778: function(arg0, arg1) {
                arg0.attributes = arg1;
            },
            __wbg_set_autofocus_7125a4a223a1d570: function() { return handleError(function (arg0, arg1) {
                arg0.autofocus = arg1 !== 0;
            }, arguments); },
            __wbg_set_b_abe04844afe738aa: function(arg0, arg1) {
                arg0.b = arg1;
            },
            __wbg_set_base_array_layer_d0aec00c6ce11c59: function(arg0, arg1) {
                arg0.baseArrayLayer = arg1 >>> 0;
            },
            __wbg_set_base_mip_level_2d86c3f1daea9063: function(arg0, arg1) {
                arg0.baseMipLevel = arg1 >>> 0;
            },
            __wbg_set_beginning_of_pass_write_index_6ee8a3f822497ae7: function(arg0, arg1) {
                arg0.beginningOfPassWriteIndex = arg1 >>> 0;
            },
            __wbg_set_bind_group_layouts_9d981ee94cce8e45: function(arg0, arg1) {
                arg0.bindGroupLayouts = arg1;
            },
            __wbg_set_binding_230162b0fa51b760: function(arg0, arg1) {
                arg0.binding = arg1 >>> 0;
            },
            __wbg_set_binding_6c66225792d271c3: function(arg0, arg1) {
                arg0.binding = arg1 >>> 0;
            },
            __wbg_set_blend_cf176076161a064a: function(arg0, arg1) {
                arg0.blend = arg1;
            },
            __wbg_set_body_9a7e00afe3cfe244: function(arg0, arg1) {
                arg0.body = arg1;
            },
            __wbg_set_box_73d3355c6f95f24d: function(arg0, arg1) {
                arg0.box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
            },
            __wbg_set_buffer_4f54d56a182e0dad: function(arg0, arg1) {
                arg0.buffer = arg1;
            },
            __wbg_set_buffer_939c4aef1128eb8e: function(arg0, arg1) {
                arg0.buffer = arg1;
            },
            __wbg_set_buffer_fc36a1f85371495a: function(arg0, arg1) {
                arg0.buffer = arg1;
            },
            __wbg_set_buffers_fd0e77e629965f2d: function(arg0, arg1) {
                arg0.buffers = arg1;
            },
            __wbg_set_bytes_per_row_486d540d1bd89dda: function(arg0, arg1) {
                arg0.bytesPerRow = arg1 >>> 0;
            },
            __wbg_set_bytes_per_row_b19360e681626711: function(arg0, arg1) {
                arg0.bytesPerRow = arg1 >>> 0;
            },
            __wbg_set_clear_value_2b3005d4abd2895b: function(arg0, arg1) {
                arg0.clearValue = arg1;
            },
            __wbg_set_code_748f8b7693fdfd1e: function(arg0, arg1, arg2) {
                arg0.code = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_color_817d1570d68784da: function(arg0, arg1) {
                arg0.color = arg1;
            },
            __wbg_set_color_attachments_48cfcf75e56660c7: function(arg0, arg1) {
                arg0.colorAttachments = arg1;
            },
            __wbg_set_compare_63853b60f773c0fa: function(arg0, arg1) {
                arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
            },
            __wbg_set_compare_73ed6b1f8fc43c92: function(arg0, arg1) {
                arg0.compare = __wbindgen_enum_GpuCompareFunction[arg1];
            },
            __wbg_set_count_c81a29474bd45d7d: function(arg0, arg1) {
                arg0.count = arg1 >>> 0;
            },
            __wbg_set_credentials_c4a58d2e05ef24fb: function(arg0, arg1) {
                arg0.credentials = __wbindgen_enum_RequestCredentials[arg1];
            },
            __wbg_set_cull_mode_30d87e53b40bae08: function(arg0, arg1) {
                arg0.cullMode = __wbindgen_enum_GpuCullMode[arg1];
            },
            __wbg_set_db769d02949a271d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
                arg0.set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            }, arguments); },
            __wbg_set_depth_bias_87ff216d22ff3100: function(arg0, arg1) {
                arg0.depthBias = arg1;
            },
            __wbg_set_depth_bias_clamp_edbef59c26311d66: function(arg0, arg1) {
                arg0.depthBiasClamp = arg1;
            },
            __wbg_set_depth_bias_slope_scale_433d525d0c142ea2: function(arg0, arg1) {
                arg0.depthBiasSlopeScale = arg1;
            },
            __wbg_set_depth_clear_value_403949872adddfaa: function(arg0, arg1) {
                arg0.depthClearValue = arg1;
            },
            __wbg_set_depth_compare_7819c8ae486e7a58: function(arg0, arg1) {
                arg0.depthCompare = __wbindgen_enum_GpuCompareFunction[arg1];
            },
            __wbg_set_depth_fail_op_f3ad3e1e3eb77494: function(arg0, arg1) {
                arg0.depthFailOp = __wbindgen_enum_GpuStencilOperation[arg1];
            },
            __wbg_set_depth_load_op_28c58f61f51ccdae: function(arg0, arg1) {
                arg0.depthLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
            },
            __wbg_set_depth_or_array_layers_4ece18c4a3938729: function(arg0, arg1) {
                arg0.depthOrArrayLayers = arg1 >>> 0;
            },
            __wbg_set_depth_read_only_1d73171d2b676836: function(arg0, arg1) {
                arg0.depthReadOnly = arg1 !== 0;
            },
            __wbg_set_depth_stencil_64b70a01966ebf0a: function(arg0, arg1) {
                arg0.depthStencil = arg1;
            },
            __wbg_set_depth_stencil_attachment_3c6240abf74695fb: function(arg0, arg1) {
                arg0.depthStencilAttachment = arg1;
            },
            __wbg_set_depth_store_op_39be9a86544512dd: function(arg0, arg1) {
                arg0.depthStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
            },
            __wbg_set_depth_write_enabled_8106e7b0166a2a3d: function(arg0, arg1) {
                arg0.depthWriteEnabled = arg1 !== 0;
            },
            __wbg_set_device_39ad6e4fd72a192c: function(arg0, arg1) {
                arg0.device = arg1;
            },
            __wbg_set_dimension_505601577421b8f9: function(arg0, arg1) {
                arg0.dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
            },
            __wbg_set_dimension_8b95f5be6ca4ec1d: function(arg0, arg1) {
                arg0.dimension = __wbindgen_enum_GpuTextureDimension[arg1];
            },
            __wbg_set_dst_factor_aad0809a443d5eac: function(arg0, arg1) {
                arg0.dstFactor = __wbindgen_enum_GpuBlendFactor[arg1];
            },
            __wbg_set_end_of_pass_write_index_b358e26926026888: function(arg0, arg1) {
                arg0.endOfPassWriteIndex = arg1 >>> 0;
            },
            __wbg_set_entries_038388746b4d37b9: function(arg0, arg1) {
                arg0.entries = arg1;
            },
            __wbg_set_entries_1f8eeef1ee7cbed2: function(arg0, arg1) {
                arg0.entries = arg1;
            },
            __wbg_set_entry_point_23c5933381c3253d: function(arg0, arg1, arg2) {
                arg0.entryPoint = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_entry_point_ed51e31dd33d7cc0: function(arg0, arg1, arg2) {
                arg0.entryPoint = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_external_texture_37cab8d798319c11: function(arg0, arg1) {
                arg0.externalTexture = arg1;
            },
            __wbg_set_fail_op_258dfe8d16a13962: function(arg0, arg1) {
                arg0.failOp = __wbindgen_enum_GpuStencilOperation[arg1];
            },
            __wbg_set_format_10ee31e59bf9c588: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
            },
            __wbg_set_format_46e6f5cf4638957f: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
            },
            __wbg_set_format_b8fc1f6a6c86d548: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
            },
            __wbg_set_format_bdeb9499230d6695: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuVertexFormat[arg1];
            },
            __wbg_set_format_bed30ff331780937: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
            },
            __wbg_set_format_c95e8adb2c43529f: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
            },
            __wbg_set_format_f7f4303041af47f0: function(arg0, arg1) {
                arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
            },
            __wbg_set_fragment_26f2bc835fe3dd26: function(arg0, arg1) {
                arg0.fragment = arg1;
            },
            __wbg_set_front_face_d443b924636a625c: function(arg0, arg1) {
                arg0.frontFace = __wbindgen_enum_GpuFrontFace[arg1];
            },
            __wbg_set_g_5caf24e00918e38d: function(arg0, arg1) {
                arg0.g = arg1;
            },
            __wbg_set_has_dynamic_offset_e98e853afeb20fe5: function(arg0, arg1) {
                arg0.hasDynamicOffset = arg1 !== 0;
            },
            __wbg_set_height_67f19a97b6df7f3d: function(arg0, arg1) {
                arg0.height = arg1 >>> 0;
            },
            __wbg_set_height_b386c0f603610637: function(arg0, arg1) {
                arg0.height = arg1 >>> 0;
            },
            __wbg_set_height_f21f985387070100: function(arg0, arg1) {
                arg0.height = arg1 >>> 0;
            },
            __wbg_set_label_0a43d49a6442d456: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_1059a675c396dde4: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_15742e1e407f2ab1: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_199441638231dcef: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_34b30207d87aa532: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_537f747748002093: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_69182e604d72f587: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_7f30da541dfb121a: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_92f33b635db94332: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_a46d3a361b41ff50: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_ba4399efd1e4f90b: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_d4be983bd53a9b2e: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_label_ecff92c7377cce4e: function(arg0, arg1, arg2) {
                arg0.label = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_layout_0197c6cec854b4b7: function(arg0, arg1) {
                arg0.layout = arg1;
            },
            __wbg_set_layout_d6d118016029cd3b: function(arg0, arg1) {
                arg0.layout = arg1;
            },
            __wbg_set_load_op_cf6c1fd9d7c7fe72: function(arg0, arg1) {
                arg0.loadOp = __wbindgen_enum_GpuLoadOp[arg1];
            },
            __wbg_set_lod_max_clamp_029e170267e49b7a: function(arg0, arg1) {
                arg0.lodMaxClamp = arg1;
            },
            __wbg_set_lod_min_clamp_c607c040f5dba77e: function(arg0, arg1) {
                arg0.lodMinClamp = arg1;
            },
            __wbg_set_mag_filter_f38693f2f6d11acc: function(arg0, arg1) {
                arg0.magFilter = __wbindgen_enum_GpuFilterMode[arg1];
            },
            __wbg_set_mapped_at_creation_351e4d7f15e88bf0: function(arg0, arg1) {
                arg0.mappedAtCreation = arg1 !== 0;
            },
            __wbg_set_mask_85271a7a8af27d05: function(arg0, arg1) {
                arg0.mask = arg1 >>> 0;
            },
            __wbg_set_max_anisotropy_fffcf9ba9cfa836c: function(arg0, arg1) {
                arg0.maxAnisotropy = arg1;
            },
            __wbg_set_method_c3e20375f5ae7fac: function(arg0, arg1, arg2) {
                arg0.method = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_min_binding_size_41a0afad77a26d8d: function(arg0, arg1) {
                arg0.minBindingSize = arg1;
            },
            __wbg_set_min_filter_4469ba17c4924ae9: function(arg0, arg1) {
                arg0.minFilter = __wbindgen_enum_GpuFilterMode[arg1];
            },
            __wbg_set_mip_level_6285d0186348c3e1: function(arg0, arg1) {
                arg0.mipLevel = arg1 >>> 0;
            },
            __wbg_set_mip_level_count_5a73f1a71f6d41f2: function(arg0, arg1) {
                arg0.mipLevelCount = arg1 >>> 0;
            },
            __wbg_set_mip_level_count_fce9c0b2dd1cdebd: function(arg0, arg1) {
                arg0.mipLevelCount = arg1 >>> 0;
            },
            __wbg_set_mipmap_filter_bb60fb57944e1e3a: function(arg0, arg1) {
                arg0.mipmapFilter = __wbindgen_enum_GpuMipmapFilterMode[arg1];
            },
            __wbg_set_mode_b13642c312648202: function(arg0, arg1) {
                arg0.mode = __wbindgen_enum_RequestMode[arg1];
            },
            __wbg_set_module_12ef1d0563faefd8: function(arg0, arg1) {
                arg0.module = arg1;
            },
            __wbg_set_module_8def5f3d6859cca7: function(arg0, arg1) {
                arg0.module = arg1;
            },
            __wbg_set_multisample_5349527d6f717d50: function(arg0, arg1) {
                arg0.multisample = arg1;
            },
            __wbg_set_multisampled_5341db022947df02: function(arg0, arg1) {
                arg0.multisampled = arg1 !== 0;
            },
            __wbg_set_offset_8daa7abde1bc1b01: function(arg0, arg1) {
                arg0.offset = arg1;
            },
            __wbg_set_offset_909176c397100bc2: function(arg0, arg1) {
                arg0.offset = arg1;
            },
            __wbg_set_offset_a44801e18fa065cd: function(arg0, arg1) {
                arg0.offset = arg1;
            },
            __wbg_set_offset_e63293bf0e713a9a: function(arg0, arg1) {
                arg0.offset = arg1;
            },
            __wbg_set_once_56ba1b87a9884c15: function(arg0, arg1) {
                arg0.once = arg1 !== 0;
            },
            __wbg_set_operation_600db10edfc0655e: function(arg0, arg1) {
                arg0.operation = __wbindgen_enum_GpuBlendOperation[arg1];
            },
            __wbg_set_origin_f42930c5bf8bdfb8: function(arg0, arg1) {
                arg0.origin = arg1;
            },
            __wbg_set_pass_op_4b76a4df4538aaa3: function(arg0, arg1) {
                arg0.passOp = __wbindgen_enum_GpuStencilOperation[arg1];
            },
            __wbg_set_pitch_91774e1f0bbc52fa: function(arg0, arg1) {
                arg0.pitch = arg1;
            },
            __wbg_set_power_preference_f275a8e9756ebf77: function(arg0, arg1) {
                arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
            },
            __wbg_set_primitive_dcffbddf426b5b75: function(arg0, arg1) {
                arg0.primitive = arg1;
            },
            __wbg_set_query_set_a4fb33d110a1d366: function(arg0, arg1) {
                arg0.querySet = arg1;
            },
            __wbg_set_r_a776094a3b33a826: function(arg0, arg1) {
                arg0.r = arg1;
            },
            __wbg_set_rate_63dfcd5b7bcdbc07: function(arg0, arg1) {
                arg0.rate = arg1;
            },
            __wbg_set_required_features_cf6592709f10a96e: function(arg0, arg1) {
                arg0.requiredFeatures = arg1;
            },
            __wbg_set_required_limits_8eba4583c099b088: function(arg0, arg1) {
                arg0.requiredLimits = arg1;
            },
            __wbg_set_resolve_target_877664a0dddfd6cb: function(arg0, arg1) {
                arg0.resolveTarget = arg1;
            },
            __wbg_set_resource_95c951ed73c3ad39: function(arg0, arg1) {
                arg0.resource = arg1;
            },
            __wbg_set_rows_per_image_76746d7bbef0c2a2: function(arg0, arg1) {
                arg0.rowsPerImage = arg1 >>> 0;
            },
            __wbg_set_rows_per_image_faa59fdd452c984c: function(arg0, arg1) {
                arg0.rowsPerImage = arg1 >>> 0;
            },
            __wbg_set_sample_count_8f4ea5a1774f7455: function(arg0, arg1) {
                arg0.sampleCount = arg1 >>> 0;
            },
            __wbg_set_sample_type_48ca92d79a4a6a5c: function(arg0, arg1) {
                arg0.sampleType = __wbindgen_enum_GpuTextureSampleType[arg1];
            },
            __wbg_set_sampler_b04fbadc0dd7b312: function(arg0, arg1) {
                arg0.sampler = arg1;
            },
            __wbg_set_shader_location_7bb8f0b6ec2b560d: function(arg0, arg1) {
                arg0.shaderLocation = arg1 >>> 0;
            },
            __wbg_set_size_8453dd3174586a12: function(arg0, arg1) {
                arg0.size = arg1;
            },
            __wbg_set_size_b8194a82a1bf607e: function(arg0, arg1) {
                arg0.size = arg1;
            },
            __wbg_set_size_dee3955c04716042: function(arg0, arg1) {
                arg0.size = arg1;
            },
            __wbg_set_src_factor_e3764952caebcb86: function(arg0, arg1) {
                arg0.srcFactor = __wbindgen_enum_GpuBlendFactor[arg1];
            },
            __wbg_set_stencil_back_846a0a7b05716ac8: function(arg0, arg1) {
                arg0.stencilBack = arg1;
            },
            __wbg_set_stencil_clear_value_31971c7cfd4f2726: function(arg0, arg1) {
                arg0.stencilClearValue = arg1 >>> 0;
            },
            __wbg_set_stencil_front_b759a97769e6e9cf: function(arg0, arg1) {
                arg0.stencilFront = arg1;
            },
            __wbg_set_stencil_load_op_dadf695a74451439: function(arg0, arg1) {
                arg0.stencilLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
            },
            __wbg_set_stencil_read_mask_bebc0448676e5ecf: function(arg0, arg1) {
                arg0.stencilReadMask = arg1 >>> 0;
            },
            __wbg_set_stencil_read_only_9809d3c73e75fccc: function(arg0, arg1) {
                arg0.stencilReadOnly = arg1 !== 0;
            },
            __wbg_set_stencil_store_op_d0fb502ae3bfbcd1: function(arg0, arg1) {
                arg0.stencilStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
            },
            __wbg_set_stencil_write_mask_c60526bae9c88e97: function(arg0, arg1) {
                arg0.stencilWriteMask = arg1 >>> 0;
            },
            __wbg_set_step_mode_6fd1eb53dbf82c16: function(arg0, arg1) {
                arg0.stepMode = __wbindgen_enum_GpuVertexStepMode[arg1];
            },
            __wbg_set_storage_texture_c2fb1b11554be0cd: function(arg0, arg1) {
                arg0.storageTexture = arg1;
            },
            __wbg_set_store_op_de06d158fb84ddde: function(arg0, arg1) {
                arg0.storeOp = __wbindgen_enum_GpuStoreOp[arg1];
            },
            __wbg_set_strip_index_format_103836464cce8e9c: function(arg0, arg1) {
                arg0.stripIndexFormat = __wbindgen_enum_GpuIndexFormat[arg1];
            },
            __wbg_set_tabIndex_eb89b6ffe111cd2c: function(arg0, arg1) {
                arg0.tabIndex = arg1;
            },
            __wbg_set_targets_70e64d9d5ea29fd9: function(arg0, arg1) {
                arg0.targets = arg1;
            },
            __wbg_set_texture_15ea7b969f073839: function(arg0, arg1) {
                arg0.texture = arg1;
            },
            __wbg_set_texture_ffe777ada2fc4317: function(arg0, arg1) {
                arg0.texture = arg1;
            },
            __wbg_set_timestamp_writes_2b3415c1c7a8359d: function(arg0, arg1) {
                arg0.timestampWrites = arg1;
            },
            __wbg_set_topology_d74afad55e4e3182: function(arg0, arg1) {
                arg0.topology = __wbindgen_enum_GpuPrimitiveTopology[arg1];
            },
            __wbg_set_type_148de20768639245: function(arg0, arg1, arg2) {
                arg0.type = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_type_41a129313637b03c: function(arg0, arg1) {
                arg0.type = __wbindgen_enum_GpuBufferBindingType[arg1];
            },
            __wbg_set_type_82031fa1254c1c96: function(arg0, arg1) {
                arg0.type = __wbindgen_enum_GpuSamplerBindingType[arg1];
            },
            __wbg_set_type_abc37fa3c213f717: function(arg0, arg1, arg2) {
                arg0.type = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_unclipped_depth_577a90d88a4eefde: function(arg0, arg1) {
                arg0.unclippedDepth = arg1 !== 0;
            },
            __wbg_set_usage_19364b5a25155cab: function(arg0, arg1) {
                arg0.usage = arg1 >>> 0;
            },
            __wbg_set_usage_48de846827712752: function(arg0, arg1) {
                arg0.usage = arg1 >>> 0;
            },
            __wbg_set_usage_5740e0bb1b4f8133: function(arg0, arg1) {
                arg0.usage = arg1 >>> 0;
            },
            __wbg_set_usage_57687fc43157e54a: function(arg0, arg1) {
                arg0.usage = arg1 >>> 0;
            },
            __wbg_set_value_62a965e38b22b38c: function(arg0, arg1, arg2) {
                arg0.value = getStringFromWasm0(arg1, arg2);
            },
            __wbg_set_vertex_390fea9da64e39df: function(arg0, arg1) {
                arg0.vertex = arg1;
            },
            __wbg_set_view_b39958b9a0c447cc: function(arg0, arg1) {
                arg0.view = arg1;
            },
            __wbg_set_view_dimension_ca629a93f8aa5b77: function(arg0, arg1) {
                arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
            },
            __wbg_set_view_dimension_d3942d75edf20ddd: function(arg0, arg1) {
                arg0.viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
            },
            __wbg_set_view_f72854c23ae9f6cf: function(arg0, arg1) {
                arg0.view = arg1;
            },
            __wbg_set_view_formats_cedc6791da403dd0: function(arg0, arg1) {
                arg0.viewFormats = arg1;
            },
            __wbg_set_view_formats_d71615b406b48c55: function(arg0, arg1) {
                arg0.viewFormats = arg1;
            },
            __wbg_set_visibility_e573b286c07466a9: function(arg0, arg1) {
                arg0.visibility = arg1 >>> 0;
            },
            __wbg_set_volume_789cc971c19a2e24: function(arg0, arg1) {
                arg0.volume = arg1;
            },
            __wbg_set_width_271237ac36113877: function(arg0, arg1) {
                arg0.width = arg1 >>> 0;
            },
            __wbg_set_width_7f07715a20503914: function(arg0, arg1) {
                arg0.width = arg1 >>> 0;
            },
            __wbg_set_width_d60bc4f2f20c56a4: function(arg0, arg1) {
                arg0.width = arg1 >>> 0;
            },
            __wbg_set_write_mask_a862c33ddae25b05: function(arg0, arg1) {
                arg0.writeMask = arg1 >>> 0;
            },
            __wbg_set_x_ca6153125c88f94a: function(arg0, arg1) {
                arg0.x = arg1 >>> 0;
            },
            __wbg_set_y_e6bfda8575a95ce1: function(arg0, arg1) {
                arg0.y = arg1 >>> 0;
            },
            __wbg_set_z_cad8b9af4fa03da8: function(arg0, arg1) {
                arg0.z = arg1 >>> 0;
            },
            __wbg_shaderSource_32425cfe6e5a1e52: function(arg0, arg1, arg2, arg3) {
                arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
            },
            __wbg_shaderSource_8f4bda03f70359df: function(arg0, arg1, arg2, arg3) {
                arg0.shaderSource(arg1, getStringFromWasm0(arg2, arg3));
            },
            __wbg_shiftKey_5558a3288542c985: function(arg0) {
                const ret = arg0.shiftKey;
                return ret;
            },
            __wbg_shiftKey_564be91ec842bcc4: function(arg0) {
                const ret = arg0.shiftKey;
                return ret;
            },
            __wbg_size_bd22487996a18767: function(arg0) {
                const ret = arg0.size;
                return ret;
            },
            __wbg_size_e05d31cc6049815f: function(arg0) {
                const ret = arg0.size;
                return ret;
            },
            __wbg_speak_67796aeb0b5fd194: function(arg0, arg1) {
                arg0.speak(arg1);
            },
            __wbg_speechSynthesis_95b40351572bb250: function() { return handleError(function (arg0) {
                const ret = arg0.speechSynthesis;
                return ret;
            }, arguments); },
            __wbg_stack_2b6d2b0787c0b48b: function(arg0, arg1) {
                const ret = arg1.stack;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_static_accessor_GLOBAL_12837167ad935116: function() {
                const ret = typeof global === 'undefined' ? null : global;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_static_accessor_GLOBAL_THIS_e628e89ab3b1c95f: function() {
                const ret = typeof globalThis === 'undefined' ? null : globalThis;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_static_accessor_SELF_a621d3dfbb60d0ce: function() {
                const ret = typeof self === 'undefined' ? null : self;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_static_accessor_WINDOW_f8727f0cf888e0bd: function() {
                const ret = typeof window === 'undefined' ? null : window;
                return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
            },
            __wbg_statusText_556131a02d60f5cd: function(arg0, arg1) {
                const ret = arg1.statusText;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_status_89d7e803db911ee7: function(arg0) {
                const ret = arg0.status;
                return ret;
            },
            __wbg_stencilFuncSeparate_10d043d0af14366f: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
            },
            __wbg_stencilFuncSeparate_1798f5cca257f313: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
            },
            __wbg_stencilMaskSeparate_28d53625c02d9c7f: function(arg0, arg1, arg2) {
                arg0.stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_stencilMaskSeparate_c24c1a28b8dd8a63: function(arg0, arg1, arg2) {
                arg0.stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_stencilMask_0eca090c4c47f8f7: function(arg0, arg1) {
                arg0.stencilMask(arg1 >>> 0);
            },
            __wbg_stencilMask_732dcc5aada10e4c: function(arg0, arg1) {
                arg0.stencilMask(arg1 >>> 0);
            },
            __wbg_stencilOpSeparate_4657523b1d3b184f: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
            },
            __wbg_stencilOpSeparate_de257f3c29e604cd: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
            },
            __wbg_stopPropagation_6e5e2a085214ac63: function(arg0) {
                arg0.stopPropagation();
            },
            __wbg_style_0b7c9bd318f8b807: function(arg0) {
                const ret = arg0.style;
                return ret;
            },
            __wbg_submit_2e1d8a082d7d5bf5: function(arg0, arg1) {
                arg0.submit(arg1);
            },
            __wbg_texImage2D_087ef94df78081f0: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texImage2D_13414a4692836804: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texImage2D_e71049312f3172d9: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texImage3D_2082006a8a9b28a7: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
                arg0.texImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8 >>> 0, arg9 >>> 0, arg10);
            }, arguments); },
            __wbg_texImage3D_bd2b0bd2cfcdb278: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
                arg0.texImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8 >>> 0, arg9 >>> 0, arg10);
            }, arguments); },
            __wbg_texParameteri_0d45be2c88d6bad8: function(arg0, arg1, arg2, arg3) {
                arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
            },
            __wbg_texParameteri_ec937d2161018946: function(arg0, arg1, arg2, arg3) {
                arg0.texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
            },
            __wbg_texStorage2D_9504743abf5a986a: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.texStorage2D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
            },
            __wbg_texStorage3D_e9e1b58fee218abe: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.texStorage3D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5, arg6);
            },
            __wbg_texSubImage2D_117d29278542feb0: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_19ae4cadb809f264: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_5d270af600a7fc4a: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_bd034db2e58c352c: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_bf72e56edeeed376: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_d17a39cdec4a3495: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_e193f1d28439217c: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage2D_edf5bd70fda3feaf: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
                arg0.texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
            }, arguments); },
            __wbg_texSubImage3D_1102c12a20bf56d5: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_texSubImage3D_18d7f3c65567c885: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_texSubImage3D_3b653017c4c5d721: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_texSubImage3D_45591e5655d1ed5c: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_texSubImage3D_47643556a8a4bf86: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_texSubImage3D_59b8e24fb05787aa: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_texSubImage3D_eff5cd6ab84f44ee: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
                arg0.texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
            }, arguments); },
            __wbg_then_0d9fe2c7b1857d32: function(arg0, arg1, arg2) {
                const ret = arg0.then(arg1, arg2);
                return ret;
            },
            __wbg_then_b9e7b3b5f1a9e1b5: function(arg0, arg1) {
                const ret = arg0.then(arg1);
                return ret;
            },
            __wbg_top_3d27ff6f468cf3fc: function(arg0) {
                const ret = arg0.top;
                return ret;
            },
            __wbg_touches_55ce167b42bcdf52: function(arg0) {
                const ret = arg0.touches;
                return ret;
            },
            __wbg_type_9a3860e6dd3a4156: function(arg0, arg1) {
                const ret = arg1.type;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_type_e8c7fade6d73451b: function(arg0, arg1) {
                const ret = arg1.type;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_uniform1f_b500ede5b612bea2: function(arg0, arg1, arg2) {
                arg0.uniform1f(arg1, arg2);
            },
            __wbg_uniform1f_c148eeaf4b531059: function(arg0, arg1, arg2) {
                arg0.uniform1f(arg1, arg2);
            },
            __wbg_uniform1i_9f3f72dbcb98ada9: function(arg0, arg1, arg2) {
                arg0.uniform1i(arg1, arg2);
            },
            __wbg_uniform1i_e9aee4b9e7fe8c4b: function(arg0, arg1, arg2) {
                arg0.uniform1i(arg1, arg2);
            },
            __wbg_uniform1ui_a0f911ff174715d0: function(arg0, arg1, arg2) {
                arg0.uniform1ui(arg1, arg2 >>> 0);
            },
            __wbg_uniform2fv_04c304b93cbf7f55: function(arg0, arg1, arg2, arg3) {
                arg0.uniform2fv(arg1, getArrayF32FromWasm0(arg2, arg3));
            },
            __wbg_uniform2fv_2fb47cfe06330cc7: function(arg0, arg1, arg2, arg3) {
                arg0.uniform2fv(arg1, getArrayF32FromWasm0(arg2, arg3));
            },
            __wbg_uniform2iv_095baf208f172131: function(arg0, arg1, arg2, arg3) {
                arg0.uniform2iv(arg1, getArrayI32FromWasm0(arg2, arg3));
            },
            __wbg_uniform2iv_ccf2ed44ac8e602e: function(arg0, arg1, arg2, arg3) {
                arg0.uniform2iv(arg1, getArrayI32FromWasm0(arg2, arg3));
            },
            __wbg_uniform2uiv_3030d7e769f5e82a: function(arg0, arg1, arg2, arg3) {
                arg0.uniform2uiv(arg1, getArrayU32FromWasm0(arg2, arg3));
            },
            __wbg_uniform3fv_aa35ef21e14d5469: function(arg0, arg1, arg2, arg3) {
                arg0.uniform3fv(arg1, getArrayF32FromWasm0(arg2, arg3));
            },
            __wbg_uniform3fv_c0872003729939a5: function(arg0, arg1, arg2, arg3) {
                arg0.uniform3fv(arg1, getArrayF32FromWasm0(arg2, arg3));
            },
            __wbg_uniform3iv_6aa2b0791e659d14: function(arg0, arg1, arg2, arg3) {
                arg0.uniform3iv(arg1, getArrayI32FromWasm0(arg2, arg3));
            },
            __wbg_uniform3iv_e912f444d4ff8269: function(arg0, arg1, arg2, arg3) {
                arg0.uniform3iv(arg1, getArrayI32FromWasm0(arg2, arg3));
            },
            __wbg_uniform3uiv_86941e7eeb8ee0a3: function(arg0, arg1, arg2, arg3) {
                arg0.uniform3uiv(arg1, getArrayU32FromWasm0(arg2, arg3));
            },
            __wbg_uniform4f_71ec75443e58cecc: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.uniform4f(arg1, arg2, arg3, arg4, arg5);
            },
            __wbg_uniform4f_f6b5e2024636033a: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.uniform4f(arg1, arg2, arg3, arg4, arg5);
            },
            __wbg_uniform4fv_498bd80dc5aa16ff: function(arg0, arg1, arg2, arg3) {
                arg0.uniform4fv(arg1, getArrayF32FromWasm0(arg2, arg3));
            },
            __wbg_uniform4fv_e6c73702e9a3be5c: function(arg0, arg1, arg2, arg3) {
                arg0.uniform4fv(arg1, getArrayF32FromWasm0(arg2, arg3));
            },
            __wbg_uniform4iv_375332584c65e61b: function(arg0, arg1, arg2, arg3) {
                arg0.uniform4iv(arg1, getArrayI32FromWasm0(arg2, arg3));
            },
            __wbg_uniform4iv_8a8219fda39dffd5: function(arg0, arg1, arg2, arg3) {
                arg0.uniform4iv(arg1, getArrayI32FromWasm0(arg2, arg3));
            },
            __wbg_uniform4uiv_046ee400bb80547d: function(arg0, arg1, arg2, arg3) {
                arg0.uniform4uiv(arg1, getArrayU32FromWasm0(arg2, arg3));
            },
            __wbg_uniformBlockBinding_1cf9fd2c49adf0f3: function(arg0, arg1, arg2, arg3) {
                arg0.uniformBlockBinding(arg1, arg2 >>> 0, arg3 >>> 0);
            },
            __wbg_uniformMatrix2fv_24430076c7afb5e3: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix2fv_e2806601f5b95102: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix2x3fv_a377326104a8faf4: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix2x3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix2x4fv_b7a4d810e7a1cf7d: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix2x4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix3fv_6f822361173d8046: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix3fv_b94a764c63aa6468: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix3x2fv_69a4cf0ce5b09f8b: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix3x2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix3x4fv_cc72e31a1baaf9c9: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix3x4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix4fv_0e724dbebd372526: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix4fv_923b55ad503fdc56: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix4fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix4x2fv_8c9fb646f3b90b63: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix4x2fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_uniformMatrix4x3fv_ee0bed9a1330400d: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.uniformMatrix4x3fv(arg1, arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
            },
            __wbg_unmap_a07053d5b72652e1: function(arg0) {
                arg0.unmap();
            },
            __wbg_url_c484c26b1fbf5126: function(arg0, arg1) {
                const ret = arg1.url;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_usage_13ab801eb829ddfc: function(arg0) {
                const ret = arg0.usage;
                return ret;
            },
            __wbg_useProgram_e82c1a5f87d81579: function(arg0, arg1) {
                arg0.useProgram(arg1);
            },
            __wbg_useProgram_fe720ade4d3b6edb: function(arg0, arg1) {
                arg0.useProgram(arg1);
            },
            __wbg_userAgent_34463fd660ba4a2a: function() { return handleError(function (arg0, arg1) {
                const ret = arg1.userAgent;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            }, arguments); },
            __wbg_value_0546255b415e96c1: function(arg0) {
                const ret = arg0.value;
                return ret;
            },
            __wbg_value_e506a07878790ca0: function(arg0, arg1) {
                const ret = arg1.value;
                const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                const len1 = WASM_VECTOR_LEN;
                getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
                getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
            },
            __wbg_vertexAttribDivisorANGLE_eaa3c29423ea6da4: function(arg0, arg1, arg2) {
                arg0.vertexAttribDivisorANGLE(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_vertexAttribDivisor_744c0ca468594894: function(arg0, arg1, arg2) {
                arg0.vertexAttribDivisor(arg1 >>> 0, arg2 >>> 0);
            },
            __wbg_vertexAttribIPointer_b9020d0c2e759912: function(arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.vertexAttribIPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
            },
            __wbg_vertexAttribPointer_75f6ff47f6c9f8cb: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
            },
            __wbg_vertexAttribPointer_adbd1853cce679ad: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
            },
            __wbg_viewport_174ae1c2209344ae: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.viewport(arg1, arg2, arg3, arg4);
            },
            __wbg_viewport_df236eac68bc7467: function(arg0, arg1, arg2, arg3, arg4) {
                arg0.viewport(arg1, arg2, arg3, arg4);
            },
            __wbg_warn_92e5ddd61d4392d0: function(arg0, arg1) {
                console.warn(getStringFromWasm0(arg0, arg1));
            },
            __wbg_width_5f66bde2e810fbde: function(arg0) {
                const ret = arg0.width;
                return ret;
            },
            __wbg_width_7444cca5dfea0645: function(arg0) {
                const ret = arg0.width;
                return ret;
            },
            __wbg_writeBuffer_538dfa14a81f396b: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
                arg0.writeBuffer(arg1, arg2, getArrayU8FromWasm0(arg3, arg4), arg5, arg6);
            }, arguments); },
            __wbg_writeText_be1c3b83a3e46230: function(arg0, arg1, arg2) {
                const ret = arg0.writeText(getStringFromWasm0(arg1, arg2));
                return ret;
            },
            __wbg_writeTexture_7a28cd36573abda9: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
                arg0.writeTexture(arg1, getArrayU8FromWasm0(arg2, arg3), arg4, arg5);
            }, arguments); },
            __wbg_write_d429ce72e918e180: function(arg0, arg1) {
                const ret = arg0.write(arg1);
                return ret;
            },
            __wbindgen_cast_0000000000000001: function(arg0, arg1) {
                // Cast intrinsic for `Closure(Closure { dtor_idx: 1437, function: Function { arguments: [NamedExternref("Array<any>")], shim_idx: 1438, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
                const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h1364469e549f152d, wasm_bindgen__convert__closures_____invoke__h66db19cbc3dfcd68);
                return ret;
            },
            __wbindgen_cast_0000000000000002: function(arg0, arg1) {
                // Cast intrinsic for `Closure(Closure { dtor_idx: 1437, function: Function { arguments: [NamedExternref("Event")], shim_idx: 1438, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
                const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h1364469e549f152d, wasm_bindgen__convert__closures_____invoke__h66db19cbc3dfcd68);
                return ret;
            },
            __wbindgen_cast_0000000000000003: function(arg0, arg1) {
                // Cast intrinsic for `Closure(Closure { dtor_idx: 1437, function: Function { arguments: [], shim_idx: 1441, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
                const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h1364469e549f152d, wasm_bindgen__convert__closures_____invoke__h844c247ab67d3e0a);
                return ret;
            },
            __wbindgen_cast_0000000000000004: function(arg0, arg1) {
                // Cast intrinsic for `Closure(Closure { dtor_idx: 2498, function: Function { arguments: [Externref], shim_idx: 2499, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
                const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h3c2e7905d69549d6, wasm_bindgen__convert__closures_____invoke__h798b5d7aafd894b9);
                return ret;
            },
            __wbindgen_cast_0000000000000005: function(arg0) {
                // Cast intrinsic for `F64 -> Externref`.
                const ret = arg0;
                return ret;
            },
            __wbindgen_cast_0000000000000006: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
                const ret = getArrayF32FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_0000000000000007: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(I16)) -> NamedExternref("Int16Array")`.
                const ret = getArrayI16FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_0000000000000008: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(I32)) -> NamedExternref("Int32Array")`.
                const ret = getArrayI32FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_0000000000000009: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(I8)) -> NamedExternref("Int8Array")`.
                const ret = getArrayI8FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_000000000000000a: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(U16)) -> NamedExternref("Uint16Array")`.
                const ret = getArrayU16FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_000000000000000b: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(U32)) -> NamedExternref("Uint32Array")`.
                const ret = getArrayU32FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_000000000000000c: function(arg0, arg1) {
                // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
                const ret = getArrayU8FromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_cast_000000000000000d: function(arg0, arg1) {
                // Cast intrinsic for `Ref(String) -> Externref`.
                const ret = getStringFromWasm0(arg0, arg1);
                return ret;
            },
            __wbindgen_init_externref_table: function() {
                const table = wasm.__wbindgen_externrefs;
                const offset = table.grow(4);
                table.set(0, undefined);
                table.set(offset + 0, undefined);
                table.set(offset + 1, null);
                table.set(offset + 2, true);
                table.set(offset + 3, false);
            },
        };
        return {
            __proto__: null,
            "./egui_demo_app_bg.js": import0,
        };
    }

    function wasm_bindgen__convert__closures_____invoke__h844c247ab67d3e0a(arg0, arg1) {
        const ret = wasm.wasm_bindgen__convert__closures_____invoke__h844c247ab67d3e0a(arg0, arg1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }

    function wasm_bindgen__convert__closures_____invoke__h66db19cbc3dfcd68(arg0, arg1, arg2) {
        wasm.wasm_bindgen__convert__closures_____invoke__h66db19cbc3dfcd68(arg0, arg1, arg2);
    }

    function wasm_bindgen__convert__closures_____invoke__h798b5d7aafd894b9(arg0, arg1, arg2) {
        wasm.wasm_bindgen__convert__closures_____invoke__h798b5d7aafd894b9(arg0, arg1, arg2);
    }

    function wasm_bindgen__convert__closures_____invoke__h67b96ab6d28eb353(arg0, arg1, arg2, arg3) {
        wasm.wasm_bindgen__convert__closures_____invoke__h67b96ab6d28eb353(arg0, arg1, arg2, arg3);
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


    const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];


    const __wbindgen_enum_RequestMode = ["same-origin", "no-cors", "cors", "navigate"];


    const __wbindgen_enum_ResizeObserverBoxOptions = ["border-box", "content-box", "device-pixel-content-box"];
    const WebHandleFinalization = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(ptr => wasm.__wbg_webhandle_free(ptr >>> 0, 1));

    function addToExternrefTable0(obj) {
        const idx = wasm.__externref_table_alloc();
        wasm.__wbindgen_externrefs.set(idx, obj);
        return idx;
    }

    const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(state => state.dtor(state.a, state.b));

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

    function getArrayF32FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
    }

    function getArrayI16FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getInt16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
    }

    function getArrayI32FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
    }

    function getArrayI8FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getInt8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
    }

    function getArrayU16FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getUint16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
    }

    function getArrayU32FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
    }

    function getArrayU8FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
    }

    let cachedDataViewMemory0 = null;
    function getDataViewMemory0() {
        if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
            cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
        }
        return cachedDataViewMemory0;
    }

    let cachedFloat32ArrayMemory0 = null;
    function getFloat32ArrayMemory0() {
        if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
            cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
        }
        return cachedFloat32ArrayMemory0;
    }

    let cachedInt16ArrayMemory0 = null;
    function getInt16ArrayMemory0() {
        if (cachedInt16ArrayMemory0 === null || cachedInt16ArrayMemory0.byteLength === 0) {
            cachedInt16ArrayMemory0 = new Int16Array(wasm.memory.buffer);
        }
        return cachedInt16ArrayMemory0;
    }

    let cachedInt32ArrayMemory0 = null;
    function getInt32ArrayMemory0() {
        if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
            cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
        }
        return cachedInt32ArrayMemory0;
    }

    let cachedInt8ArrayMemory0 = null;
    function getInt8ArrayMemory0() {
        if (cachedInt8ArrayMemory0 === null || cachedInt8ArrayMemory0.byteLength === 0) {
            cachedInt8ArrayMemory0 = new Int8Array(wasm.memory.buffer);
        }
        return cachedInt8ArrayMemory0;
    }

    function getStringFromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return decodeText(ptr, len);
    }

    let cachedUint16ArrayMemory0 = null;
    function getUint16ArrayMemory0() {
        if (cachedUint16ArrayMemory0 === null || cachedUint16ArrayMemory0.byteLength === 0) {
            cachedUint16ArrayMemory0 = new Uint16Array(wasm.memory.buffer);
        }
        return cachedUint16ArrayMemory0;
    }

    let cachedUint32ArrayMemory0 = null;
    function getUint32ArrayMemory0() {
        if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
            cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
        }
        return cachedUint32ArrayMemory0;
    }

    let cachedUint8ArrayMemory0 = null;
    function getUint8ArrayMemory0() {
        if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
            cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
        }
        return cachedUint8ArrayMemory0;
    }

    function handleError(f, args) {
        try {
            return f.apply(this, args);
        } catch (e) {
            const idx = addToExternrefTable0(e);
            wasm.__wbindgen_exn_store(idx);
        }
    }

    function isLikeNone(x) {
        return x === undefined || x === null;
    }

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
                state.a = a;
                real._wbg_cb_unref();
            }
        };
        real._wbg_cb_unref = () => {
            if (--state.cnt === 0) {
                state.dtor(state.a, state.b);
                state.a = 0;
                CLOSURE_DTORS.unregister(state);
            }
        };
        CLOSURE_DTORS.register(real, state, state);
        return real;
    }

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
            const ret = cachedTextEncoder.encodeInto(arg, view);

            offset += ret.written;
            ptr = realloc(ptr, len, offset, 1) >>> 0;
        }

        WASM_VECTOR_LEN = offset;
        return ptr;
    }

    function takeFromExternrefTable0(idx) {
        const value = wasm.__wbindgen_externrefs.get(idx);
        wasm.__externref_table_dealloc(idx);
        return value;
    }

    let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
    cachedTextDecoder.decode();
    function decodeText(ptr, len) {
        return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
    }

    const cachedTextEncoder = new TextEncoder();

    if (!('encodeInto' in cachedTextEncoder)) {
        cachedTextEncoder.encodeInto = function (arg, view) {
            const buf = cachedTextEncoder.encode(arg);
            view.set(buf);
            return {
                read: arg.length,
                written: buf.length
            };
        };
    }

    let WASM_VECTOR_LEN = 0;

    let wasmModule, wasm;
    function __wbg_finalize_init(instance, module) {
        wasm = instance.exports;
        wasmModule = module;
        cachedDataViewMemory0 = null;
        cachedFloat32ArrayMemory0 = null;
        cachedInt16ArrayMemory0 = null;
        cachedInt32ArrayMemory0 = null;
        cachedInt8ArrayMemory0 = null;
        cachedUint16ArrayMemory0 = null;
        cachedUint32ArrayMemory0 = null;
        cachedUint8ArrayMemory0 = null;
        wasm.__wbindgen_start();
        return wasm;
    }

    async function __wbg_load(module, imports) {
        if (typeof Response === 'function' && module instanceof Response) {
            if (typeof WebAssembly.instantiateStreaming === 'function') {
                try {
                    return await WebAssembly.instantiateStreaming(module, imports);
                } catch (e) {
                    const validResponse = module.ok && expectedResponseType(module.type);

                    if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                        console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                    } else { throw e; }
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

        function expectedResponseType(type) {
            switch (type) {
                case 'basic': case 'cors': case 'default': return true;
            }
            return false;
        }
    }

    function initSync(module) {
        if (wasm !== undefined) return wasm;


        if (module !== undefined) {
            if (Object.getPrototypeOf(module) === Object.prototype) {
                ({module} = module)
            } else {
                console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
            }
        }

        const imports = __wbg_get_imports();
        if (!(module instanceof WebAssembly.Module)) {
            module = new WebAssembly.Module(module);
        }
        const instance = new WebAssembly.Instance(module, imports);
        return __wbg_finalize_init(instance, module);
    }

    async function __wbg_init(module_or_path) {
        if (wasm !== undefined) return wasm;


        if (module_or_path !== undefined) {
            if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
                ({module_or_path} = module_or_path)
            } else {
                console.warn('using deprecated parameters for the initialization function; pass a single object instead')
            }
        }

        if (module_or_path === undefined && script_src !== undefined) {
            module_or_path = script_src.replace(/\.js$/, "_bg.wasm");
        }
        const imports = __wbg_get_imports();

        if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
            module_or_path = fetch(module_or_path);
        }

        const { instance, module } = await __wbg_load(await module_or_path, imports);

        return __wbg_finalize_init(instance, module);
    }

    return Object.assign(__wbg_init, { initSync }, exports);
})({ __proto__: null });
