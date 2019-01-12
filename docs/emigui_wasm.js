(function() {
    var wasm;
    const __exports = {};


    let cachedTextEncoder = new TextEncoder('utf-8');

    let cachegetUint8Memory = null;
    function getUint8Memory() {
        if (cachegetUint8Memory === null || cachegetUint8Memory.buffer !== wasm.memory.buffer) {
            cachegetUint8Memory = new Uint8Array(wasm.memory.buffer);
        }
        return cachegetUint8Memory;
    }

    let WASM_VECTOR_LEN = 0;

    function passStringToWasm(arg) {

        const buf = cachedTextEncoder.encode(arg);
        const ptr = wasm.__wbindgen_malloc(buf.length);
        getUint8Memory().set(buf, ptr);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }
    /**
    * @param {string} arg0
    * @returns {State}
    */
    __exports.new_webgl_gui = function(arg0) {
        const ptr0 = passStringToWasm(arg0);
        const len0 = WASM_VECTOR_LEN;
        try {
            return State.__wrap(wasm.new_webgl_gui(ptr0, len0));

        } finally {
            wasm.__wbindgen_free(ptr0, len0 * 1);

        }

    };

    /**
    * @param {State} arg0
    * @param {string} arg1
    * @returns {void}
    */
    __exports.run_gui = function(arg0, arg1) {
        const ptr1 = passStringToWasm(arg1);
        const len1 = WASM_VECTOR_LEN;
        try {
            return wasm.run_gui(arg0.ptr, ptr1, len1);

        } finally {
            wasm.__wbindgen_free(ptr1, len1 * 1);

        }

    };

    const heap = new Array(32);

    heap.fill(undefined);

    heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let cachedTextDecoder = new TextDecoder('utf-8');

function getStringFromWasm(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory().subarray(ptr, ptr + len));
}

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

const __widl_f_get_element_by_id_Document_target = typeof Document === 'undefined' ? null : Document.prototype.getElementById || function() {
    throw new Error(`wasm-bindgen: Document.getElementById does not exist`);
};

__exports.__widl_f_get_element_by_id_Document = function(arg0, arg1, arg2) {
    let varg1 = getStringFromWasm(arg1, arg2);

    const val = __widl_f_get_element_by_id_Document_target.call(getObject(arg0), varg1);
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

__exports.__widl_instanceof_HTMLCanvasElement = function(idx) {
    return getObject(idx) instanceof HTMLCanvasElement ? 1 : 0;
};

const __widl_f_get_context_HTMLCanvasElement_target = typeof HTMLCanvasElement === 'undefined' ? null : HTMLCanvasElement.prototype.getContext || function() {
    throw new Error(`wasm-bindgen: HTMLCanvasElement.getContext does not exist`);
};

let cachegetUint32Memory = null;
function getUint32Memory() {
    if (cachegetUint32Memory === null || cachegetUint32Memory.buffer !== wasm.memory.buffer) {
        cachegetUint32Memory = new Uint32Array(wasm.memory.buffer);
    }
    return cachegetUint32Memory;
}

__exports.__widl_f_get_context_HTMLCanvasElement = function(arg0, arg1, arg2, exnptr) {
    let varg1 = getStringFromWasm(arg1, arg2);
    try {

        const val = __widl_f_get_context_HTMLCanvasElement_target.call(getObject(arg0), varg1);
        return isLikeNone(val) ? 0 : addHeapObject(val);

    } catch (e) {
        const view = getUint32Memory();
        view[exnptr / 4] = 1;
        view[exnptr / 4 + 1] = addHeapObject(e);

    }
};

function GetOwnOrInheritedPropertyDescriptor(obj, id) {
    while (obj) {
        let desc = Object.getOwnPropertyDescriptor(obj, id);
        if (desc) return desc;
        obj = Object.getPrototypeOf(obj);
    }
return {}
}

const __widl_f_width_HTMLCanvasElement_target = GetOwnOrInheritedPropertyDescriptor(typeof HTMLCanvasElement === 'undefined' ? null : HTMLCanvasElement.prototype, 'width').get || function() {
    throw new Error(`wasm-bindgen: HTMLCanvasElement.width does not exist`);
};

__exports.__widl_f_width_HTMLCanvasElement = function(arg0) {
    return __widl_f_width_HTMLCanvasElement_target.call(getObject(arg0));
};

const __widl_f_height_HTMLCanvasElement_target = GetOwnOrInheritedPropertyDescriptor(typeof HTMLCanvasElement === 'undefined' ? null : HTMLCanvasElement.prototype, 'height').get || function() {
    throw new Error(`wasm-bindgen: HTMLCanvasElement.height does not exist`);
};

__exports.__widl_f_height_HTMLCanvasElement = function(arg0) {
    return __widl_f_height_HTMLCanvasElement_target.call(getObject(arg0));
};

const __widl_f_now_Performance_target = typeof Performance === 'undefined' ? null : Performance.prototype.now || function() {
    throw new Error(`wasm-bindgen: Performance.now does not exist`);
};

__exports.__widl_f_now_Performance = function(arg0) {
    return __widl_f_now_Performance_target.call(getObject(arg0));
};

__exports.__widl_instanceof_WebGLRenderingContext = function(idx) {
    return getObject(idx) instanceof WebGLRenderingContext ? 1 : 0;
};

const __widl_f_buffer_data_with_array_buffer_view_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.bufferData || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.bufferData does not exist`);
};

__exports.__widl_f_buffer_data_with_array_buffer_view_WebGLRenderingContext = function(arg0, arg1, arg2, arg3) {
    __widl_f_buffer_data_with_array_buffer_view_WebGLRenderingContext_target.call(getObject(arg0), arg1, getObject(arg2), arg3);
};

function getArrayU8FromWasm(ptr, len) {
    return getUint8Memory().subarray(ptr / 1, ptr / 1 + len);
}

const __widl_f_tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.texImage2D || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.texImage2D does not exist`);
};

