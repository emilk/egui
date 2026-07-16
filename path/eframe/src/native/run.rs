// Copyright 2023 The eframe Authors. All rights reserved.
// Use of this source code is governed by the Apache License 2.0
// that can be found in the LICENSE file.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::unix::EventLoopExtUnix;
use crate::native::event_loop_context::EventLoopContext;

pub struct Run {
    event_loop: EventLoop<()>,
    control_flow: Arc<AtomicBool>,
}

impl Run {
    pub fn new() -> Self {
        Self {
            event_loop: EventLoop::new(),
            control_flow: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn run(&self) {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                winit::Event::RedrawRequested(_) => {
                    if self.control_flow.load(Ordering::Relaxed) {
                        self.event_loop.request_redraw();
                    }
                }
                _ => (),
            }
        });
    }

    pub fn set_control_flow(&self, control_flow: ControlFlow) {
        self.control_flow.store(control_flow == ControlFlow::Poll, Ordering::Relaxed);
    }
}