use std::collections::VecDeque;
use std::sync::mpsc;

use core::time::Duration;
use raw_window_handle::RawDisplayHandle;

use backend::Backend;
use worker::Worker;

mod backend;
mod worker;

/// How long [`Clipboard::read`] waits for the clipboard on X11 and Wayland.
///
/// Another app answers well within this, so a paste usually arrives right away.
///
/// On other platforms the read is done before we get to wait.
const READ_WAIT: Duration = Duration::from_millis(50);

/// What a paste gets from the clipboard.
#[derive(Clone, Debug)]
pub enum ClipboardContent {
    Text(String),
    Image(egui::ColorImage),
}

/// Which clipboard [`Clipboard::read`] reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardKind {
    /// The normal clipboard. A read gets its text, or its image if it has no text.
    Clipboard,

    /// The X11/Wayland PRIMARY selection, which only has text.
    ///
    /// It is filled in by selecting text, and pasted with the middle mouse button.
    /// It is always empty on other platforms.
    Primary,
}

/// The result of [`Clipboard::read`] and [`Clipboard::poll_read`].
#[derive(Clone, Debug)]
pub enum ClipboardRead {
    /// The read is done. `None` means there is nothing to paste.
    Done(Option<ClipboardContent>),

    /// The read is still underway. Ask for the result with [`Clipboard::poll_read`].
    Pending,
}

/// Handles interfacing with the OS clipboard.
///
/// If the "clipboard" feature is off, or we cannot connect to the OS clipboard,
/// then a fallback clipboard that just works within the same app is used instead.
///
/// On Linux and the BSDs, the clipboard is used on its own thread. See [`Self::read`] for why.
pub struct Clipboard {
    worker: Worker,

    /// The reads that [`Self::read`] stopped waiting for, oldest first.
    late_reads: VecDeque<mpsc::Receiver<Option<ClipboardContent>>>,
}

impl Clipboard {
    /// Construct a new instance.
    ///
    /// # Safety
    ///
    /// If `raw_display_handle` is `Some`, the display handle must remain valid for the
    /// entire lifetime of the returned `Clipboard` instance.
    #[expect(unsafe_code)]
    pub unsafe fn new(raw_display_handle: Option<RawDisplayHandle>) -> Self {
        Self {
            // SAFETY: The caller guarantees that the display handle remains valid.
            worker: unsafe { Worker::new(raw_display_handle) },
            late_reads: Default::default(),
        }
    }

    /// Read the clipboard for a paste, without holding up the caller for long.
    ///
    /// On X11 and Wayland, the app that copied keeps the content, and a paste asks that app
    /// for it. The answer can take long. It never comes while we wait on the main thread,
    /// if something in our own process owns the clipboard and needs the main thread to answer
    /// (for example an embedded chromium webview).
    ///
    /// So if the content takes more than a moment, this returns [`ClipboardRead::Pending`].
    /// [`Self::poll_read`] returns the content once it is there.
    ///
    /// `wake` is called when the read is done. That can be on another thread.
    ///
    /// On other platforms the read is always done right away.
    pub fn read(
        &mut self,
        kind: ClipboardKind,
        wake: impl FnOnce() + Send + 'static,
    ) -> ClipboardRead {
        let (reply, result) = mpsc::channel();
        self.worker.run(move |backend| {
            _ = reply.send(backend.read(kind));
            wake();
        });

        // If an older read is still underway, this one must not overtake it.
        if self.late_reads.is_empty() {
            match result.recv_timeout(READ_WAIT) {
                Ok(content) => return ClipboardRead::Done(content),
                Err(mpsc::RecvTimeoutError::Disconnected) => return ClipboardRead::Done(None),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        self.late_reads.push_back(result);
        ClipboardRead::Pending
    }

    /// The result of the oldest [`Self::read`] that returned [`ClipboardRead::Pending`].
    ///
    /// Returns [`ClipboardRead::Pending`] if that read is still underway, or if there is none.
    pub fn poll_read(&mut self) -> ClipboardRead {
        let Some(result) = self.late_reads.front() else {
            return ClipboardRead::Pending;
        };
        let content = match result.try_recv() {
            Ok(content) => content,
            Err(mpsc::TryRecvError::Empty) => return ClipboardRead::Pending,
            Err(mpsc::TryRecvError::Disconnected) => None,
        };
        self.late_reads.pop_front();
        ClipboardRead::Done(content)
    }

    /// Get the text on the clipboard.
    ///
    /// This waits for the app that owns the clipboard. [`Self::read`] doesn't.
    #[deprecated = "It can block the event loop on X11 and Wayland. Use `Clipboard::read` instead."]
    pub fn get(&mut self) -> Option<String> {
        self.call(Backend::get)
    }

    /// Put text on the clipboard.
    ///
    /// On Linux and the BSDs this returns right away, and the clipboard is set a moment
    /// later, on the clipboard thread.
    pub fn set_text(&mut self, text: String) {
        self.worker.run(move |backend| backend.set_text(text));
    }

    /// Get an image from the clipboard, if there is one and the platform backend supports it.
    ///
    /// This mirrors [`Self::set_image`] for the opposite direction, so that a Ctrl+V/Cmd+V
    /// paste can carry an image (e.g. a screenshot or a copied image) instead of text — see
    /// [`egui::Event::PasteImage`].
    ///
    /// This waits for the app that owns the clipboard. [`Self::read`] doesn't.
    #[deprecated = "It can block the event loop on X11 and Wayland. Use `Clipboard::read` instead."]
    pub fn get_image(&mut self) -> Option<egui::ColorImage> {
        self.call(Backend::get_image)
    }

    /// Put an image on the clipboard, if the platform backend supports it.
    ///
    /// On Linux and the BSDs this returns right away, and the clipboard is set a moment
    /// later, on the clipboard thread.
    pub fn set_image(&mut self, image: &egui::ColorImage) {
        let image = image.clone();
        self.worker.run(move |backend| backend.set_image(&image));
    }

    /// Read the X11/Wayland PRIMARY selection, if there is one.
    ///
    /// It is filled in by selecting text, and pasted with the middle mouse button.
    /// Always `None` on other platforms.
    ///
    /// This waits for the app that owns the selection. [`Self::read`] doesn't.
    #[deprecated = "It can block the event loop on X11 and Wayland. Use `Clipboard::read` instead."]
    pub fn get_primary_text(&mut self) -> Option<String> {
        self.call(Backend::get_primary_text)
    }

    /// Set the X11/Wayland PRIMARY selection. Does nothing on other platforms.
    ///
    /// This returns right away, and the selection is set a moment later, on the clipboard
    /// thread.
    pub fn set_primary_text(&mut self, text: String) {
        self.worker
            .run(move |backend| backend.set_primary_text(text));
    }

    /// Runs `job` with the clipboard backend, and waits for its result.
    fn call<R: Send + 'static>(
        &mut self,
        job: impl FnOnce(&mut Backend) -> Option<R> + Send + 'static,
    ) -> Option<R> {
        let (reply, result) = mpsc::channel();
        self.worker.run(move |backend| {
            _ = reply.send(job(backend));
        });
        result.recv().ok().flatten()
    }
}