__exports.__widl_f_tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array_WebGLRenderingContext = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, exnptr) {
    let varg9 = arg9 == 0 ? undefined : getArrayU8FromWasm(arg9, arg10);
    try {
        __widl_f_tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, varg9);
    } catch (e) {
        const view = getUint32Memory();
        view[exnptr / 4] = 1;
        view[exnptr / 4 + 1] = addHeapObject(e);

    }
};

const __widl_f_active_texture_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.activeTexture || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.activeTexture does not exist`);
};

__exports.__widl_f_active_texture_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_active_texture_WebGLRenderingContext_target.call(getObject(arg0), arg1);
};

const __widl_f_attach_shader_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.attachShader || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.attachShader does not exist`);
};

__exports.__widl_f_attach_shader_WebGLRenderingContext = function(arg0, arg1, arg2) {
    __widl_f_attach_shader_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), getObject(arg2));
};

const __widl_f_bind_buffer_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.bindBuffer || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.bindBuffer does not exist`);
};

__exports.__widl_f_bind_buffer_WebGLRenderingContext = function(arg0, arg1, arg2) {
    __widl_f_bind_buffer_WebGLRenderingContext_target.call(getObject(arg0), arg1, getObject(arg2));
};

const __widl_f_bind_texture_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.bindTexture || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.bindTexture does not exist`);
};

__exports.__widl_f_bind_texture_WebGLRenderingContext = function(arg0, arg1, arg2) {
    __widl_f_bind_texture_WebGLRenderingContext_target.call(getObject(arg0), arg1, getObject(arg2));
};

const __widl_f_blend_func_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.blendFunc || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.blendFunc does not exist`);
};

__exports.__widl_f_blend_func_WebGLRenderingContext = function(arg0, arg1, arg2) {
    __widl_f_blend_func_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2);
};

const __widl_f_clear_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.clear || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.clear does not exist`);
};

__exports.__widl_f_clear_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_clear_WebGLRenderingContext_target.call(getObject(arg0), arg1);
};

const __widl_f_clear_color_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.clearColor || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.clearColor does not exist`);
};

__exports.__widl_f_clear_color_WebGLRenderingContext = function(arg0, arg1, arg2, arg3, arg4) {
    __widl_f_clear_color_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2, arg3, arg4);
};

const __widl_f_compile_shader_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.compileShader || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.compileShader does not exist`);
};

__exports.__widl_f_compile_shader_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_compile_shader_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1));
};

const __widl_f_create_buffer_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.createBuffer || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.createBuffer does not exist`);
};

__exports.__widl_f_create_buffer_WebGLRenderingContext = function(arg0) {

    const val = __widl_f_create_buffer_WebGLRenderingContext_target.call(getObject(arg0));
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

const __widl_f_create_program_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.createProgram || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.createProgram does not exist`);
};

