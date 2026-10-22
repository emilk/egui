importScripts("egui_demo_app.js");

let ready = false;
let pending_resize = null;

self.onmessage = async (event) => {
  const message = event.data;
  if (!ready) {
    if (message.type === "resize") {
      pending_resize = message;
      return;
    }
    if (message.type !== "init") { return; }
    await wasm_bindgen("egui_demo_app_bg.wasm");
    await wasm_bindgen.start_offscreen_egui(message.canvas);
    ready = true;
    if (pending_resize) {
      wasm_bindgen.offscreen_on_message(pending_resize);
      pending_resize = null;
    }
    return;
  }
  wasm_bindgen.offscreen_on_message(message);
};
