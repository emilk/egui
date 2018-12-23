// ----------------------------------------------------------------------------
// Paint module:

interface Clear {
  kind: "clear";
  fill_style: string;
}

interface Line {
  kind: "line";
  from: [number, number];
  line_width: number;
  stroke_style: string;
  to: [number, number];
}

interface Circle {
  kind: "circle";
  center: [number, number];
  fill_style: string;
  radius: number;
}

interface RoundedRect {
  kind: "rounded_rect";
  fill_style: string;
  pos: [number, number];
  radius: number;
  size: [number, number];
}

interface Text {
  kind: "text";
  fill_style: string;
  font: string;
  pos: [number, number];
  text: string;
  text_align: "start" | "center" | "end";
}

type PaintCmd = Clear | Line | Circle | RoundedRect | Text;

function paintCommand(canvas, cmd: PaintCmd) {
  const ctx = canvas.getContext("2d");

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
      const x = cmd.pos[0];
      const y = cmd.pos[1];
      const width = cmd.size[0];
      const height = cmd.size[1];
      const radius = cmd.radius;
      ctx.beginPath();
      ctx.moveTo(x + radius, y);
      ctx.lineTo(x + width - radius, y);
      ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
      ctx.lineTo(x + width, y + height - radius);
      ctx.quadraticCurveTo(
        x + width,
        y + height,
        x + width - radius,
        y + height,
      );
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

// ----------------------------------------------------------------------------

interface Coord {
  x: number;
  y: number;
}

interface Input {
  mouse_x: number;
  mouse_y: number;
  screen_height: number;
  screen_width: number;
  // TODO: mouse down etc
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
wasm_bindgen("./emgui_bg.wasm")
  .then(wasm_loaded)
  .catch(console.error);

function rust_gui(input: Input): PaintCmd[] {
  return JSON.parse(wasm_bindgen.show_gui(JSON.stringify(input)));
}

// ----------------------------------------------------------------------------

function js_gui(input: Input): PaintCmd[] {
  const commands = [];

  commands.push({
    fillStyle: "#111111",
    kind: "clear",
  });

  commands.push({
    fillStyle: "#ff1111",
    kind: "rounded_rect",
    pos: [100, 100],
    radius: 20,
    size: [200, 100],
  });

  return commands;
}

function paint_gui(canvas, mouse_pos) {
  const input = {
    mouse_x: mouse_pos.x,
    mouse_y: mouse_pos.y,
    screen_height: canvas.height,
    screen_width: canvas.width,
  };
  const commands = rust_gui(input);

  for (const cmd of commands) {
    paintCommand(canvas, cmd);
  }
}

// ----------------------------------------------------------------------------

function mouse_pos_from_event(canvas, evt): Coord {
  const rect = canvas.getBoundingClientRect();
  return {
    x: evt.clientX - rect.left,
    y: evt.clientY - rect.top,
  };
}

function initialize() {
  const canvas = document.getElementById("canvas");

  canvas.addEventListener(
    "mousemove",
    (evt) => {
      const mouse_pos = mouse_pos_from_event(canvas, evt);
      paint_gui(canvas, mouse_pos);
    },
    false,
  );

  canvas.addEventListener(
    "mousedown",
    (evt) => {
      const mouse_pos = mouse_pos_from_event(canvas, evt);
      paint_gui(canvas, mouse_pos);
    },
    false,
  );

  paint_gui(canvas, { x: 0, y: 0 });
}