__exports.__widl_f_create_program_WebGLRenderingContext = function(arg0) {

    const val = __widl_f_create_program_WebGLRenderingContext_target.call(getObject(arg0));
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

const __widl_f_create_shader_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.createShader || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.createShader does not exist`);
};

__exports.__widl_f_create_shader_WebGLRenderingContext = function(arg0, arg1) {

    const val = __widl_f_create_shader_WebGLRenderingContext_target.call(getObject(arg0), arg1);
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

const __widl_f_create_texture_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.createTexture || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.createTexture does not exist`);
};

__exports.__widl_f_create_texture_WebGLRenderingContext = function(arg0) {

    const val = __widl_f_create_texture_WebGLRenderingContext_target.call(getObject(arg0));
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

const __widl_f_draw_elements_with_i32_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.drawElements || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.drawElements does not exist`);
};

__exports.__widl_f_draw_elements_with_i32_WebGLRenderingContext = function(arg0, arg1, arg2, arg3, arg4) {
    __widl_f_draw_elements_with_i32_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2, arg3, arg4);
};

const __widl_f_enable_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.enable || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.enable does not exist`);
};

__exports.__widl_f_enable_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_enable_WebGLRenderingContext_target.call(getObject(arg0), arg1);
};

const __widl_f_enable_vertex_attrib_array_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.enableVertexAttribArray || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.enableVertexAttribArray does not exist`);
};

__exports.__widl_f_enable_vertex_attrib_array_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_enable_vertex_attrib_array_WebGLRenderingContext_target.call(getObject(arg0), arg1);
};

const __widl_f_get_attrib_location_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.getAttribLocation || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.getAttribLocation does not exist`);
};

__exports.__widl_f_get_attrib_location_WebGLRenderingContext = function(arg0, arg1, arg2, arg3) {
    let varg2 = getStringFromWasm(arg2, arg3);
    return __widl_f_get_attrib_location_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), varg2);
};

const __widl_f_get_program_info_log_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.getProgramInfoLog || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.getProgramInfoLog does not exist`);
};

__exports.__widl_f_get_program_info_log_WebGLRenderingContext = function(ret, arg0, arg1) {
    const val = __widl_f_get_program_info_log_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1));
    const retptr = isLikeNone(val) ? [0, 0] : passStringToWasm(val);
    const retlen = WASM_VECTOR_LEN;
    const mem = getUint32Memory();
    mem[ret / 4] = retptr;
    mem[ret / 4 + 1] = retlen;

};

const __widl_f_get_program_parameter_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.getProgramParameter || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.getProgramParameter does not exist`);
};

__exports.__widl_f_get_program_parameter_WebGLRenderingContext = function(arg0, arg1, arg2) {
    return addHeapObject(__widl_f_get_program_parameter_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), arg2));
};

const __widl_f_get_shader_info_log_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.getShaderInfoLog || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.getShaderInfoLog does not exist`);
};

__exports.__widl_f_get_shader_info_log_WebGLRenderingContext = function(ret, arg0, arg1) {
    const val = __widl_f_get_shader_info_log_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1));
    const retptr = isLikeNone(val) ? [0, 0] : passStringToWasm(val);
    const retlen = WASM_VECTOR_LEN;
    const mem = getUint32Memory();
    mem[ret / 4] = retptr;
    mem[ret / 4 + 1] = retlen;

};

const __widl_f_get_shader_parameter_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.getShaderParameter || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.getShaderParameter does not exist`);
};

__exports.__widl_f_get_shader_parameter_WebGLRenderingContext = function(arg0, arg1, arg2) {
    return addHeapObject(__widl_f_get_shader_parameter_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), arg2));
};

const __widl_f_get_uniform_location_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.getUniformLocation || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.getUniformLocation does not exist`);
};

__exports.__widl_f_get_uniform_location_WebGLRenderingContext = function(arg0, arg1, arg2, arg3) {
    let varg2 = getStringFromWasm(arg2, arg3);

    const val = __widl_f_get_uniform_location_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), varg2);
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

const __widl_f_link_program_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.linkProgram || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.linkProgram does not exist`);
};

__exports.__widl_f_link_program_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_link_program_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1));
};

