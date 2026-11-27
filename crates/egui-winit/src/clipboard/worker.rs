//! Where the clipboard backend runs.

use raw_window_handle::RawDisplayHandle;

use super::backend::Backend;

cfg_select! {
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ) => {
        use std::sync::mpsc;

        /// Something to do with the clipboard backend.
        type Job = Box<dyn FnOnce(&mut Backend) + Send>;

        /// Owns the clipboard backend, on its own thread.
        ///
        /// That way a slow clipboard never holds up the thread that the event loop runs on.
        pub struct Worker {
            /// `None` only while dropping.
            jobs: Option<mpsc::Sender<Job>>,

            thread: Option<std::thread::JoinHandle<()>>,
        }

        impl Worker {
            /// # Safety
            ///
            /// If `raw_display_handle` is `Some`, the display handle must remain valid for
            /// the entire lifetime of the returned `Worker`.
            #[expect(unsafe_code)]
            pub unsafe fn new(raw_display_handle: Option<RawDisplayHandle>) -> Self {
                let display = SendDisplayHandle(raw_display_handle);
                let (jobs, jobs_rx) = mpsc::channel::<Job>();
                let thread = std::thread::Builder::new()
                    .name("egui-clipboard".to_owned())
                    .spawn(move || {
                        // SAFETY: the caller of `new` keeps the display valid for as long as
                        // the `Worker` lives, and the `Worker` waits for this thread when it
                        // is dropped.
                        let mut backend = unsafe { Backend::new(display.into_inner()) };
                        for job in jobs_rx {
                            job(&mut backend);
                        }
                    })
                    .inspect_err(|err| log::error!("Failed to start the clipboard thread: {err}"))
                    .ok();
                Self {
                    jobs: Some(jobs),
                    thread,
                }
            }

            /// Runs `job` on the clipboard thread, after the jobs before it.
            #[expect(
                clippy::needless_pass_by_ref_mut,
                reason = "the same signature as on the other platforms"
            )]
            pub fn run(&mut self, job: impl FnOnce(&mut Backend) + Send + 'static) {
                if let Some(jobs) = &self.jobs {
                    // This only fails if the thread is gone. Then the job is dropped, and
                    // whoever waits for its result gets none.
                    _ = jobs.send(Box::new(job));
                }
            }
        }

        impl Drop for Worker {
            fn drop(&mut self) {
                // The thread stops when it has done all jobs.
                self.jobs = None;
                // The backend may use the display, which is only valid as long as we live.
                // So wait for the thread. A read that is underway holds this up.
                if let Some(thread) = self.thread.take() {
                    _ = thread.join();
                }
            }
        }

        /// Lets the display handle move to the clipboard thread.
        struct SendDisplayHandle(Option<RawDisplayHandle>);

        impl SendDisplayHandle {
            fn into_inner(self) -> Option<RawDisplayHandle> {
                self.0
            }
        }

        // SAFETY: the backend only uses the handle of a Wayland display, and that may be used
        // from any thread.
        #[expect(unsafe_code)]
        unsafe impl Send for SendDisplayHandle {}
    }
    _ => {
        /// Owns the clipboard backend, and uses it right away.
        ///
        /// On these platforms the system keeps what was copied, so a read doesn't take long.
        pub struct Worker {
            backend: Backend,
        }

        impl Worker {
            /// # Safety
            ///
            /// If `raw_display_handle` is `Some`, the display handle must remain valid for
            /// the entire lifetime of the returned `Worker`.
            #[expect(unsafe_code)]
            pub unsafe fn new(raw_display_handle: Option<RawDisplayHandle>) -> Self {
                Self {
                    // SAFETY: The caller guarantees that the display handle remains valid.
                    backend: unsafe { Backend::new(raw_display_handle) },
                }
            }

            pub fn run(&mut self, job: impl FnOnce(&mut Backend)) {
                job(&mut self.backend);
            }
        }
    }
}
