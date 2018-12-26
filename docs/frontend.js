function paintCommand(canvas, cmd) {
    var ctx = canvas.getContext("2d");
    // console.log(`cmd: ${JSON.stringify(cmd)}`);
    switch (cmd.kind) {
        case "circle":
            ctx.beginPath();
            ctx.arc(cmd.center.x, cmd.center.y, cmd.radius, 0, 2 * Math.PI, false);
            if (cmd.fill_style) {
                ctx.fillStyle = cmd.fill_style;
                ctx.fill();
            }
            if (cmd.outline) {
                ctx.lineWidth = cmd.outline.width;
                ctx.strokeStyle = cmd.outline.style;
                ctx.stroke();
            }
            return;
        case "clear":
            ctx.fillStyle = cmd.fill_style;
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            return;
        case "line":
            ctx.beginPath();
            ctx.moveTo(cmd.points[0].x, cmd.points[0].y);
            for (var _i = 0, _a = cmd.points; _i < _a.length; _i++) {
                var point = _a[_i];
                ctx.lineTo(point.x, point.y);
            }
            ctx.lineWidth = cmd.width;
            ctx.strokeStyle = cmd.style;
            ctx.stroke();
            return;
        case "rect":
            var x = cmd.pos.x;
            var y = cmd.pos.y;
            var width = cmd.size.x;
            var height = cmd.size.y;
            var r = Math.min(cmd.corner_radius, width / 2, height / 2);
            ctx.beginPath();
            ctx.moveTo(x + r, y);
            ctx.lineTo(x + width - r, y);
            ctx.quadraticCurveTo(x + width, y, x + width, y + r);
            ctx.lineTo(x + width, y + height - r);
            ctx.quadraticCurveTo(x + width, y + height, x + width - r, y + height);
            ctx.lineTo(x + r, y + height);
            ctx.quadraticCurveTo(x, y + height, x, y + height - r);
            ctx.lineTo(x, y + r);
            ctx.quadraticCurveTo(x, y, x + r, y);
            ctx.closePath();
            if (cmd.fill_style) {
                ctx.fillStyle = cmd.fill_style;
                ctx.fill();
            }
            if (cmd.outline) {
                ctx.lineWidth = cmd.outline.width;
                ctx.strokeStyle = cmd.outline.style;
                ctx.stroke();
            }
            return;
        case "text":
            ctx.font = cmd.font;
            ctx.fillStyle = cmd.fill_style;
            ctx.textAlign = cmd.text_align;
            ctx.fillText(cmd.text, cmd.pos.x, cmd.pos.y);
            return;
    }
}
// we'll defer our execution until the wasm is ready to go
function wasm_loaded() {
    console.log("wasm loaded");
    initialize();
}
// here we tell bindgen the path to the wasm file so it can start
// initialization and return to us a promise when it's done
wasm_bindgen("./emgui_bg.wasm")
    .then(wasm_loaded)["catch"](console.error);
function rust_gui(input) {
    return JSON.parse(wasm_bindgen.show_gui(JSON.stringify(input)));
}
// ----------------------------------------------------------------------------
function js_gui(input) {
    var commands = [];
    commands.push({
        fillStyle: "#111111",
        kind: "clear"
    });
    commands.push({
        fillStyle: "#ff1111",
        kind: "rect",
        pos: { x: 100, y: 100 },
        radius: 20,
        size: { x: 200, y: 100 }
    });
    return commands;
}
function paint_gui(canvas, input) {
    var commands = rust_gui(input);
    commands.unshift({
        fill_style: "#00000000",
        kind: "clear"
    });
    for (var _i = 0, commands_1 = commands; _i < commands_1.length; _i++) {
        var cmd = commands_1[_i];
        paintCommand(canvas, cmd);
    }
}
// ----------------------------------------------------------------------------
var g_mouse_pos = { x: -1000.0, y: -1000.0 };
var g_mouse_down = false;
function get_input(canvas) {
    return {
        mouse_down: g_mouse_down,
        mouse_pos: g_mouse_pos,
        screen_size: { x: canvas.width, y: canvas.height }
    };
}
function mouse_pos_from_event(canvas, evt) {
    var rect = canvas.getBoundingClientRect();
    return {
        x: evt.clientX - rect.left,
        y: evt.clientY - rect.top
    };
}
function initialize() {
    var canvas = document.getElementById("canvas");
    canvas.addEventListener("mousemove", function (evt) {
        g_mouse_pos = mouse_pos_from_event(canvas, evt);
        paint_gui(canvas, get_input(canvas));
    }, false);
    canvas.addEventListener("mouseleave", function (evt) {
        g_mouse_pos = { x: -1000.0, y: -1000.0 };
        paint_gui(canvas, get_input(canvas));
    }, false);
    canvas.addEventListener("mousedown", function (evt) {
        g_mouse_pos = mouse_pos_from_event(canvas, evt);
        g_mouse_down = true;
        paint_gui(canvas, get_input(canvas));
    }, false);
    canvas.addEventListener("mouseup", function (evt) {
        g_mouse_pos = mouse_pos_from_event(canvas, evt);
        g_mouse_down = false;
        paint_gui(canvas, get_input(canvas));
    }, false);
    paint_gui(canvas, get_input(canvas));
}
