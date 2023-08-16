//! Set the native app icon at runtime.
//!
//! TODO(emilk): port this to [`winit`].

use crate::IconData;

pub struct AppTitleIconSetter {
    title: String,
    icon_data: Option<IconData>,
    status: AppIconStatus,
}

impl AppTitleIconSetter {
    pub fn new(title: String, icon_data: Option<IconData>) -> Self {
        Self {
            title,
            icon_data,
            status: AppIconStatus::NotSetTryAgain,
        }
    }

    /// Call once per frame; we will set the icon when we can.
    pub fn update(&mut self) {
        if self.status == AppIconStatus::NotSetTryAgain {
            self.status = set_title_and_icon(&self.title, self.icon_data.as_ref());
        }
    }
}

/// In which state the app icon is (as far as we know).
#[derive(PartialEq, Eq)]
enum AppIconStatus {
    /// We did not set it or failed to do it. In any case we won't try again.
    NotSetIgnored,

    /// We haven't set the icon yet, we should try again next frame.
    ///
    /// This can happen repeatedly due to lazy window creation on some platforms.
    NotSetTryAgain,

    /// We successfully set the icon and it should be visible now.
    #[allow(dead_code)] // Not used on Linux
    Set,
}

/// Sets app icon at runtime.
///
/// By setting the icon at runtime and not via resource files etc. we ensure that we'll get the chance
/// to set the same icon when the process/window is started from python (which sets its own icon ahead of us!).
///
/// Since window creation can be lazy, call this every frame until it's either successfully or gave up.
/// (See [`AppIconStatus`])
fn set_title_and_icon(_title: &str, _icon_data: Option<&IconData>) -> AppIconStatus {
    crate::profile_function!();

    #[cfg(target_os = "windows")]
    {
        if let Some(icon_data) = _icon_data {
            return set_app_icon_windows(icon_data);
        }
    }

    #[cfg(target_os = "macos")]
    return set_title_and_icon_mac(_title, _icon_data);

    #[allow(unreachable_code)]
    AppIconStatus::NotSetIgnored
}

