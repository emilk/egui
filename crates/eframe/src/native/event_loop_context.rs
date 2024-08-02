use std::cell::Cell;
use winit::event_loop::ActiveEventLoop;

thread_local! {
    static CURRENT_EVENT_LOOP: Cell<Option<*const ActiveEventLoop>> = Cell::new(None);
}

struct EventLoopGuard;

impl EventLoopGuard {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        CURRENT_EVENT_LOOP.with(|cell| {
            assert!(
                cell.get().is_none(),
                "Attempted to set a new event loop while one is already set"
            );
            cell.set(Some(event_loop as *const ActiveEventLoop));
        });
        Self
    }
}

impl Drop for EventLoopGuard {
    fn drop(&mut self) {
        CURRENT_EVENT_LOOP.with(|cell| cell.set(None));
    }
}

// Helper function to safely use the current event loop
#[allow(unsafe_code)]
pub fn with_current_event_loop<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&ActiveEventLoop) -> R,
{
    CURRENT_EVENT_LOOP.with(|cell| {
        cell.get().map(|ptr| {
            // SAFETY:
            // 1. The pointer is guaranteed to be valid when it's Some, as the EventLoopGuard that created it
            //    lives at least as long as the reference, and clears it when it's dropped. Only run_with_event_loop creates
            //    a new EventLoopGuard, and does not leak it.
            // 2. Since the pointer was created from a borrow which lives at least as long as this pointer there are
            //    no mutable references to the ActiveEventLoop.
            let event_loop = unsafe { &*ptr };
            f(event_loop)
        })
    })
}

// The only public interface to use the event loop
pub fn with_event_loop_context(event_loop: &ActiveEventLoop, f: impl FnOnce()) {
    // NOTE: For safety, this guard must NOT be leaked.
    let _guard = EventLoopGuard::new(event_loop);
    f();
}