const __widl_f_shader_source_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.shaderSource || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.shaderSource does not exist`);
};

__exports.__widl_f_shader_source_WebGLRenderingContext = function(arg0, arg1, arg2, arg3) {
    let varg2 = getStringFromWasm(arg2, arg3);
    __widl_f_shader_source_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), varg2);
};

const __widl_f_tex_parameteri_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.texParameteri || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.texParameteri does not exist`);
};

__exports.__widl_f_tex_parameteri_WebGLRenderingContext = function(arg0, arg1, arg2, arg3) {
    __widl_f_tex_parameteri_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2, arg3);
};

const __widl_f_uniform1i_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.uniform1i || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.uniform1i does not exist`);
};

__exports.__widl_f_uniform1i_WebGLRenderingContext = function(arg0, arg1, arg2) {
    __widl_f_uniform1i_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), arg2);
};

const __widl_f_uniform2f_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.uniform2f || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.uniform2f does not exist`);
};

__exports.__widl_f_uniform2f_WebGLRenderingContext = function(arg0, arg1, arg2, arg3) {
    __widl_f_uniform2f_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1), arg2, arg3);
};

const __widl_f_use_program_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.useProgram || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.useProgram does not exist`);
};

__exports.__widl_f_use_program_WebGLRenderingContext = function(arg0, arg1) {
    __widl_f_use_program_WebGLRenderingContext_target.call(getObject(arg0), getObject(arg1));
};

const __widl_f_vertex_attrib_pointer_with_i32_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.vertexAttribPointer || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.vertexAttribPointer does not exist`);
};

__exports.__widl_f_vertex_attrib_pointer_with_i32_WebGLRenderingContext = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
    __widl_f_vertex_attrib_pointer_with_i32_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2, arg3, arg4 !== 0, arg5, arg6);
};

const __widl_f_viewport_WebGLRenderingContext_target = typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype.viewport || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.viewport does not exist`);
};

__exports.__widl_f_viewport_WebGLRenderingContext = function(arg0, arg1, arg2, arg3, arg4) {
    __widl_f_viewport_WebGLRenderingContext_target.call(getObject(arg0), arg1, arg2, arg3, arg4);
};

const __widl_f_drawing_buffer_width_WebGLRenderingContext_target = GetOwnOrInheritedPropertyDescriptor(typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype, 'drawingBufferWidth').get || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.drawingBufferWidth does not exist`);
};

__exports.__widl_f_drawing_buffer_width_WebGLRenderingContext = function(arg0) {
    return __widl_f_drawing_buffer_width_WebGLRenderingContext_target.call(getObject(arg0));
};

const __widl_f_drawing_buffer_height_WebGLRenderingContext_target = GetOwnOrInheritedPropertyDescriptor(typeof WebGLRenderingContext === 'undefined' ? null : WebGLRenderingContext.prototype, 'drawingBufferHeight').get || function() {
    throw new Error(`wasm-bindgen: WebGLRenderingContext.drawingBufferHeight does not exist`);
};

__exports.__widl_f_drawing_buffer_height_WebGLRenderingContext = function(arg0) {
    return __widl_f_drawing_buffer_height_WebGLRenderingContext_target.call(getObject(arg0));
};

__exports.__widl_instanceof_Window = function(idx) {
    return getObject(idx) instanceof Window ? 1 : 0;
};

