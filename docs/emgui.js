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

    function passStringToWasm(arg) {

        const buf = cachedTextEncoder.encode(arg);
        const ptr = wasm.__wbindgen_malloc(buf.length);
        getUint8Memory().set(buf, ptr);
        return [ptr, buf.length];
    }

    let cachedTextDecoder = new TextDecoder('utf-8');

    function getStringFromWasm(ptr, len) {
        return cachedTextDecoder.decode(getUint8Memory().subarray(ptr, ptr + len));
    }

    let cachedGlobalArgumentPtr = null;
    function globalArgumentPtr() {
        if (cachedGlobalArgumentPtr === null) {
            cachedGlobalArgumentPtr = wasm.__wbindgen_global_argument_ptr();
        }
        return cachedGlobalArgumentPtr;
    }

    let cachegetUint32Memory = null;
    function getUint32Memory() {
        if (cachegetUint32Memory === null || cachegetUint32Memory.buffer !== wasm.memory.buffer) {
            cachegetUint32Memory = new Uint32Array(wasm.memory.buffer);
        }
        return cachegetUint32Memory;
    }
    /**
    * @param {string} arg0
    * @returns {string}
    */
    __exports.show_gui = function(arg0) {
        const [ptr0, len0] = passStringToWasm(arg0);
        const retptr = globalArgumentPtr();
        try {
            wasm.show_gui(retptr, ptr0, len0);
            const mem = getUint32Memory();
            const rustptr = mem[retptr / 4];
            const rustlen = mem[retptr / 4 + 1];

            const realRet = getStringFromWasm(rustptr, rustlen).slice();
            wasm.__wbindgen_free(rustptr, rustlen * 1);
            return realRet;


        } finally {
            wasm.__wbindgen_free(ptr0, len0 * 1);

        }

    };

    function freeInput(ptr) {

        wasm.__wbg_input_free(ptr);
    }
    /**
    */
    class Input {

        free() {
            const ptr = this.ptr;
            this.ptr = 0;
            freeInput(ptr);
        }

        /**
        * @returns {number}
        */
        get screen_width() {
            return wasm.__wbg_get_input_screen_width(this.ptr);
        }
        set screen_width(arg0) {
            return wasm.__wbg_set_input_screen_width(this.ptr, arg0);
        }
        /**
        * @returns {number}
        */
        get screen_height() {
            return wasm.__wbg_get_input_screen_height(this.ptr);
        }
        set screen_height(arg0) {
            return wasm.__wbg_set_input_screen_height(this.ptr, arg0);
        }
        /**
        * @returns {number}
        */
        get mouse_x() {
            return wasm.__wbg_get_input_mouse_x(this.ptr);
        }
        set mouse_x(arg0) {
            return wasm.__wbg_set_input_mouse_x(this.ptr, arg0);
        }
        /**
        * @returns {number}
        */
        get mouse_y() {
            return wasm.__wbg_get_input_mouse_y(this.ptr);
        }
        set mouse_y(arg0) {
            return wasm.__wbg_set_input_mouse_y(this.ptr, arg0);
        }
    }
    __exports.Input = Input;

    __exports.__wbindgen_throw = function(ptr, len) {
        throw new Error(getStringFromWasm(ptr, len));
    };

    function init(path_or_module) {
        let instantiation;
        const imports = { './emgui': __exports };
        if (path_or_module instanceof WebAssembly.Module) {
            instantiation = WebAssembly.instantiate(path_or_module, imports)
            .then(instance => {
            return { instance, module: module_or_path }
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
        return;
    });
};
self.wasm_bindgen = Object.assign(init, __exports);
})();
