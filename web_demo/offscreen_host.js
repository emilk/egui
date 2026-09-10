// Main thread: owns the DOM canvas, transfers it to a worker, forwards input.
(async () => {
  const canvas = document.getElementById("the_canvas");
  const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
  const offscreen = canvas.transferControlToOffscreen();

  const worker = new Worker("offscreen_worker.js");
  worker.postMessage({ type: "init", canvas: offscreen }, [offscreen]);

  const post = (message) => worker.postMessage(message);

  // --- sizing ---------------------------------------------------------
  const sendResize = () => {
    const rect = canvas.getBoundingClientRect();
    const style = getComputedStyle(canvas);
    const px = (name) => parseFloat(style.getPropertyValue(name)) || 0;
    post({
      type: "resize",
      width: Math.max(1, Math.round(rect.width * window.devicePixelRatio)),
      height: Math.max(1, Math.round(rect.height * window.devicePixelRatio)),
      dpr: window.devicePixelRatio,
      rect: {
        left: rect.left + px("padding-left") + px("border-left-width"),
        top: rect.top + px("padding-top") + px("border-top-width"),
        right: rect.right - px("padding-right") - px("border-right-width"),
        bottom: rect.bottom - px("padding-bottom") - px("border-bottom-width"),
      },
    });
  };
  new ResizeObserver(sendResize).observe(canvas);
  window.addEventListener("resize", sendResize);
  sendResize();

  // The worker asks for one frame at a time (`request_frame`)
  let frame_scheduled = false;
  const scheduleFrame = () => {
    if (frame_scheduled) return;
    frame_scheduled = true;
    const run = () => {
      frame_scheduled = false;
      post({ type: "frame" });
    };
    requestAnimationFrame(run);
  };

  const mods = (e) => ({ alt: e.altKey, ctrl: e.ctrlKey, meta: e.metaKey, shift: e.shiftKey, isMac });

  canvas.addEventListener("pointerdown", (e) => {
    try { canvas.setPointerCapture(e.pointerId); } catch (_) { }
    post({ type: "pointer", kind: "down", x: e.clientX, y: e.clientY, button: e.button, ...mods(e) });
  });
  canvas.addEventListener("pointerup", (e) =>
    post({ type: "pointer", kind: "up", x: e.clientX, y: e.clientY, button: e.button, ...mods(e) }));
  canvas.addEventListener("pointermove", (e) =>
    post({ type: "pointer", kind: "move", x: e.clientX, y: e.clientY, button: e.button, ...mods(e) }));
  canvas.addEventListener("pointercancel", () => post({ type: "pointer", kind: "cancel" }));
  canvas.addEventListener("pointerleave", () => post({ type: "pointer", kind: "leave" }));
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    post({ type: "wheel", dx: e.deltaX, dy: e.deltaY, mode: e.deltaMode, ...mods(e) });
  }, { passive: false });

  window.addEventListener("focus", () => post({ type: "focus", focused: true }));
  window.addEventListener("blur", () => post({ type: "focus", focused: false }));
  document.addEventListener("visibilitychange", () => {
    post({ type: "visibility", hidden: document.hidden });
    // A `requestAnimationFrame` that is already pending is paused while the tab is
    // hidden, so re-arm with the API that matches the new visibility.
    frame_scheduled = false;
    scheduleFrame();
  });

  worker.addEventListener("message", (event) => {
    const message = event.data;
    if (message.type === "request_frame") {
      scheduleFrame();
    } else if (message.type === "cursor") {
      canvas.style.cursor = message.icon;
    }
  });
})();