__exports.__widl_f_document_Window = function(arg0) {

    const val = getObject(arg0).document;
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

__exports.__widl_f_performance_Window = function(arg0) {

    const val = getObject(arg0).performance;
    return isLikeNone(val) ? 0 : addHeapObject(val);

};

__exports.__wbg_new_c1b585153cd441ad = function(arg0) {
    return addHeapObject(new Float32Array(getObject(arg0)));
};

__exports.__wbg_subarray_6bef35518c0617bd = function(arg0, arg1, arg2) {
    return addHeapObject(getObject(arg0).subarray(arg1, arg2));
};

__exports.__wbg_newnoargs_6a80f84471205fc8 = function(arg0, arg1) {
    let varg0 = getStringFromWasm(arg0, arg1);
    return addHeapObject(new Function(varg0));
};

__exports.__wbg_call_582b20dfcad7fee4 = function(arg0, arg1, exnptr) {
    try {
        return addHeapObject(getObject(arg0).call(getObject(arg1)));
    } catch (e) {
        const view = getUint32Memory();
        view[exnptr / 4] = 1;
        view[exnptr / 4 + 1] = addHeapObject(e);

    }
};

__exports.__wbg_new_b192e0932f97e870 = function(arg0) {
    return addHeapObject(new Int16Array(getObject(arg0)));
};

__exports.__wbg_subarray_46088290ce2ce733 = function(arg0, arg1, arg2) {
    return addHeapObject(getObject(arg0).subarray(arg1, arg2));
};

__exports.__wbg_new_bc68e7aae4d4f791 = function(arg0) {
    return addHeapObject(new Uint8Array(getObject(arg0)));
};

__exports.__wbg_subarray_705096b76e15e94e = function(arg0, arg1, arg2) {
    return addHeapObject(getObject(arg0).subarray(arg1, arg2));
};

__exports.__wbg_new_3153ff3e90269012 = function(arg0) {
    return addHeapObject(new Uint16Array(getObject(arg0)));
};

__exports.__wbg_subarray_6f181829e5fd3854 = function(arg0, arg1, arg2) {
    return addHeapObject(getObject(arg0).subarray(arg1, arg2));
};

__exports.__wbg_instanceof_Memory_d223615e29613829 = function(idx) {
    return getObject(idx) instanceof WebAssembly.Memory ? 1 : 0;
};

__exports.__wbg_buffer_0413d5edaa0ff323 = function(arg0) {
    return addHeapObject(getObject(arg0).buffer);
};

function freeState(ptr) {

    wasm.__wbg_state_free(ptr);
}
/**
*/
class State {

    static __wrap(ptr) {
        const obj = Object.create(State.prototype);
        obj.ptr = ptr;

        return obj;
    }

    free() {
        const ptr = this.ptr;
        this.ptr = 0;
        freeState(ptr);
    }

}
__exports.State = State;

__exports.__wbindgen_object_clone_ref = function(idx) {
    return addHeapObject(getObject(idx));
};

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

__exports.__wbindgen_object_drop_ref = function(i) { dropObject(i); };

__exports.__wbindgen_string_new = function(p, l) {
    return addHeapObject(getStringFromWasm(p, l));
};

__exports.__wbindgen_number_get = function(n, invalid) {
    let obj = getObject(n);
    if (typeof(obj) === 'number') return obj;
    getUint8Memory()[invalid] = 1;
    return 0;
};

__exports.__wbindgen_is_null = function(idx) {
    return getObject(idx) === null ? 1 : 0;
};

__exports.__wbindgen_is_undefined = function(idx) {
    return getObject(idx) === undefined ? 1 : 0;
};

__exports.__wbindgen_boolean_get = function(i) {
    let v = getObject(i);
    if (typeof(v) === 'boolean') {
        return v ? 1 : 0;
    } else {
        return 2;
    }
};

__exports.__wbindgen_is_symbol = function(i) {
    return typeof(getObject(i)) === 'symbol' ? 1 : 0;
};

__exports.__wbindgen_string_get = function(i, len_ptr) {
    let obj = getObject(i);
    if (typeof(obj) !== 'string') return 0;
    const ptr = passStringToWasm(obj);
    getUint32Memory()[len_ptr / 4] = WASM_VECTOR_LEN;
    return ptr;
};

__exports.__wbindgen_memory = function() { return addHeapObject(wasm.memory); };

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

__exports.__wbindgen_rethrow = function(idx) { throw takeObject(idx); };

__exports.__wbindgen_throw = function(ptr, len) {
    throw new Error(getStringFromWasm(ptr, len));
};

function init(path_or_module) {
    let instantiation;
    const imports = { './emigui_wasm': __exports };
    if (path_or_module instanceof WebAssembly.Module) {
        instantiation = WebAssembly.instantiate(path_or_module, imports)
        .then(instance => {
        return { instance, module: path_or_module }
    });
} else {
    const data = fetch(path_or_module);
    if (typeof WebAssembly.instantiateStreaming === 'function') {
        instantiation = WebAssembly.instantiateStreaming(data, imports);
    } else {
        instantiation = data
        .then(response => response.arrayBuffer())
        .then(buffer => WebAssembly.instantiate(buffer, imports));
    }
}
return instantiation.then(({instance}) => {
    wasm = init.wasm = instance.exports;

});
};
self.wasm_bindgen = Object.assign(init, __exports);
})();
