interface Vec2 {
  x: number;
  y: number;
}

/// What the integration gives to the gui.
interface RawInput {
  /// Is the button currently down?
  mouse_down: boolean;

  /// Current position of the mouse in points.
  mouse_pos: Vec2;

  /// Size of the screen in points.
  screen_size: Vec2;
}

// ----------------------------------------------------------------------------

// the `wasm_bindgen` global is set to the exports of the Rust module. Override with wasm-bindgen --no-modules-global
declare var wasm_bindgen: any;

// we'll defer our execution until the wasm is ready to go
function wasm_loaded() {
  console.log(`wasm loaded`);
  initialize();
}

// here we tell bindgen the path to the wasm file so it can start
// initialization and return to us a promise when it's done
wasm_bindgen("./emgui_wasm_bg.wasm")
  .then(wasm_loaded)
  .catch(console.error);

// ----------------------------------------------------------------------------

let g_webgl_painter = null;

function paint_gui(canvas, input: RawInput) {
  if (g_webgl_painter === null) {
    g_webgl_painter = wasm_bindgen.new_webgl_painter("canvas");
  }
  wasm_bindgen.paint_webgl(g_webgl_painter, JSON.stringify(input));
}

// ----------------------------------------------------------------------------

let g_mouse_pos = { x: -1000.0, y: -1000.0 };
let g_mouse_down = false;

function auto_resize_canvas(canvas) {
  if (true) {
    canvas.setAttribute("width", window.innerWidth);
    canvas.setAttribute("height", window.innerHeight);
  } else {
    // TODO: this stuff
    const pixels_per_point = window.devicePixelRatio || 1;

    const ctx = canvas.getContext("2d");
    ctx.scale(pixels_per_point, pixels_per_point);

    canvas.setAttribute("width", window.innerWidth * pixels_per_point);
    canvas.setAttribute("height", window.innerHeight * pixels_per_point);
  }
}

function get_input(canvas): RawInput {
  return {
    mouse_down: g_mouse_down,
    mouse_pos: g_mouse_pos,
    screen_size: { x: canvas.width, y: canvas.height },
  };
}

function mouse_pos_from_event(canvas, event): Vec2 {
  const rect = canvas.getBoundingClientRect();
  return {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top,
  };
}

function initialize() {
  console.log(`window.devicePixelRatio: ${window.devicePixelRatio}`);

  const canvas = document.getElementById("canvas");
  auto_resize_canvas(canvas);
  const repaint = () => paint_gui(canvas, get_input(canvas));

  canvas.addEventListener("mousemove", event => {
    g_mouse_pos = mouse_pos_from_event(canvas, event);
    repaint();
    event.stopPropagation();
    event.preventDefault();
  });

  canvas.addEventListener("mouseleave", event => {
    g_mouse_pos = { x: -1000.0, y: -1000.0 };
    repaint();
    event.stopPropagation();
    event.preventDefault();
  });

  canvas.addEventListener("mousedown", event => {
    g_mouse_pos = mouse_pos_from_event(canvas, event);
    g_mouse_down = true;
    repaint();
    event.stopPropagation();
    event.preventDefault();
  });

  canvas.addEventListener("mouseup", event => {
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
