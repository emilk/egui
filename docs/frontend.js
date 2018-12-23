// ----------------------------------------------------------------------------
// Paint module:
function paintCommand(canvas, cmd) {
    var ctx = canvas.getContext("2d");
    switch (cmd.kind) {
        case "clear":
            ctx.fillStyle = cmd.fill_style;
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            return;
        case "line":
            ctx.beginPath();
            ctx.lineWidth = cmd.line_width;
            ctx.strokeStyle = cmd.stroke_style;
            ctx.moveTo(cmd.from[0], cmd.from[1]);
            ctx.lineTo(cmd.to[0], cmd.to[1]);
            ctx.stroke();
            return;
        case "circle":
            ctx.fillStyle = cmd.fill_style;
            ctx.beginPath();
            ctx.arc(cmd.center[0], cmd.center[1], cmd.radius, 0, 2 * Math.PI, false);
            ctx.fill();
            return;
        case "rounded_rect":
            ctx.fillStyle = cmd.fill_style;
            var x = cmd.pos[0];
            var y = cmd.pos[1];
            var width = cmd.size[0];
            var height = cmd.size[1];
            var radius = cmd.radius;
            ctx.beginPath();
            ctx.moveTo(x + radius, y);
            ctx.lineTo(x + width - radius, y);
            ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
            ctx.lineTo(x + width, y + height - radius);
            ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
            ctx.lineTo(x + radius, y + height);
            ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
            ctx.lineTo(x, y + radius);
            ctx.quadraticCurveTo(x, y, x + radius, y);
            ctx.closePath();
            ctx.fill();
            return;
        case "text":
            ctx.font = cmd.font;
            ctx.fillStyle = cmd.fill_style;
            ctx.textAlign = cmd.text_align;
            ctx.fillText(cmd.text, cmd.pos[0], cmd.pos[1]);
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
        kind: "rounded_rect",
        pos: [100, 100],
        radius: 20,
        size: [200, 100]
    });
    return commands;
}
function paint_gui(canvas, mouse_pos) {
    var input = {
        mouse_x: mouse_pos.x,
        mouse_y: mouse_pos.y,
        screen_height: canvas.height,
        screen_width: canvas.width
    };
    var commands = rust_gui(input);
    for (var _i = 0, commands_1 = commands; _i < commands_1.length; _i++) {
        var cmd = commands_1[_i];
        paintCommand(canvas, cmd);
    }
}
// ----------------------------------------------------------------------------
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
        var mouse_pos = mouse_pos_from_event(canvas, evt);
        paint_gui(canvas, mouse_pos);
    }, false);
    canvas.addEventListener("mousedown", function (evt) {
        var mouse_pos = mouse_pos_from_event(canvas, evt);
        paint_gui(canvas, mouse_pos);
    }, false);
    paint_gui(canvas, { x: 0, y: 0 });
}
