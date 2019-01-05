// we'll defer our execution until the wasm is ready to go
function wasm_loaded() {
    console.log("wasm loaded");
    initialize();
}
// here we tell bindgen the path to the wasm file so it can start
// initialization and return to us a promise when it's done
wasm_bindgen("./emgui_wasm_bg.wasm")
    .then(wasm_loaded)["catch"](console.error);
// ----------------------------------------------------------------------------
var g_webgl_painter = null;
function paint_gui(canvas, input) {
    if (g_webgl_painter === null) {
        g_webgl_painter = wasm_bindgen.new_webgl_painter("canvas");
    }
    wasm_bindgen.paint_webgl(g_webgl_painter, JSON.stringify(input));
}
// ----------------------------------------------------------------------------
var g_mouse_pos = { x: -1000.0, y: -1000.0 };
var g_mouse_down = false;
function auto_resize_canvas(canvas) {
    if (true) {
        canvas.setAttribute("width", window.innerWidth);
        canvas.setAttribute("height", window.innerHeight);
    }
    else {
        // TODO: this stuff
        var pixels_per_point = window.devicePixelRatio || 1;
        var ctx = canvas.getContext("2d");
        ctx.scale(pixels_per_point, pixels_per_point);
        canvas.setAttribute("width", window.innerWidth * pixels_per_point);
        canvas.setAttribute("height", window.innerHeight * pixels_per_point);
    }
}
function get_input(canvas) {
    return {
        mouse_down: g_mouse_down,
        mouse_pos: g_mouse_pos,
        screen_size: { x: canvas.width, y: canvas.height }
    };
}
function mouse_pos_from_event(canvas, event) {
    var rect = canvas.getBoundingClientRect();
    return {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top
    };
}
function initialize() {
    console.log("window.devicePixelRatio: " + window.devicePixelRatio);
    var canvas = document.getElementById("canvas");
    auto_resize_canvas(canvas);
    var repaint = function () { return paint_gui(canvas, get_input(canvas)); };
    canvas.addEventListener("mousemove", function (event) {
        g_mouse_pos = mouse_pos_from_event(canvas, event);
        repaint();
        event.stopPropagation();
        event.preventDefault();
    });
    canvas.addEventListener("mouseleave", function (event) {
        g_mouse_pos = { x: -1000.0, y: -1000.0 };
        repaint();
        event.stopPropagation();
        event.preventDefault();
    });
    canvas.addEventListener("mousedown", function (event) {
        g_mouse_pos = mouse_pos_from_event(canvas, event);
        g_mouse_down = true;
        repaint();
        event.stopPropagation();
        event.preventDefault();
    });
    canvas.addEventListener("mouseup", function (event) {
        g_mouse_pos = mouse_pos_from_event(canvas, event);
        g_mouse_down = false;
        repaint();
        event.stopPropagation();
        event.preventDefault();
    });
    window.addEventListener("load", repaint);
    window.addEventListener("pagehide", repaint);
    window.addEventListener("pageshow", repaint);
    window.addEventListener("resize", repaint);
    // setInterval(repaint, 16);
    repaint();
}
