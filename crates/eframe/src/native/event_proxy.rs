//! Keep typed eframe events beside winit's coalescing wake-up notification.
use super::winit_integration::UserEvent;
use egui::mutex::Mutex;
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct UserEventProxy {
    proxy: winit::event_loop::EventLoopProxy,
    events: Arc<Mutex<Vec<UserEvent>>>,
}

impl UserEventProxy {
    pub fn new(proxy: winit::event_loop::EventLoopProxy) -> Self {
        Self {
            proxy,
            events: Default::default(),
        }
    }

    pub fn send_event(&self, event: UserEvent) {
        self.events.lock().push(event);
        self.proxy.wake_up();
    }

    pub fn take(&self) -> Vec<UserEvent> {
        core::mem::take(&mut *self.events.lock())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug, Default)]
    struct CoalescedWake(AtomicBool);

    impl winit::event_loop::EventLoopProxyProvider for CoalescedWake {
        fn wake_up(&self) {
            self.0.store(true, Ordering::Release);
        }
    }

    #[test]
    fn coalesced_wakes_preserve_every_threaded_event() {
        let wake = Arc::new(CoalescedWake::default());
        let proxy = UserEventProxy::new(winit::event_loop::EventLoopProxy::new(
            Arc::<CoalescedWake>::clone(&wake),
        ));
        std::thread::scope(|scope| {
            for sender in 0..4 {
                let proxy = proxy.clone();
                scope.spawn(move || {
                    for item in 0..32 {
                        proxy.send_event(UserEvent::RequestRepaint {
                            viewport_id: egui::ViewportId::ROOT,
                            when: std::time::Instant::now(),
                            cumulative_pass_nr: sender * 32 + item,
                        });
                    }
                });
            }
        });
        assert!(wake.0.swap(false, Ordering::Acquire));
        let mut passes: Vec<_> = proxy
            .take()
            .into_iter()
            .map(|event| match event {
                UserEvent::RequestRepaint {
                    cumulative_pass_nr, ..
                } => cumulative_pass_nr,
                #[cfg(feature = "accesskit")]
                UserEvent::AccessKitActionRequest(_) => u64::MAX,
            })
            .collect();
        passes.sort_unstable();
        assert_eq!(passes, (0..128).collect::<Vec<_>>());
        assert!(proxy.take().is_empty());
    }
}