/// Set icon for Windows applications.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn set_app_icon_windows(icon_data: &IconData) -> AppIconStatus {
    use winapi::um::winuser;

    // We would get fairly far already with winit's `set_window_icon` (which is exposed to eframe) actually!
    // However, it only sets ICON_SMALL, i.e. doesn't allow us to set a higher resolution icon for the task bar.
    // Also, there is scaling issues, detailed below.

    // TODO(andreas): This does not set the task bar icon for when our application is started from python.
    //      Things tried so far:
    //      * Querying for an owning window and setting icon there (there doesn't seem to be an owning window)
    //      * using undocumented SetConsoleIcon method (successfully queried via GetProcAddress)

    // SAFETY: WinApi function without side-effects.
    let window_handle = unsafe { winuser::GetActiveWindow() };
    if window_handle.is_null() {
        // The Window isn't available yet. Try again later!
        return AppIconStatus::NotSetTryAgain;
    }

    fn create_hicon_with_scale(
        unscaled_image: &image::RgbaImage,
        target_size: i32,
    ) -> winapi::shared::windef::HICON {
        let image_scaled = image::imageops::resize(
            unscaled_image,
            target_size as _,
            target_size as _,
            image::imageops::Lanczos3,
        );

        // Creating transparent icons with WinApi is a huge mess.
        // We'd need to go through CreateIconIndirect's ICONINFO struct which then
        // takes a mask HBITMAP and a color HBITMAP and creating each of these is pain.
        // Instead we workaround this by creating a png which CreateIconFromResourceEx magically understands.
        // This is a pretty horrible hack as we spend a lot of time encoding, but at least the code is a lot shorter.
        let mut image_scaled_bytes: Vec<u8> = Vec::new();
        if image_scaled
            .write_to(
                &mut std::io::Cursor::new(&mut image_scaled_bytes),
                image::ImageOutputFormat::Png,
            )
            .is_err()
        {
            return std::ptr::null_mut();
        }

        // SAFETY: Creating an HICON which should be readonly on our data.
        unsafe {
            winuser::CreateIconFromResourceEx(
                image_scaled_bytes.as_mut_ptr(),
                image_scaled_bytes.len() as u32,
                1,           // Means this is an icon, not a cursor.
                0x00030000,  // Version number of the HICON
                target_size, // Note that this method can scale, but it does so *very* poorly. So let's avoid that!
                target_size,
                winuser::LR_DEFAULTCOLOR,
            )
        }
    }

    let unscaled_image = match icon_data.to_image() {
        Ok(unscaled_image) => unscaled_image,
        Err(err) => {
            log::warn!("Invalid icon: {err}");
            return AppIconStatus::NotSetIgnored;
        }
    };

    // Only setting ICON_BIG with the icon size for big icons (SM_CXICON) works fine
    // but the scaling it does then for the small icon is pretty bad.
    // Instead we set the correct sizes manually and take over the scaling ourselves.
    // For this to work we first need to set the big icon and then the small one.
    //
    // Note that ICON_SMALL may be used even if we don't render a title bar as it may be used in alt+tab!
    {
        // SAFETY: WinAPI getter function with no known side effects.
        let icon_size_big = unsafe { winuser::GetSystemMetrics(winuser::SM_CXICON) };
        let icon_big = create_hicon_with_scale(&unscaled_image, icon_size_big);
        if icon_big.is_null() {
            log::warn!("Failed to create HICON (for big icon) from embedded png data.");
            return AppIconStatus::NotSetIgnored; // We could try independently with the small icon but what's the point, it would look bad!
        } else {
            // SAFETY: Unsafe WinApi function, takes objects previously created with WinAPI, all checked for null prior.
            unsafe {
                winuser::SendMessageW(
                    window_handle,
                    winuser::WM_SETICON,
                    winuser::ICON_BIG as usize,
                    icon_big as isize,
                );
            }
        }
    }
    {
        // SAFETY: WinAPI getter function with no known side effects.
        let icon_size_small = unsafe { winuser::GetSystemMetrics(winuser::SM_CXSMICON) };
        let icon_small = create_hicon_with_scale(&unscaled_image, icon_size_small);
        if icon_small.is_null() {
            log::warn!("Failed to create HICON (for small icon) from embedded png data.");
            return AppIconStatus::NotSetIgnored;
        } else {
            // SAFETY: Unsafe WinApi function, takes objects previously created with WinAPI, all checked for null prior.
            unsafe {
                winuser::SendMessageW(
                    window_handle,
                    winuser::WM_SETICON,
                    winuser::ICON_SMALL as usize,
                    icon_small as isize,
                );
            }
        }
    }

    // It _probably_ worked out.
    AppIconStatus::Set
}

/// Set icon & app title for `MacOS` applications.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn set_title_and_icon_mac(title: &str, icon_data: Option<&IconData>) -> AppIconStatus {
    use cocoa::{
        appkit::{NSApp, NSApplication, NSImage, NSMenu, NSWindow},
        base::{id, nil},
        foundation::{NSData, NSString},
    };
    use objc::{msg_send, sel, sel_impl};

    let png_bytes = if let Some(icon_data) = icon_data {
        match icon_data.to_png_bytes() {
            Ok(png_bytes) => Some(png_bytes),
            Err(err) => {
                log::warn!("Failed to convert IconData to png: {err}");
                return AppIconStatus::NotSetIgnored;
            }
        }
    } else {
        None
    };

    // SAFETY: Accessing raw data from icon in a read-only manner. Icon data is static!
    unsafe {
        let app = NSApp();

        if let Some(png_bytes) = png_bytes {
            let data = NSData::dataWithBytes_length_(
                nil,
                png_bytes.as_ptr().cast::<std::ffi::c_void>(),
                png_bytes.len() as u64,
            );
            let app_icon = NSImage::initWithData_(NSImage::alloc(nil), data);
            app.setApplicationIconImage_(app_icon);
        }

        // Change the title in the top bar - for python processes this would be again "python" otherwise.
        let main_menu = app.mainMenu();
        let app_menu: id = msg_send![main_menu.itemAtIndex_(0), submenu];
        app_menu.setTitle_(NSString::alloc(nil).init_str(title));

        // The title in the Dock apparently can't be changed.
        // At least these people didn't figure it out either:
        // https://stackoverflow.com/questions/69831167/qt-change-application-title-dynamically-on-macos
        // https://stackoverflow.com/questions/28808226/changing-cocoa-app-icon-title-and-menu-labels-at-runtime
    }

    AppIconStatus::Set
}
